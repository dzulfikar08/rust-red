// Licensed under the Apache License, Version 2.0
// Copyright EdgeLink contributors; RustRED changes (C) Dzulfikar Ats Tsauri
// Based on Node-RED 10-mqtt.js MQTT Out node

//! MQTT Out Node
//!
//! This node is compatible with Node-RED's MQTT Out node. It can:
//! - Publish messages to an MQTT broker
//! - Support dynamic topic and QoS from incoming messages
//! - Handle connect/disconnect actions
//! - Validate topics for publishing (no wildcards)
//! - Convert various payload types to MQTT message format
//!
//! Configuration:
//! - `broker`: Broker configuration node ID
//! - `topic`: Default topic (can be overridden by msg.topic)
//! - `qos`: Default QoS level (0, 1, or 2)
//! - `retain`: Default retain flag
//! - MQTT v5 properties (future enhancement)
//!
//! Message properties:
//! - `msg.topic`: Topic to publish to (overrides config)
//! - `msg.payload`: Payload to publish (required unless action is specified)
//! - `msg.qos`: QoS level (overrides config)
//! - `msg.retain`: Retain flag (overrides config)
//! - `msg.action`: Special actions ("connect", "disconnect")
//!
//! Behavior matches Node-RED:
//! - If no payload property exists, message passes through without publishing
//! - Invalid topics generate warnings but don't stop the flow
//! - Supports JSON stringification for objects/arrays
//! - Handles various data types (strings, numbers, buffers, etc.)

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::mqtt_broker::{build_mqtt_options, resolve_broker_settings};
use crate::runtime::model::json::deser::{lenient_bool, lenient_opt_u32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(try_from = "serde_json::Value")]
enum MqttQoS {
    #[default]
    AtMost = 0,
    AtLeast = 1,
    Exactly = 2,
}

/// Node-RED saves the QoS as a string (`"1"`), and as an empty string when it is not set.
impl TryFrom<serde_json::Value> for MqttQoS {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let level = match &value {
            serde_json::Value::Null => Some(0),
            serde_json::Value::Number(n) => n.as_u64(),
            serde_json::Value::String(s) if s.trim().is_empty() => Some(0),
            serde_json::Value::String(s) => s.trim().parse::<u64>().ok(),
            _ => None,
        };
        match level {
            Some(0) => Ok(MqttQoS::AtMost),
            Some(1) => Ok(MqttQoS::AtLeast),
            Some(2) => Ok(MqttQoS::Exactly),
            _ => Err(format!("invalid QoS {value}")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct MqttOutNodeConfig {
    /// MQTT broker connection ID (reference to broker config node)
    broker: String,

    /// Default topic to publish to (can be overridden by message)
    #[serde(default)]
    topic: String,

    /// Default QoS level
    #[serde(default)]
    qos: MqttQoS,

    /// Default retain flag (Node-RED saves it as `""`, `"true"` or `"false"`)
    #[serde(default, deserialize_with = "lenient_bool")]
    retain: bool,

    /// Response topic for MQTT v5
    #[serde(rename = "respTopic", default)]
    response_topic: String,

    /// Correlation data for MQTT v5
    #[serde(rename = "correl", default)]
    correlation_data: String,

    /// Content type for MQTT v5
    #[serde(rename = "contentType", default)]
    content_type: String,

    /// Message expiry interval for MQTT v5
    #[serde(rename = "expiry", default, deserialize_with = "lenient_opt_u32")]
    message_expiry_interval: Option<u32>,

    /// User properties for MQTT v5 (JSON string)
    #[serde(rename = "userProps", default)]
    user_properties: String,
}

#[derive(Default)]
struct MqttConnection {
    client: Option<rumqttc::AsyncClient>,
    /// Stops the task that drives the connection (`rumqttc` only sends and receives while it is polled).
    cancel: Option<CancellationToken>,
    connected: bool,
}

impl MqttConnection {
    fn stop_event_loop(&mut self) {
        if let Some(token) = self.cancel.take() {
            token.cancel();
        }
    }
}

impl std::fmt::Debug for MqttConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttConnection")
            .field("client", &self.client.is_some())
            .field("event_loop", &self.cancel.is_some())
            .field("connected", &self.connected)
            .finish()
    }
}

#[derive(Debug)]
#[flow_node("mqtt out", red_name = "mqtt")]
struct MqttOutNode {
    base: BaseFlowNodeState,
    config: MqttOutNodeConfig,
    connection: Mutex<MqttConnection>,
}

impl MqttOutNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let mqtt_config = MqttOutNodeConfig::deserialize(&config.rest)?;

