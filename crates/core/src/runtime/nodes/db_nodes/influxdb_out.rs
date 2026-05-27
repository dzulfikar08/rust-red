use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mustache::MapBuilder;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::influxdb_config::InfluxDbConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct InfluxDbOutConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    query: String,
    #[serde(rename = "timeoutMs")]
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}

#[derive(Debug)]
#[flow_node("influxdb-out", red_name = "influxdb-out", module = "rust-red")]
struct InfluxDbOutNode {
    base: BaseFlowNodeState,
    config: InfluxDbOutConfig,
}

impl InfluxDbOutNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let out_config = InfluxDbOutConfig::deserialize(&config.rest)?;
        Ok(Box::new(InfluxDbOutNode { base: base_node, config: out_config }))
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

    /// Render mustache template in the query using message values.
    fn render_query(&self, msg: &Msg) -> crate::Result<String> {
        // Check if the query contains any mustache delimiters
        if !self.config.query.contains("{{") {
            return Ok(self.config.query.clone());
        }

        let msg_json = serde_json::to_value(msg)?;
        let mut context_map = MapBuilder::new();

        if let serde_json::Value::Object(obj) = &msg_json {
            for (key, value) in obj {
                match value {
                    serde_json::Value::String(s) => {
                        context_map = context_map.insert_str(key, s);
                    }
                    serde_json::Value::Number(n) => {
                        context_map = context_map.insert_str(key, n.to_string());
                    }
                    serde_json::Value::Bool(b) => {
                        context_map = context_map.insert_str(key, b.to_string());
                    }
                    serde_json::Value::Null => {
                        context_map = context_map.insert_str(key, "");
                    }
                    _ => {
                        context_map = context_map.insert(key, value).map_err(|e| {
                            crate::RustRedError::invalid_operation(&format!("Query template context error: {}", e))
                        })?;
                    }
                }
            }
        }

        let data = context_map.build();
        let template = mustache::compile_str(&self.config.query)
            .map_err(|e| crate::RustRedError::invalid_operation(&format!("Query template compilation error: {}", e)))?;

        template
            .render_data_to_string(&data)
            .map_err(|e| crate::RustRedError::invalid_operation(&format!("Query template rendering error: {}", e)))
    }

    /// Parse the InfluxDB JSON query response into a Variant array.
    ///
    /// InfluxDB v2 query with `Accept: application/json` returns the annotated CSV
    /// response. We attempt to parse it as JSON first; if that fails, we fall back
    /// to CSV parsing.
    fn parse_response(&self, body: &str) -> Variant {
        // Try JSON parse first
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
            return Variant::from(json_val);
        }

        // Fall back to CSV/annotated CSV parsing
        self.parse_csv_response(body)
    }

    /// Parse annotated CSV from InfluxDB v2 query response.
    ///
    /// InfluxDB returns annotated CSV with lines prefixed by # for annotations.
    /// Data rows follow the header row.
    fn parse_csv_response(&self, body: &str) -> Variant {
        let lines: Vec<&str> = body.lines().collect();
        if lines.is_empty() {
            return Variant::Array(Vec::new());
        }

        // Find the header line (first non-comment, non-empty line)
        let mut header_idx = None;
        let mut headers: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // This is the data header
            headers = trimmed.split(',').map(|s| s.trim().to_string()).collect();
            header_idx = Some(i);
            break;
        }

        let Some(start_idx) = header_idx else {
            return Variant::Array(Vec::new());
        };

        let mut rows: Vec<Variant> = Vec::new();

        for line in lines.iter().skip(start_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let values: Vec<&str> = trimmed.split(',').collect();
            let mut row = VariantObjectMap::new();

            for (j, header) in headers.iter().enumerate() {
                let val_str = values.get(j).map(|v| v.trim()).unwrap_or("");
                if val_str.is_empty() {
                    row.set_property(header.clone(), Variant::Null);
                } else if let Ok(b) = val_str.parse::<bool>() {
                    row.set_property(header.clone(), Variant::Bool(b));
                } else if let Ok(i) = val_str.parse::<i64>() {
                    row.set_property(header.clone(), Variant::from(i));
                } else if let Ok(f) = val_str.parse::<f64>() {
                    row.set_property(header.clone(), Variant::from(f));
                } else {
                    row.set_property(header.clone(), Variant::String(val_str.to_string()));
                }
            }

            rows.push(Variant::Object(row));
        }

        Variant::Array(rows)
    }
}

#[async_trait]
impl FlowNodeBehavior for InfluxDbOutNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[influxdb-out:{}] {}", self.name(), e);
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
                    let cfg_inner = cfg.as_any().downcast_ref::<InfluxDbConfigNode>().unwrap();

                    // Render the query template with message context
                    let rendered_query = {
                        let msg_read = msg.read().await;
                        let result = node.render_query(&msg_read);
                        drop(msg_read);
                        match result {
                            Ok(q) => q,
                            Err(e) => {
                                log::warn!("[influxdb-out:{}] Failed to render query template: {}", node.name(), e);
                                {
                                    let mut guard = msg.write().await;
                                    guard.set(
                                        "error".to_string(),
                                        Variant::String(format!("Query template error: {}", e)),
                                    );
                                }
                                let envelope = Envelope { port: 0, msg };
                                node.fan_out_one(envelope, CancellationToken::new()).await?;
                                return Ok(());
                            }
                        }
                    };

                    log::debug!("[influxdb-out:{}] Executing query: {}", node.name(), rendered_query);

                    // Execute the query with timeout
                    let timeout = Duration::from_millis(node.config.timeout_ms);
                    let result = tokio::time::timeout(timeout, cfg_inner.query_flux(&rendered_query)).await;

                    match result {
                        Ok(Ok(response_body)) => {
                            let parsed = node.parse_response(&response_body);
                            {
                                let mut guard = msg.write().await;
                                guard.set("payload".to_string(), parsed);
                                guard.set("query".to_string(), Variant::String(rendered_query));
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Ok(Err(e)) => {
                            log::warn!("[influxdb-out:{}] Query error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e.to_string()));
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Err(_) => {
                            log::warn!(
                                "[influxdb-out:{}] Query timed out after {}ms",
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
