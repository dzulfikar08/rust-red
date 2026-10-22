// Licensed under the Apache License, Version 2.0
// Copyright EdgeLink contributors; RustRED changes (C) Dzulfikar Ats Tsauri
// Based on Node-RED 10-mqtt.js MQTT In node

//! MQTT In Node
//!
//! This node is compatible with Node-RED's MQTT In node. It can:
//! - Subscribe to MQTT topics and receive messages
//! - Support static topic subscription from configuration
//! - Support dynamic topic subscription via input messages
//! - Handle various data types and output formats
//! - Speak MQTT 3.1.1 or MQTT v5, as set by the `protocolVersion` of the broker config node
//!
//! Configuration:
//! - `broker`: Broker configuration node ID
//! - `topic`: Topic to subscribe to (supports wildcards)
//! - `qos`: Quality of Service level (0, 1, or 2)
//! - `datatype`: Output format ("auto-detect", "buffer", "utf8", "base64", "json")
//! - `nl`, `rap`, `rh`, `subscriptionIdentifier`: MQTT v5 subscription options (ignored for MQTT 3.1.1)
//!
//! Dynamic subscription (when inputs=1):
//! - `msg.action`: "subscribe", "unsubscribe", "connect", "disconnect", "getSubscriptions"
//! - `msg.topic`: Topic(s) to subscribe/unsubscribe
//! - `msg.qos`: QoS level for subscription
//!
//! Output message:
//! - `msg.topic`: The topic the message was received on
//! - `msg.payload`: The message payload (converted according to datatype)
//! - `msg.responseTopic`, `msg.correlationData`, `msg.contentType`, `msg.messageExpiryInterval`,
//!   `msg.userProperties`: MQTT v5 publish properties, set when the publisher sent them
//! - `msg.qos`: QoS level of received message
//! - `msg.retain`: Retain flag of received message
//! - `msg._topic`: Original topic (for localhost broker only)
//!
//! Behavior matches Node-RED:
//! - Wildcard topic support (+ and #)
//! - Auto-detection of payload format
//! - JSON parsing when appropriate
//! - Buffer/string conversion based on content

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::mqtt_broker::{build_mqtt_options, build_mqtt_v5_options, qos_to_v5, resolve_broker_settings};
use crate::runtime::model::json::deser::{lenient_opt_bool, lenient_opt_u8, lenient_opt_u32};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(try_from = "serde_json::Value")]
enum MqttQoS {
    #[default]
    AtMost = 0,
    AtLeast = 1,
    Exactly = 2,
}

/// Node-RED saves the QoS as a string (`"2"`), and as an empty string when it is not set.
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
enum MqttDataType {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "auto-detect")]
    #[default]
    AutoDetect,
    #[serde(rename = "buffer")]
    Buffer,
    #[serde(rename = "utf8")]
    Utf8,
    #[serde(rename = "base64")]
    Base64,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct MqttInNodeConfig {
    /// MQTT broker connection ID (reference to broker config node)
    broker: String,

    /// Topic to subscribe to (supports wildcards + and #)
    #[serde(default)]
    topic: String,

    /// QoS level for subscription
    #[serde(default)]
    qos: MqttQoS,

    /// Data type for output
    #[serde(default)]
    datatype: MqttDataType,

    /// Number of inputs (0 = static subscription, 1 = dynamic subscription)
    #[serde(default)]
    inputs: u8,

    /// MQTT v5 subscription identifier
    #[serde(rename = "subscriptionIdentifier", default, deserialize_with = "lenient_opt_u32")]
    subscription_identifier: Option<u32>,

    /// MQTT v5 no local flag
    #[serde(default, deserialize_with = "lenient_opt_bool")]
    nl: Option<bool>,

    /// MQTT v5 retain as published flag
    #[serde(default, deserialize_with = "lenient_opt_bool")]
    rap: Option<bool>,

    /// MQTT v5 retain handling
    #[serde(default, deserialize_with = "lenient_opt_u8")]
    rh: Option<u8>,
}

