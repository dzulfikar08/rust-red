use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::bacnet_client;
use super::bacnet_config::BacnetConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct BacnetWriteConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "objectType")]
    #[serde(default = "default_object_type")]
    object_type: String,
    #[serde(rename = "objectInstance")]
    object_instance: u32,
    #[serde(default = "default_property")]
    property: String,
    /// Command priority 1-16 for a commandable property. Empty or 0: not sent, the device uses its default (16).
    /// `msg.priority` overrides it.
    #[serde(default, deserialize_with = "priority_from_any")]
    priority: Option<u8>,
    /// BACnet type of the value: `auto` (the default; Present_Value follows the object type), `real`, `double`,
    /// `unsigned`, `signed`, `enumerated`, `boolean` or `string`.
    #[serde(default = "default_value_type", rename = "valueType")]
    value_type: String,
    /// Relinquish the priority slot instead of writing `msg.payload`. `msg.relinquish = true` does the same.
    #[serde(default)]
    relinquish: bool,
}

fn default_value_type() -> String {
    "auto".to_string()
}

/// Priority as the editor saves it: a number, a numeric string, or an empty string.
fn priority_from_any<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    let n = match &v {
        serde_json::Value::Null => return Ok(None),
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) if s.trim().is_empty() => return Ok(None),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    match n {
        Some(0) => Ok(None),
        Some(p @ 1..=16) => Ok(Some(p as u8)),
        _ => Err(serde::de::Error::custom(format!("BACnet priority must be 1-16, got {v}"))),
    }
}

fn default_object_type() -> String {
    "analogOutput".to_string()
}
fn default_property() -> String {
    "presentValue".to_string()
}

#[derive(Debug)]
#[flow_node("bacnet write", red_name = "bacnet-write", module = "node-red")]
struct BacnetWriteNode {
    base: BaseFlowNodeState,
    config: BacnetWriteConfig,
}

impl BacnetWriteNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let write_config = BacnetWriteConfig::deserialize(&config.rest)?;
        Ok(Box::new(BacnetWriteNode { base: base_node, config: write_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

        let global = engine
            .find_global_node_by_ref(&self.config.config_node)
            .ok_or_else(|| anyhow::anyhow!("Config node not found"))?;

        Ok(global)
    }
}

#[async_trait]
impl FlowNodeBehavior for BacnetWriteNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[bacnet-write:{}] {}", self.name(), e);
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
            let send_cancel = cancel.child_token();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let cfg_inner = cfg.as_any().downcast_ref::<BacnetConfigNode>().unwrap();
                    let (target, timeout, retries, device_id) = {
                        let conn = cfg_inner.connection.lock().await;
                        (conn.target_addr(), conn.apdu_timeout_ms(), conn.retries(), conn.device_id())
                    };

                    let (payload, priority_override, relinquish) = {
                        let guard = msg.read().await;
                        let payload = guard.get("payload").cloned().unwrap_or(Variant::Null);
                        let priority = guard.get("priority").and_then(|p| p.as_i64());
                        let relinquish = guard.get("relinquish").and_then(|r| r.as_bool()).unwrap_or(false);
                        (serde_json::to_value(&payload)?, priority, relinquish)
                    };
                    let priority = match priority_override {
                        Some(p @ 1..=16) => Some(p as u8),
                        Some(p) => return Err(anyhow::anyhow!("msg.priority must be 1-16, got {p}")),
                        None => node.config.priority,
                    };
                    let relinquish = relinquish || node.config.relinquish;
                    if relinquish && priority.is_none() {
                        return Err(anyhow::anyhow!("Relinquish needs a priority 1-16 (node setting or msg.priority)"));
                    }

                    let object_type = bacnet_client::object_type_id(&node.config.object_type)
                        .ok_or_else(|| anyhow::anyhow!("Unknown BACnet object type '{}'", node.config.object_type))?;
                    let property = bacnet_client::property_id(&node.config.property)
                        .ok_or_else(|| anyhow::anyhow!("Unknown BACnet property '{}'", node.config.property))?;
                    let value = if relinquish {
                        bacnet_client::WriteValue::Null
                    } else {
                        bacnet_client::write_value(object_type, property, &payload, &node.config.value_type)
                            .map_err(|e| anyhow::anyhow!(e))?
                    };
                    let addr = tokio::net::lookup_host(target.as_str())
                        .await
                        .map_err(|e| anyhow::anyhow!("Cannot resolve BACnet target {target}: {e}"))?
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Cannot resolve BACnet target {target}"))?;

                    log::debug!(
                        "[bacnet-write:{}] Write {:?} to {}:{} {} at {} priority {:?} (device_id={})",
                        node.name(),
                        value,
                        node.config.object_type,
                        node.config.object_instance,
                        node.config.property,
                        target,
                        priority,
                        device_id
                    );
                    bacnet_client::write_property(
                        addr,
                        object_type,
                        node.config.object_instance,
                        property,
                        &value,
                        priority,
                        std::time::Duration::from_millis(timeout as u64),
                        retries,
                    )
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "{e} ({}:{} {})",
                            node.config.object_type,
                            node.config.object_instance,
                            node.config.property
                        )
                    })?;

                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), Variant::Bool(true));
                    guard.set(
                        "bacnet".to_string(),
                        Variant::from(serde_json::json!({
                            "objectType": node.config.object_type,
                            "objectInstance": node.config.object_instance,
                            "property": node.config.property,
                            "endpoint": target,
                            "deviceId": device_id,
                            "priority": priority,
                            "relinquished": relinquish,
                            "value": payload,
                        })),
                    );
                    drop(guard);
                    node.fan_out_one(Envelope { port: 0, msg: msg.clone() }, send_cancel).await?;
                    Ok(())
                }
            })
            .await;
        }
    }
}
