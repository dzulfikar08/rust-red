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

use super::mssql_config::MssqlConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct MssqlQueryConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    query: String,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}

#[derive(Debug)]
#[flow_node("mssql-query", red_name = "mssql-query", module = "rust-red")]
struct MssqlQueryNode {
    base: BaseFlowNodeState,
    config: MssqlQueryConfig,
}

impl MssqlQueryNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let query_config = MssqlQueryConfig::deserialize(&config.rest)?;
        Ok(Box::new(MssqlQueryNode { base: base_node, config: query_config }))
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

    /// Convert tiberius rows to a Variant array of objects.
    fn rows_to_variant(rows: Vec<tiberius::Row>) -> Variant {
        let result: Vec<Variant> = rows
            .into_iter()
            .map(|row| {
                let mut map = VariantObjectMap::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let key = col.name().to_string();
                    let val = Self::column_to_variant(&row, i);
                    map.set_property(key, val);
                }
                Variant::Object(map)
            })
            .collect();
        Variant::from(result)
    }

    /// Attempt to extract a Variant from a tiberius row column by index,
    /// trying common SQL types in order.
    fn column_to_variant(row: &tiberius::Row, idx: usize) -> Variant {
        // Try &str (NVARCHAR, VARCHAR, TEXT, etc.) - tiberius only implements
        // FromSql<'a> for &str, not String.
        if let Ok(Some(s)) = row.try_get::<&str, usize>(idx) {
            return Variant::String(s.to_string());
        }
        // Try i32 (INT)
        if let Ok(Some(n)) = row.try_get::<i32, usize>(idx) {
            return Variant::from(n as i64);
        }
        // Try i64 (BIGINT)
        if let Ok(Some(n)) = row.try_get::<i64, usize>(idx) {
            return Variant::from(n);
        }
        // Try f64 (FLOAT)
        if let Ok(Some(n)) = row.try_get::<f64, usize>(idx) {
            return Variant::from(n);
        }
        // Try bool (BIT)
        if let Ok(Some(b)) = row.try_get::<bool, usize>(idx) {
            return Variant::Bool(b);
        }
        // Try NaiveDateTime (DATETIME, DATETIME2)
        if let Ok(Some(dt)) = row.try_get::<chrono::NaiveDateTime, usize>(idx) {
            return Variant::String(dt.to_string());
        }
        // Try NaiveDate (DATE)
        if let Ok(Some(d)) = row.try_get::<chrono::NaiveDate, usize>(idx) {
            return Variant::String(d.to_string());
        }
        // Try NaiveTime (TIME)
        if let Ok(Some(t)) = row.try_get::<chrono::NaiveTime, usize>(idx) {
            return Variant::String(t.to_string());
        }
        Variant::Null
    }

    /// Build a tiberius `Query` with bound parameters from msg.queryParams.
    fn build_query(&self, msg: &Msg) -> tiberius::Query<'_> {
        let mut query = tiberius::Query::new(&self.config.query);

        if let Some(query_params) = msg.get("queryParams") {
            if let Some(arr) = query_params.as_array() {
                for val in arr {
                    match val {
                        Variant::String(s) => query.bind(s.clone()),
                        Variant::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                query.bind(i)
                            } else if let Some(f) = n.as_f64() {
                                query.bind(f)
                            } else {
                                query.bind(val.to_string().unwrap_or_default())
                            }
                        }
                        Variant::Bool(b) => query.bind(*b),
                        Variant::Null => query.bind(Option::<String>::None),
                        _ => {
                            if let Ok(s) = val.to_string() {
                                query.bind(s)
                            }
                        }
                    };
                }
            }
        }

        query
    }
}

#[async_trait]
impl FlowNodeBehavior for MssqlQueryNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[mssql-query:{}] {}", self.name(), e);
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
                    let cfg_inner = cfg.as_any().downcast_ref::<MssqlConfigNode>().unwrap();
                    let mut pool_obj = match cfg_inner.get_pool().await {
                        Ok(obj) => obj,
                        Err(e) => {
                            log::error!("[mssql-query:{}] Pool error: {}", node.name(), e);
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

                    let msg_read = msg.read().await;
                    let query = node.build_query(&*msg_read);
                    drop(msg_read);

                    let timeout = Duration::from_millis(node.config.timeout_ms);
                    let query_result = tokio::time::timeout(timeout, query.query(&mut *pool_obj)).await;

                    match query_result {
                        Ok(Ok(stream)) => match stream.into_first_result().await {
                            Ok(rows) => {
                                let count = rows.len();
                                {
                                    let mut guard = msg.write().await;
                                    let variant_rows = Self::rows_to_variant(rows);
                                    guard.set("payload".to_string(), variant_rows);
                                    guard.set("rowCount".to_string(), Variant::from(count as i64));
                                }
                                let envelope = Envelope { port: 0, msg };
                                node.fan_out_one(envelope, CancellationToken::new()).await?;
                            }
                            Err(e) => {
                                log::warn!("[mssql-query:{}] Result collection error: {}", node.name(), e);
                                {
                                    let mut guard = msg.write().await;
                                    guard.set("error".to_string(), Variant::String(e.to_string()));
                                }
                                let envelope = Envelope { port: 0, msg };
                                node.fan_out_one(envelope, CancellationToken::new()).await?;
                            }
                        },
                        Ok(Err(e)) => {
                            log::warn!("[mssql-query:{}] Query error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e.to_string()));
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Err(_) => {
                            log::warn!(
                                "[mssql-query:{}] Query timed out after {}ms",
                                node.name(),
                                node.config.timeout_ms
                            );
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String("Query timed out".into()));
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
