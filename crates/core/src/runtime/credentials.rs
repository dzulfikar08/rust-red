//! Node-RED compatible credentials.
//!
//! Node-RED keeps node credentials (broker and database passwords, HTTP basic auth, ...) out of
//! the flows file. They live in a file next to it, named `<flows basename>_cred.json`
//! (`flows_cred.json` for `flows.json`). This module reads and writes that file in the format
//! Node-RED defines, so a flow exported from Node-RED runs here with its credentials unmodified.
//!
//! # File format (credit: Node-RED, `@node-red/runtime/lib/nodes/credentials.js`)
//!
//! * Encrypted: `{"$": "<32 hex chars IV><base64 ciphertext>"}`. The cipher is AES-256-CTR, the key
//!   is the SHA-256 digest of the credential secret, and the plaintext is a JSON object mapping a
//!   node id to that node's credentials object.
//! * Plain: the JSON object itself, which Node-RED writes when its `credentialSecret` setting is
//!   `false`.
//!
//! # Secret resolution (same order as Node-RED)
//!
//! 1. The `credential_secret` setting (`--credential-secret`, `NODE_RED_CREDENTIAL_SECRET`, or the
//!    config file). `credential_secret = false` disables encryption.
//! 2. `_credentialSecret` in `.config.runtime.json` (legacy `.config.json`) in the user directory.
//!    Node-RED generates this key when no secret is configured.
//!
//! When both exist and the file does not decrypt with the first, the second is tried, which is how
//! Node-RED migrates a flow that was encrypted with its generated key to a user supplied secret.
//!
//! Secrets are never logged, and no error message contains one.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use aes::Aes256;
use base64::Engine as _;
use ctr::Ctr128BE;
use ctr::cipher::{KeyIvInit, StreamCipher};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

type Aes256Ctr = Ctr128BE<Aes256>;

/// The value Node-RED's editor sends back for a password field the user did not touch.
const UNCHANGED_PASSWORD: &str = "__PWRD__";

/// Runtime settings file names, newest first. Both are read, like Node-RED does.
const RUNTIME_CONFIG_FILES: [&str; 2] = [".config.runtime.json", ".config.json"];

/// Environment variable naming the credential secret, the same one Node-RED's docs use.
pub const CREDENTIAL_SECRET_ENV: &str = "NODE_RED_CREDENTIAL_SECRET";

/// A secret that cannot be printed by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl FromStr for SecretString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// What the `credential_secret` setting says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SecretSetting {
    /// Not configured: fall back to `_credentialSecret` from the runtime config file.
    #[default]
    Unset,
    /// `credential_secret = false`: store credentials as plain JSON.
    Disabled,
    /// A user supplied secret.
    Secret(SecretString),
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error(
        "cannot decrypt the credentials file `{0}`: the credential secret is wrong (decrypting did not \
         produce valid JSON). Use the secret the file was encrypted with: set `credential_secret` in the \
         config, pass `--credential-secret`, or set NODE_RED_CREDENTIAL_SECRET, or restore the matching \
         `_credentialSecret` in `.config.runtime.json`"
    )]
    WrongSecret(PathBuf),

    #[error(
        "the credentials file `{0}` is encrypted but no credential secret is available: set \
         `credential_secret` in the config, pass `--credential-secret`, set NODE_RED_CREDENTIAL_SECRET, or \
         put the `_credentialSecret` from the Node-RED user directory's `.config.runtime.json` next to the flows"
    )]
    MissingSecret(PathBuf),

    #[error(
        "the credentials file `{0}` is encrypted but `credential_secret` is false and no `_credentialSecret` \
         was found in `.config.runtime.json`"
    )]
    EncryptedButDisabled(PathBuf),

    #[error("malformed credentials file `{path}`: {reason}")]
    Malformed { path: PathBuf, reason: String },

    #[error("credentials file `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Where the credential secret and the user directory come from.
#[derive(Debug, Clone, Default)]
pub struct CredentialSettings {
    pub secret: SecretSetting,
    /// `--user-dir`, when given.
    pub user_dir: Option<PathBuf>,
    /// The Rust-Red home directory.
    pub home_dir: Option<PathBuf>,
}

