// Licensed under the Apache License, Version 2.0
// Copyright EdgeLink contributors; RustRED changes (C) Dzulfikar Ats Tsauri
// Based on Node-RED 10-mqtt.js MQTT Broker Config node

//! MQTT Broker Config Node
//!
//! Compatible with the stock Node-RED `mqtt-broker` config node.
//!
//! Node-RED stores the broker as separate `broker`, `port` and `usetls` properties, and
//! most numbers and booleans as strings (`"1883"`, `"false"`). This node accepts that
//! shape, and also the `url` form (`mqtt://host:1883`) used by flows written for RustRED.
//! When `url` is absent the URL is built from `broker`, `port` and `usetls`, the same way
//! Node-RED builds its `brokerurl`.
//!
//! Credentials are read from a `credentials` object (`user`, `password`) or from top level
//! `user`/`username`/`password` properties. The encrypted `flows_cred.json` is not read.
//!
//! The settings design (a `mqtt_settings()` hook on the global node trait) follows the
//! upstream EdgeLinkd project.

use serde::Deserialize;
use std::sync::Arc;

use crate::runtime::model::json::deser::{lenient_opt_bool, lenient_opt_string, lenient_opt_u8, lenient_opt_u16};
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rumqttc::{AsyncClient, EventLoop, LastWill, MqttOptions, QoS, Transport};
use rust_red_macro::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default, Deserialize)]
struct MqttBrokerCredentials {
    #[serde(default, alias = "username", deserialize_with = "lenient_opt_string")]
    user: Option<String>,
    #[serde(default, deserialize_with = "lenient_opt_string")]
    password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct MqttBrokerConfig {
    /// Broker URL (e.g., mqtt://localhost:1883). Optional, see `broker` and `port`.
    #[serde(default)]
    url: String,

    /// Node-RED stores host and port separately.
    #[serde(default, deserialize_with = "lenient_opt_string")]
    broker: Option<String>,
    #[serde(default, deserialize_with = "lenient_opt_u16")]
    port: Option<u16>,

    /// Client ID (optional, auto-generate if empty)
    #[serde(default, deserialize_with = "lenient_opt_string")]
    clientid: Option<String>,

    /// Username (optional)
    #[serde(default, alias = "user", deserialize_with = "lenient_opt_string")]
    username: Option<String>,

    /// Password (optional)
    #[serde(default, deserialize_with = "lenient_opt_string")]
    password: Option<String>,

    /// Credentials object as exported by Node-RED
    #[serde(default)]
    credentials: Option<MqttBrokerCredentials>,

    /// Use TLS/SSL (Node-RED `usetls`)
    #[serde(default, deserialize_with = "lenient_opt_bool")]
    usetls: Option<bool>,

    /// Id of a `tls-config` node (Node-RED `tls`)
    #[serde(default, deserialize_with = "lenient_opt_string")]
    tls: Option<String>,

    /// Keepalive interval (seconds)
    #[serde(default, deserialize_with = "lenient_opt_u16")]
    keepalive: Option<u16>,

    /// Clean session flag (Node-RED `cleansession`)
    #[serde(default, alias = "cleansession", deserialize_with = "lenient_opt_bool")]
    clean: Option<bool>,

    #[serde(default, rename = "autoConnect", deserialize_with = "lenient_opt_bool")]
    auto_connect: Option<bool>,

    #[serde(default, rename = "protocolVersion", deserialize_with = "lenient_opt_u8")]
    protocol_version: Option<u8>,

    /// Reconnect period (ms)
    #[serde(default = "default_reconnect_period")]
    reconnect_period: u64,

    /// Connect timeout (ms)
    #[serde(default = "default_connect_timeout")]
    connect_timeout: u64,

    /// Will topic (optional)
    #[serde(default, alias = "willTopic", deserialize_with = "lenient_opt_string")]
    will_topic: Option<String>,

    /// Will payload (optional)
    #[serde(default, alias = "willPayload", deserialize_with = "lenient_opt_string")]
    will_payload: Option<String>,

    /// Will QoS
    #[serde(default, alias = "willQos", deserialize_with = "lenient_opt_u8")]
    will_qos: Option<u8>,

    /// Will retain
    #[serde(default, alias = "willRetain", deserialize_with = "lenient_opt_bool")]
    will_retain: Option<bool>,
}

fn default_reconnect_period() -> u64 {
    5000
}

fn default_connect_timeout() -> u64 {
    30000
}

/// Split a broker URL into `(host, port, tls)`. Schemes `mqtt`, `tcp`, `mqtts`, `ssl` and `tls` are understood.
fn split_broker_url(url: &str) -> (String, u16, bool) {
    let (scheme, rest) = url.split_once("://").unwrap_or(("mqtt", url));
    let tls = matches!(scheme, "mqtts" | "ssl" | "tls");
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map_or(authority, |(_, host)| host);

    let (host, port) = if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal: [::1]:1883
        match stripped.split_once(']') {
            Some((host, tail)) => (host, tail.strip_prefix(':').and_then(|p| p.parse::<u16>().ok())),
            None => (stripped, None),
        }
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => match port.parse::<u16>() {
                Ok(port) => (host, Some(port)),
                Err(_) => (authority, None),
            },
            None => (authority, None),
        }
    };

    let host = if host.is_empty() { "localhost" } else { host };
    (host.to_string(), port.unwrap_or(if tls { 8883 } else { 1883 }), tls)
}

