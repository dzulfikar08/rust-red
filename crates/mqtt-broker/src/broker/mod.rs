pub mod retained;
pub mod session;
pub mod subscription;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::config::BrokerConfig;
use crate::error::{BrokerError, BrokerResult};
use crate::protocol::codec;
use crate::protocol::packets::*;

use session::next_packet_id;

#[derive(Debug)]
pub struct BrokerMetrics {
    pub active_connections: AtomicU64,
    pub total_connections: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub subscriptions_count: AtomicU64,
}

impl Default for BrokerMetrics {
    fn default() -> Self {
        Self {
            active_connections: AtomicU64::new(0),
            total_connections: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            subscriptions_count: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub client_id: String,
    pub keep_alive: u16,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubscriptionInfo {
    pub topic_filter: String,
    pub client_id: String,
    pub qos: u8,
}

pub struct MqttBroker {
    config: BrokerConfig,
    sessions: Arc<RwLock<HashMap<String, Arc<session::Session>>>>,
    topic_trie: Arc<RwLock<subscription::TopicTrie>>,
    retained_store: Arc<RwLock<retained::RetainedStore>>,
    metrics: Arc<BrokerMetrics>,
}

impl MqttBroker {
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            topic_trie: Arc::new(RwLock::new(subscription::TopicTrie::new())),
            retained_store: Arc::new(RwLock::new(retained::RetainedStore::new())),
            metrics: Arc::new(BrokerMetrics::default()),
        }
    }

    pub fn metrics(&self) -> &Arc<BrokerMetrics> {
        &self.metrics
    }

    /// Start the broker, listening for TCP connections.
    /// This method runs indefinitely until the process is terminated.
    pub async fn start(self: Arc<Self>) -> BrokerResult<()> {
        let addr: SocketAddr =
            self.config.bind.parse().map_err(|e: std::net::AddrParseError| {
                BrokerError::ProtocolError(format!("Invalid bind address: {e}"))
            })?;
        let listener =
            TcpListener::bind(addr).await.map_err(|e| BrokerError::ProtocolError(format!("Failed to bind: {e}")))?;
        log::info!("[mqtt-broker] Listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let broker = self.clone();
                    self.metrics.total_connections.fetch_add(1, Ordering::Relaxed);
                    self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);
                    tokio::spawn(async move {
                        log::debug!("[mqtt-broker] New connection from {}", peer);
                        if let Err(e) = broker.handle_connection(stream).await {
                            if !e.to_string().contains("ConnectionClosed") {
                                log::warn!("[mqtt-broker] Connection error from {}: {}", peer, e);
                            }
                        }
                        broker.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(e) => {
                    log::error!("[mqtt-broker] Accept error: {}", e);
                }
            }
        }
    }

    /// Start the broker in a background task and return the bound address.
    /// Useful for testing with port 0 (OS-assigned port).
    pub async fn start_background(self: Arc<Self>) -> BrokerResult<SocketAddr> {
        let addr: SocketAddr =
            self.config.bind.parse().map_err(|e: std::net::AddrParseError| {
                BrokerError::ProtocolError(format!("Invalid bind address: {e}"))
            })?;
        let listener =
            TcpListener::bind(addr).await.map_err(|e| BrokerError::ProtocolError(format!("Failed to bind: {e}")))?;
        let local_addr = listener.local_addr().map_err(BrokerError::Io)?;
        log::info!("[mqtt-broker] Listening on {}", local_addr);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        let broker = self.clone();
                        self.metrics.total_connections.fetch_add(1, Ordering::Relaxed);
                        self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);
                        tokio::spawn(async move {
                            log::debug!("[mqtt-broker] New connection from {}", peer);
                            if let Err(e) = broker.handle_connection(stream).await {
                                if !e.to_string().contains("ConnectionClosed") {
                                    log::warn!("[mqtt-broker] Connection error from {}: {}", peer, e);
                                }
                            }
                            broker.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => {
                        log::error!("[mqtt-broker] Accept error: {}", e);
                    }
                }
            }
        });

        Ok(local_addr)
    }

    /// Start the broker, accepting connections until `cancel` is fired.
    /// Returns the bound address on success.
    pub async fn start_with_cancel(
        self: Arc<Self>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> BrokerResult<SocketAddr> {
        let addr: SocketAddr =
            self.config.bind.parse().map_err(|e: std::net::AddrParseError| {
                BrokerError::ProtocolError(format!("Invalid bind address: {e}"))
            })?;
        let listener =
            TcpListener::bind(addr).await.map_err(|e| BrokerError::ProtocolError(format!("Failed to bind: {e}")))?;
        let local_addr = listener.local_addr().map_err(BrokerError::Io)?;
        log::info!("[mqtt-broker] Listening on {}", local_addr);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    log::info!("[mqtt-broker] Stopped");
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer)) => {
                            let broker = self.clone();
                            self.metrics.total_connections.fetch_add(1, Ordering::Relaxed);
                            self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);
                            tokio::spawn(async move {
                                log::debug!("[mqtt-broker] New connection from {}", peer);
                                if let Err(e) = broker.handle_connection(stream).await {
                                    if !e.to_string().contains("ConnectionClosed") {
                                        log::warn!("[mqtt-broker] Connection error from {}: {}", peer, e);
                                    }
                                }
                                broker.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        Err(e) => {
                            log::error!("[mqtt-broker] Accept error: {}", e);
                        }
                    }
                }
            }
        }

        Ok(local_addr)
    }

    async fn handle_connection(&self, stream: TcpStream) -> BrokerResult<()> {
        // Split the TCP stream into read and write halves
        let (mut read_half, mut write_half) = tokio::io::split(stream);

        // Channel for outbound raw packets (writer task writes them to TCP)
        let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);

        // Read the CONNECT packet
        let (header, mut buf) = codec::read_packet_from(&mut read_half, self.config.max_packet_size)
            .await
            .map_err(BrokerError::Io)?
            .ok_or(BrokerError::ConnectionClosed)?;

        if header.packet_type != PacketType::Connect {
            return Err(BrokerError::ProtocolError("First packet must be CONNECT".into()));
        }

        let connect =
            codec::decode_connect(&mut buf).map_err(|e| BrokerError::ProtocolError(format!("Bad CONNECT: {e}")))?;

        // Authenticate
        if let Err(e) = self.authenticate(&connect).await {
            let _ = codec::write_connack_to(&mut write_half, false, 0x05).await;
            return Err(e);
        }

        let client_id = connect.client_id.clone();

        // Create session with outbound channel
        let session = session::Session::new(client_id.clone(), connect.keep_alive, outbound_tx);
        let session = Arc::new(session);

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(client_id.clone(), session.clone());
        }

        // Send CONNACK success
        codec::write_connack_to(&mut write_half, false, 0x00).await.map_err(BrokerError::Io)?;
        log::info!("[mqtt-broker] Client '{}' connected", client_id);

        // Save will info for cleanup on abnormal disconnect
        let will_info = connect
            .will_topic
            .as_ref()
            .map(|topic| (topic.clone(), connect.will_payload.clone(), connect.will_qos, connect.will_retain));

        // Spawn writer task: reads raw packets from channel and writes to TCP
        let writer_handle = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(raw_packet) = outbound_rx.recv().await {
                if write_half.write_all(&raw_packet).await.is_err() {
                    break;
                }
            }
        });

        // Main read loop
        let result = self.connection_loop(&mut read_half, &client_id).await;

        // Cleanup
        writer_handle.abort();

        // Publish will message on abnormal disconnect
        if result.is_err() {
            if let Some((will_topic, will_payload, will_qos, will_retain)) = will_info {
                if let Some(payload) = will_payload {
                    let _ = self.handle_publish_inner(&will_topic, &payload, will_qos, will_retain).await;
                }
            }
        }

        // Remove session and subscriptions
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(&client_id);
        }
        {
            let mut trie = self.topic_trie.write().await;
            trie.remove_subscriptions_for_client(&client_id);
        }

        log::info!("[mqtt-broker] Client '{}' cleaned up", client_id);
        result
    }

    async fn connection_loop(
        &self,
        read_half: &mut tokio::io::ReadHalf<TcpStream>,
        client_id: &str,
    ) -> BrokerResult<()> {
        loop {
            match codec::read_packet_from(read_half, self.config.max_packet_size).await {
                Ok(Some((header, mut buf))) => {
                    self.metrics.bytes_received.fetch_add(buf.len() as u64, Ordering::Relaxed);
                    match header.packet_type {
                        PacketType::Publish => {
                            let publish = codec::decode_publish(&mut buf, &header)
                                .map_err(|e| BrokerError::ProtocolError(format!("Bad PUBLISH: {e}")))?;
                            self.metrics.messages_received.fetch_add(1, Ordering::Relaxed);
                            self.handle_publish(client_id, publish).await?;
                        }
                        PacketType::Subscribe => {
                            let subscribe = codec::decode_subscribe(&mut buf)
                                .map_err(|e| BrokerError::ProtocolError(format!("Bad SUBSCRIBE: {e}")))?;
                            self.handle_subscribe(client_id, subscribe).await?;
                        }
                        PacketType::Unsubscribe => {
                            let unsubscribe = codec::decode_unsubscribe(&mut buf)
                                .map_err(|e| BrokerError::ProtocolError(format!("Bad UNSUBSCRIBE: {e}")))?;
                            self.handle_unsubscribe(client_id, unsubscribe).await?;
                        }
                        PacketType::PingReq => {
                            // Send PINGRESP via session channel
                            let sessions = self.sessions.read().await;
                            if let Some(session) = sessions.get(client_id) {
                                let _ = session.send_raw(codec::encode_pingresp());
                            }
                        }
                        PacketType::Disconnect => {
                            log::info!("[mqtt-broker] Client '{}' sent DISCONNECT", client_id);
                            return Ok(());
                        }
                        PacketType::PubAck => {
                            // QoS 2 puback - acknowledged, nothing to do
                        }
                        _ => {
                            log::debug!("[mqtt-broker] Unhandled packet type: {:?}", header.packet_type);
                        }
                    }
                }
                Ok(None) => {
                    log::debug!("[mqtt-broker] Client '{}' EOF", client_id);
                    return Err(BrokerError::ConnectionClosed);
                }
                Err(e) => {
                    log::debug!("[mqtt-broker] Read error for '{}': {}", client_id, e);
                    return Err(BrokerError::Io(e));
                }
            }
        }
    }

    async fn authenticate(&self, connect: &ConnectPacket) -> BrokerResult<()> {
        if self.config.auth.username.is_some() {
            let expected_user = self.config.auth.username.as_deref().unwrap();
            let expected_pass = self.config.auth.password.as_deref().unwrap_or("");
            let provided_user = connect.username.as_deref().unwrap_or("");
            let provided_pass =
                connect.password.as_deref().map(|p| String::from_utf8_lossy(p).to_string()).unwrap_or_default();
            if provided_user != expected_user || provided_pass != expected_pass {
                return Err(BrokerError::AuthenticationFailed(connect.client_id.clone()));
            }
        }
        Ok(())
    }

    async fn handle_publish(&self, client_id: &str, publish: PublishPacket) -> BrokerResult<()> {
        // Send PUBACK for QoS 1 back to the publishing client
        if publish.qos == QoS::AtLeastOnce {
            if let Some(pid) = publish.packet_id {
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(client_id) {
                    let _ = session.send_raw(codec::encode_puback(pid));
                }
            }
        }
        // Send PUBREC for QoS 2
        if publish.qos == QoS::ExactlyOnce {
            if let Some(pid) = publish.packet_id {
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(client_id) {
                    let _ = session.send_raw(codec::encode_puback(pid)); // Simplified: PUBREC = same format
                }
            }
        }

        self.handle_publish_inner(&publish.topic, &publish.payload, publish.qos, publish.retain).await
    }

    async fn handle_subscribe(&self, client_id: &str, subscribe: SubscribePacket) -> BrokerResult<()> {
        let mut reason_codes = Vec::with_capacity(subscribe.subscriptions.len());
        {
            let mut trie = self.topic_trie.write().await;
            for sub_filter in &subscribe.subscriptions {
                let effective_qos = sub_filter.qos;
                trie.subscribe(client_id.to_string(), sub_filter.topic_filter.clone(), effective_qos);
                reason_codes.push(effective_qos as u8);
                self.metrics.subscriptions_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Send SUBACK via session channel
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(client_id) {
            let _ = session.send_raw(codec::encode_suback(subscribe.packet_id, &reason_codes));

            // Send retained messages for matching subscriptions
            for sub_filter in &subscribe.subscriptions {
                let retained = {
                    let store = self.retained_store.read().await;
                    store.match_retained(&sub_filter.topic_filter)
                };

                for (retained_topic, retained_msg) in retained {
                    let effective_qos = sub_filter.qos.min(retained_msg.qos);
                    let packet_id = if effective_qos != QoS::AtMostOnce { Some(next_packet_id()) } else { None };

                    let _ = session.send_raw(codec::encode_publish(
                        &retained_topic,
                        &retained_msg.payload,
                        effective_qos,
                        false,
                        true, // retained messages have retain=true
                        packet_id,
                    ));
                    self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Ok(())
    }

    async fn handle_unsubscribe(&self, client_id: &str, unsubscribe: UnsubscribePacket) -> BrokerResult<()> {
        {
            let mut trie = self.topic_trie.write().await;
            for filter in &unsubscribe.topic_filters {
                trie.unsubscribe(client_id, filter);
                self.metrics.subscriptions_count.fetch_sub(1, Ordering::Relaxed);
            }
        }

        // Send UNSUBACK via session channel
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(client_id) {
            let _ = session.send_raw(codec::encode_unsuback(unsubscribe.packet_id));
        }

        Ok(())
    }

    async fn handle_publish_inner(&self, topic: &str, payload: &[u8], qos: QoS, retain: bool) -> BrokerResult<()> {
        // Handle retained messages
        if retain && self.config.retain_available {
            let mut store = self.retained_store.write().await;
            if payload.is_empty() {
                store.remove(topic);
            } else {
                store.store(topic.to_string(), payload.to_vec().into(), qos);
            }
        }

        // Find matching subscribers and deliver messages
        let matches = {
            let trie = self.topic_trie.read().await;
            trie.match_topic(topic)
        };

        let sessions = self.sessions.read().await;
        for sub in matches {
            if let Some(session) = sessions.get(&sub.client_id) {
                // Downgrade QoS to the subscription's maximum
                let effective_qos = qos.min(sub.qos);
                let packet_id = if effective_qos != QoS::AtMostOnce { Some(next_packet_id()) } else { None };

                let raw_packet = codec::encode_publish(topic, payload, effective_qos, false, retain, packet_id);

                if session.send_raw(raw_packet) {
                    self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        Ok(())
    }

    pub async fn get_sessions_info(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|s| SessionInfo {
                client_id: s.client_id().to_string(),
                keep_alive: s.keep_alive(),
                connected_at: s.connected_at(),
            })
            .collect()
    }

    pub async fn get_subscriptions_info(&self) -> Vec<SubscriptionInfo> {
        let trie = self.topic_trie.read().await;
        trie.dump_subscriptions()
    }
}