impl CredentialSettings {
    /// Read `credential_secret` (or `credentialSecret`), `user_dir` and `home_dir` from the config.
    pub fn from_config(cfg: &config::Config) -> Self {
        let secret = ["credential_secret", "credentialSecret"]
            .iter()
            .find_map(|key| cfg.get::<config::Value>(key).ok())
            .map(|value| match value.kind {
                config::ValueKind::Boolean(false) => SecretSetting::Disabled,
                config::ValueKind::String(s) if !s.is_empty() => SecretSetting::Secret(SecretString::new(s)),
                config::ValueKind::Boolean(true) => {
                    log::warn!("`credential_secret = true` is not a secret, ignoring it. Use a string, or false.");
                    SecretSetting::Unset
                }
                _ => SecretSetting::Unset,
            })
            .unwrap_or_default();
        Self {
            secret,
            user_dir: cfg.get_string("user_dir").ok().map(PathBuf::from),
            home_dir: cfg.get_string("home_dir").ok().map(PathBuf::from),
        }
    }
}

/// The credentials file that goes with one flows file.
#[derive(Debug, Clone)]
pub struct CredentialStore {
    path: PathBuf,
    secret: SecretSetting,
    /// Directories searched for `.config.runtime.json`, most specific first. The first one is
    /// where a newly generated secret is written.
    config_dirs: Vec<PathBuf>,
}

/// `<dir>/<flows basename>_cred.<ext>`, for example `flows.json` becomes `flows_cred.json`.
pub fn credentials_path_for(flows_path: &Path) -> PathBuf {
    let ext = flows_path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let stem = flows_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "flows".to_owned());
    flows_path.with_file_name(format!("{stem}_cred{ext}"))
}