        let node =
            MqttOutNode { base: base_node, config: mqtt_config, connection: Mutex::new(MqttConnection::default()) };

        Ok(Box::new(node))
    }

    async fn broker_settings(&self) -> crate::Result<MqttBrokerSettings> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;
        resolve_broker_settings(&engine, &self.config.broker).await
    }

    async fn ensure_connection(&self) -> crate::Result<()> {
        let mut connection = self.connection.lock().await;

        if connection.connected && connection.client.is_some() {
            return Ok(());
        }

        let settings = self.broker_settings().await?;
        let (host, port) = (settings.host.clone(), settings.port);
        let mqttoptions = build_mqtt_options(&settings, "rust_red_out", self.id());

        let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 10);

        // Try to connect with timeout - similar to Node-RED's connection logic
        match timeout(Duration::from_secs(10), async {
            loop {
                match eventloop.poll().await {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(connack))) => {
                        if connack.code == rumqttc::ConnectReturnCode::Success {
                            log::info!("MQTT connected successfully to {}:{}", host, port);
                            return Ok(());
                        } else {
                            log::error!("MQTT connection failed with code: {:?}", connack.code);
                            return Err(format!("Connection refused: {:?}", connack.code));
                        }
                    }
                    Ok(rumqttc::Event::Outgoing(_)) => {
                        log::debug!("MQTT outgoing packet sent");
                        continue;
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        log::warn!("MQTT connection error: {e:?}");
                        return Err(format!("Connection error: {e}"));
                    }
                }
            }
        })
        .await
        {
            Ok(Ok(())) => {
                connection.client = Some(client);
                connection.connected = true;

                // Keep driving the connection: without polling, nothing is ever published and the
                // client reconnects after a broker restart only while it is polled.
                let cancel = CancellationToken::new();
                connection.cancel = Some(cancel.clone());
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            result = eventloop.poll() => {
                                if let Err(error) = result {
                                    log::warn!("MQTT out connection error, retrying: {error}");
                                    tokio::select! {
                                        _ = cancel.cancelled() => break,
                                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                                    }
                                }
                            }
                        }
                    }
                });
                Ok(())
            }
            Ok(Err(e)) => {
                log::error!("MQTT connection failed: {e}");
                Err(crate::RustRedError::invalid_operation(&format!("MQTT connection failed: {e}")))
            }
            Err(_) => {
                log::error!("MQTT connection timeout after 10 seconds");
                Err(crate::RustRedError::invalid_operation("MQTT connection timeout"))
            }
        }
    }

    async fn publish_message(&self, msg: &Msg) -> crate::Result<()> {
        self.ensure_connection().await?;

        let connection = self.connection.lock().await;
        let client = connection
            .client
            .as_ref()
            .ok_or_else(|| crate::RustRedError::invalid_operation("MQTT client not available"))?;

        // Like Node-RED: a topic set on the node wins, msg.topic is used only when the node has none.
        let topic = if !self.config.topic.is_empty() {
            self.config.topic.clone()
        } else {
            msg.get("topic").and_then(|v| v.as_str()).unwrap_or_default().to_string()
        };

        if topic.is_empty() {
            return Err(crate::RustRedError::invalid_operation("No topic specified for MQTT publish"));
        }

        // Validate topic for publishing (no wildcards allowed, no control characters)
        if !self.is_valid_publish_topic(&topic) {
            return Err(crate::RustRedError::invalid_operation(&format!("Invalid topic for publishing: '{topic}'")));
        }

        // Get QoS from message or config (message overrides config)
        let qos = if let Some(qos_from_msg) = msg.get("qos") {
            self.parse_qos(qos_from_msg)?
        } else {
            match self.config.qos {
                MqttQoS::AtMost => rumqttc::QoS::AtMostOnce,
                MqttQoS::AtLeast => rumqttc::QoS::AtLeastOnce,
                MqttQoS::Exactly => rumqttc::QoS::ExactlyOnce,
            }
        };

        // Get retain flag from message or config (message overrides config)
        let retain = if let Some(retain_from_msg) = msg.get("retain") {
            self.parse_retain(retain_from_msg)
        } else {
            self.config.retain
        };

        // Check if payload exists - if not specified, pass through without publishing
        let payload = msg.get("payload");
        if payload.is_none() {
            // Node-RED behavior: if no payload property, just pass message through
            return Ok(());
        }

        let payload = payload.unwrap();

        // Convert payload to bytes following Node-RED conversion rules
        let payload_bytes = self.convert_payload_to_bytes(payload)?;

        // Build MQTT v5 properties if needed (for future MQTT v5 support)
        // For now, we use rumqttc which primarily supports MQTT v3.1.1

        // Publish the message
        client
            .publish(topic, qos, retain, payload_bytes)
            .await
            .map_err(|e| crate::RustRedError::invalid_operation(&format!("MQTT publish failed: {e}")))?;

        Ok(())
    }

    /// Validate topic for publishing (similar to Node-RED's isValidPublishTopic)
    fn is_valid_publish_topic(&self, topic: &str) -> bool {
        if topic.is_empty() {
            return false;
        }

        // Check for wildcards and control characters
        !topic.chars().any(|c| matches!(c, '+' | '#' | '\x08' | '\x0C' | '\n' | '\r' | '\t' | '\x0B' | '\0'))
    }

    /// Parse QoS value from Variant (supports numbers and strings)
    fn parse_qos(&self, qos_val: &Variant) -> crate::Result<rumqttc::QoS> {
        match qos_val {
            Variant::Number(n) => match n.as_u64() {
                Some(0) => Ok(rumqttc::QoS::AtMostOnce),
                Some(1) => Ok(rumqttc::QoS::AtLeastOnce),
                Some(2) => Ok(rumqttc::QoS::ExactlyOnce),
                _ => {
                    log::warn!("Invalid QoS value: {n}, using default 0");
                    Ok(rumqttc::QoS::AtMostOnce)
                }
            },
            Variant::String(s) => match s.as_str() {
                "0" => Ok(rumqttc::QoS::AtMostOnce),
                "1" => Ok(rumqttc::QoS::AtLeastOnce),
                "2" => Ok(rumqttc::QoS::ExactlyOnce),
                _ => {
                    log::warn!("Invalid QoS string: '{s}', using default 0");
                    Ok(rumqttc::QoS::AtMostOnce)
                }
            },
            _ => {
                log::warn!("Invalid QoS type: {qos_val:?}, using default 0");
                Ok(rumqttc::QoS::AtMostOnce)
            }
        }
    }

    /// Parse retain flag from Variant (supports booleans and strings)
    fn parse_retain(&self, retain_val: &Variant) -> bool {
        match retain_val {
            Variant::Bool(b) => *b,
            Variant::String(s) => s == "true",
            Variant::Number(n) => n.as_u64().unwrap_or(0) != 0,
            _ => false,
        }
    }

    /// Convert payload to bytes following Node-RED rules
    fn convert_payload_to_bytes(&self, payload: &Variant) -> crate::Result<Vec<u8>> {
        match payload {
            Variant::Null => Ok(Vec::new()),
            Variant::String(s) => Ok(s.as_bytes().to_vec()),
            Variant::Bytes(bytes) => Ok(bytes.clone()),
            Variant::Number(n) => Ok(n.to_string().into_bytes()),
            Variant::Bool(b) => Ok(b.to_string().into_bytes()),
            Variant::Object(_) | Variant::Array(_) => {
                // For objects and arrays, stringify to JSON
                serde_json::to_vec(payload)
                    .map_err(|e| crate::RustRedError::invalid_operation(&format!("Failed to serialize payload: {e}")))
            }
            Variant::Date(d) => {
                // Convert SystemTime to ISO 8601 string
                match d.duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => {
                        let timestamp = duration.as_secs();
                        Ok(format!("{timestamp}Z").into_bytes())
                    }
                    Err(_) => Ok("Invalid Date".to_string().into_bytes()),
                }
            }
            Variant::Regexp(r) => Ok(format!("/{r}/").into_bytes()),
        }
    }

    async fn handle_action(&self, msg: &Msg) -> crate::Result<()> {
        if let Some(action) = msg.get("action").and_then(|v| v.as_str()) {
            match action {
                "connect" => {
                    // Handle connect action - similar to Node-RED handleConnectAction
                    if let Some(_broker_data) = msg.get("broker") {
                        // TODO: Handle dynamic broker configuration from msg.broker
                        // For now, we just force reconnection with existing config
                        log::info!("MQTT connect action with broker override (not implemented yet)");
                    }

                    // Check if we can connect
                    let connection = self.connection.lock().await;
                    let already_connected = connection.connected;
                    drop(connection);

                    if !already_connected {
                        // Not currently connected - trigger the connect
                        self.ensure_connection().await?;
                        log::info!("MQTT connection established");
                    } else {
                        // Already connected - check for force flag
                        if let Some(force) = msg.get("force").and_then(|v| v.as_bool()) {
                            if force {
                                // Force reconnection
                                let mut connection = self.connection.lock().await;
                                if let Some(client) = connection.client.take() {
                                    let _ = client.disconnect().await;
                                }
                                connection.connected = false;
                                connection.stop_event_loop();
                                drop(connection);

                                self.ensure_connection().await?;
                                log::info!("MQTT forced reconnection completed");
                            } else {
                                log::info!("MQTT already connected, no force flag");
                            }
                        } else {
                            log::info!("MQTT already connected");
                        }
                    }
                }
                "disconnect" => {
                    // Handle disconnect action - similar to Node-RED handleDisconnectAction
                    let mut connection = self.connection.lock().await;
                    if let Some(client) = connection.client.take() {
                        let _ = client.disconnect().await;
                        log::info!("MQTT disconnected");
                    }
                    connection.connected = false;
                    connection.stop_event_loop();
                }
                _ => {
                    return Err(crate::RustRedError::invalid_operation(&format!(
                        "Invalid MQTT action: '{action}'. Valid actions are 'connect' and 'disconnect'"
                    )));
                }
            }
            Ok(())
        } else {
            // No action, this is a publish request - follow Node-RED doPublish logic
            self.publish_message(msg).await
        }
    }
}

