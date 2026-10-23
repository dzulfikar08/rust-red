//! Credentials written by a real Node-RED (`flows_cred.json`) reach the nodes that need them.
//!
//! The three fixture directories were produced by Node-RED 4.1.10 through its admin API from the
//! same flow (an `mqtt-broker`, a `postgreSQLConfig` and an `http request` node with credentials):
//! one with the key Node-RED generates itself (`.config.runtime.json`), one with a user supplied
//! `credentialSecret`, one with `credentialSecret: false`.

use std::path::{Path, PathBuf};

use rust_red_core::runtime::engine::Engine;
use rust_red_core::runtime::model::json::deser::parse_red_id_str;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/node_red_credentials");
const USER_SECRET: &str = "k\u{e9}y secret \u{2713} for RustRED";
const BROKER: &str = "4c8a1f6e3b7d0925";
const HTTP_REQUEST: &str = "0b5e9c2d7a4f1836";
const BROKER_PASSWORD: &str = "p\u{e4}ss/w\u{f6}rd\"1\\x";

/// A copy of one fixture directory, so a test can never touch the checked in files.
fn copy_fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    for file in ["flows.json", "flows_cred.json", ".config.runtime.json"] {
        std::fs::copy(Path::new(DIR).join(name).join(file), tmp.path().join(file)).unwrap();
    }
    let flows = tmp.path().join("flows.json");
    (tmp, flows)
}

fn config(credential_secret: Option<config::Value>) -> config::Config {
    let mut builder = config::Config::builder()
        .set_override("runtime.context.default", "memory")
        .unwrap()
        .set_override("runtime.context.stores.memory.provider", "memory")
        .unwrap();
    if let Some(secret) = credential_secret {
        builder = builder.set_override("credential_secret", secret).unwrap();
    }
    builder.build().unwrap()
}

async fn load(flows: &Path, cfg: config::Config) -> rust_red_core::Result<Engine> {
    let registry = rust_red_core::runtime::registry::RegistryBuilder::default().build().unwrap();
    Engine::with_flows_file(&registry, flows.to_str().unwrap(), Some(cfg)).await
}

fn assert_broker_has_credentials(engine: &Engine) {
    let broker = engine.find_global_node_by_ref(BROKER).expect("the mqtt-broker config node is loaded");
    let settings = broker.mqtt_settings().expect("it is an MQTT broker config");
    assert_eq!(settings.username.as_deref(), Some("plant"));
    assert_eq!(settings.password.as_deref(), Some(BROKER_PASSWORD));
    assert_eq!((settings.host.as_str(), settings.port), ("127.0.0.1", 1883));
    // The flow's other nodes are built with their config, too.
    let http = engine.find_flow_node_by_id(&parse_red_id_str(HTTP_REQUEST).unwrap()).expect("http request is loaded");
    assert_eq!(http.type_str(), "http request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flow_encrypted_with_the_key_node_red_generated() {
    let (_tmp, flows) = copy_fixture("default-secret");
    let engine = load(&flows, config(None)).await.expect("the flow loads with the runtime config secret");
    assert_broker_has_credentials(&engine);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flow_encrypted_with_credential_secret() {
    let (_tmp, flows) = copy_fixture("user-secret");
    let engine = load(&flows, config(Some(USER_SECRET.into()))).await.expect("the flow loads with the secret");
    assert_broker_has_credentials(&engine);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flow_with_plain_credentials_when_encryption_is_disabled() {
    let (_tmp, flows) = copy_fixture("plaintext");
    let engine = load(&flows, config(Some(false.into()))).await.expect("the flow loads");
    assert_broker_has_credentials(&engine);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_secret_stops_the_load_with_a_clear_error() {
    let (_tmp, flows) = copy_fixture("user-secret");
    let err = load(&flows, config(Some("not the secret".into()))).await.err().expect("a wrong secret must fail");
    let text = format!("{err:#}");
    assert!(text.contains("credential secret is wrong"), "{text}");
    assert!(text.contains("flows_cred.json"), "{text}");
    assert!(!text.contains("not the secret"), "the secret must not be in the error: {text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn encrypted_flow_without_any_secret_says_so() {
    let (_tmp, flows) = copy_fixture("user-secret");
    let err = load(&flows, config(None)).await.err().expect("no secret must fail");
    assert!(format!("{err:#}").contains("no credential secret is available"), "{err:#}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inline_credentials_in_the_flows_file_still_work_and_win_per_key() {
    let (tmp, flows) = copy_fixture("plaintext");
    // Delete the credentials file: only what the flows file holds is left.
    std::fs::remove_file(tmp.path().join("flows_cred.json")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&flows).unwrap()).unwrap();
    let broker = json.as_array_mut().unwrap().iter_mut().find(|n| n["id"] == BROKER).unwrap();
    broker["credentials"] = serde_json::json!({"user": "inline-user", "password": "inline-pw"});
    std::fs::write(&flows, serde_json::to_string(&json).unwrap()).unwrap();

    let engine = load(&flows, config(Some(false.into()))).await.unwrap();
    let settings = engine.find_global_node_by_ref(BROKER).unwrap().mqtt_settings().unwrap();
    assert_eq!(settings.username.as_deref(), Some("inline-user"));
    assert_eq!(settings.password.as_deref(), Some("inline-pw"));

    // And with a credentials file next to it, the inline password still wins, the file adds the user.
    let (_tmp2, flows2) = copy_fixture("plaintext");
    let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&flows2).unwrap()).unwrap();
    let broker = json.as_array_mut().unwrap().iter_mut().find(|n| n["id"] == BROKER).unwrap();
    broker["credentials"] = serde_json::json!({"password": "inline-pw"});
    std::fs::write(&flows2, serde_json::to_string(&json).unwrap()).unwrap();
    let engine = load(&flows2, config(Some(false.into()))).await.unwrap();
    let settings = engine.find_global_node_by_ref(BROKER).unwrap().mqtt_settings().unwrap();
    assert_eq!(settings.username.as_deref(), Some("plant"));
    assert_eq!(settings.password.as_deref(), Some("inline-pw"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flows_without_a_credentials_file_load_as_before() {
    let (tmp, flows) = copy_fixture("plaintext");
    std::fs::remove_file(tmp.path().join("flows_cred.json")).unwrap();
    let engine = load(&flows, config(None)).await.unwrap();
    let settings = engine.find_global_node_by_ref(BROKER).unwrap().mqtt_settings().unwrap();
    assert_eq!(settings.username, None);
}
