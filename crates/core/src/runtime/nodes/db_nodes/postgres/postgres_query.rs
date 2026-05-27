use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::postgres_config::PostgresConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct PostgresQueryConfig {
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
fn default_output_mode() -> String {
    "rows".to_string()
}

#[derive(Debug)]
#[flow_node("postgres-query", red_name = "postgres-query", module = "rust-red")]
struct PostgresQueryNode {
    base: BaseFlowNodeState,
    config: PostgresQueryConfig,
}

impl PostgresQueryNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let query_config = PostgresQueryConfig::deserialize(&config.rest)?;
        Ok(Box::new(PostgresQueryNode { base: base_node, config: query_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

        // Try parsing config_node as an ElementId (hex string)
        let eid_opt = ElementId::from_str(&self.config.config_node).ok();
        let global = eid_opt
            .and_then(|eid| engine.find_global_node_by_id(&eid))
            .or_else(|| engine.find_global_node_by_name(&self.config.config_node).ok().flatten())
            .ok_or_else(|| anyhow::anyhow!("Config node '{}' not found", self.config.config_node))?;

        Ok(global)
    }

    fn bind_params(msg: &Msg) -> Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> {
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        if let Some(query_params) = msg.get("queryParams") {
            if let Some(arr) = query_params.as_array() {
                for val in arr {
                    match val {
                        Variant::String(s) => params.push(Box::new(s.clone())),
                        Variant::Number(n) => {
                            // Always send as f64 (FLOAT8) to avoid type mismatches.
                            // PostgreSQL will cast to the column type automatically.
                            if let Some(f) = n.as_f64() {
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
        }
        params
    }

    fn rows_to_variant(rows: Vec<tokio_postgres::Row>) -> Variant {
        let result: Vec<Variant> = rows
            .into_iter()
            .map(|row| {
                let mut map = VariantObjectMap::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let key = col.name().to_string();
                    let val: Result<serde_json::Value, _> = row.try_get(i);
                    if let Ok(v) = val {
                        map.set_property(key, Variant::from(v));
                    }
                }
                Variant::Object(map)
            })
            .collect();
        Variant::from(result)
    }
}

#[async_trait]
impl FlowNodeBehavior for PostgresQueryNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[postgres-query:{}] {}", self.name(), e);
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
                    let cfg_inner = cfg.as_any().downcast_ref::<PostgresConfigNode>().unwrap();
                    let pool_obj: deadpool_postgres::Object = match cfg_inner.get_pool().await {
                        Ok(obj) => obj,
                        Err(e) => {
                            log::error!("[postgres-query:{}] Pool error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e.to_string()));
                            }
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Red),
                                    shape: Some(StatusShape::Ring),
                                    text: Some(format!("{}", e)),
                                },
                                cancel.child_token(),
                            )
                            .await;
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                            return Ok(());
                        }
                    };

                    let (query, params) = {
                        let guard = msg.read().await;
                        // Allow msg.query to override the configured query
                        let query = guard
                            .get("query")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| node.config.query.clone());
                        let params = Self::bind_params(&guard);
                        (query, params)
                    };
                    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                        params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();

                    let timeout = Duration::from_millis(node.config.timeout_ms);
                    let result: Result<std::result::Result<Vec<tokio_postgres::Row>, tokio_postgres::Error>, _> =
                        tokio::time::timeout(timeout, pool_obj.query(&query, &param_refs)).await;

                    match result {
                        Ok(Ok(rows)) => {
                            let count = {
                                let mut guard = msg.write().await;
                                let variant_rows = Self::rows_to_variant(rows);
                                guard.set("payload".to_string(), variant_rows);
                                let count =
                                    guard.get("payload").and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                                guard.set("rowCount".to_string(), Variant::from(count as i64));
                                count
                            };
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Green),
                                    shape: Some(StatusShape::Dot),
                                    text: Some(format!("{} rows", count)),
                                },
                                cancel.child_token(),
                            )
                            .await;
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Ok(Err(e)) => {
                            log::warn!("[postgres-query:{}] Query error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e.to_string()));
                            }
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Red),
                                    shape: Some(StatusShape::Ring),
                                    text: Some(format!("{}", e)),
                                },
                                cancel.child_token(),
                            )
                            .await;
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Err(_) => {
                            log::warn!(
                                "[postgres-query:{}] Query timed out after {}ms",
                                node.name(),
                                node.config.timeout_ms
                            );
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String("Query timed out".into()));
                            }
                            node.report_status(
                                StatusObject {
                                    fill: Some(StatusFill::Red),
                                    shape: Some(StatusShape::Ring),
                                    text: Some("timeout".into()),
                                },
                                cancel.child_token(),
                            )
                            .await;
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