impl MqttBrokerConfig {
    /// The broker URL. Built from `broker`, `port` and `usetls` when `url` is empty,
    /// following Node-RED's own construction of `brokerurl`.
    fn resolved_url(&self) -> String {
        if !self.url.is_empty() {
            return self.url.clone();
        }
        match self.broker.as_deref() {
            Some(broker) if broker.contains("://") => broker.to_string(),
            Some(broker) => {
                let scheme = if self.usetls.unwrap_or(false) { "mqtts" } else { "mqtt" };
                let host = if broker.contains(':') && !broker.starts_with('[') {
                    format!("[{broker}]")
                } else {
                    broker.to_string()
                };
                format!("{scheme}://{host}:{}", self.port.unwrap_or(1883))
            }
            None => "mqtt://localhost:1883".to_string(),
        }
    }

    fn settings(&self) -> MqttBrokerSettings {
        let (host, port, url_tls) = split_broker_url(&self.resolved_url());
        let tls_node = self.tls.as_deref().is_some_and(|t| !t.is_empty());
        let credentials = self.credentials.clone().unwrap_or_default();
        MqttBrokerSettings {
            host,
            port,
            username: self.username.clone().or(credentials.user).filter(|u| !u.is_empty()),
            password: self.password.clone().or(credentials.password),
            tls: self.usetls.unwrap_or(url_tls || tls_node),
            keepalive: self.keepalive.filter(|k| *k > 0).unwrap_or(60),
            clean: self.clean.unwrap_or(true),
            client_id: self.clientid.clone(),
            auto_connect: self.auto_connect.unwrap_or(true),
            protocol_version: self.protocol_version.unwrap_or(4),
            will_topic: self.will_topic.clone(),
            will_payload: self.will_payload.clone(),
            will_qos: self.will_qos.unwrap_or(0),
            will_retain: self.will_retain.unwrap_or(false),
        }
    }
}

fn qos_from_u8(qos: u8) -> QoS {
    match qos {
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    }
}

/// Build `rumqttc` options from broker settings.
///
/// Node-RED shares one connection between all nodes of a broker config, while here every
/// `mqtt in`/`mqtt out` node connects on its own. A configured client id is therefore
/// suffixed with the role and the node id, otherwise the broker would drop the earlier
/// connection each time another node connects with the same id. Without a configured id,
/// a random `{id_prefix}_{random}` id is used.
///
/// MQTT v5 (`protocolVersion` 5) is accepted in the config, but the connection is made
/// with MQTT 3.1.1, which every v5 broker also speaks.
pub(super) fn build_mqtt_options(settings: &MqttBrokerSettings, id_prefix: &str, node_id: ElementId) -> MqttOptions {
    let client_id = match &settings.client_id {
        Some(configured) => format!("{configured}-{id_prefix}-{node_id}"),
        None => format!("{id_prefix}_{}", &uuid::Uuid::new_v4().to_string()[..8]),
    };
    let mut opts = MqttOptions::new(client_id, settings.host.clone(), settings.port);
    opts.set_keep_alive(Duration::from_secs(settings.keepalive as u64));
    opts.set_clean_session(settings.clean);

    if let Some(username) = &settings.username {
        opts.set_credentials(username, settings.password.as_deref().unwrap_or(""));
    }

    if settings.tls {
        opts.set_transport(Transport::tls_with_default_config());
    }

    if let Some(will_topic) = &settings.will_topic
        && !will_topic.is_empty()
    {
        opts.set_last_will(LastWill {
            topic: will_topic.clone(),
            message: settings.will_payload.clone().unwrap_or_default().into_bytes().into(),
            qos: qos_from_u8(settings.will_qos),
            retain: settings.will_retain,
        });
    }

    opts
}

