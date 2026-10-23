//! PostgreSQL connection config node.
//!
//! Registered as `postgres-config`, and also answers to `postgreSQLConfig`, the config node of
//! `node-red-contrib-postgresql`. That node stores `database` (not `dbname`), `max`,
//! `connectionTimeout` and `applicationName`, a `*FieldType` next to each value, and the user and
//! password in `credentials` (or, in older flows, as plain `user` and `password`). All of these are read.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use deadpool_postgres::{Config, Pool};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio_postgres::NoTls;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

/// How the server connection is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PostgresTls {
    /// Plain TCP.
    Off,
    /// TLS, server certificate verified against the system roots (`ssl: true` in node-postgres).
    Verify,
    /// TLS without certificate verification (`ssl: {"rejectUnauthorized": false}`).
    NoVerify,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PostgresConfig {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) dbname: String,
    pub(crate) user: String,
    pub(crate) password: String,
    pub(crate) pool_max_size: usize,
    pub(crate) connect_timeout_ms: u64,
    pub(crate) tls: PostgresTls,
    pub(crate) idle_timeout_ms: Option<u64>,
    pub(crate) application_name: Option<String>,
}

/// Read a property whose value may come from the environment (`*FieldType` of `env`),
/// or be typed (`num`, `bool`, `json`) as the contrib config node allows.
fn field(rest: &Value, key: &str, type_key: &str) -> crate::Result<Option<Value>> {
    let Some(value) = rest.get(key).filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let kind = rest.get(type_key).and_then(Value::as_str).unwrap_or("");
    let text = || value.as_str().map(str::to_string);
    match kind {
        "env" => {
            let name = text().ok_or_else(|| anyhow::anyhow!("'{key}' must name an environment variable"))?;
            Ok(std::env::var(&name).ok().map(Value::String))
        }
        "global" | "flow" => Err(anyhow::anyhow!("'{key}' reads from {kind} context, which a config node cannot do")),
        "num" => match value {
            Value::String(s) if s.trim().is_empty() => Ok(None),
            Value::String(s) => Ok(Some(
                s.trim()
                    .parse::<f64>()
                    .map(Value::from)
                    .map_err(|_| anyhow::anyhow!("'{key}' is not a number: '{s}'"))?,
            )),
            other => Ok(Some(other.clone())),
        },
        "bool" | "json" => match value {
            Value::String(s) if s.trim().is_empty() => Ok(None),
            Value::String(s) => Ok(Some(
                serde_json::from_str(s).map_err(|e| anyhow::anyhow!("'{key}' is not valid JSON ('{s}'): {e}"))?,
            )),
            other => Ok(Some(other.clone())),
        },
        _ => Ok(Some(value.clone())),
    }
}

fn as_u64(value: &Value, key: &str) -> crate::Result<u64> {
    match value {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0 && f.fract() == 0.0).map(|f| f as u64))
            .ok_or_else(|| anyhow::anyhow!("'{key}' must be a non-negative integer, got {n}")),
        Value::String(s) => s.trim().parse().map_err(|_| anyhow::anyhow!("'{key}' must be an integer, got '{s}'")),
        other => Err(anyhow::anyhow!("'{key}' must be an integer, got {other}")),
    }
}

fn as_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

impl PostgresConfig {
    pub(crate) fn from_json(rest: &Value) -> crate::Result<Self> {
        let host = field(rest, "host", "hostFieldType")?.map(|v| as_string(&v)).filter(|h| !h.is_empty());
        let host = host.unwrap_or_else(|| "127.0.0.1".to_string());

        let port = match field(rest, "port", "portFieldType")? {
            Some(v) => u16::try_from(as_u64(&v, "port")?).map_err(|_| anyhow::anyhow!("'port' is out of range"))?,
            None => 5432,
        };

        let dbname = field(rest, "dbname", "dbnameFieldType")?
            .or(field(rest, "database", "databaseFieldType")?)
            .map(|v| as_string(&v))
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "postgres".to_string());

        let credentials = rest.get("credentials");
        let credential = |name: &str| credentials.and_then(|c| c.get(name)).and_then(Value::as_str).map(str::to_string);

        // `userFieldType`/`passwordFieldType` of `env` read the variable named in `userEnv`/`passwordEnv`.
        let secret = |name: &str, env_key: &str| -> crate::Result<Option<String>> {
            let type_key = format!("{name}FieldType");
            if matches!(rest.get(&type_key).and_then(Value::as_str), Some("env" | "global")) {
                let var = rest.get(env_key).or_else(|| rest.get(name)).and_then(Value::as_str).unwrap_or("");
                return Ok(std::env::var(var).ok());
            }
            Ok(credential(name).or_else(|| rest.get(name).and_then(Value::as_str).map(str::to_string)))
        };

