use std::sync::Arc;

use async_trait::async_trait;
use deadpool_postgres::{Config, Pool};
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio_postgres::NoTls;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PostgresConfig {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    dbname: String,
    user: String,
    password: String,
    #[serde(default = "default_pool_max")]
    pool_max_size: u32,
    #[serde(default = "default_connect_timeout_ms")]
    connect_timeout_ms: u64,
    ssl: Option<bool>,
    idle_timeout_ms: Option<u64>,
    application_name: Option<String>,
}

fn default_port() -> u16 {
    5432
}
fn default_pool_max() -> u32 {
    10
}
fn default_connect_timeout_ms() -> u64 {
    5000
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
        let pg_config = PostgresConfig::deserialize(&config.rest)?;
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
                let mut cfg = Config::new();
                cfg.host = Some(self.config.host.clone());
                cfg.port = Some(self.config.port);
                cfg.dbname = Some(self.config.dbname.clone());
                cfg.user = Some(self.config.user.clone());
                cfg.password = Some(self.config.password.clone());
                cfg.manager = Some(deadpool_postgres::ManagerConfig {
                    recycling_method: deadpool_postgres::RecyclingMethod::Fast,
                });
                let pool = cfg
                    .builder(NoTls)
                    .map_err(|e| anyhow::anyhow!("Pool config error: {e}"))?
                    .max_size(self.config.pool_max_size as usize)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Pool build error: {e}"))?;
                *guard = Some(pool);
                log::info!(
                    "[postgres-config:{}] Created connection pool (max_size={})",
                    self.name(),
                    self.config.pool_max_size
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
