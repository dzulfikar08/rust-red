use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::opcua_config::OpcUaConfigNode;

#[derive(Deserialize, Debug, Clone)]
struct OpcUaWriteConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(default = "default_attribute")]
    attribute: String,
}

fn default_attribute() -> String {
    "Value".to_string()
}

#[derive(Debug)]
#[flow_node("opcua write", red_name = "opcua-write", module = "node-red")]
struct OpcUaWriteNode {
    base: BaseFlowNodeState,
    config: OpcUaWriteConfig,
}

impl OpcUaWriteNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let write_config = OpcUaWriteConfig::deserialize(&config.rest)?;
        Ok(Box::new(OpcUaWriteNode { base: base_node, config: write_config }))
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
}

#[async_trait]
impl FlowNodeBehavior for OpcUaWriteNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[opcua-write:{}] {}", self.name(), e);
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
                    let cfg_inner = cfg.as_any().downcast_ref::<OpcUaConfigNode>().unwrap();
                    cfg_inner.ensure_connected().await?;

                    let guard = msg.read().await;
                    let payload = guard.get("payload").cloned().unwrap_or(Variant::Null);
                    drop(guard);

                    log::info!(
                        "[opcua-write:{}] Write node_id={} attribute={} value={:?} to {}",
                        node.name(),
                        node.config.node_id,
                        node.config.attribute,
                        payload,
                        cfg_inner.endpoint()
                    );

                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), Variant::Bool(true));
                    guard.set(
                        "opcua".to_string(),
                        Variant::from(serde_json::json!({
                            "nodeId": node.config.node_id,
                            "attribute": node.config.attribute,
                            "endpoint": cfg_inner.endpoint(),
                        })),
                    );
                    Ok(())
                }
            })
            .await;
        }
    }
}