impl MqttInNodeConfig {
    /// The MQTT v5 subscription options of the node (`nl`, `rap`, `rh`).
    fn v5_filter(&self, topic: &str, qos: MqttQoS) -> rumqttc::v5::mqttbytes::v5::Filter {
        use rumqttc::v5::mqttbytes::v5::RetainForwardRule;
        rumqttc::v5::mqttbytes::v5::Filter {
            path: topic.to_string(),
            qos: qos_to_v5(qos as u8),
            nolocal: self.nl.unwrap_or(false),
            preserve_retain: self.rap.unwrap_or(false),
            retain_forward_rule: match self.rh {
                Some(1) => RetainForwardRule::OnNewSubscribe,
                Some(2) => RetainForwardRule::Never,
                _ => RetainForwardRule::OnEverySubscribe,
            },
        }
    }

    fn v5_subscribe_properties(&self) -> Option<rumqttc::v5::mqttbytes::v5::SubscribeProperties> {
        self.subscription_identifier.filter(|id| *id > 0).map(|id| rumqttc::v5::mqttbytes::v5::SubscribeProperties {
            id: Some(id as usize),
            user_properties: Vec::new(),
        })
    }
}

/// Subscription information for dynamic subscriptions
#[derive(Debug, Clone)]
struct DynamicSubscription {
    topic: String,
    qos: MqttQoS,
    datatype: MqttDataType,
}

/// Client of the connection, MQTT 3.1.1 or MQTT v5 depending on the broker's `protocolVersion`.
#[derive(Clone)]
enum MqttInClient {
    V4(rumqttc::AsyncClient),
    V5(rumqttc::v5::AsyncClient),
}

impl MqttInClient {
    async fn unsubscribe(&self, topic: &str) -> Result<(), String> {
        match self {
            Self::V4(client) => client.unsubscribe(topic).await.map_err(|e| e.to_string()),
            Self::V5(client) => client.unsubscribe(topic).await.map_err(|e| e.to_string()),
        }
    }

    async fn disconnect(&self) -> Result<(), String> {
        match self {
            Self::V4(client) => client.disconnect().await.map_err(|e| e.to_string()),
            Self::V5(client) => client.disconnect().await.map_err(|e| e.to_string()),
        }
    }
}

enum MqttInEventLoop {
    V4(Box<rumqttc::EventLoop>),
    V5(Box<rumqttc::v5::EventLoop>),
}

enum MqttInEvent {
    ConnAck { accepted: bool, detail: String },
    Publish(InboundPublish),
    Other,
}

/// A received publish, independent of the protocol version.
struct InboundPublish {
    topic: String,
    payload: bytes::Bytes,
    qos: u8,
    retain: bool,
    /// Only set for MQTT v5
    properties: Option<rumqttc::v5::mqttbytes::v5::PublishProperties>,
}

impl MqttInEventLoop {
    async fn poll_event(&mut self) -> Result<MqttInEvent, String> {
        match self {
            Self::V4(event_loop) => match event_loop.poll().await.map_err(|e| e.to_string())? {
                rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(connack)) => Ok(MqttInEvent::ConnAck {
                    accepted: connack.code == rumqttc::ConnectReturnCode::Success,
                    detail: format!("{:?}", connack.code),
                }),
                rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) => {
                    Ok(MqttInEvent::Publish(InboundPublish {
                        topic: publish.topic,
                        payload: publish.payload,
                        qos: publish.qos as u8,
                        retain: publish.retain,
                        properties: None,
                    }))
                }
                _ => Ok(MqttInEvent::Other),
            },
            Self::V5(event_loop) => {
                use rumqttc::v5::mqttbytes::v5::{ConnectReturnCode, Packet};
                match event_loop.poll().await.map_err(|e| e.to_string())? {
                    rumqttc::v5::Event::Incoming(Packet::ConnAck(connack)) => Ok(MqttInEvent::ConnAck {
                        accepted: connack.code == ConnectReturnCode::Success,
                        detail: format!("{:?}", connack.code),
                    }),
                    rumqttc::v5::Event::Incoming(Packet::Publish(publish)) => {
                        Ok(MqttInEvent::Publish(InboundPublish {
                            topic: String::from_utf8_lossy(&publish.topic).into_owned(),
                            payload: publish.payload,
                            qos: publish.qos as u8,
                            retain: publish.retain,
                            properties: publish.properties,
                        }))
                    }
                    _ => Ok(MqttInEvent::Other),
                }
            }
        }
    }
}

