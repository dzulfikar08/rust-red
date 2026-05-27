//! # rust-red-cluster
//!
//! Clustering and high-availability infrastructure for Rust-Red.
//!
//! This crate provides multi-instance coordination so that several Rust-Red
//! nodes can operate as a single logical cluster:
//!
//! - **Cluster discovery** -- static configuration, UDP multicast, or DNS.
//! - **Gossip membership protocol** -- heartbeat exchange, failure detection,
//!   and cluster state propagation.
//! - **Distributed flow execution** -- flow partitioning and automatic
//!   failover when a node dies.
//! - **State synchronization** -- global context replication, deployment
//!   coordination, session affinity.
//! - **Management API** -- Axum routes for cluster status and operations.
//!
//! # Quick start
//!
//! ```toml
//! [cluster]
//! enabled = true
//! node_id = "node-1"
//! bind = "0.0.0.0:7980"
//! peers = ["192.168.1.2:7980", "192.168.1.3:7980"]
//! heartbeat_interval_ms = 2000
//! failure_timeout_ms = 10000
//! ```
//!
//! ```rust,no_run
//! use rust_red_cluster::{ClusterConfig, ClusterManager};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ClusterConfig::default();
//! let cancel = tokio_util::sync::CancellationToken::new();
//! let manager = ClusterManager::new(config, cancel);
//! manager.start().await?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod bridge;
pub mod config;
pub mod gossip;
pub mod member;
pub mod partition;
pub mod sync;

// Re-exports for convenience
pub use config::ClusterConfig;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use gossip::GossipEngine;
use member::ClusterMember;
use partition::PartitionManager;
use sync::SyncManager;

/// Top-level facade that owns all clustering subsystems.
///
/// The `ClusterManager` is the single entry-point for the rest of the
/// application. It creates and wires together the gossip engine, partition
/// manager, and sync manager.
pub struct ClusterManager {
    config: ClusterConfig,
    gossip: Arc<GossipEngine>,
    pub partition_manager: Arc<PartitionManager>,
    pub sync_manager: Arc<SyncManager>,
}

impl ClusterManager {
    /// Create a new cluster manager from configuration.
    pub fn new(config: ClusterConfig, cancel: CancellationToken) -> Self {
        let gossip = GossipEngine::new(config.clone(), cancel);
        let members = gossip.members_handle();
        let local_id = gossip.local_id().to_string();

        let partition_manager = PartitionManager::new(Arc::clone(&members), local_id.clone());
        let sync_manager = SyncManager::new(members, local_id);

        Self {
            config,
            gossip: Arc::new(gossip),
            partition_manager: Arc::new(partition_manager),
            sync_manager: Arc::new(sync_manager),
        }
    }

    /// Start all cluster subsystems (gossip listener, heartbeat loop,
    /// failure detector).
    pub async fn start(&self) -> anyhow::Result<()> {
        self.config.validate()?;
        self.gossip.start().await?;

        // If multicast discovery is enabled, also start the multicast
        // announcement/listen loop.
        if self.config.discovery_mode == "multicast" {
            let gossip = Arc::clone(&self.gossip);
            tokio::spawn(async move {
                if let Err(e) = gossip.multicast_listen().await {
                    log::error!("cluster: multicast listener error: {}", e);
                }
            });
        }

        log::info!("cluster: node {} started, bind={}", self.gossip.local_id(), self.config.bind);
        Ok(())
    }

    /// Gracefully leave the cluster.
    pub async fn leave(&self) {
        self.gossip.leave().await;
    }

    /// The local node id.
    pub fn local_id(&self) -> &str {
        self.gossip.local_id()
    }

    /// Get a snapshot of all known members.
    pub fn members(&self) -> Vec<ClusterMember> {
        self.gossip.members()
    }

    /// Whether clustering is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}
