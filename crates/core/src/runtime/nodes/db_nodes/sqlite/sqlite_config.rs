use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::types::ToSql;
use serde::Deserialize;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct SqliteConfig {
    #[serde(default = "default_path")]
    path: String,
    #[serde(default = "default_true", alias = "walMode")]
    wal_mode: bool,
    #[serde(default = "default_busy_timeout_ms", alias = "busyTimeoutMs")]
    busy_timeout_ms: u64,
}

fn default_path() -> String {
    ":memory:".to_string()
}

fn default_true() -> bool {
    true
}

fn default_busy_timeout_ms() -> u64 {
    5000
}

#[derive(Debug)]
#[global_node("sqlite-config", red_name = "sqlite-config", module = "rust-red")]
pub(crate) struct SqliteConfigNode {
    base: BaseGlobalNodeState,
    #[allow(dead_code)]
    config: SqliteConfig,
    connection: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let sqlite_config = SqliteConfig::deserialize(&config.rest)?;
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "sqlite-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };

        let conn = if sqlite_config.path == ":memory:" {
            rusqlite::Connection::open_in_memory()
                .map_err(|e| anyhow::anyhow!("Failed to open in-memory SQLite: {e}"))?
        } else {
            rusqlite::Connection::open(&sqlite_config.path)
                .map_err(|e| anyhow::anyhow!("Failed to open SQLite at '{}': {e}", sqlite_config.path))?
        };

        // Enable WAL mode if configured
        if sqlite_config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL;")
                .map_err(|e| anyhow::anyhow!("Failed to set WAL mode: {e}"))?;
        }

        // Set busy timeout
        conn.execute_batch(&format!("PRAGMA busy_timeout={};", sqlite_config.busy_timeout_ms))
            .map_err(|e| anyhow::anyhow!("Failed to set busy timeout: {e}"))?;

        log::info!(
            "[sqlite-config:{}] Opened SQLite database: {}",
            state.name,
            if sqlite_config.path == ":memory:" { ":memory:".to_string() } else { sqlite_config.path.clone() }
        );

        Ok(Box::new(SqliteConfigNode { base: state, config: sqlite_config, connection: Arc::new(Mutex::new(conn)) }))
    }

    /// Execute a non-query statement (INSERT, UPDATE, DELETE, etc.) and return rows affected.
    /// Runs synchronously -- caller should wrap in `spawn_blocking` if needed from async context.
    #[allow(dead_code)]
    pub fn execute_sync(&self, sql: &str, params: &[&dyn ToSql]) -> crate::Result<usize> {
        let conn = self.connection.lock().map_err(|e| anyhow::anyhow!("Connection lock poisoned: {e}"))?;
        let affected = conn.execute(sql, params).map_err(|e| anyhow::anyhow!("SQLite execute error: {e}"))?;
        Ok(affected)
    }

    /// Execute a SELECT query and return rows as Vec<Variant>.
    /// Runs synchronously -- caller should wrap in `spawn_blocking` if needed from async context.
    #[allow(dead_code)]
    pub fn query_sync(&self, sql: &str, params: &[&dyn ToSql]) -> crate::Result<Vec<Variant>> {
        let conn = self.connection.lock().map_err(|e| anyhow::anyhow!("Connection lock poisoned: {e}"))?;
        let mut stmt = conn.prepare(sql).map_err(|e| anyhow::anyhow!("SQLite prepare error: {e}"))?;
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = stmt
            .query_map(params, |row| {
                let mut map = VariantObjectMap::new();
                for (i, col_name) in column_names.iter().enumerate() {
                    let key = col_name.clone();
                    let val = super::row_to_variant(row, i);
                    map.set_property(key, val);
                }
                Ok(map)
            })
            .map_err(|e| anyhow::anyhow!("SQLite query error: {e}"))?;

        let mut result = Vec::new();
        for row_result in rows {
            let map = row_result.map_err(|e| anyhow::anyhow!("SQLite row error: {e}"))?;
            result.push(Variant::Object(map));
        }
        Ok(result)
    }

    /// Get a clone of the inner connection Arc for spawn_blocking usage
    pub fn connection(&self) -> Arc<Mutex<rusqlite::Connection>> {
        self.connection.clone()
    }
}

#[async_trait]
impl GlobalNodeBehavior for SqliteConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