        let user = secret("user", "userEnv")?
            .filter(|u| !u.is_empty())
            .or_else(|| std::env::var("PGUSER").ok())
            .unwrap_or_else(|| "postgres".to_string());
        let password = secret("password", "passwordEnv")?.unwrap_or_default();

        let pool_max_size = match field(rest, "pool_max_size", "")?.or(field(rest, "poolMaxSize", "")?) {
            Some(v) => as_u64(&v, "poolMaxSize")? as usize,
            None => match field(rest, "max", "maxFieldType")? {
                Some(v) => as_u64(&v, "max")? as usize,
                None => 10,
            },
        }
        .max(1);

        let connect_timeout_ms = match field(rest, "connectTimeoutMs", "")? {
            Some(v) => as_u64(&v, "connectTimeoutMs")?,
            None => match field(rest, "connectionTimeout", "connectionTimeoutFieldType")? {
                Some(v) => as_u64(&v, "connectionTimeout")?,
                None => 5000,
            },
        };
        let connect_timeout_ms = if connect_timeout_ms == 0 { 5000 } else { connect_timeout_ms };

        let idle_timeout_ms = match field(rest, "idleTimeoutMs", "")? {
            Some(v) => Some(as_u64(&v, "idleTimeoutMs")?),
            None => field(rest, "idle", "idleFieldType")?.map(|v| as_u64(&v, "idle")).transpose()?,
        };

        let application_name =
            field(rest, "applicationName", "applicationNameType")?.map(|v| as_string(&v)).filter(|a| !a.is_empty());

        let tls = match field(rest, "ssl", "sslFieldType")? {
            None | Some(Value::Null) | Some(Value::Bool(false)) => PostgresTls::Off,
            Some(Value::Bool(true)) => PostgresTls::Verify,
            Some(Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
                "" | "false" | "0" | "no" | "off" => PostgresTls::Off,
                "true" | "1" | "yes" | "on" | "require" | "verify-full" | "verify-ca" => PostgresTls::Verify,
                "no-verify" => PostgresTls::NoVerify,
                other => return Err(anyhow::anyhow!("unsupported 'ssl' value '{other}'")),
            },
            Some(Value::Object(options)) => {
                if options.get("rejectUnauthorized").and_then(Value::as_bool) == Some(false) {
                    PostgresTls::NoVerify
                } else {
                    PostgresTls::Verify
                }
            }
            Some(other) => return Err(anyhow::anyhow!("unsupported 'ssl' value {other}")),
        };

        Ok(Self {
            host,
            port,
            dbname,
            user,
            password,
            pool_max_size,
            connect_timeout_ms,
            tls,
            idle_timeout_ms,
            application_name,
        })
    }
}

#[derive(Debug)]
#[global_node("postgres-config", red_name = "postgres-config", module = "rust-red")]
pub(crate) struct PostgresConfigNode {
    base: BaseGlobalNodeState,
    config: PostgresConfig,
    pool: Arc<RwLock<Option<Pool>>>,
}

impl PostgresConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let pg_config = PostgresConfig::from_json(&config.rest)?;
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "postgres-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(PostgresConfigNode { base: state, config: pg_config, pool: Arc::new(RwLock::new(None)) }))
    }

    fn build_pool(&self) -> crate::Result<Pool> {
        let mut cfg = Config::new();
        cfg.host = Some(self.config.host.clone());
        cfg.port = Some(self.config.port);
        cfg.dbname = Some(self.config.dbname.clone());
        cfg.user = Some(self.config.user.clone());
        cfg.password = Some(self.config.password.clone());
        cfg.application_name = self.config.application_name.clone();
        cfg.connect_timeout = Some(Duration::from_millis(self.config.connect_timeout_ms));
        cfg.manager =
            Some(deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast });

        let max_size = self.config.pool_max_size;
        let pool = match self.config.tls {
            PostgresTls::Off => cfg.builder(NoTls).map_err(|e| anyhow::anyhow!("Pool config error: {e}"))?,
            tls => cfg
                .builder(super::postgres_tls::connector(tls == PostgresTls::NoVerify)?)
                .map_err(|e| anyhow::anyhow!("Pool config error: {e}"))?,
        }
        .max_size(max_size)
        .build()
        .map_err(|e| anyhow::anyhow!("Pool build error: {e}"))?;
        Ok(pool)
    }

    pub async fn get_pool(&self) -> crate::Result<deadpool_postgres::Object> {
        {
            let guard = self.pool.read().await;
            if let Some(pool) = guard.as_ref() {
                let obj = pool.get().await.map_err(|e| anyhow::anyhow!("Pool get error: {e}"))?;
                return Ok(obj);
            }
        }
        {
            let mut guard = self.pool.write().await;
            if guard.is_none() {
                *guard = Some(self.build_pool()?);
                log::info!(
                    "[postgres-config:{}] Created connection pool (max_size={}, tls={:?})",
                    self.name(),
                    self.config.pool_max_size,
                    self.config.tls
                );
            }
            let pool = guard.as_ref().unwrap();
            pool.get().await.map_err(|e| anyhow::anyhow!("Pool get error: {e}"))
        }
    }
}

