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
struct OpcUaReadConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(default = "default_attribute")]
    attribute: String,
    /// Action to perform: "read" (default) or "browse".
    #[serde(default = "default_action")]
    action: String,
    #[serde(default, rename = "intervalMs")]
    interval_ms: Option<u64>,
    #[serde(default)]
    deadband: Option<f64>,
}

fn default_attribute() -> String {
    "Value".to_string()
}
fn default_action() -> String {
    "read".to_string()
}

#[derive(Debug)]
#[flow_node("opcua read", red_name = "opcua-read", module = "node-red")]
struct OpcUaReadNode {
    base: BaseFlowNodeState,
    config: OpcUaReadConfig,
}

impl OpcUaReadNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let read_config = OpcUaReadConfig::deserialize(&config.rest)?;
        Ok(Box::new(OpcUaReadNode { base: base_node, config: read_config }))
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

    /// Perform a browse operation on the configured node, returning child references.
    async fn perform_browse(&self, cfg_inner: &OpcUaConfigNode, msg: &MsgHandle) -> crate::Result<()> {
        log::info!("[opcua-read:{}] Browse node_id={} from {}", self.name(), self.config.node_id, cfg_inner.endpoint());

        // TODO: When opcua client is fully wired, use session.browse() here.
        // The stub returns an empty array so the flow continues gracefully.
        let mut guard = msg.write().await;
        guard.set("payload".to_string(), Variant::from(Vec::<Variant>::new()));
        guard.set(
            "opcua".to_string(),
            Variant::from(serde_json::json!({
                "action": "browse",
                "nodeId": self.config.node_id,
                "endpoint": cfg_inner.endpoint(),
            })),
        );
        Ok(())
    }

    /// Perform a read operation on the configured node/attribute.
    async fn perform_read(&self, cfg_inner: &OpcUaConfigNode, msg: &MsgHandle) -> crate::Result<()> {
        log::info!(
            "[opcua-read:{}] Read node_id={} attribute={} from {}",
            self.name(),
            self.config.node_id,
            self.config.attribute,
            cfg_inner.endpoint()
        );

        // TODO: When opcua client is fully wired, use session.read() here.
        let mut guard = msg.write().await;
        guard.set("payload".to_string(), Variant::Null);
        guard.set(
            "opcua".to_string(),
            Variant::from(serde_json::json!({
                "action": "read",
                "nodeId": self.config.node_id,
                "attribute": self.config.attribute,
                "endpoint": cfg_inner.endpoint(),
            })),
        );
        Ok(())
    }
}

#[async_trait]
impl FlowNodeBehavior for OpcUaReadNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[opcua-read:{}] {}", self.name(), e);
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

                    match node.config.action.as_str() {
                        "browse" => node.perform_browse(cfg_inner, &msg).await?,
                        _ => node.perform_read(cfg_inner, &msg).await?,
                    }
                    Ok(())
                }
            })
            .await;
        }
    }
}