/// Resolve a broker reference from an `mqtt in`/`mqtt out` node into connection settings.
pub(super) async fn resolve_broker_settings(
    engine: &crate::runtime::engine::Engine,
    reference: &str,
) -> crate::Result<MqttBrokerSettings> {
    let global = engine
        .find_global_node_by_ref(reference)
        .ok_or_else(|| anyhow::anyhow!("Broker config '{reference}' not found, or it failed to load"))?;

    if let Some(settings) = global.mqtt_settings() {
        return Ok(settings);
    }

    // In-process broker: use the address it actually bound to, if it is already running.
    if let Some(embedded) = global.as_any().downcast_ref::<super::mqtt_broker_embedded::MqttBrokerEmbeddedNode>() {
        if let Some(addr) = *embedded.bound_addr.read().await {
            return Ok(MqttBrokerSettings::plain(addr.ip().to_string(), addr.port()));
        }
        let (host, port) = embedded.configured_addr();
        return Ok(MqttBrokerSettings::plain(host, port));
    }

    Err(anyhow::anyhow!("Broker config '{reference}' is not an mqtt-broker node"))
}

/// MQTT Broker Node
/// Implements the Node-RED compatible MQTT Broker node logic.
#[global_node("mqtt-broker", red_name = "mqtt-broker", module = "node-red")]
#[allow(dead_code)]
pub struct MqttBrokerNode {
    /// Base node state (Node-RED global node pattern)
    base: BaseGlobalNodeState,
    /// Broker configuration
    config: MqttBrokerConfig,
    /// MQTT client pool (keyed by clientId)
    clients: Arc<Mutex<HashMap<String, AsyncClient>>>,
    /// Event loop pool (keyed by clientId)
    event_loops: Arc<Mutex<HashMap<String, EventLoop>>>,
    /// Subscription management (topic -> clientId)
    subscriptions: Arc<Mutex<HashMap<String, String>>>,
    /// Connection state (true if connected)
    connected: Arc<Mutex<bool>>,
}

#[allow(dead_code)]
impl MqttBrokerNode {
    /// Build a new MQTT Broker node from config
    pub fn build(
        engine: &crate::runtime::engine::Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let broker_config = MqttBrokerConfig::deserialize(&config.rest)?;
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "mqtt-broker",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        let node = MqttBrokerNode {
            base: state,
            config: broker_config,
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_loops: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(false)),
        };
        Ok(Box::new(node))
    }

    /// Connect to the MQTT broker and store the client in the pool
    pub async fn connect(&self) -> crate::Result<()> {
        let opts = build_mqtt_options(&self.config.settings(), "nodered", self.base.id);

        let client_id = opts.client_id().to_string();
        let (client, mut event_loop) = AsyncClient::new(opts, 100);
        // Set connection timeout directly on event_loop's network options
        event_loop.network_options.set_connection_timeout(Duration::from_millis(self.config.connect_timeout).as_secs());

        // Spawn event loop in background (do not store event_loop after move)
        let connected = self.connected.clone();
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(connack))) => {
                        if connack.code == rumqttc::ConnectReturnCode::Success {
                            let mut conn = connected.lock().await;
                            *conn = true;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("MQTT event loop error: {e}");
                        let mut conn = connected.lock().await;
                        *conn = false;
                        break;
                    }
                }
            }
        });

        // Store client only
        self.clients.lock().await.insert(client_id, client);
        Ok(())
    }

    /// Disconnect all clients
    pub async fn disconnect(&self) -> crate::Result<()> {
        let mut clients = self.clients.lock().await;
        for (_id, client) in clients.iter() {
            let _ = client.disconnect().await;
        }
        clients.clear();
        let mut conn = self.connected.lock().await;
        *conn = false;
        Ok(())
    }

    /// Reconnect logic: disconnect then connect
    pub async fn reconnect(&self) -> crate::Result<()> {
        self.disconnect().await?;
        self.connect().await
    }

    /// Check if broker is connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.lock().await
    }

    /// Get a client by clientId (for use by mqtt_in/mqtt_out)
    pub async fn get_client(&self, client_id: &str) -> Option<AsyncClient> {
        self.clients.lock().await.get(client_id).cloned()
    }

    /// Add a subscription (topic -> clientId)
    pub async fn add_subscription(&self, topic: &str, client_id: &str) {
        self.subscriptions.lock().await.insert(topic.to_string(), client_id.to_string());
    }

    /// Remove a subscription
    pub async fn remove_subscription(&self, topic: &str) {
        self.subscriptions.lock().await.remove(topic);
    }

    /// List all subscriptions
    pub async fn list_subscriptions(&self) -> Vec<String> {
        self.subscriptions.lock().await.keys().cloned().collect()
    }
}

