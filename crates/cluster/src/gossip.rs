use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use dashmap::DashMap;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::config::ClusterConfig;
use crate::member::{ClusterMember, MemberState};

// ---------------------------------------------------------------------------
// Wire protocol messages
// ---------------------------------------------------------------------------

/// Wire message types exchanged between cluster nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Periodic heartbeat from a node.
    Heartbeat(HeartbeatPayload),
    /// Full membership table broadcast (piggybacked on heartbeat).
    FullSync(FullSyncPayload),
    /// Notification that a node is leaving gracefully.
    Leave(LeavePayload),
    /// Acknowledgement.
    Ack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub node_id: String,
    pub addr: SocketAddr,
    pub incarnation: u64,
    pub state: MemberState,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSyncPayload {
    pub members: Vec<MemberDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeavePayload {
    pub node_id: String,
}

/// Compact representation of a member used during full sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberDigest {
    pub node_id: String,
    pub addr: SocketAddr,
    pub state: MemberState,
    pub incarnation: u64,
}

// ---------------------------------------------------------------------------
// Gossip engine
// ---------------------------------------------------------------------------

/// The gossip membership engine. Manages the local member table,
/// sends heartbeats, detects failures, and runs the gossip protocol.
pub struct GossipEngine {
    config: ClusterConfig,
    local_id: String,
    members: Arc<DashMap<String, ClusterMember>>,
    listener: Arc<RwLock<Option<TcpListener>>>,
    cancel: CancellationToken,
}

impl GossipEngine {
    /// Create a new gossip engine from the cluster configuration.
    pub fn new(config: ClusterConfig, cancel: CancellationToken) -> Self {
        let local_id =
            if config.node_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { config.node_id.clone() };

        Self { config, local_id, members: Arc::new(DashMap::new()), listener: Arc::new(RwLock::new(None)), cancel }
    }

    /// Return the local node identifier.
    pub fn local_id(&self) -> &str {
        &self.local_id
    }

    /// Get a snapshot of all known members.
    pub fn members(&self) -> Vec<ClusterMember> {
        self.members.iter().map(|r| r.value().clone()).collect()
    }

