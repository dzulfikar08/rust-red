use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::member::ClusterMember;

/// Describes which node is responsible for running a particular flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAssignment {
    /// Flow identifier (from the flows.json).
    pub flow_id: String,
    /// Node id of the cluster member that owns this flow.
    pub assigned_to: String,
    /// Generation counter, incremented on every reassignment.
    pub generation: u64,
}

/// The partition manager decides which flows run on which cluster nodes.
///
/// Strategy (initial implementation): consistent-hash style assignment.
/// The leader (lowest alive node-id, lexicographic) computes the
/// assignment table and distributes it via gossip sync.
pub struct PartitionManager {
    members: Arc<DashMap<String, ClusterMember>>,
    assignments: Arc<RwLock<HashMap<String, FlowAssignment>>>,
    local_id: String,
}

impl PartitionManager {
    pub fn new(members: Arc<DashMap<String, ClusterMember>>, local_id: String) -> Self {
        Self { members, assignments: Arc::new(RwLock::new(HashMap::new())), local_id }
    }

    /// Compute a stable sort key for leader election. The leader is the
    /// alive member with the lexicographically smallest node_id.
    pub fn leader_id(&self) -> Option<String> {
        self.members.iter().filter(|m| m.is_alive()).map(|m| m.node_id.clone()).min()
    }

    /// Whether this node is the current leader.
    pub fn is_leader(&self) -> bool {
        self.leader_id().as_deref() == Some(&self.local_id)
    }

    /// Compute fresh flow assignments for a given list of flow ids.
    ///
    /// Uses a simple modulo-based distribution across alive members.
    /// Only the leader should call this.
    pub async fn compute_assignments(&self, flow_ids: &[String]) -> Vec<FlowAssignment> {
        let alive_nodes: Vec<String> =
            self.members.iter().filter(|m| m.is_alive()).map(|m| m.node_id.clone()).collect();

        if alive_nodes.is_empty() {
            return Vec::new();
        }

        let mut assignments = HashMap::new();
        let n = alive_nodes.len();
        for (i, flow_id) in flow_ids.iter().enumerate() {
            let node_idx = i % n;
            assignments.insert(
                flow_id.clone(),
                FlowAssignment { flow_id: flow_id.clone(), assigned_to: alive_nodes[node_idx].clone(), generation: 0 },
            );
        }

        // Merge with existing, bumping generation only for changed assignments.
        let mut guard = self.assignments.write().await;
        for (fid, new_a) in &assignments {
            match guard.get(fid) {
                Some(existing) if existing.assigned_to == new_a.assigned_to => {
                    // No change, keep existing generation.
                }
                _ => {
                    let new_gen = guard.get(fid).map(|e| e.generation).unwrap_or(0) + 1;
                    guard.insert(
                        fid.clone(),
                        FlowAssignment {
                            flow_id: fid.clone(),
                            assigned_to: new_a.assigned_to.clone(),
                            generation: new_gen,
                        },
                    );
                }
            }
        }

        // Remove assignments for flows that no longer exist.
        guard.retain(|fid, _| assignments.contains_key(fid));

        guard.values().cloned().collect()
    }

    /// Rebalance flows when a node is detected as dead.
    ///
    /// Reassigns all flows that were owned by `dead_node_id` to remaining
    /// alive nodes using round-robin.
    pub async fn rebalance_after_failure(&self, dead_node_id: &str) -> Vec<FlowAssignment> {
        let alive_nodes: Vec<String> = self
            .members
            .iter()
            .filter(|m| m.is_alive() && m.node_id != dead_node_id)
            .map(|m| m.node_id.clone())
            .collect();

        if alive_nodes.is_empty() {
            return Vec::new();
        }

        let mut guard = self.assignments.write().await;
        let n = alive_nodes.len();
        let mut idx = 0;

        for (_key, entry) in guard.iter_mut() {
            if entry.assigned_to == dead_node_id {
                entry.assigned_to = alive_nodes[idx % n].clone();
                entry.generation += 1;
                idx += 1;
                log::info!(
                    "cluster: reassigned flow {} to {} (gen {})",
                    entry.flow_id,
                    entry.assigned_to,
                    entry.generation
                );
            }
        }

        guard.values().cloned().collect()
    }

    /// Get all current assignments.
    pub async fn get_assignments(&self) -> Vec<FlowAssignment> {
        self.assignments.read().await.values().cloned().collect()
    }

    /// Get the node id that a flow is assigned to.
    pub async fn flow_owner(&self, flow_id: &str) -> Option<String> {
        self.assignments.read().await.get(flow_id).map(|a| a.assigned_to.clone())
    }

    /// Check whether this node is responsible for a given flow.
    pub async fn owns_flow(&self, flow_id: &str) -> bool {
        self.flow_owner(flow_id).await.as_deref() == Some(&self.local_id)
    }

    /// Apply a list of assignments received from the leader.
    pub async fn apply_assignments(&self, incoming: Vec<FlowAssignment>) {
        let mut guard = self.assignments.write().await;
        for a in incoming {
            match guard.get(&a.flow_id) {
                Some(existing) if existing.generation >= a.generation => continue,
                _ => {
                    guard.insert(a.flow_id.clone(), a);
                }
            }
        }
    }

    /// Local node id.
    pub fn local_id(&self) -> &str {
        &self.local_id
    }
}
