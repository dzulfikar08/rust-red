use std::sync::Arc;
use std::time::Duration;

use deadpool::managed::{self, Manager, Pool, RecycleResult};
use serde::Deserialize;
use tiberius::{AuthMethod, Client, Config as TiberiusConfig};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

/// The concrete tiberius Client type when used with tokio's TcpStream.
type TiberiusClient = Client<Compat<TcpStream>>;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct MssqlConfig {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    database: String,
    user: String,
    password: String,
    #[serde(default = "default_encrypt")]
    encrypt: bool,
    #[serde(default)]
    trust_server_certificate: bool,
    #[serde(default = "default_pool_max")]
    pool_max_size: u32,
    #[serde(default = "default_connect_timeout_ms")]
    connect_timeout_ms: u64,
}

fn default_port() -> u16 {
    1433
}
fn default_encrypt() -> bool {
    true
}
fn default_pool_max() -> u32 {
    10
}
fn default_connect_timeout_ms() -> u64 {
    5000
}

/// A deadpool `Manager` that creates and recycles tiberius `Client` connections.
#[derive(Debug)]
pub(crate) struct MssqlManager {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    trust_server_certificate: bool,
}

impl MssqlManager {
    fn new(mssql_config: &MssqlConfig) -> Self {
        Self {
            host: mssql_config.host.clone(),
            port: mssql_config.port,
            database: mssql_config.database.clone(),
            user: mssql_config.user.clone(),
            password: mssql_config.password.clone(),
            trust_server_certificate: mssql_config.trust_server_certificate,
        }
    }

    fn build_tiberius_config(&self) -> TiberiusConfig {
        let mut config = TiberiusConfig::new();
        config.host(&self.host);
        config.port(self.port);
        config.database(&self.database);
        config.authentication(AuthMethod::sql_server(&self.user, &self.password));
        if self.trust_server_certificate {
            config.trust_cert();
        }
        config
    }
}

impl Manager for MssqlManager {
    type Type = TiberiusClient;
    type Error = anyhow::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let config = self.build_tiberius_config();
        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let client = Client::connect(config, tcp.compat_write()).await?;
        Ok(client)
    }

    async fn recycle(&self, client: &mut Self::Type, _: &managed::Metrics) -> RecycleResult<Self::Error> {
        // Simple liveness check: execute a lightweight query.
        // Must consume the result to keep the connection in a clean state.
        let stream =
            client.simple_query("SELECT 1").await.map_err(|e| managed::RecycleError::Backend(anyhow::anyhow!(e)))?;
        // into_results() consumes the stream completely
        stream.into_results().await.map_err(|e| managed::RecycleError::Backend(anyhow::anyhow!(e)))?;
        Ok(())
    }
}

#[derive(Debug)]
#[global_node("mssql-config", red_name = "mssql-config", module = "rust-red")]
pub(crate) struct MssqlConfigNode {
    base: BaseGlobalNodeState,
    config: MssqlConfig,
    pool: Arc<RwLock<Option<Pool<MssqlManager>>>>,
}

impl MssqlConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let mssql_config = MssqlConfig::deserialize(&config.rest)?;
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "mssql-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(MssqlConfigNode { base: state, config: mssql_config, pool: Arc::new(RwLock::new(None)) }))
    }

    pub async fn get_pool(&self) -> crate::Result<deadpool::managed::Object<MssqlManager>> {
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
                let manager = MssqlManager::new(&self.config);
                let pool = Pool::builder(manager)
                    .max_size(self.config.pool_max_size as usize)
                    .wait_timeout(Some(Duration::from_millis(self.config.connect_timeout_ms)))
                    .build()
                    .map_err(|e| anyhow::anyhow!("Pool build error: {e}"))?;
                *guard = Some(pool);
                log::info!(
                    "[mssql-config:{}] Created connection pool (max_size={})",
                    self.name(),
                    self.config.pool_max_size
                );
            }
            let pool = guard.as_ref().unwrap();
            pool.get().await.map_err(|e| anyhow::anyhow!("Pool get error: {e}"))
        }
    }

    #[allow(dead_code)]
    pub fn database(&self) -> &str {
        &self.config.database
    }
}

#[async_trait]
impl GlobalNodeBehavior for MssqlConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