#[async_trait]
impl FlowNodeBehavior for MqttOutNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        while !stop_token.is_cancelled() {
            let node = self.clone();

            with_uow(node.as_ref(), stop_token.clone(), |node, msg| async move {
                let msg_guard = msg.read().await;

                match node.handle_action(&msg_guard).await {
                    Ok(()) => {
                        // Success - message handled
                        Ok(())
                    }
                    Err(e) => {
                        // Log the error but don't propagate it to avoid stopping the flow
                        // This matches Node-RED behavior where MQTT errors are logged but don't break the flow
                        log::warn!("MQTT out node error: {e}");

                        // Check if this is a warning (like invalid topic) vs a real error
                        if e.to_string().contains("Invalid topic") || e.to_string().contains("Invalid MQTT action") {
                            // These are warnings in Node-RED, not hard errors
                            log::debug!("MQTT warning: {e}");
                        }

                        // Always return Ok to continue processing
                        Ok(())
                    }
                }
            })
            .await;
        }

        // Cleanup connection on shutdown
        let mut connection = self.connection.lock().await;
        if let Some(client) = connection.client.take() {
            log::info!("Disconnecting MQTT client on shutdown");
            let _ = client.disconnect().await;
        }
        connection.connected = false;
        connection.stop_event_loop();

        log::debug!("MqttOutNode process() task has been terminated.");
    }
}
