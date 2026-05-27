use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;
use rust_red_mqtt_broker::broker::MqttBroker;
use rust_red_mqtt_broker::config::BrokerConfig;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct MqttBrokerNodeConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_max_connections")]
    max_connections: usize,
    #[serde(default, rename = "wsEnabled")]
    ws_enabled: Option<bool>,
    #[serde(default, rename = "wsPath")]
    ws_path: Option<String>,
    persistence: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    1883
}
fn default_max_connections() -> usize {
    100
}

#[derive(Debug)]
#[flow_node("mqtt broker embedded", red_name = "mqtt-broker-embedded", module = "node-red")]
pub(crate) struct MqttBrokerEmbeddedNode {
    base: BaseFlowNodeState,
    config: MqttBrokerNodeConfig,
    /// The actual address the broker bound to (populated once started).
    pub(crate) bound_addr: Arc<RwLock<Option<SocketAddr>>>,
}

impl MqttBrokerEmbeddedNode {
    fn build(
        _flow: &Flow,
        state: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let cfg = MqttBrokerNodeConfig::deserialize(&config.rest)?;
        Ok(Box::new(MqttBrokerEmbeddedNode { base: state, config: cfg, bound_addr: Arc::new(RwLock::new(None)) }))
    }

    /// The configured host/port (may differ from bound_addr when port=0).
    pub(crate) fn configured_addr(&self) -> (String, u16) {
        (self.config.host.clone(), self.config.port)
    }

    fn make_event_msg(&self, topic: &str, payload: Variant) -> MsgHandle {
        let mut body = std::collections::BTreeMap::new();
        body.insert("topic".to_string(), Variant::String(topic.to_string()));
        body.insert("payload".to_string(), payload);
        MsgHandle::with_properties(body)
    }
}

#[async_trait]
impl FlowNodeBehavior for MqttBrokerEmbeddedNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let broker_config = BrokerConfig {
            enabled: true,
            bind: format!("{}:{}", self.config.host, self.config.port),
            max_connections: self.config.max_connections,
            auth: rust_red_mqtt_broker::config::AuthConfig {
                username: self.config.username.clone(),
                password: self.config.password.clone(),
            },
            ..Default::default()
        };

        let broker = Arc::new(MqttBroker::new(broker_config));

        let result = broker.clone().start_with_cancel(stop_token.clone()).await;

        match result {
            Ok(addr) => {
                {
                    let mut ba = self.bound_addr.write().await;
                    *ba = Some(addr);
                }
                log::info!("[mqtt-broker:{}] Listening on {}", self.name(), addr);
                self.report_status(
                    StatusObject {
                        fill: Some(StatusFill::Green),
                        shape: Some(StatusShape::Dot),
                        text: Some(format!("listening :{}", addr.port())),
                    },
                    stop_token.clone(),
                )
                .await;

                // Emit startup event
                let msg = self.make_event_msg(
                    "broker/start",
                    Variant::Object({
                        let mut m = std::collections::BTreeMap::new();
                        m.insert("host".to_string(), Variant::String(self.config.host.clone()));
                        m.insert("port".to_string(), Variant::from(self.config.port as i64));
                        m
                    }),
                );
                let _ = self.fan_out_one(Envelope { port: 0, msg }, stop_token.clone()).await;

                // Periodically emit metrics while running
                loop {
                    tokio::select! {
                        _ = stop_token.cancelled() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                            let metrics = broker.metrics();
                            let mut m = std::collections::BTreeMap::new();
                            m.insert("activeConnections".to_string(), Variant::from(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed) as i64));
                            m.insert("totalConnections".to_string(), Variant::from(metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed) as i64));
                            m.insert("messagesReceived".to_string(), Variant::from(metrics.messages_received.load(std::sync::atomic::Ordering::Relaxed) as i64));
                            m.insert("messagesSent".to_string(), Variant::from(metrics.messages_sent.load(std::sync::atomic::Ordering::Relaxed) as i64));
                            let msg = self.make_event_msg("broker/metrics", Variant::Object(m));
                            let _ = self.fan_out_one(Envelope { port: 0, msg }, stop_token.clone()).await;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("[mqtt-broker:{}] Bind failed: {e}", self.name());
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
            }
        }

        log::info!("[mqtt-broker:{}] Stopped", self.name());
    }
}