#[async_trait]
impl GlobalNodeBehavior for PostgresConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contrib_config_with_credentials_loads() {
        let cfg = PostgresConfig::from_json(&json!({
            "host": "db.local", "hostFieldType": "str", "port": "5433", "portFieldType": "num",
            "database": "plant", "databaseFieldType": "str", "ssl": "false", "sslFieldType": "bool",
            "applicationName": "", "max": "4", "maxFieldType": "num", "idle": 1000,
            "connectionTimeout": 7000, "userFieldType": "cred", "passwordFieldType": "cred",
            "credentials": {"user": "scada", "password": "pw"}
        }))
        .unwrap();
        assert_eq!((cfg.host.as_str(), cfg.port, cfg.dbname.as_str()), ("db.local", 5433, "plant"));
        assert_eq!((cfg.user.as_str(), cfg.password.as_str()), ("scada", "pw"));
        assert_eq!(cfg.pool_max_size, 4);
        assert_eq!(cfg.connect_timeout_ms, 7000);
        assert_eq!(cfg.tls, PostgresTls::Off);
    }

    #[test]
    fn credentials_from_a_node_red_credentials_file_reach_the_config() {
        use crate::runtime::credentials::{CredentialSettings, CredentialStore, SecretSetting};
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/node_red_credentials/plaintext");
        let flows_path = std::path::Path::new(dir).join("flows.json");
        let mut flows: Value = serde_json::from_str(&std::fs::read_to_string(&flows_path).unwrap()).unwrap();
        let settings = CredentialSettings { secret: SecretSetting::Disabled, ..Default::default() };
        CredentialStore::for_flows_file(&flows_path, &settings).merge_into_flows(&mut flows).unwrap();
        let node = flows.as_array().unwrap().iter().find(|n| n["id"] == "a81d5f3c9e2b6047").unwrap();
        let cfg = PostgresConfig::from_json(node).unwrap();
        assert_eq!((cfg.user.as_str(), cfg.password.as_str()), ("scada", "pg-secret-1"));
        assert_eq!((cfg.host.as_str(), cfg.port, cfg.dbname.as_str()), ("127.0.0.1", 5432, "plant"));
    }

    #[test]
    fn legacy_plain_text_user_and_native_names_load() {
        let cfg = PostgresConfig::from_json(&json!({
            "host": "h", "dbname": "d", "user": "u", "password": "p", "poolMaxSize": 3, "connectTimeoutMs": 900
        }))
        .unwrap();
        assert_eq!((cfg.dbname.as_str(), cfg.user.as_str(), cfg.password.as_str()), ("d", "u", "p"));
        assert_eq!((cfg.pool_max_size, cfg.connect_timeout_ms), (3, 900));
    }

    #[test]
    fn ssl_values_map_to_tls_modes() {
        let tls = |v: Value| PostgresConfig::from_json(&json!({"ssl": v, "sslFieldType": "json"})).unwrap().tls;
        assert_eq!(tls(json!(true)), PostgresTls::Verify);
        assert_eq!(tls(json!("true")), PostgresTls::Verify);
        assert_eq!(tls(json!(false)), PostgresTls::Off);
        assert_eq!(tls(json!({"rejectUnauthorized": false})), PostgresTls::NoVerify);
    }

    #[test]
    fn env_field_types_read_the_environment() {
        // SAFETY: the variable name is unique to this test.
        unsafe { std::env::set_var("RUST_RED_TEST_PG_HOST", "from-env") };
        let cfg = PostgresConfig::from_json(&json!({"host": "RUST_RED_TEST_PG_HOST", "hostFieldType": "env"})).unwrap();
        assert_eq!(cfg.host, "from-env");
    }

    #[test]
    fn bad_port_is_an_error() {
        assert!(PostgresConfig::from_json(&json!({"port": "x", "portFieldType": "num"})).is_err());
    }
}