#[derive(Default)]
struct MqttConnection {
    client: Option<MqttInClient>,
    event_loop: Option<MqttInEventLoop>,
    connected: bool,
}

impl std::fmt::Debug for MqttConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttConnection")
            .field("client", &self.client.is_some())
            .field("event_loop", &self.event_loop.is_some())
            .field("connected", &self.connected)
            .finish()
    }
}

#[derive(Debug)]
#[flow_node("mqtt in", red_name = "mqtt")]
struct MqttInNode {
    base: BaseFlowNodeState,
    config: MqttInNodeConfig,
    connection: Mutex<MqttConnection>,
    dynamic_subscriptions: RwLock<HashMap<String, DynamicSubscription>>,
    /// Whether this node supports dynamic subscriptions
    is_dynamic: bool,
}

impl MqttInNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let mqtt_config = MqttInNodeConfig::deserialize(&config.rest)?;
        let is_dynamic = mqtt_config.inputs == 1;

        let node = MqttInNode {
            base: base_node,
            config: mqtt_config,
            connection: Mutex::new(MqttConnection::default()),
            dynamic_subscriptions: RwLock::new(HashMap::new()),
            is_dynamic,
        };

        Ok(Box::new(node))
    }

    async fn broker_settings(&self) -> crate::Result<MqttBrokerSettings> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;
        resolve_broker_settings(&engine, &self.config.broker).await
    }

    async fn ensure_connection(&self) -> crate::Result<MqttInClient> {
        let mut connection = self.connection.lock().await;

        if connection.connected && connection.client.is_some() {
            return Ok(connection.client.clone().unwrap());
        }

        let settings = self.broker_settings().await?;
        let (host, port) = (settings.host.clone(), settings.port);
        let (client, mut eventloop) = if settings.protocol_version == 5 {
            let options = build_mqtt_v5_options(&settings, "rust_red_in", self.id());
            let (client, eventloop) = rumqttc::v5::AsyncClient::new(options, 100);
            (MqttInClient::V5(client), MqttInEventLoop::V5(Box::new(eventloop)))
        } else {
            let options = build_mqtt_options(&settings, "rust_red_in", self.id());
            let (client, eventloop) = rumqttc::AsyncClient::new(options, 100);
            (MqttInClient::V4(client), MqttInEventLoop::V4(Box::new(eventloop)))
        };

        // Try to connect with timeout
        match timeout(Duration::from_secs(10), async {
            loop {
                match eventloop.poll_event().await {
                    Ok(MqttInEvent::ConnAck { accepted: true, .. }) => {
                        log::info!(
                            "MQTT In connected successfully to {}:{} (MQTT v{})",
                            host,
                            port,
                            if settings.protocol_version == 5 { "5" } else { "3.1.1" }
                        );
                        return Ok(());
                    }
                    Ok(MqttInEvent::ConnAck { detail, .. }) => {
                        log::error!("MQTT In connection failed with code: {detail}");
                        return Err(format!("Connection refused: {detail}"));
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        log::warn!("MQTT In connection error: {e:?}");
                        return Err(format!("Connection error: {e}"));
                    }
                }
            }
        })
        .await
        {
            Ok(Ok(())) => {
                // Subscribe to the configured topic if this is a static subscription
                if !self.is_dynamic && !self.config.topic.is_empty() {
                    if self.is_valid_subscription_topic(&self.config.topic) {
                        if let Err(e) = self.subscribe(&client, &self.config.topic, self.config.qos).await {
                            log::error!("Failed to subscribe to topic '{}': {}", self.config.topic, e);
                        } else {
                            log::info!("Subscribed to topic: {}", self.config.topic);
                        }
                    } else {
                        log::error!("Invalid topic for subscription: '{}'", self.config.topic);
                    }
                }

                connection.client = Some(client.clone());
                connection.event_loop = Some(eventloop);
                connection.connected = true;
                Ok(client)
            }
            Ok(Err(e)) => {
                log::error!("MQTT In connection failed: {e}");
                Err(crate::RustRedError::invalid_operation(&format!("MQTT In connection failed: {e}")))
            }
            Err(_) => {
                log::error!("MQTT In connection timeout after 10 seconds");
                Err(crate::RustRedError::invalid_operation("MQTT In connection timeout"))
            }
        }
    }

    /// Subscribe with the client's protocol version. MQTT v5 also sends the subscription options
    /// (`nl`, `rap`, `rh`) and the subscription identifier from the node config.
    async fn subscribe(&self, client: &MqttInClient, topic: &str, qos: MqttQoS) -> Result<(), String> {
        match client {
            MqttInClient::V4(client) => {
                client.subscribe(topic, self.mqtt_qos_to_rumqttc(qos)).await.map_err(|e| e.to_string())
            }
            MqttInClient::V5(client) => {
                let filter = self.config.v5_filter(topic, qos);
                match self.config.v5_subscribe_properties() {
                    Some(properties) => client.subscribe_many_with_properties([filter], properties).await,
                    None => client.subscribe_many([filter]).await,
                }
                .map_err(|e| e.to_string())
            }
        }
    }

    /// Turn a received publish into a message and send it to the node's output.
    async fn handle_publish(&self, publish: InboundPublish) {
        let mut mqtt_msg = Msg::default();
        mqtt_msg.set("topic".to_string(), Variant::String(publish.topic.clone()));
        mqtt_msg.set("qos".to_string(), Variant::Number(serde_json::Number::from(publish.qos)));
        mqtt_msg.set("retain".to_string(), Variant::Bool(publish.retain));
        let datatype = if self.is_dynamic {
            let subs = self.dynamic_subscriptions.read().await;
            subs.values()
                .find(|sub| Self::topic_matches(&sub.topic, &publish.topic))
                .map(|sub| sub.datatype.clone())
                .unwrap_or(self.config.datatype.clone())
        } else {
            self.config.datatype.clone()
        };
        mqtt_msg.set("payload".to_string(), Self::convert_payload(&publish.payload, &datatype));

        // MQTT v5 properties, named as in Node-RED
        if let Some(properties) = publish.properties {
            if let Some(response_topic) = properties.response_topic {
                mqtt_msg.set("responseTopic".to_string(), Variant::String(response_topic));
            }
            if let Some(correlation_data) = properties.correlation_data {
                mqtt_msg.set("correlationData".to_string(), Variant::Bytes(correlation_data.to_vec()));
            }
            if let Some(content_type) = properties.content_type {
                mqtt_msg.set("contentType".to_string(), Variant::String(content_type));
            }
            if let Some(expiry) = properties.message_expiry_interval {
                mqtt_msg.set("messageExpiryInterval".to_string(), Variant::Number(serde_json::Number::from(expiry)));
            }
            if !properties.user_properties.is_empty() {
                let user_properties =
                    properties.user_properties.into_iter().map(|(k, v)| (k, Variant::String(v))).collect();
                mqtt_msg.set("userProperties".to_string(), Variant::Object(user_properties));
            }
        }

        // Add _topic for localhost broker (Node-RED compatibility)
        mqtt_msg.set("_topic".to_string(), Variant::String(publish.topic));

        if let Err(e) =
            self.fan_out_one(Envelope { port: 0, msg: MsgHandle::new(mqtt_msg) }, CancellationToken::new()).await
        {
            log::warn!("Failed to send MQTT message: {e}");
        }
    }

    /// Validate topic for subscription (allows wildcards)
    fn is_valid_subscription_topic(&self, topic: &str) -> bool {
        if topic.is_empty() {
            return false;
        }

        // Simple validation for MQTT subscription topics
        // Allows wildcards + and # but ensures they are properly placed
        !topic.chars().any(|c| matches!(c, '\x08' | '\x0C' | '\n' | '\r' | '\t' | '\x0B' | '\0'))
    }

    /// Convert internal QoS enum to rumqttc QoS
    fn mqtt_qos_to_rumqttc(&self, qos: MqttQoS) -> rumqttc::QoS {
        match qos {
            MqttQoS::AtMost => rumqttc::QoS::AtMostOnce,
            MqttQoS::AtLeast => rumqttc::QoS::AtLeastOnce,
            MqttQoS::Exactly => rumqttc::QoS::ExactlyOnce,
        }
    }

    /// Whether an MQTT topic filter (with `+` and `#` wildcards) matches a concrete topic.
    fn topic_matches(filter: &str, topic: &str) -> bool {
        let filter_levels: Vec<&str> = filter.split('/').collect();
        let topic_levels: Vec<&str> = topic.split('/').collect();
        let mut ti = 0;
        for (index, level) in filter_levels.iter().enumerate() {
            if *level == "#" {
                return index + 1 == filter_levels.len();
            }
            if ti >= topic_levels.len() || (*level != "+" && *level != topic_levels[ti]) {
                return false;
            }
            ti += 1;
        }
        ti == topic_levels.len()
    }

    /// Convert payload based on data type setting
    fn convert_payload(payload: &[u8], datatype: &MqttDataType) -> Variant {
        match datatype {
            MqttDataType::Buffer => Variant::Bytes(payload.to_vec()),
            MqttDataType::Base64 => {
                use base64::{Engine as _, engine::general_purpose};
                let base64_string = general_purpose::STANDARD.encode(payload);
                Variant::String(base64_string)
            }
            MqttDataType::Utf8 => {
                match std::str::from_utf8(payload) {
                    Ok(s) => Variant::String(s.to_string()),
                    Err(_) => Variant::Bytes(payload.to_vec()), // Fallback to buffer
                }
            }
            MqttDataType::Json => {
                match std::str::from_utf8(payload) {
                    Ok(s) => {
                        match serde_json::from_str::<serde_json::Value>(s) {
                            Ok(json_val) => Self::json_value_to_variant(json_val),
                            Err(_) => Variant::String(s.to_string()), // Fallback to string
                        }
                    }
                    Err(_) => Variant::Bytes(payload.to_vec()), // Fallback to buffer
                }
            }
            MqttDataType::Auto | MqttDataType::AutoDetect => {
                // Auto-detect the best format
                if let Ok(utf8_str) = std::str::from_utf8(payload) {
                    // Try to parse as JSON first
                    if datatype == &MqttDataType::AutoDetect
                        && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(utf8_str)
                    {
                        return Self::json_value_to_variant(json_val);
                    }
                    // Return as string
                    Variant::String(utf8_str.to_string())
                } else {
                    // Not valid UTF-8, return as buffer
                    Variant::Bytes(payload.to_vec())
                }
            }
        }
    }

    /// Convert JSON value to Variant
    fn json_value_to_variant(json_val: serde_json::Value) -> Variant {
        match json_val {
            serde_json::Value::Null => Variant::Null,
            serde_json::Value::Bool(b) => Variant::Bool(b),
            serde_json::Value::Number(n) => Variant::Number(n),
            serde_json::Value::String(s) => Variant::String(s),
            serde_json::Value::Array(arr) => {
                let variants: Vec<Variant> = arr.into_iter().map(Self::json_value_to_variant).collect();
                Variant::Array(variants)
            }
            serde_json::Value::Object(obj) => {
                let map: std::collections::BTreeMap<String, Variant> =
                    obj.into_iter().map(|(k, v)| (k, Self::json_value_to_variant(v))).collect();
                Variant::Object(map)
            }
        }
    }

    /// Handle subscription action
    async fn handle_subscribe_action(
        &self,
        topics: Vec<String>,
        qos_level: Option<u8>,
        datatype: Option<MqttDataType>,
    ) -> crate::Result<()> {
        let client = self.ensure_connection().await?;
        let mut subs = self.dynamic_subscriptions.write().await;

        // Determine datatype once outside the loop
        let dt = datatype.unwrap_or(self.config.datatype.clone());

        for topic in topics {
            if !self.is_valid_subscription_topic(&topic) {
                log::warn!("Invalid subscription topic: '{topic}'");
                continue;
            }

            // Unsubscribe if already subscribed
            if subs.contains_key(&topic)
                && let Err(e) = client.unsubscribe(&topic).await
            {
                log::warn!("Failed to unsubscribe from '{topic}': {e}");
            }

            // Determine QoS
            let qos = match qos_level.unwrap_or(2) {
                0 => MqttQoS::AtMost,
                1 => MqttQoS::AtLeast,
                2 => MqttQoS::Exactly,
                _ => MqttQoS::Exactly, // Default to QoS 2
            };

            // Subscribe
            if let Err(e) = self.subscribe(&client, &topic, qos).await {
                log::error!("Failed to subscribe to '{topic}': {e}");
                continue;
            }

            // Store subscription info
            subs.insert(topic.clone(), DynamicSubscription { topic: topic.clone(), qos, datatype: dt.clone() });

            log::info!("Subscribed to topic: {topic}");
        }

        Ok(())
    }

    /// Handle unsubscribe action
    async fn handle_unsubscribe_action(&self, topics: Vec<String>) -> crate::Result<()> {
        let client = self.ensure_connection().await?;
        let mut subs = self.dynamic_subscriptions.write().await;

        for topic in topics {
            if subs.contains_key(&topic) {
                if let Err(e) = client.unsubscribe(&topic).await {
                    log::warn!("Failed to unsubscribe from '{topic}': {e}");
                } else {
                    log::info!("Unsubscribed from topic: {topic}");
                }
                subs.remove(&topic);
            }
        }

        Ok(())
    }

    /// Handle input message for dynamic subscriptions
    async fn handle_input_message(&self, msg: &Msg) -> crate::Result<Option<Msg>> {
        if let Some(action) = msg.get("action").and_then(|v| v.as_str()) {
            match action {
                "connect" => {
                    // Force reconnection
                    let mut connection = self.connection.lock().await;
                    connection.connected = false;
                    connection.client = None;
                    connection.event_loop = None;
                    drop(connection);

                    self.ensure_connection().await?;
                    log::info!("MQTT In reconnection completed");
                }
                "disconnect" => {
                    let mut connection = self.connection.lock().await;
                    if let Some(client) = connection.client.take() {
                        let _ = client.disconnect().await;
                        log::info!("MQTT In disconnected");
                    }
                    connection.connected = false;
                    connection.event_loop = None;
                }
                "subscribe" => {
                    let topics = self.extract_topics_from_message(msg)?;
                    let qos = msg.get("qos").and_then(|v| v.as_number()).and_then(|n| n.as_u64()).map(|n| n as u8);
                    let datatype = msg.get("datatype").and_then(|v| v.as_str()).and_then(|s| match s {
                        "auto" => Some(MqttDataType::Auto),
                        "auto-detect" => Some(MqttDataType::AutoDetect),
                        "buffer" => Some(MqttDataType::Buffer),
                        "utf8" => Some(MqttDataType::Utf8),
                        "base64" => Some(MqttDataType::Base64),
                        "json" => Some(MqttDataType::Json),
                        _ => None,
                    });

                    self.handle_subscribe_action(topics, qos, datatype).await?;

                    // Return updated subscriptions in message
                    let subs = self.dynamic_subscriptions.read().await;
                    let sub_list: Vec<Variant> = subs
                        .values()
                        .map(|s| {
                            let mut sub_obj = std::collections::BTreeMap::new();
                            sub_obj.insert("topic".to_string(), Variant::String(s.topic.clone()));
                            sub_obj.insert("qos".to_string(), Variant::Number(serde_json::Number::from(s.qos as u8)));
                            Variant::Object(sub_obj)
                        })
                        .collect();

                    let mut response_msg = msg.clone();
                    response_msg.set("subscriptions".to_string(), Variant::Array(sub_list));
                    return Ok(Some(response_msg));
                }
                "unsubscribe" => {
                    let topics = if let Some(Variant::Bool(true)) = msg.get("topic") {
                        // Unsubscribe from all
                        let subs = self.dynamic_subscriptions.read().await;
                        subs.keys().cloned().collect()
                    } else {
                        self.extract_topics_from_message(msg)?
                    };

                    self.handle_unsubscribe_action(topics).await?;

                    // Return updated subscriptions in message
                    let subs = self.dynamic_subscriptions.read().await;
                    let sub_list: Vec<Variant> = subs
                        .values()
                        .map(|s| {
                            let mut sub_obj = std::collections::BTreeMap::new();
                            sub_obj.insert("topic".to_string(), Variant::String(s.topic.clone()));
                            sub_obj.insert("qos".to_string(), Variant::Number(serde_json::Number::from(s.qos as u8)));
                            Variant::Object(sub_obj)
                        })
                        .collect();

                    let mut response_msg = msg.clone();
                    response_msg.set("subscriptions".to_string(), Variant::Array(sub_list));
                    return Ok(Some(response_msg));
                }
                "getSubscriptions" => {
                    let subs = self.dynamic_subscriptions.read().await;
                    let sub_list: Vec<Variant> = subs
                        .values()
                        .map(|s| {
                            let mut sub_obj = std::collections::BTreeMap::new();
                            sub_obj.insert("topic".to_string(), Variant::String(s.topic.clone()));
                            sub_obj.insert("qos".to_string(), Variant::Number(serde_json::Number::from(s.qos as u8)));
                            Variant::Object(sub_obj)
                        })
                        .collect();

                    let mut response_msg = msg.clone();
                    response_msg.set("topic".to_string(), Variant::String("subscriptions".to_string()));
                    response_msg.set("payload".to_string(), Variant::Array(sub_list));
                    return Ok(Some(response_msg));
                }
                _ => {
                    return Err(crate::RustRedError::invalid_operation(&format!(
                        "Invalid MQTT In action: '{action}'. Valid actions are 'connect', 'disconnect', 'subscribe', 'unsubscribe', 'getSubscriptions'"
                    )));
                }
            }
        }

        Ok(None)
    }

    /// Extract topics from message
    fn extract_topics_from_message(&self, msg: &Msg) -> crate::Result<Vec<String>> {
        if let Some(topic_val) = msg.get("topic") {
            match topic_val {
                Variant::String(s) => Ok(vec![s.clone()]),
                Variant::Array(arr) => {
                    let mut topics = Vec::new();
                    for item in arr {
                        if let Variant::String(s) = item {
                            topics.push(s.clone());
                        } else if let Variant::Object(obj) = item
                            && let Some(Variant::String(topic)) = obj.get("topic")
                        {
                            topics.push(topic.clone());
                        }
                    }
                    Ok(topics)
                }
                _ => Err(crate::RustRedError::invalid_operation("Invalid topic format in message")),
            }
        } else {
            Err(crate::RustRedError::invalid_operation("No topic specified in message"))
        }
    }
}

