use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::member::ClusterMember;

/// A key-value entry in the replicated context store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub key: String,
    pub value: serde_json::Value,
    /// Monotonically increasing version, used for conflict resolution.
    pub version: u64,
}

/// A deployment request that should be broadcast to all cluster nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployRequest {
    /// Unique id for this deployment operation.
    pub deploy_id: String,
    /// The full flows JSON to deploy.
    pub flows: serde_json::Value,
    /// Generation / revision string.
    pub revision: String,
}

/// A deployment acknowledgement from a cluster node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployAck {
    pub deploy_id: String,
    pub node_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Callback type invoked when a remote deploy request arrives.
pub type DeployCallback = Arc<dyn Fn(DeployRequest) -> bool + Send + Sync>;

/// Handles state synchronization across the cluster:
/// - Global context replication
/// - Flow deployment coordination
/// - Session affinity metadata
pub struct SyncManager {
    members: Arc<DashMap<String, ClusterMember>>,
    /// Replicated global context store.
    context: Arc<RwLock<HashMap<String, ContextEntry>>>,
    /// Pending deploy acknowledgements.
    deploy_acks: Arc<RwLock<HashMap<String, Vec<DeployAck>>>>,
    /// Callback invoked when this node receives a deploy request.
    deploy_callback: Arc<RwLock<Option<DeployCallback>>>,
    local_id: String,
}

impl SyncManager {
    pub fn new(members: Arc<DashMap<String, ClusterMember>>, local_id: String) -> Self {
        Self {
            members,
            context: Arc::new(RwLock::new(HashMap::new())),
            deploy_acks: Arc::new(RwLock::new(HashMap::new())),
            deploy_callback: Arc::new(RwLock::new(None)),
            local_id,
        }
    }

    /// Register a callback that is invoked when a cluster-wide deploy
    /// request is received by this node.
    pub async fn set_deploy_callback(&self, cb: DeployCallback) {
        *self.deploy_callback.write().await = Some(cb);
    }

    // ------------------------------------------------------------------
    // Context replication
    // ------------------------------------------------------------------

    /// Set a key in the replicated context store (local write).
    pub async fn context_set(&self, key: String, value: serde_json::Value) {
        let mut ctx = self.context.write().await;
        let version = ctx.get(&key).map(|e| e.version).unwrap_or(0) + 1;
        ctx.insert(key.clone(), ContextEntry { key, value, version });
    }

    /// Get a key from the replicated context store.
    pub async fn context_get(&self, key: &str) -> Option<serde_json::Value> {
        let ctx = self.context.read().await;
        ctx.get(key).map(|e| e.value.clone())
    }

    /// Delete a key from the replicated context store.
    pub async fn context_delete(&self, key: &str) {
        self.context.write().await.remove(key);
    }

    /// Apply incoming context updates from a remote node. Uses version
    /// numbers for last-writer-wins conflict resolution.
    pub async fn apply_context_updates(&self, entries: Vec<ContextEntry>) {
        let mut ctx = self.context.write().await;
        for entry in entries {
            match ctx.get(&entry.key) {
                Some(existing) if existing.version >= entry.version => continue,
                _ => {
                    ctx.insert(entry.key.clone(), entry);
                }
            }
        }
    }

    /// Get a snapshot of the entire context.
    pub async fn context_snapshot(&self) -> HashMap<String, ContextEntry> {
        self.context.read().await.clone()
    }

    // ------------------------------------------------------------------
    // Deploy coordination
    // ------------------------------------------------------------------

    /// Record a deploy ack from a cluster member.
    pub async fn record_deploy_ack(&self, ack: DeployAck) {
        let mut acks = self.deploy_acks.write().await;
        acks.entry(ack.deploy_id.clone()).or_default().push(ack);
    }

    /// Check how many nodes have acknowledged a given deploy.
    pub async fn deploy_ack_count(&self, deploy_id: &str) -> usize {
        self.deploy_acks.read().await.get(deploy_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Return all acks for a deploy.
    pub async fn deploy_acks_for(&self, deploy_id: &str) -> Vec<DeployAck> {
        self.deploy_acks.read().await.get(deploy_id).cloned().unwrap_or_default()
    }

    /// Invoke the local deploy callback (called when a remote deploy
    /// request reaches this node).
    pub async fn handle_deploy(&self, req: DeployRequest) -> bool {
        let guard = self.deploy_callback.read().await;
        if let Some(ref cb) = *guard {
            cb(req)
        } else {
            log::warn!("cluster: received deploy request but no callback registered");
            false
        }
    }

    /// Session affinity: compute which node should handle a given
    /// session key. Uses consistent hashing over alive member ids.
    pub fn session_affinity(&self, session_key: &str) -> Option<String> {
        let mut candidates: Vec<String> =
            self.members.iter().filter(|m| m.is_alive()).map(|m| m.node_id.clone()).collect();

        if candidates.is_empty() {
            return None;
        }

        candidates.sort();
        // Simple hash-based selection.
        let hash = simple_hash(session_key);
        let idx = hash as usize % candidates.len();
        Some(candidates[idx].clone())
    }

    /// Whether this node owns the given session key.
    pub fn owns_session(&self, session_key: &str) -> bool {
        self.session_affinity(session_key).as_deref() == Some(&self.local_id)
    }
}

/// A simple, deterministic hash function for session affinity.
fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}
