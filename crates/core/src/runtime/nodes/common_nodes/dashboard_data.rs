use std::sync::Arc;

use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::dashboard_channel::DashboardMessage;
use crate::runtime::flow::Flow;
use crate::runtime::model::json::RedFlowNodeConfig;
use crate::runtime::nodes::*;
use rust_red_macro::*;

/// Configuration for a dashboard data node.
///
/// In a Node-RED flow the node is configured with the ID of the dashboard
/// widget it feeds data to. The node reads `msg.payload` (and optionally
/// `msg.topic`) and pushes the value to the dashboard channel so that
/// connected browser clients receive a real-time update.
#[derive(Debug, Clone, Deserialize)]
struct DashboardDataNodeConfig {
    /// The widget ID to target on the dashboard.
    #[serde(default)]
    widget_id: String,

    /// Optional dashboard ID for scoping.
    #[serde(default)]
    dashboard_id: Option<String>,

    /// Which message property to send. Defaults to "payload".
    #[serde(default = "default_property")]
    property: String,
}

fn default_property() -> String {
    "payload".to_string()
}

#[derive(Debug)]
#[flow_node("ui_dashboard_data", red_name = "ui_dashboard_data", module = "rust_red_core")]
struct DashboardDataNode {
    base: BaseFlowNodeState,
    config: DashboardDataNodeConfig,
}

impl DashboardDataNode {
    fn build(
        _flow: &Flow,
        state: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let dashboard_config: DashboardDataNodeConfig = DashboardDataNodeConfig::deserialize(&config.rest)?;
        let node = DashboardDataNode { base: state, config: dashboard_config };
        Ok(Box::new(node))
    }
}

#[async_trait]
impl FlowNodeBehavior for DashboardDataNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            with_uow(self.as_ref(), cancel.child_token(), |node, msg| async move {
                let guard = msg.read().await;

                // Resolve the widget_id: prefer node config, fall back to msg.widget_id
                let widget_id = if !node.config.widget_id.is_empty() {
                    node.config.widget_id.clone()
                } else {
                    match guard.get("widget_id") {
                        Some(Variant::String(s)) => s.to_string(),
                        Some(v) => format!("{v:?}"),
                        None => {
                            log::warn!(
                                "[ui_dashboard_data:{}] No widget_id configured and msg.widget_id missing",
                                node.name()
                            );
                            return Ok(());
                        }
                    }
                };

                // Extract the configured property (default: payload)
                let payload_value = match guard.get(&node.config.property) {
                    Some(variant) => serde_json::to_value(variant).unwrap_or_else(|e| {
                        log::warn!("[ui_dashboard_data:{}] Failed to serialize payload: {e}", node.name());
                        serde_json::Value::Null
                    }),
                    None => {
                        log::debug!(
                            "[ui_dashboard_data:{}] msg.{} not found, sending null",
                            node.name(),
                            node.config.property
                        );
                        serde_json::Value::Null
                    }
                };

                drop(guard);

                // Publish to the dashboard broadcast channel
                if let Some(engine) = node.engine() {
                    let message = DashboardMessage {
                        widget_id,
                        dashboard_id: node.config.dashboard_id.clone(),
                        payload: payload_value,
                        timestamp: chrono::Utc::now().timestamp_millis(),
                    };
                    engine.dashboard_channel().send(message);
                } else {
                    log::warn!("[ui_dashboard_data:{}] No engine available, dropping dashboard message", node.name());
                }

                // Pass the message through to the next node (if any output wire)
                Ok(())
            })
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_dashboard_data_config_defaults() {
        let json = serde_json::json!({
            "widget_id": "abc-123",
        });
        let config: DashboardDataNodeConfig = DashboardDataNodeConfig::deserialize(&json).unwrap();
        assert_eq!(config.widget_id, "abc-123");
        assert!(config.dashboard_id.is_none());
        assert_eq!(config.property, "payload");
    }

    #[test]
    fn deserialize_dashboard_data_config_full() {
        let json = serde_json::json!({
            "widget_id": "w1",
            "dashboard_id": "d1",
            "property": "temperature"
        });
        let config: DashboardDataNodeConfig = DashboardDataNodeConfig::deserialize(&json).unwrap();
        assert_eq!(config.widget_id, "w1");
        assert_eq!(config.dashboard_id, Some("d1".to_string()));
        assert_eq!(config.property, "temperature");
    }

    #[test]
    fn deserialize_dashboard_data_config_empty_widget() {
        let json = serde_json::json!({});
        let config: DashboardDataNodeConfig = DashboardDataNodeConfig::deserialize(&json).unwrap();
        assert!(config.widget_id.is_empty());
    }
}
