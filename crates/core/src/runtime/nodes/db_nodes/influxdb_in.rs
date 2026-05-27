use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::influxdb_config::InfluxDbConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct InfluxDbInConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    measurement: String,
    #[serde(rename = "tagColumns")]
    #[serde(default)]
    tag_columns: Option<Vec<String>>,
    #[serde(rename = "fieldColumns")]
    #[serde(default)]
    field_columns: Option<Vec<String>>,
    #[serde(rename = "timestampColumn")]
    #[serde(default)]
    timestamp_column: Option<String>,
}

#[derive(Debug)]
#[flow_node("influxdb-in", red_name = "influxdb-in", module = "rust-red")]
struct InfluxDbInNode {
    base: BaseFlowNodeState,
    config: InfluxDbInConfig,
}

impl InfluxDbInNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let in_config = InfluxDbInConfig::deserialize(&config.rest)?;
        Ok(Box::new(InfluxDbInNode { base: base_node, config: in_config }))
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

    /// Build InfluxDB line protocol from the message.
    ///
    /// Format: `measurement,tag1=val1,tag2=val2 field1=val1,field2=val2 timestamp`
    fn build_line_protocol(&self, msg: &Msg) -> crate::Result<String> {
        let mut line = String::new();

        // Measurement name (escape spaces and commas)
        let measurement = escape_measurement(&self.config.measurement);
        line.push_str(&measurement);

        // Tags
        if let Some(ref tag_cols) = self.config.tag_columns {
            if !tag_cols.is_empty() {
                for col in tag_cols {
                    if let Some(val) = msg.get(col) {
                        if let Some(s) = val.as_str() {
                            line.push(',');
                            line.push_str(&escape_tag_key(col));
                            line.push('=');
                            line.push_str(&escape_tag_value(s));
                        } else {
                            // For non-string tag values, convert to string
                            let s = variant_to_string(val);
                            line.push(',');
                            line.push_str(&escape_tag_key(col));
                            line.push('=');
                            line.push_str(&escape_tag_value(&s));
                        }
                    }
                }
            }
        }

        // Space separator between tags and fields
        line.push(' ');

        // Fields
        let field_cols = self.config.field_columns.as_ref().filter(|c| !c.is_empty());

        if let Some(field_col_list) = field_cols {
            let mut first = true;
            for col in field_col_list {
                if let Some(val) = msg.get(col) {
                    if !first {
                        line.push(',');
                    }
                    first = false;
                    line.push_str(&escape_field_key(col));
                    line.push('=');
                    line.push_str(&format_field_value(val));
                }
            }
        } else {
            // No explicit field columns: try to use payload as the sole field source
            if let Some(payload) = msg.get("payload") {
                match payload {
                    Variant::Object(map) => {
                        let mut first = true;
                        for (key, val) in map.iter() {
                            if !first {
                                line.push(',');
                            }
                            first = false;
                            line.push_str(&escape_field_key(key));
                            line.push('=');
                            line.push_str(&format_field_value(val));
                        }
                    }
                    other => {
                        // Use "value" as the default field key
                        line.push_str("value=");
                        line.push_str(&format_field_value(other));
                    }
                }
            }
        }

        // Timestamp (optional)
        if let Some(ref ts_col) = self.config.timestamp_column {
            if let Some(val) = msg.get(ts_col) {
                if let Some(ts_str) = val.as_str() {
                    line.push(' ');
                    line.push_str(ts_str);
                } else if let Some(ts_i64) = val.as_i64() {
                    line.push(' ');
                    line.push_str(&ts_i64.to_string());
                }
            }
        }

        Ok(line)
    }
}

/// Escape measurement name: escape commas and spaces.
fn escape_measurement(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ',' => out.push_str("\\,"),
            ' ' => out.push_str("\\ "),
            _ => out.push(c),
        }
    }
    out
}

/// Escape tag key: escape commas, equals, and spaces.
fn escape_tag_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ',' => out.push_str("\\,"),
            '=' => out.push_str("\\="),
            ' ' => out.push_str("\\ "),
            _ => out.push(c),
        }
    }
    out
}

/// Escape tag value: same rules as tag key.
fn escape_tag_value(s: &str) -> String {
    escape_tag_key(s)
}

/// Escape field key: escape commas, equals, and spaces.
fn escape_field_key(s: &str) -> String {
    escape_tag_key(s)
}

/// Convert a Variant to a plain string representation.
fn variant_to_string(val: &Variant) -> String {
    match val {
        Variant::String(s) => s.clone(),
        Variant::Number(n) => n.to_string(),
        Variant::Bool(b) => b.to_string(),
        Variant::Null => String::new(),
        _ => format!("{:?}", val),
    }
}

/// Format a Variant as an InfluxDB field value.
///
/// - Strings are double-quoted
/// - Integers are bare numbers with trailing `i`
/// - Floats are bare numbers
/// - Booleans are `true`/`false`
/// - Null is omitted (returns empty string)
fn format_field_value(val: &Variant) -> String {
    match val {
        Variant::String(s) => {
            // Escape double quotes and backslashes inside the string
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\"", escaped)
        }
        Variant::Number(n) => {
            if let Some(i) = n.as_i64() {
                format!("{}i", i)
            } else if let Some(f) = n.as_f64() {
                format!("{}", f)
            } else {
                "0i".to_string()
            }
        }
        Variant::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Variant::Null => String::new(),
        _ => {
            // Fallback: convert to string and quote it
            let s = variant_to_string(val);
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\"", escaped)
        }
    }
}

#[async_trait]
impl FlowNodeBehavior for InfluxDbInNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[influxdb-in:{}] {}", self.name(), e);
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

                    // Build line protocol from the message
                    let line_protocol = {
                        let msg_read = msg.read().await;
                        let result = node.build_line_protocol(&msg_read);
                        drop(msg_read);
                        match result {
                            Ok(lp) => lp,
                            Err(e) => {
                                log::warn!("[influxdb-in:{}] Failed to build line protocol: {}", node.name(), e);
                                {
                                    let mut guard = msg.write().await;
                                    guard.set(
                                        "error".to_string(),
                                        Variant::String(format!("Failed to build line protocol: {}", e)),
                                    );
                                }
                                let envelope = Envelope { port: 0, msg };
                                node.fan_out_one(envelope, CancellationToken::new()).await?;
                                return Ok(());
                            }
                        }
                    };

                    log::debug!("[influxdb-in:{}] Writing line protocol: {}", node.name(), line_protocol);

                    match cfg_inner.write_line_protocol(&line_protocol, "ms").await {
                        Ok(()) => {
                            {
                                let mut guard = msg.write().await;
                                let mut result_map = VariantObjectMap::new();
                                result_map.set_property("success".to_string(), Variant::Bool(true));
                                result_map.set_property(
                                    "measurement".to_string(),
                                    Variant::String(node.config.measurement.clone()),
                                );
                                result_map.set_property("lineProtocol".to_string(), Variant::String(line_protocol));
                                guard.set("payload".to_string(), Variant::Object(result_map));
                            }
                            let envelope = Envelope { port: 0, msg };
                            node.fan_out_one(envelope, CancellationToken::new()).await?;
                        }
                        Err(e) => {
                            log::warn!("[influxdb-in:{}] Write error: {}", node.name(), e);
                            {
                                let mut guard = msg.write().await;
                                guard.set("error".to_string(), Variant::String(e.to_string()));
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