impl CredentialStore {
    pub fn for_flows_file(flows_path: &Path, settings: &CredentialSettings) -> Self {
        let flows_dir = flows_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_owned();
        let mut config_dirs = Vec::new();
        // An explicit --user-dir is the user directory, as in Node-RED. Otherwise the directory the
        // flows live in is, which is the Node-RED default layout.
        let ordered = if settings.user_dir.is_some() {
            [settings.user_dir.clone(), Some(flows_dir), settings.home_dir.clone()]
        } else {
            [Some(flows_dir), settings.home_dir.clone(), None]
        };
        for dir in ordered.into_iter().flatten() {
            if !config_dirs.contains(&dir) {
                config_dirs.push(dir);
            }
        }
        Self { path: credentials_path_for(flows_path), secret: settings.secret.clone(), config_dirs }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `_credentialSecret` from the runtime config file, and the directory it was found in.
    fn runtime_secret(&self) -> Option<(PathBuf, SecretString)> {
        for dir in &self.config_dirs {
            for name in RUNTIME_CONFIG_FILES {
                let Ok(text) = std::fs::read_to_string(dir.join(name)) else { continue };
                let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&text) else { continue };
                if let Some(secret) = obj.get("_credentialSecret").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                    return Some((dir.clone(), SecretString::new(secret)));
                }
            }
        }
        None
    }

    fn io_err(&self, source: std::io::Error) -> CredentialError {
        CredentialError::Io { path: self.path.clone(), source }
    }

    /// Read the credentials, node id to credentials object. A missing file is no credentials.
    pub fn load(&self) -> Result<Map<String, Value>, CredentialError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
            Err(e) => return Err(self.io_err(e)),
        };
        if text.trim().is_empty() {
            return Ok(Map::new());
        }
        let root: Value = serde_json::from_str(&text)
            .map_err(|e| CredentialError::Malformed { path: self.path.clone(), reason: e.to_string() })?;
        let Value::Object(root) = root else {
            return Err(CredentialError::Malformed {
                path: self.path.clone(),
                reason: "expected a JSON object".to_owned(),
            });
        };

        let creds = match root.get("$").and_then(Value::as_str).filter(|_| root.len() == 1) {
            Some(encrypted) => self.decrypt_with_candidates(encrypted)?,
            None => root,
        };
        Ok(creds.into_iter().filter(|(_, v)| v.is_object()).collect())
    }

    fn decrypt_with_candidates(&self, encrypted: &str) -> Result<Map<String, Value>, CredentialError> {
        let runtime = self.runtime_secret().map(|(_, s)| s);
        let mut candidates: Vec<&SecretString> = Vec::new();
        if let SecretSetting::Secret(s) = &self.secret {
            candidates.push(s);
        }
        if let Some(s) = runtime.as_ref()
            && !candidates.contains(&s)
        {
            candidates.push(s);
        }
        if candidates.is_empty() {
            return Err(match self.secret {
                SecretSetting::Disabled => CredentialError::EncryptedButDisabled(self.path.clone()),
                _ => CredentialError::MissingSecret(self.path.clone()),
            });
        }
        for secret in candidates {
            if let Some(creds) = decrypt_credentials(secret.expose(), encrypted) {
                return Ok(creds);
            }
        }
        Err(CredentialError::WrongSecret(self.path.clone()))
    }

    /// Write the credentials in Node-RED's format: encrypted unless `credential_secret = false`.
    /// Nothing is written when there are no credentials and no file yet. When no secret exists
    /// anywhere, a random one is generated and stored as `_credentialSecret`, as Node-RED does.
    pub fn save(&self, creds: &Map<String, Value>) -> Result<(), CredentialError> {
        if creds.is_empty() && !self.path.exists() {
            return Ok(());
        }
        let plaintext = serde_json::to_string(creds).expect("a JSON map serializes");
        let content = match &self.secret {
            SecretSetting::Disabled => serde_json::to_string_pretty(creds).expect("a JSON map serializes"),
            SecretSetting::Secret(secret) => encrypted_document(secret.expose(), &plaintext),
            SecretSetting::Unset => {
                let secret = match self.runtime_secret() {
                    Some((_, secret)) => secret,
                    None => self.generate_runtime_secret()?,
                };
                encrypted_document(secret.expose(), &plaintext)
            }
        };
        write_private_file(&self.path, content.as_bytes()).map_err(|e| self.io_err(e))
    }

    fn generate_runtime_secret(&self) -> Result<SecretString, CredentialError> {
        let bytes: [u8; 32] = rand::random();
        let secret = SecretString::new(hex::encode(bytes));
        let dir = self.config_dirs.first().cloned().unwrap_or_else(|| PathBuf::from("."));
        let file = dir.join(RUNTIME_CONFIG_FILES[0]);
        let mut obj = match std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            Some(Value::Object(obj)) => obj,
            _ => Map::new(),
        };
        obj.insert("_credentialSecret".to_owned(), Value::String(secret.expose().to_owned()));
        let text = serde_json::to_string_pretty(&obj).expect("a JSON map serializes");
        write_private_file(&file, text.as_bytes())
            .map_err(|source| CredentialError::Io { path: file.clone(), source })?;
        log::info!("Generated a credential secret in `{}`", file.display());
        Ok(secret)
    }

    /// Merge the stored credentials into the `credentials` property of each node, before the nodes
    /// are built. See [`apply_credentials`].
    pub fn merge_into_flows(&self, flows: &mut Value) -> Result<(), CredentialError> {
        let creds = self.load()?;
        if !creds.is_empty() {
            log::info!("Loaded credentials for {} node(s) from `{}`", creds.len(), self.path.display());
        }
        apply_credentials(flows, &creds);
        Ok(())
    }

    /// Prepare a deploy the way Node-RED does: take the `credentials` the editor sent out of the
    /// nodes, merge them into the stored ones, drop the credentials of nodes that no longer exist,
    /// and save the file when something changed. `flows` leaves without any `credentials`, ready
    /// to be written to the flows file.
    ///
    /// `current_flows` is what the flows file holds now. Credentials written inline there by hand
    /// (the pre-credentials-file way, or a template) are kept: they move to the credentials file
    /// the first time flows are deployed. Fails, leaving every file alone, when the stored
    /// credentials cannot be read, so a wrong secret never overwrites them.
    pub fn extract_for_deploy(&self, flows: &mut [Value], current_flows: &[Value]) -> Result<(), CredentialError> {
        let stored = self.load()?;
        let mut creds = stored.clone();
        for node in current_flows {
            let (Some(id), Some(Value::Object(inline))) =
                (node.get("id").and_then(Value::as_str), node.get("credentials"))
            else {
                continue;
            };
            let entry = creds.entry(id.to_owned()).or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(entry) = entry {
                overlay_inline(entry, inline);
            }
        }
        creds.retain(|_, v| v.as_object().is_some_and(|o| !o.is_empty()));

        extract_credentials(flows, &mut creds);

        if creds != stored {
            self.save(&creds)?;
            log::info!("Saved credentials for {} node(s) to `{}`", creds.len(), self.path.display());
        }
        Ok(())
    }
}

