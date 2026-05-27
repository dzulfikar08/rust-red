use std::sync::Arc;
use std::time::Duration;

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
#[allow(dead_code)]
struct TimescaleDbConfig {
    #[serde(default = "default_host")]
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
    hypertable: Option<String>,
}

fn default_host() -> String {
    "localhost".to_string()
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
#[global_node("timescaledb-config", red_name = "timescaledb-config", module = "rust-red")]
pub(crate) struct TimescaleDbConfigNode {
    base: BaseGlobalNodeState,
    config: TimescaleDbConfig,
    pool: Arc<RwLock<Option<Pool>>>,
}

impl TimescaleDbConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let ts_config = TimescaleDbConfig::deserialize(&config.rest)?;
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "timescaledb-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(TimescaleDbConfigNode { base: state, config: ts_config, pool: Arc::new(RwLock::new(None)) }))
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
                    .wait_timeout(Some(Duration::from_millis(self.config.connect_timeout_ms)))
                    .build()
                    .map_err(|e| anyhow::anyhow!("Pool build error: {e}"))?;
                *guard = Some(pool);
                log::info!(
                    "[timescaledb-config:{}] Created connection pool (max_size={})",
                    self.name(),
                    self.config.pool_max_size
                );
            }
            let pool = guard.as_ref().unwrap();
            pool.get().await.map_err(|e| anyhow::anyhow!("Pool get error: {e}"))
        }
    }

    /// Ensure a hypertable exists for the given table and time column.
    /// Executes `SELECT create_hypertable(...)` if the table is not already a hypertable.
    #[allow(dead_code)]
    pub async fn ensure_hypertable(&self, table: &str, time_column: &str) -> crate::Result<()> {
        let pool_obj = self.get_pool().await?;

        // Check if the table is already a hypertable
        let check_sql = "SELECT EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = $1)";
        let rows =
            pool_obj.query(check_sql, &[&table]).await.map_err(|e| anyhow::anyhow!("Hypertable check failed: {e}"))?;

        let already_hypertable: bool = rows.first().and_then(|r| r.try_get::<_, bool>(0).ok()).unwrap_or(false);

        if !already_hypertable {
            let create_sql =
                format!("SELECT create_hypertable('\"{}\"', '{}', migrate_data => true)", table, time_column);
            pool_obj
                .query(&create_sql, &[])
                .await
                .map_err(|e| anyhow::anyhow!("create_hypertable failed for '{table}': {e}"))?;
            log::info!(
                "[timescaledb-config:{}] Created hypertable '{}' on column '{}'",
                self.name(),
                table,
                time_column
            );
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn dbname(&self) -> &str {
        &self.config.dbname
    }
}

#[async_trait]
impl GlobalNodeBehavior for TimescaleDbConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
