use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use rusqlite::types::ToSql;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::sqlite_config::SqliteConfigNode;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct SqliteQueryConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    query: String,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
    #[serde(default = "default_output_mode")]
    output_mode: String,
}

fn default_timeout_ms() -> u64 {
    30000
}

/// Result of a SQLite query execution.
struct SqliteQueryResult {
    rows: Vec<Variant>,
    changes: usize,
}

fn default_output_mode() -> String {
    "rows".to_string()
}

#[derive(Debug)]
#[flow_node("sqlite-query", red_name = "sqlite-query", module = "rust-red")]
struct SqliteQueryNode {
    base: BaseFlowNodeState,
    config: SqliteQueryConfig,
}

impl SqliteQueryNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let query_config = SqliteQueryConfig::deserialize(&config.rest)?;
        Ok(Box::new(SqliteQueryNode { base: base_node, config: query_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

        let eid_opt = ElementId::from_str(&self.config.config_node).ok();
        let global = eid_opt
            .and_then(|eid| engine.find_global_node_by_id(&eid))
            .or_else(|| engine.find_global_node_by_name(&self.config.config_node).ok().flatten())
            .ok_or_else(|| anyhow::anyhow!("Config node '{}' not found", self.config.config_node))?;

        Ok(global)
    }

    /// Extract params from msg.queryParams (array of values) into rusqlite-compatible owned params
    fn extract_params(msg: &Msg) -> Vec<Box<dyn ToSql + Send>> {
        let mut params: Vec<Box<dyn ToSql + Send>> = Vec::new();
        if let Some(query_params) = msg.get("queryParams")
            && let Some(arr) = query_params.as_array()
        {
            for val in arr {
                match val {
                    Variant::String(s) => params.push(Box::new(s.clone())),
                    Variant::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            params.push(Box::new(i));
                        } else if let Some(f) = n.as_f64() {
                            params.push(Box::new(f));
                        }
                    }
                    Variant::Bool(b) => params.push(Box::new(*b)),
                    Variant::Null => params.push(Box::new(Option::<String>::None)),
                    _ => {
                        if let Ok(s) = val.to_string() {
                            params.push(Box::new(s));
                        }
                    }
                }
            }
        }
        params
    }

    /// Execute a query on a background thread, returning the result rows.
    ///
    /// For SELECT statements, returns the rows as Vec<Variant>.
    /// For DML/DDL statements (INSERT, UPDATE, DELETE, CREATE TABLE, etc.),
    /// returns an empty Vec and sets `changes` on the message.
    async fn do_query(
        conn: Arc<Mutex<rusqlite::Connection>>,
        sql: String,
        params: Vec<Box<dyn ToSql + Send>>,
        timeout: Duration,
    ) -> Result<SqliteQueryResult, String> {
        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let conn_guard = conn.lock().map_err(|e| anyhow::anyhow!("Connection lock poisoned: {e}"))?;

                let trimmed = sql.trim();
                let is_select = trimmed.to_uppercase().starts_with("SELECT")
                    || trimmed.to_uppercase().starts_with("PRAGMA")
                    || trimmed.to_uppercase().starts_with("EXPLAIN")
                    || trimmed.to_uppercase().starts_with("WITH");

                if is_select {
                    let mut stmt =
                        conn_guard.prepare(&sql).map_err(|e| anyhow::anyhow!("SQLite prepare error: {e}"))?;

                    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

                    // Only pass params if the SQL contains placeholders
                    let has_placeholders = sql.contains('?') || sql.contains('$') || sql.contains(':');
                    let param_refs: Vec<&dyn ToSql> = if has_placeholders {
                        params.iter().map(|p| p.as_ref() as &dyn ToSql).collect()
                    } else {
                        Vec::new()
                    };

                    let rows = stmt
                        .query_map(param_refs.as_slice(), |row| {
                            let mut map = VariantObjectMap::new();
                            for (i, col_name) in column_names.iter().enumerate() {
                                let key = col_name.clone();
                                let val = super::row_to_variant(row, i);
                                map.set_property(key, val);
                            }
                            Ok(map)
                        })
                        .map_err(|e| anyhow::anyhow!("SQLite query error: {e}"))?;

                    let mut result_rows = Vec::new();
                    for row_result in rows {
                        let map = row_result.map_err(|e| anyhow::anyhow!("SQLite row error: {e}"))?;
                        result_rows.push(Variant::Object(map));
                    }
                    Ok::<SqliteQueryResult, anyhow::Error>(SqliteQueryResult { rows: result_rows, changes: 0 })
                } else {
                    // DML/DDL: use execute() instead of query_map()
                    // Only pass params if the SQL contains placeholders
                    let has_placeholders = sql.contains('?') || sql.contains('$') || sql.contains(':');
                    let changes = if has_placeholders && !params.is_empty() {
                        let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref() as &dyn ToSql).collect();
                        conn_guard
                            .execute(&sql, param_refs.as_slice())
                            .map_err(|e| anyhow::anyhow!("SQLite execute error: {e}"))?
                    } else {
                        conn_guard.execute(&sql, []).map_err(|e| anyhow::anyhow!("SQLite execute error: {e}"))?
                    };
                    Ok::<SqliteQueryResult, anyhow::Error>(SqliteQueryResult { rows: Vec::new(), changes })
                }
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(query_result))) => Ok(query_result),
            Ok(Ok(Err(e))) => Err(e.to_string()),
            Ok(Err(e)) => Err(format!("Blocking task error: {e}")),
            Err(_) => Err("Query timed out".to_string()),
        }
    }
}

#[async_trait]
impl FlowNodeBehavior for SqliteQueryNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[sqlite-query:{}] {}", self.name(), e);
                self.report_status(
                    StatusObject {
                        fill: Some(StatusFill::Red),
                        shape: Some(StatusShape::Ring),
                        text: Some(e.to_string()),
                    },
                    stop_token.clone(),
                )
                .await;
                stop_token.cancelled().await;
                return;
            }
        };

        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            let this = self.clone();
            let cfg = config_node.clone();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let cfg_inner = cfg.as_any().downcast_ref::<SqliteConfigNode>().unwrap();

                    // Extract params from message
                    let params = Self::extract_params(&*msg.read().await);

                    let sql = node.config.query.clone();
                    let conn = cfg_inner.connection();
                    let timeout = Duration::from_millis(node.config.timeout_ms);

                    let result = Self::do_query(conn, sql, params, timeout).await;

                    match result {
                        Ok(query_result) => {
                            let count = query_result.rows.len();
                            {
                                let mut guard = msg.write().await;
                                guard.set("payload".to_string(), Variant::from(query_result.rows));
                                guard.set("rowCount".to_string(), Variant::from(count as i64));
                                if query_result.changes > 0 {
                                    guard.set("changes".to_string(), Variant::from(query_result.changes as i64));
                                }
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Err(e) => {
                            log::warn!("[sqlite-query:{}] Query error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e));
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                    }
                    Ok(())
                }
            })
            .await;
        }
    }
}
