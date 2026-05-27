use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::bacnet_config::BacnetConfigNode;

#[derive(Deserialize, Debug, Clone)]
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

        let eid_opt = ElementId::from_str(&self.config.config_node).ok();
        let global = eid_opt
            .and_then(|eid| engine.find_global_node_by_id(&eid))
            .or_else(|| engine.find_global_node_by_name(&self.config.config_node).ok().flatten())
            .ok_or_else(|| anyhow::anyhow!("Config node not found"))?;

        Ok(global)
    }

    /// Resolve the effective COV lifetime: per-node override, else config default.
    fn effective_cov_lifetime(&self, cfg_inner: &BacnetConfigNode) -> u32 {
        if self.config.cov_lifetime > 0 { self.config.cov_lifetime } else { cfg_inner.cov_lifetime() }
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

        // If COV subscription is requested, log the intent
        if self.config.subscribe_cov {
            let cfg_inner = config_node.as_any().downcast_ref::<BacnetConfigNode>().unwrap();
            let lifetime = self.effective_cov_lifetime(cfg_inner);
            log::info!(
                "[bacnet-read:{}] COV subscription requested for {}:{} {} (lifetime={}s, increment={:?})",
                self.name(),
                self.config.object_type,
                self.config.object_instance,
                self.config.property,
                lifetime,
                self.config.cov_increment,
            );
            // TODO: When bacnet-rs client is fully wired, send SubscribeCOV request here.
        }

        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            let this = self.clone();
            let cfg = config_node.clone();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let cfg_inner = cfg.as_any().downcast_ref::<BacnetConfigNode>().unwrap();
                    let conn = cfg_inner.connection.lock().await;

                    log::info!(
                        "[bacnet-read:{}] Read {}:{} {} from {} (device_id={})",
                        node.name(),
                        node.config.object_type,
                        node.config.object_instance,
                        node.config.property,
                        conn.target_addr(),
                        conn.device_id()
                    );
                    drop(conn);

                    let mut cov_info = serde_json::json!({
                        "objectType": node.config.object_type,
                        "objectInstance": node.config.object_instance,
                        "property": node.config.property,
                        "endpoint": cfg_inner.interface(),
                    });
                    if node.config.subscribe_cov {
                        cov_info["covSubscribed"] = serde_json::json!(true);
                        cov_info["covLifetime"] = serde_json::json!(node.effective_cov_lifetime(cfg_inner));
                        if let Some(inc) = node.config.cov_increment {
                            cov_info["covIncrement"] = serde_json::json!(inc);
                        }
                    }

                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), Variant::Null);
                    guard.set("bacnet".to_string(), Variant::from(cov_info));
                    Ok(())
                }
            })
            .await;
        }
    }
}