/// `inline` values win over `stored` ones, except an empty or null inline value, which is a
/// placeholder and never replaces a stored credential.
fn overlay_inline(stored: &mut Map<String, Value>, inline: &Map<String, Value>) {
    for (key, value) in inline {
        let placeholder = match value {
            Value::Null => true,
            Value::String(s) => s.is_empty() || s == UNCHANGED_PASSWORD,
            _ => false,
        };
        if !placeholder {
            stored.insert(key.clone(), value.clone());
        }
    }
}

/// Set each node's `credentials` to the stored object for its id, with any inline `credentials`
/// the flows file already carries layered on top (see `overlay_inline`).
pub fn apply_credentials(flows: &mut Value, creds: &Map<String, Value>) {
    let Some(nodes) = flows.as_array_mut() else { return };
    for node in nodes {
        let Some(obj) = node.as_object_mut() else { continue };
        let Some(Value::Object(stored)) = obj.get("id").and_then(Value::as_str).and_then(|id| creds.get(id)) else {
            continue;
        };
        let mut merged = stored.clone();
        if let Some(Value::Object(inline)) = obj.get("credentials") {
            overlay_inline(&mut merged, inline);
        }
        obj.insert("credentials".to_owned(), Value::Object(merged));
    }
}

/// Node-RED's `credentials.extract` and `credentials.clean`: move `credentials` out of the nodes
/// into `creds`. `__PWRD__` (an untouched password) keeps the stored value, an empty string or
/// null deletes it, anything else replaces it. Credentials of ids not in `flows` are dropped.
pub fn extract_credentials(flows: &mut [Value], creds: &mut Map<String, Value>) {
    let mut ids = std::collections::HashSet::new();
    for node in flows.iter_mut() {
        let Some(obj) = node.as_object_mut() else { continue };
        let Some(id) = obj.get("id").and_then(Value::as_str).map(str::to_owned) else { continue };
        ids.insert(id.clone());
        let Some(Value::Object(sent)) = obj.remove("credentials") else { continue };
        let entry = creds.entry(id.clone()).or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(entry) = entry {
            for (key, value) in sent {
                match value {
                    Value::String(s) if s == UNCHANGED_PASSWORD => {}
                    Value::Null => {
                        entry.remove(&key);
                    }
                    Value::String(s) if s.trim().is_empty() => {
                        entry.remove(&key);
                    }
                    other => {
                        entry.insert(key, other);
                    }
                }
            }
            if entry.is_empty() {
                creds.remove(&id);
            }
        }
    }
    creds.retain(|id, _| ids.contains(id));
}

/// Names whose value the credentials API never returns, only `has_<name>: true`, like
/// Node-RED does for `password` type fields. The server has no node definitions to ask, so the
/// conventional names are used.
const SECRET_FIELDS: [&str; 7] = ["password", "passphrase", "token", "secret", "apikey", "clientsecret", "accesstoken"];

