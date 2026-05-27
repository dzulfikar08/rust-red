use std::sync::Arc;

use crate::partition::PartitionManager;

/// Bridge that implements `ClusterFlowPartitioner` from `rust_red_core`
/// by delegating to the cluster crate's `PartitionManager`.
pub struct ClusterPartitionerBridge {
    partition_manager: Arc<PartitionManager>,
}

impl ClusterPartitionerBridge {
    pub fn new(partition_manager: Arc<PartitionManager>) -> Self {
        Self { partition_manager }
    }
}

impl rust_red_core::runtime::cluster_aware::ClusterFlowPartitioner for ClusterPartitionerBridge {
    fn owns_flow(&self, flow_id: &str) -> bool {
        // PartitionManager::owns_flow is async, so we use tokio::task::block_in_place
        // since this may be called from an async context within the engine.
        // The PartitionManager uses internal async locking but the operation is fast.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { self.partition_manager.owns_flow(flow_id).await })
        })
    }

    fn is_enabled(&self) -> bool {
        // If we have a bridge, clustering is enabled
        true
    }

    fn local_node_id(&self) -> &str {
        self.partition_manager.local_id()
    }
}