#[async_trait::async_trait]
impl GlobalNodeBehavior for MqttBrokerNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }

    fn mqtt_settings(&self) -> Option<MqttBrokerSettings> {
        Some(self.config.settings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(json: serde_json::Value) -> MqttBrokerConfig {
        MqttBrokerConfig::deserialize(&json).expect("config must load")
    }

    #[test]
    fn stock_node_red_config_loads_without_url() {
        let settings = config(serde_json::json!({
            "name": "local", "broker": "broker.example.com", "port": "1884", "clientid": "plc-1",
            "autoConnect": true, "usetls": false, "protocolVersion": "4", "keepalive": "30",
            "cleansession": false, "birthTopic": "", "willTopic": "", "willQos": "0", "willRetain": "false",
            "credentials": {"user": "alice", "password": "secret"}
        }))
        .settings();
        assert_eq!(settings.host, "broker.example.com");
        assert_eq!(settings.port, 1884);
        assert_eq!(settings.client_id.as_deref(), Some("plc-1"));
        assert_eq!(settings.keepalive, 30);
        assert!(!settings.clean);
        assert!(!settings.tls);
        assert_eq!(settings.protocol_version, 4);
        assert_eq!(settings.username.as_deref(), Some("alice"));
        assert_eq!(settings.password.as_deref(), Some("secret"));
        assert!(settings.will_topic.is_none());
    }

    #[test]
    fn url_is_built_from_broker_port_and_tls() {
        let cfg = config(serde_json::json!({"broker": "example.org", "port": 8883, "usetls": "true"}));
        assert_eq!(cfg.resolved_url(), "mqtts://example.org:8883");
        let settings = cfg.settings();
        assert!(settings.tls);
        assert_eq!((settings.host.as_str(), settings.port), ("example.org", 8883));

        let defaults = config(serde_json::json!({"broker": "example.org"}));
        assert_eq!(defaults.resolved_url(), "mqtt://example.org:1883");
        assert_eq!(config(serde_json::json!({})).resolved_url(), "mqtt://localhost:1883");
    }

    #[test]
    fn explicit_url_wins_and_top_level_credentials_work() {
        let settings = config(serde_json::json!({
            "url": "mqtts://secure.example.com:9999", "user": "bob", "password": "pw", "protocolVersion": 5
        }))
        .settings();
        assert_eq!((settings.host.as_str(), settings.port, settings.tls), ("secure.example.com", 9999, true));
        assert_eq!(settings.username.as_deref(), Some("bob"));
        assert_eq!(settings.protocol_version, 5);
    }

    #[test]
    fn ipv6_and_default_ports_are_parsed() {
        assert_eq!(split_broker_url("mqtt://[::1]:1999"), ("::1".to_string(), 1999, false));
        assert_eq!(split_broker_url("mqtts://host"), ("host".to_string(), 8883, true));
        assert_eq!(split_broker_url("host:1883"), ("host".to_string(), 1883, false));
    }

    #[test]
    fn garbage_port_is_an_error_not_a_panic() {
        assert!(MqttBrokerConfig::deserialize(&serde_json::json!({"broker": "h", "port": "abc"})).is_err());
    }
}