/// The shape `GET /credentials/<type>/<id>` returns for the editor.
pub fn editor_view(creds: &Map<String, Value>) -> Value {
    let mut out = Map::new();
    for (key, value) in creds {
        if SECRET_FIELDS.contains(&key.to_ascii_lowercase().as_str()) {
            let present = !matches!(value, Value::Null) && value.as_str() != Some("");
            if present {
                out.insert(format!("has_{key}"), Value::Bool(true));
            }
        } else {
            out.insert(key.clone(), value.clone());
        }
    }
    Value::Object(out)
}

/// Remove every node's `credentials`, as Node-RED's `GET /flows` never returns them.
pub fn strip_credentials(flows: &mut [Value]) {
    for node in flows {
        if let Some(obj) = node.as_object_mut() {
            obj.remove("credentials");
        }
    }
}

fn derive_key(secret: &str) -> [u8; 32] {
    Sha256::digest(secret.as_bytes()).into()
}

/// Decrypt a `$` value. `None` when the secret is wrong or the data is not what Node-RED wrote:
/// decrypting with a wrong key yields garbage, so valid JSON is the only proof of a right key.
fn decrypt_credentials(secret: &str, encrypted: &str) -> Option<Map<String, Value>> {
    let iv_hex = encrypted.get(..32)?;
    let iv: [u8; 16] = hex::decode(iv_hex).ok()?.try_into().ok()?;
    let mut data = base64::engine::general_purpose::STANDARD.decode(encrypted.get(32..)?.trim()).ok()?;
    Aes256Ctr::new(&derive_key(secret).into(), &iv.into()).apply_keystream(&mut data);
    match serde_json::from_slice::<Value>(&data).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn encrypt_with_iv(secret: &str, plaintext: &str, iv: [u8; 16]) -> String {
    let mut data = plaintext.as_bytes().to_vec();
    Aes256Ctr::new(&derive_key(secret).into(), &iv.into()).apply_keystream(&mut data);
    format!("{}{}", hex::encode(iv), base64::engine::general_purpose::STANDARD.encode(data))
}

fn encrypted_document(secret: &str, plaintext: &str) -> String {
    let iv: [u8; 16] = rand::random();
    let doc = serde_json::json!({ "$": encrypt_with_iv(secret, plaintext, iv) });
    serde_json::to_string_pretty(&doc).expect("a JSON value serializes")
}

/// Write through a temporary file so a crash never leaves a half written credentials file, readable
/// by the owner only.
fn write_private_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(content)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/node_red_credentials");
    const USER_SECRET: &str = "k\u{e9}y secret \u{2713} for RustRED";
    const BROKER: &str = "4c8a1f6e3b7d0925";
    const POSTGRES: &str = "a81d5f3c9e2b6047";
    const HTTP: &str = "0b5e9c2d7a4f1836";

    fn copy_fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        for file in ["flows.json", "flows_cred.json", ".config.runtime.json"] {
            std::fs::copy(Path::new(DIR).join(name).join(file), tmp.path().join(file)).unwrap();
        }
        let flows = tmp.path().join("flows.json");
        (tmp, flows)
    }

    fn store(flows: &Path, secret: SecretSetting) -> CredentialStore {
        CredentialStore::for_flows_file(flows, &CredentialSettings { secret, ..Default::default() })
    }

    fn expected() -> Map<String, Value> {
        match json!({
            BROKER: {"user": "plant", "password": "p\u{e4}ss/w\u{f6}rd\"1\\x"},
            POSTGRES: {"user": "scada", "password": "pg-secret-1"},
            HTTP: {"user": "api-user", "password": "api-pass"},
        }) {
            Value::Object(m) => m,
            _ => unreachable!(),
        }
    }

    #[test]
    fn path_is_the_flows_basename_with_cred() {
        assert_eq!(credentials_path_for(Path::new("/a/flows.json")), Path::new("/a/flows_cred.json"));
        assert_eq!(credentials_path_for(Path::new("plant.json")), Path::new("plant_cred.json"));
        assert_eq!(credentials_path_for(Path::new("/a/flows")), Path::new("/a/flows_cred"));
    }

    #[test]
    fn reads_a_file_node_red_encrypted_with_its_generated_secret() {
        let (_tmp, flows) = copy_fixture("default-secret");
        assert_eq!(store(&flows, SecretSetting::Unset).load().unwrap(), expected());
    }

    #[test]
    fn reads_a_file_node_red_encrypted_with_credential_secret() {
        let (_tmp, flows) = copy_fixture("user-secret");
        let secret = SecretSetting::Secret(SecretString::new(USER_SECRET));
        assert_eq!(store(&flows, secret).load().unwrap(), expected());
    }

    #[test]
    fn reads_a_plain_file_when_encryption_is_disabled() {
        let (_tmp, flows) = copy_fixture("plaintext");
        assert_eq!(store(&flows, SecretSetting::Disabled).load().unwrap(), expected());
        assert_eq!(store(&flows, SecretSetting::Unset).load().unwrap(), expected());
    }

    #[test]
    fn wrong_secret_is_a_clear_error_and_leaks_nothing() {
        let (_tmp, flows) = copy_fixture("user-secret");
        let err = store(&flows, SecretSetting::Secret(SecretString::new("not the secret"))).load().unwrap_err();
        assert!(matches!(err, CredentialError::WrongSecret(_)), "{err:?}");
        let text = err.to_string();
        assert!(text.contains("credential secret is wrong") && text.contains("flows_cred.json"));
        assert!(!text.contains("not the secret"));
    }

    #[test]
    fn encrypted_file_without_any_secret_is_a_clear_error() {
        let (tmp, flows) = copy_fixture("user-secret");
        // The fixture's runtime config has no `_credentialSecret`.
        let err = store(&flows, SecretSetting::Unset).load().unwrap_err();
        assert!(matches!(err, CredentialError::MissingSecret(_)), "{err:?}");
        let err = store(&flows, SecretSetting::Disabled).load().unwrap_err();
        assert!(matches!(err, CredentialError::EncryptedButDisabled(_)), "{err:?}");
        drop(tmp);
    }

    #[test]
    fn configured_secret_falls_back_to_the_runtime_secret() {
        // Node-RED's migration case: encrypted with the generated key, then `credentialSecret` added.
        let (_tmp, flows) = copy_fixture("default-secret");
        let secret = SecretSetting::Secret(SecretString::new("a newer secret"));
        assert_eq!(store(&flows, secret).load().unwrap(), expected());
    }

    #[test]
    fn encryption_round_trips_and_matches_the_node_red_layout() {
        let iv = [7u8; 16];
        let encrypted = encrypt_with_iv("s", r#"{"a":{"b":"c"}}"#, iv);
        assert!(encrypted.starts_with(&hex::encode(iv)));
        assert_eq!(decrypt_credentials("s", &encrypted).unwrap(), expected_small());
        assert!(decrypt_credentials("other", &encrypted).is_none());
    }

    fn expected_small() -> Map<String, Value> {
        match json!({"a": {"b": "c"}}) {
            Value::Object(m) => m,
            _ => unreachable!(),
        }
    }

    #[test]
    fn save_writes_what_node_red_can_read_back() {
        // Node-RED itself reads these files in the end-to-end check; here the layout is verified:
        // `{"$": "<32 hex><base64>"}` and nothing else.
        let tmp = tempfile::tempdir().unwrap();
        let flows = tmp.path().join("flows.json");
        let secret = SecretSetting::Secret(SecretString::new(USER_SECRET));
        let st = store(&flows, secret);
        st.save(&expected()).unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(st.path()).unwrap()).unwrap();
        let obj = doc.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        let encoded = obj["$"].as_str().unwrap();
        assert!(encoded[..32].chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(st.load().unwrap(), expected());
        assert!(!std::fs::read_to_string(st.path()).unwrap().contains("pg-secret-1"));
    }

    #[test]
    fn save_without_a_secret_generates_one_like_node_red() {
        let tmp = tempfile::tempdir().unwrap();
        let flows = tmp.path().join("flows.json");
        let st = store(&flows, SecretSetting::Unset);
        st.save(&expected()).unwrap();
        let runtime: Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join(".config.runtime.json")).unwrap()).unwrap();
        let generated = runtime["_credentialSecret"].as_str().unwrap();
        assert_eq!(generated.len(), 64);
        assert_eq!(st.load().unwrap(), expected());
        // A second store, as after a restart, reads it back with the same generated secret.
        assert_eq!(store(&flows, SecretSetting::Unset).load().unwrap(), expected());
        // Existing keys in the runtime file are kept when the secret is added.
        std::fs::write(tmp.path().join(".config.runtime.json"), r#"{"instanceId":"x"}"#).unwrap();
        std::fs::remove_file(st.path()).unwrap();
        st.save(&expected()).unwrap();
        let runtime: Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join(".config.runtime.json")).unwrap()).unwrap();
        assert_eq!(runtime["instanceId"], "x");
        assert!(runtime["_credentialSecret"].is_string());
    }

    #[test]
    fn save_with_encryption_disabled_writes_plain_json() {
        let tmp = tempfile::tempdir().unwrap();
        let flows = tmp.path().join("flows.json");
        let st = store(&flows, SecretSetting::Disabled);
        st.save(&expected()).unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(st.path()).unwrap()).unwrap();
        assert_eq!(doc[POSTGRES]["password"], "pg-secret-1");
        assert!(!tmp.path().join(".config.runtime.json").exists());
    }

    #[test]
    fn saving_nothing_creates_no_files() {
        let tmp = tempfile::tempdir().unwrap();
        let flows = tmp.path().join("flows.json");
        store(&flows, SecretSetting::Unset).save(&Map::new()).unwrap();
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    #[test]
    fn merge_sets_credentials_before_node_construction() {
        let (_tmp, flows_path) = copy_fixture("default-secret");
        let mut flows: Value = serde_json::from_str(&std::fs::read_to_string(&flows_path).unwrap()).unwrap();
        store(&flows_path, SecretSetting::Unset).merge_into_flows(&mut flows).unwrap();
        let node = |id: &str| flows.as_array().unwrap().iter().find(|n| n["id"] == id).unwrap().clone();
        assert_eq!(node(BROKER)["credentials"]["user"], "plant");
        assert_eq!(node(POSTGRES)["credentials"]["password"], "pg-secret-1");
        assert_eq!(node(HTTP)["credentials"]["user"], "api-user");
        assert!(node("b3f09a6d1c7e4825").get("credentials").is_none());
    }

    #[test]
    fn inline_credentials_win_per_key_but_empty_placeholders_do_not() {
        let creds = expected();
        let mut flows = json!([
            {"id": BROKER, "type": "mqtt-broker", "credentials": {"password": "inline-pw"}},
            {"id": POSTGRES, "type": "postgreSQLConfig", "credentials": {"user": "", "password": null}},
            {"id": "other", "type": "mqtt-broker", "credentials": {"user": "kept"}},
        ]);
        apply_credentials(&mut flows, &creds);
        assert_eq!(flows[0]["credentials"], json!({"user": "plant", "password": "inline-pw"}));
        assert_eq!(flows[1]["credentials"], json!({"user": "scada", "password": "pg-secret-1"}));
        assert_eq!(flows[2]["credentials"], json!({"user": "kept"}));
    }

    #[test]
    fn extract_follows_node_red_rules() {
        let mut creds = expected();
        let mut flows = vec![
            json!({"id": BROKER, "credentials": {"user": "new-user", "password": "__PWRD__"}}),
            json!({"id": POSTGRES, "credentials": {"user": "", "password": "changed"}}),
            json!({"id": HTTP, "type": "http request"}),
            json!({"id": "fresh", "credentials": {"user": "u"}}),
        ];
        extract_credentials(&mut flows, &mut creds);
        assert!(flows.iter().all(|n| n.get("credentials").is_none()));
        assert_eq!(creds[BROKER], json!({"user": "new-user", "password": expected()[BROKER]["password"]}));
        assert_eq!(creds[POSTGRES], json!({"password": "changed"}));
        assert_eq!(creds[HTTP], expected()[HTTP]);
        assert_eq!(creds["fresh"], json!({"user": "u"}));
    }

    #[test]
    fn extract_drops_credentials_of_deleted_nodes() {
        let mut creds = expected();
        let mut flows = vec![json!({"id": BROKER, "type": "mqtt-broker"})];
        extract_credentials(&mut flows, &mut creds);
        assert_eq!(creds.keys().collect::<Vec<_>>(), vec![BROKER]);
    }

    #[test]
    fn deploy_saves_changes_encrypted_and_migrates_inline_credentials() {
        let (tmp, flows_path) = copy_fixture("user-secret");
        let secret = SecretSetting::Secret(SecretString::new(USER_SECRET));
        let st = store(&flows_path, secret.clone());
        let before = std::fs::read_to_string(st.path()).unwrap();

        // Nothing changed: the file is left alone.
        let mut flows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&flows_path).unwrap()).unwrap();
        st.extract_for_deploy(&mut flows, &[]).unwrap();
        assert_eq!(std::fs::read_to_string(st.path()).unwrap(), before);

        // The editor changes one password and leaves the other untouched.
        let mut flows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&flows_path).unwrap()).unwrap();
        for node in flows.iter_mut() {
            if node["id"] == POSTGRES {
                node["credentials"] = json!({"user": "scada", "password": "rotated"});
            }
            if node["id"] == BROKER {
                node["credentials"] = json!({"password": "__PWRD__"});
            }
        }
        // And the previous flows file had a hand written inline credential for another node.
        let current = vec![json!({"id": "legacy", "credentials": {"user": "old", "password": "old-pw"}})];
        flows.push(json!({"id": "legacy", "type": "mqtt-broker"}));
        st.extract_for_deploy(&mut flows, &current).unwrap();

        let saved = store(&flows_path, secret).load().unwrap();
        assert_eq!(saved[POSTGRES]["password"], "rotated");
        assert_eq!(saved[BROKER], expected()[BROKER]);
        assert_eq!(saved["legacy"], json!({"user": "old", "password": "old-pw"}));
        let text = std::fs::read_to_string(st.path()).unwrap();
        assert!(text.contains("\"$\"") && !text.contains("rotated"));
        drop(tmp);
    }

    #[test]
    fn deploy_with_a_wrong_secret_fails_and_keeps_the_file() {
        let (_tmp, flows_path) = copy_fixture("user-secret");
        let st = store(&flows_path, SecretSetting::Secret(SecretString::new("wrong")));
        let before = std::fs::read_to_string(st.path()).unwrap();
        let mut flows = vec![json!({"id": BROKER, "credentials": {"password": "x"}})];
        assert!(st.extract_for_deploy(&mut flows, &[]).is_err());
        assert_eq!(std::fs::read_to_string(st.path()).unwrap(), before);
    }

    #[test]
    fn editor_view_hides_secrets() {
        let view = editor_view(expected()[BROKER].as_object().unwrap());
        assert_eq!(view, json!({"user": "plant", "has_password": true}));
    }

    #[test]
    fn settings_read_the_config() {
        let build = |k: &str, v: config::Value| {
            let _ = k;
            config::Config::builder().set_override("credential_secret", v).unwrap().build().unwrap()
        };
        assert_eq!(
            CredentialSettings::from_config(&build("s", "hunter2".into())).secret,
            SecretSetting::Secret(SecretString::new("hunter2"))
        );
        assert_eq!(CredentialSettings::from_config(&build("s", false.into())).secret, SecretSetting::Disabled);
        assert_eq!(CredentialSettings::from_config(&config::Config::default()).secret, SecretSetting::Unset);
        let camel = config::Config::builder().set_override("credentialSecret", "abc").unwrap().build().unwrap();
        assert_eq!(CredentialSettings::from_config(&camel).secret, SecretSetting::Secret(SecretString::new("abc")));
    }

    #[test]
    fn secrets_are_not_debug_printed() {
        let settings =
            CredentialSettings { secret: SecretSetting::Secret(SecretString::new("hunter2")), ..Default::default() };
        assert!(!format!("{settings:?}").contains("hunter2"));
    }
}
