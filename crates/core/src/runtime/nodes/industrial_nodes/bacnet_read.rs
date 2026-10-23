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
#[allow(dead_code)]
struct BacnetReadConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "objectType")]
    #[serde(default = "default_object_type")]
    object_type: String,
    #[serde(rename = "objectInstance")]
    object_instance: u32,
    #[serde(default = "default_property")]
    property: String,
    /// More properties to read in the same message. When more than one property is requested in
    /// total, `msg.payload` is an object keyed by property name.
    #[serde(default)]
    properties: Vec<String>,
    /// Whether to subscribe to COV (Change of Value) notifications.
    /// When true, the node will issue a SubscribeCOV request and emit
    /// messages when the property value changes.
    #[serde(default, rename = "subscribeCov")]
    subscribe_cov: bool,
    /// COV subscription lifetime in seconds. 0 means use the config-level default.
    #[serde(default, rename = "covLifetime")]
    cov_lifetime: u32,
    /// Optional COV increment threshold (for analog objects). Only emit when
    /// the value changes by at least this amount.
    #[serde(default, rename = "covIncrement")]
    cov_increment: Option<f64>,
}

fn default_object_type() -> String {
    "analogInput".to_string()
}
fn default_property() -> String {
    "presentValue".to_string()
}

#[derive(Debug)]
#[flow_node("bacnet read", red_name = "bacnet-read", module = "node-red")]
struct BacnetReadNode {
    base: BaseFlowNodeState,
    config: BacnetReadConfig,
}

impl BacnetReadNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let read_config = BacnetReadConfig::deserialize(&config.rest)?;
        Ok(Box::new(BacnetReadNode { base: base_node, config: read_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

        let global = engine
            .find_global_node_by_ref(&self.config.config_node)
            .ok_or_else(|| anyhow::anyhow!("Config node not found"))?;

        Ok(global)
    }

    /// Property names to read: `property` (may hold a comma separated list) plus `properties`, without duplicates.
    fn requested_properties(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for name in self.config.property.split(',').chain(self.config.properties.iter().map(String::as_str)) {
            let name = name.trim();
            if !name.is_empty() && !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
        if out.is_empty() {
            out.push(default_property());
        }
        out
    }
}

#[async_trait]
impl FlowNodeBehavior for BacnetReadNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[bacnet-read:{}] {}", self.name(), e);
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

        if self.config.subscribe_cov {
            log::warn!(
                "[bacnet-read:{}] COV subscriptions are not implemented; the node reads on every input message instead",
                self.name()
            );
        }

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

                    let object_type = bacnet_client::object_type_id(&node.config.object_type)
                        .ok_or_else(|| anyhow::anyhow!("Unknown BACnet object type '{}'", node.config.object_type))?;
                    let requested = node.requested_properties();
                    let addr = tokio::net::lookup_host(target.as_str())
                        .await
                        .map_err(|e| anyhow::anyhow!("Cannot resolve BACnet target {target}: {e}"))?
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Cannot resolve BACnet target {target}"))?;

                    log::debug!(
                        "[bacnet-read:{}] Read {}:{} {:?} from {} (device_id={})",
                        node.name(),
                        node.config.object_type,
                        node.config.object_instance,
                        requested,
                        target,
                        device_id
                    );

                    let mut values = serde_json::Map::new();
                    for name in &requested {
                        let property = bacnet_client::property_id(name)
                            .ok_or_else(|| anyhow::anyhow!("Unknown BACnet property '{name}'"))?;
                        let value = bacnet_client::read_property(
                            addr,
                            object_type,
                            node.config.object_instance,
                            property,
                            std::time::Duration::from_millis(timeout as u64),
                            retries,
                        )
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("{e} ({}:{} {name})", node.config.object_type, node.config.object_instance)
                        })?;
                        values.insert(name.clone(), value);
                    }
                    let payload = if requested.len() == 1 {
                        values.remove(&requested[0]).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Object(values)
                    };

                    let info = serde_json::json!({
                        "objectType": node.config.object_type,
                        "objectInstance": node.config.object_instance,
                        "property": node.config.property,
                        "properties": requested,
                        "endpoint": target,
                        "deviceId": device_id,
                    });

                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), Variant::from(payload));
                    guard.set("bacnet".to_string(), Variant::from(info));
                    drop(guard);
                    node.fan_out_one(Envelope { port: 0, msg: msg.clone() }, send_cancel).await?;
                    Ok(())
                }
            })
            .await;
        }
    }
}