    /// Get the count of alive members (including self).
    pub fn alive_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_alive()).count()
    }

    /// Look up a single member by id.
    pub fn get_member(&self, node_id: &str) -> Option<ClusterMember> {
        self.members.get(node_id).map(|r| r.value().clone())
    }

    /// Return a reference-counted handle to the member map so other
    /// subsystems (partition manager, API handlers) can query membership.
    pub fn members_handle(&self) -> Arc<DashMap<String, ClusterMember>> {
        Arc::clone(&self.members)
    }

    // ------------------------------------------------------------------
    // Bootstrap
    // ------------------------------------------------------------------

    /// Start the gossip engine: register self, bind listener, spawn tasks.
    pub async fn start(&self) -> anyhow::Result<()> {
        let bind_addr = self.config.bind_addr()?;

        // Register self in member table.
        let self_member = ClusterMember::new_self(self.local_id.clone(), bind_addr);
        self.members.insert(self.local_id.clone(), self_member);

        // Bind TCP listener for gossip messages.
        let listener = TcpListener::bind(bind_addr).await?;
        log::info!("cluster: gossip listener bound to {}", bind_addr);
        *self.listener.write().await = Some(listener);

        // Spawn background tasks.
        self.spawn_heartbeat_task();
        self.spawn_failure_detector();
        self.spawn_accept_loop();

        // Initial peer contact.
        self.contact_peers().await;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Peer discovery helpers
    // ------------------------------------------------------------------

    async fn contact_peers(&self) {
        for result in self.config.peer_addrs() {
            match result {
                Ok(addr) => {
                    // Don't dial ourselves.
                    if addr == self.config.bind_addr().unwrap_or_else(|_| addr) {
                        continue;
                    }
                    if let Err(e) = self.send_heartbeat_to(addr).await {
                        log::debug!("cluster: initial contact to {} failed: {}", addr, e);
                    }
                }
                Err(e) => {
                    log::warn!("cluster: invalid peer address: {}", e);
                }
            }
        }
    }

    /// Perform UDP multicast announcement (when discovery_mode is multicast).
    pub async fn multicast_announce(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let payload = HeartbeatPayload {
            node_id: self.local_id.clone(),
            addr: self.config.bind_addr()?,
            incarnation: 0,
            state: MemberState::Alive,
            metadata: HashMap::new(),
        };
        let msg = GossipMessage::Heartbeat(payload);
        let data = serde_json::to_vec(&msg)?;
        let mc_addr: SocketAddr = self.config.multicast_addr.parse()?;
        socket.send_to(&data, mc_addr).await?;
        Ok(())
    }

    /// Listen for UDP multicast announcements and add discovered peers.
    pub async fn multicast_listen(&self) -> anyhow::Result<()> {
        let bind_addr: SocketAddr = format!("0.0.0.0:{}", self.config.cluster_port).parse()?;
        let socket = UdpSocket::bind(bind_addr).await?;
        let mc_addr: SocketAddr = self.config.multicast_addr.parse()?;
        socket.join_multicast_v4(
            mc_addr.ip().clone().to_string().parse::<std::net::Ipv4Addr>()?,
            bind_addr.ip().clone().to_string().parse::<std::net::Ipv4Addr>()?,
        )?;

        let mut buf = vec![0u8; 4096];
        loop {
            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, _src)) => {
                            if let Ok(msg) = serde_json::from_slice::<GossipMessage>(&buf[..len]) {
                                self.handle_message(msg).await;
                            }
                        }
                        Err(e) => {
                            log::warn!("cluster: multicast recv error: {}", e);
                        }
                    }
                }
                _ = self.cancel.cancelled() => break,
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Heartbeat sender
    // ------------------------------------------------------------------

    fn spawn_heartbeat_task(&self) {
        let members = Arc::clone(&self.members);
        let config = self.config.clone();
        let local_id = self.local_id.clone();
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            let interval = config.heartbeat_interval();
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Update our own heartbeat timestamp.
                        if let Some(mut me) = members.get_mut(&local_id) {
                            me.record_heartbeat();
                        }

                        // Pick a random alive peer and send a heartbeat.
                        let targets: Vec<SocketAddr> = members
                            .iter()
                            .filter(|m| m.state == MemberState::Alive && m.node_id != local_id)
                            .map(|m| m.addr)
                            .collect();

                        if targets.is_empty() {
                            continue;
                        }

                        // Create a short-lived rng so it doesn't cross await
                        // points (ThreadRng is !Send).
                        let idx = {
                            let mut rng = rand::rng();
                            rng.random_range(0..targets.len())
                        };
                        let target = targets[idx];

                        let payload = HeartbeatPayload {
                            node_id: local_id.clone(),
                            addr: config.bind_addr().unwrap_or(target),
                            incarnation: members.get(&local_id).map(|m| m.incarnation).unwrap_or(0),
                            state: MemberState::Alive,
                            metadata: HashMap::new(),
                        };

                        let msg = GossipMessage::Heartbeat(payload);
                        let data = match serde_json::to_vec(&msg) {
                            Ok(d) => d,
                            Err(e) => {
                                log::error!("cluster: failed to serialize heartbeat: {}", e);
                                continue;
                            }
                        };

                        // Occasionally piggyback a full sync.
                        let do_full_sync = {
                            let mut rng = rand::rng();
                            rng.random_range(0..100u8) < 10
                        };
                        let msg_to_send = if do_full_sync {
                            let digests: Vec<MemberDigest> = members
                                .iter()
                                .map(|m| MemberDigest {
                                    node_id: m.node_id.clone(),
                                    addr: m.addr,
                                    state: m.state,
                                    incarnation: m.incarnation,
                                })
                                .collect();
                            let sync = FullSyncPayload { members: digests };
                            match serde_json::to_vec(&GossipMessage::FullSync(sync)) {
                                Ok(d) => d,
                                Err(_) => data,
                            }
                        } else {
                            data
                        };

                        if let Err(e) = send_tcp_raw(target, &msg_to_send).await {
                            log::debug!("cluster: heartbeat to {} failed: {}", target, e);
                        }
                    }
                    _ = cancel.cancelled() => break,
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Failure detector
    // ------------------------------------------------------------------

    fn spawn_failure_detector(&self) {
        let members = Arc::clone(&self.members);
        let config = self.config.clone();
        let local_id = self.local_id.clone();
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            let check_interval = config.heartbeat_interval();
            let timeout = config.failure_timeout();
            let mut ticker = tokio::time::interval(check_interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let now = Utc::now();

                        // Collect nodes that need state changes (can't mutate
                        // through DashMap's immutable iterator).
                        let actions: Vec<(String, MemberState)> = members
                            .iter()
                            .filter(|entry| entry.node_id != local_id)
                            .filter_map(|entry| {
                                let elapsed = now.signed_duration_since(entry.last_heartbeat);
                                let elapsed_ms = elapsed.num_milliseconds().max(0) as u64;
                                let timeout_ms = timeout.as_millis() as u64;

                                if elapsed_ms > timeout_ms && entry.state != MemberState::Dead {
                                    Some((entry.node_id.clone(), MemberState::Dead))
                                } else if elapsed_ms > timeout_ms / 2
                                    && entry.state == MemberState::Alive
                                {
                                    Some((entry.node_id.clone(), MemberState::Suspect))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        // Apply state transitions.
                        for (node_id, new_state) in actions {
                            if let Some(mut entry) = members.get_mut(&node_id) {
                                match new_state {
                                    MemberState::Dead => {
                                        log::warn!(
                                            "cluster: node {} suspected dead (no heartbeat)",
                                            node_id
                                        );
                                        entry.mark_dead();
                                    }
                                    MemberState::Suspect => {
                                        entry.mark_suspect();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ = cancel.cancelled() => break,
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Inbound accept loop
    // ------------------------------------------------------------------

    fn spawn_accept_loop(&self) {
        let listener_handle = Arc::clone(&self.listener);
        let members = Arc::clone(&self.members);
        let local_id = self.local_id.clone();
        let cancel = self.cancel.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept = async {
                        let guard = listener_handle.read().await;
                        if let Some(ref listener) = *guard {
                            listener.accept().await
                        } else {
                            // Listener not ready yet; wait a bit.
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::NotConnected,
                                "listener not bound",
                            ));
                        }
                    } => {
                        match accept {
                            Ok((stream, _addr)) => {
                                let members = Arc::clone(&members);
                                let local_id = local_id.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, members, &local_id).await {
                                        log::debug!("cluster: connection handler error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::warn!("cluster: accept error: {}", e);
                            }
                        }
                    }
                    _ = cancel.cancelled() => break,
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Message processing
    // ------------------------------------------------------------------

    async fn handle_message(&self, msg: GossipMessage) {
        match msg {
            GossipMessage::Heartbeat(payload) => {
                self.process_heartbeat(payload).await;
            }
            GossipMessage::FullSync(payload) => {
                self.process_full_sync(payload).await;
            }
            GossipMessage::Leave(payload) => {
                log::info!("cluster: node {} left the cluster", payload.node_id);
                self.members.remove(&payload.node_id);
            }
            GossipMessage::Ack => {}
        }
    }

    async fn process_heartbeat(&self, payload: HeartbeatPayload) {
        match self.members.get_mut(&payload.node_id) {
            Some(mut member) => {
                // If this heartbeat carries a higher incarnation, always accept.
                if payload.incarnation >= member.incarnation {
                    if payload.incarnation > member.incarnation {
                        member.incarnation = payload.incarnation;
                    }
                    if member.state == MemberState::Suspect && payload.state == MemberState::Alive {
                        member.refute();
                    }
                    member.state = payload.state;
                    member.addr = payload.addr;
                    member.record_heartbeat();
                    member.metadata = payload.metadata;
                }
            }
            None => {
                log::info!("cluster: discovered new node {}", payload.node_id);
                let mut m = ClusterMember::new(payload.node_id.clone(), payload.addr);
                m.state = MemberState::Alive;
                m.incarnation = payload.incarnation;
                m.metadata = payload.metadata;
                self.members.insert(payload.node_id, m);
            }
        }
    }

    async fn process_full_sync(&self, payload: FullSyncPayload) {
        for digest in payload.members {
            match self.members.get_mut(&digest.node_id) {
                Some(mut existing) => {
                    if digest.incarnation > existing.incarnation {
                        existing.incarnation = digest.incarnation;
                        existing.state = digest.state;
                        existing.addr = digest.addr;
                        existing.record_heartbeat();
                    }
                }
                None => {
                    if digest.state != MemberState::Dead {
                        log::info!("cluster: sync discovered new node {}", digest.node_id);
                        let mut m = ClusterMember::new(digest.node_id.clone(), digest.addr);
                        m.state = digest.state;
                        m.incarnation = digest.incarnation;
                        self.members.insert(digest.node_id, m);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Outbound helpers
    // ------------------------------------------------------------------

    async fn send_heartbeat_to(&self, addr: SocketAddr) -> anyhow::Result<()> {
        let payload = HeartbeatPayload {
            node_id: self.local_id.clone(),
            addr: self.config.bind_addr()?,
            incarnation: self.members.get(&self.local_id).map(|m| m.incarnation).unwrap_or(0),
            state: MemberState::Alive,
            metadata: HashMap::new(),
        };
        let msg = GossipMessage::Heartbeat(payload);
        let data = serde_json::to_vec(&msg)?;
        send_tcp_raw(addr, &data).await
    }

    /// Gracefully leave the cluster.
    pub async fn leave(&self) {
        let leave_msg = GossipMessage::Leave(LeavePayload { node_id: self.local_id.clone() });
        let data = match serde_json::to_vec(&leave_msg) {
            Ok(d) => d,
            Err(e) => {
                log::error!("cluster: failed to serialize leave message: {}", e);
                return;
            }
        };

        // Send leave to all alive peers.
        for entry in self.members.iter() {
            if entry.node_id == self.local_id {
                continue;
            }
            if entry.is_alive() {
                if let Err(e) = send_tcp_raw(entry.addr, &data).await {
                    log::debug!("cluster: leave notification to {} failed: {}", entry.addr, e);
                }
            }
        }
        log::info!("cluster: node {} left the cluster", self.local_id);
    }
}

// ---------------------------------------------------------------------------
// Utility: send raw bytes over a short-lived TCP connection
// ---------------------------------------------------------------------------

async fn send_tcp_raw(addr: SocketAddr, data: &[u8]) -> anyhow::Result<()> {
    let mut stream = tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(addr)).await??;

    // 4-byte length prefix.
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(data).await?;
    stream.shutdown().await?;
    Ok(())
}

/// Handle an inbound TCP gossip connection: read a message and process it.
async fn handle_connection(
    stream: TcpStream,
    members: Arc<DashMap<String, ClusterMember>>,
    local_id: &str,
) -> anyhow::Result<()> {
    let mut stream = stream;
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 4 * 1024 * 1024 {
        anyhow::bail!("cluster: incoming message too large ({len} bytes)");
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;

    let msg: GossipMessage = serde_json::from_slice(&buf)?;

    match msg {
        GossipMessage::Heartbeat(payload) => match members.get_mut(&payload.node_id) {
            Some(mut member) => {
                if payload.incarnation >= member.incarnation {
                    if payload.incarnation > member.incarnation {
                        member.incarnation = payload.incarnation;
                    }
                    if member.state == MemberState::Suspect && payload.state == MemberState::Alive {
                        member.refute();
                    }
                    member.state = payload.state;
                    member.addr = payload.addr;
                    member.record_heartbeat();
                    member.metadata = payload.metadata;
                }
            }
            None => {
                log::info!("cluster: discovered new node {}", payload.node_id);
                let mut m = ClusterMember::new(payload.node_id.clone(), payload.addr);
                m.state = MemberState::Alive;
                m.incarnation = payload.incarnation;
                m.metadata = payload.metadata;
                members.insert(payload.node_id, m);
            }
        },
        GossipMessage::FullSync(payload) => {
            for digest in payload.members {
                match members.get_mut(&digest.node_id) {
                    Some(mut existing) => {
                        if digest.incarnation > existing.incarnation {
                            existing.incarnation = digest.incarnation;
                            existing.state = digest.state;
                            existing.addr = digest.addr;
                            existing.record_heartbeat();
                        }
                    }
                    None => {
                        if digest.state != MemberState::Dead {
                            log::info!("cluster: sync discovered new node {}", digest.node_id);
                            let mut m = ClusterMember::new(digest.node_id.clone(), digest.addr);
                            m.state = digest.state;
                            m.incarnation = digest.incarnation;
                            members.insert(digest.node_id, m);
                        }
                    }
                }
            }
        }
        GossipMessage::Leave(payload) => {
            if payload.node_id != local_id {
                log::info!("cluster: node {} left the cluster", payload.node_id);
                members.remove(&payload.node_id);
            }
        }
        GossipMessage::Ack => {}
    }

    Ok(())
}