#[async_trait]
impl FlowNodeBehavior for MqttInNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        // For dynamic subscriptions, handle input messages
        if self.is_dynamic {
            let node = self.clone();
            let input_stop_token = stop_token.clone();
            tokio::spawn(async move {
                while !input_stop_token.is_cancelled() {
                    let node = node.clone();

                    with_uow(node.as_ref(), input_stop_token.clone(), |node, msg| async move {
                        let msg_guard = msg.read().await;

                        match node.handle_input_message(&msg_guard).await {
                            Ok(Some(response_msg)) => {
                                // Send response message if action produced one
                                drop(msg_guard);
                                let response_handle = MsgHandle::new(response_msg);
                                node.fan_out_one(Envelope { port: 0, msg: response_handle }, CancellationToken::new())
                                    .await?;
                            }
                            Ok(None) => {
                                // No response message
                            }
                            Err(e) => {
                                log::warn!("MQTT In action error: {e}");
                            }
                        }

                        Ok(())
                    })
                    .await;
                }
            });
        }

        // Main message receiving loop
        let mut event_loop_task: Option<tokio::task::JoinHandle<()>> = None;

        while !stop_token.is_cancelled() {
            // The event loop task ends when the broker drops the connection: start over with a fresh one.
            if event_loop_task.as_ref().is_some_and(|task| task.is_finished()) {
                event_loop_task = None;
                let mut connection = self.connection.lock().await;
                connection.connected = false;
                connection.client = None;
                drop(connection);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            // Ensure we have a connection, unless the broker config asks for manual connects
            let auto_connect = self.broker_settings().await.map(|settings| settings.auto_connect).unwrap_or(true);
            if auto_connect && let Err(e) = self.ensure_connection().await {
                log::error!("Failed to establish MQTT In connection: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Start event loop task if not already running
            if event_loop_task.is_none() {
                let mut connection = self.connection.lock().await;
                if let Some(mut event_loop) = connection.event_loop.take() {
                    let node = self.clone();
                    event_loop_task = Some(tokio::spawn(async move {
                        loop {
                            match event_loop.poll_event().await {
                                Ok(MqttInEvent::Publish(publish)) => node.handle_publish(publish).await,
                                Ok(MqttInEvent::ConnAck { .. } | MqttInEvent::Other) => {}
                                Err(e) => {
                                    log::error!("MQTT event loop error: {e}");
                                    break;
                                }
                            }
                        }
                    }));
                }
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Cleanup
        if let Some(task) = event_loop_task {
            task.abort();
        }

        let mut connection = self.connection.lock().await;
        if let Some(client) = connection.client.take() {
            log::info!("Disconnecting MQTT In client on shutdown");
            let _ = client.disconnect().await;
        }
        connection.connected = false;
        connection.event_loop = None;

        log::debug!("MqttInNode process() task has been terminated.");
    }
}

#[cfg(test)]
mod tests {
    use super::{MqttDataType, MqttInNode, MqttInNodeConfig, MqttQoS};
    use crate::runtime::model::Variant;
    use rumqttc::v5::mqttbytes::v5::RetainForwardRule;
    use serde::Deserialize;

    #[test]
    fn topic_matching_obeys_mqtt_wildcards() {
        assert!(MqttInNode::topic_matches("sensors/+/temperature", "sensors/kitchen/temperature"));
        assert!(!MqttInNode::topic_matches("sensors/+/temperature", "sensors/kitchen/humidity"));
        assert!(MqttInNode::topic_matches("sensors/#", "sensors/kitchen/temperature"));
        assert!(MqttInNode::topic_matches("sensors/#", "sensors"));
        assert!(!MqttInNode::topic_matches("sensors/#/extra", "sensors/kitchen"));
        assert!(!MqttInNode::topic_matches("sensors/kitchen", "sensors/kitchen/temperature"));
    }

    #[test]
    fn payload_conversion_matches_datatypes() {
        assert_eq!(
            MqttInNode::convert_payload(br#"{"value":1}"#, &MqttDataType::Auto),
            Variant::String(r#"{"value":1}"#.into())
        );
        assert!(matches!(
            MqttInNode::convert_payload(br#"{"value":1}"#, &MqttDataType::AutoDetect),
            Variant::Object(_)
        ));
        assert_eq!(MqttInNode::convert_payload(b"abc", &MqttDataType::Utf8), Variant::String("abc".into()));
        assert_eq!(MqttInNode::convert_payload(&[0xff], &MqttDataType::Utf8), Variant::Bytes(vec![0xff]));
    }

    fn config(json: serde_json::Value) -> MqttInNodeConfig {
        MqttInNodeConfig::deserialize(&json).expect("config must load")
    }

    #[test]
    fn v5_subscription_options_come_from_the_stock_node_red_config() {
        // Node-RED saves these as strings
        let cfg = config(serde_json::json!({
            "broker": "b", "topic": "a/+", "qos": "1", "nl": "true", "rap": "true", "rh": "2",
            "subscriptionIdentifier": "7"
        }));
        let filter = cfg.v5_filter("a/+", MqttQoS::AtLeast);
        assert_eq!(filter.path, "a/+");
        assert_eq!(filter.qos, rumqttc::v5::mqttbytes::QoS::AtLeastOnce);
        assert!(filter.nolocal);
        assert!(filter.preserve_retain);
        assert_eq!(filter.retain_forward_rule, RetainForwardRule::Never);
        assert_eq!(cfg.v5_subscribe_properties().and_then(|p| p.id), Some(7));
    }

    #[test]
    fn v5_subscription_options_default_to_plain_subscribe() {
        let cfg = config(serde_json::json!({"broker": "b", "topic": "a", "qos": "0"}));
        let filter = cfg.v5_filter("a", MqttQoS::AtMost);
        assert!(!filter.nolocal);
        assert!(!filter.preserve_retain);
        assert_eq!(filter.retain_forward_rule, RetainForwardRule::OnEverySubscribe);
        assert!(cfg.v5_subscribe_properties().is_none());
    }
}
