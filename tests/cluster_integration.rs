//! Integration tests for the cluster/HA feature (RRD-25).
//!
//! Run with: cargo test --features cluster --test cluster_integration

#[cfg(feature = "cluster")]
mod tests {
    use std::sync::Arc;

    use rust_red_cluster::bridge::ClusterPartitionerBridge;
    use rust_red_cluster::config::ClusterConfig;
    use rust_red_cluster::member::ClusterMember;
    use rust_red_cluster::partition::PartitionManager;
    use rust_red_cluster::ClusterManager;
    use rust_red_core::runtime::cluster_aware::ClusterFlowPartitioner;

    use dashmap::DashMap;
    use tokio_util::sync::CancellationToken;

    /// ClusterConfig defaults should have clustering disabled.
    #[test]
    fn test_cluster_config_default_disabled() {
        let cfg = ClusterConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.node_id.is_empty());
        assert!(cfg.peers.is_empty());
    }

    /// ClusterConfig::validate should reject invalid bind address.
    #[test]
    fn test_cluster_config_validate_rejects_bad_bind() {
        let cfg = ClusterConfig { enabled: true, bind: "not-an-address".to_string(), ..Default::default() };
        assert!(cfg.validate().is_err());
    }

    /// ClusterConfig::validate should accept a valid config.
    #[test]
    fn test_cluster_config_validate_ok() {
        let cfg = ClusterConfig {
            enabled: true,
            bind: "0.0.0.0:7980".to_string(),
            peers: vec!["127.0.0.1:7981".to_string()],
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    /// ClusterConfig::validate should reject zero heartbeat.
    #[test]
    fn test_cluster_config_validate_rejects_zero_heartbeat() {
        let cfg = ClusterConfig { enabled: true, heartbeat_interval_ms: 0, ..Default::default() };
        assert!(cfg.validate().is_err());
    }

    /// PartitionManager should assign flows to alive nodes.
    #[tokio::test]
    async fn test_partition_manager_assigns_flows() {
        let members = Arc::new(DashMap::new());

        members
            .insert("node-1".to_string(), ClusterMember::new("node-1".to_string(), "127.0.0.1:7980".parse().unwrap()));
        members
            .insert("node-2".to_string(), ClusterMember::new("node-2".to_string(), "127.0.0.1:7981".parse().unwrap()));

        let pm = PartitionManager::new(members, "node-1".to_string());

        let flow_ids: Vec<String> = (0..6).map(|i| format!("flow-{i}")).collect();
        let assignments = pm.compute_assignments(&flow_ids).await;

        assert_eq!(assignments.len(), 6);

        let node1_count = assignments.iter().filter(|a| a.assigned_to == "node-1").count();
        let node2_count = assignments.iter().filter(|a| a.assigned_to == "node-2").count();
        assert_eq!(node1_count, 3);
        assert_eq!(node2_count, 3);
    }

    /// PartitionManager::owns_flow should return true for locally-owned flows.
    #[tokio::test]
    async fn test_partition_manager_owns_flow() {
        let members = Arc::new(DashMap::new());
        members
            .insert("node-1".to_string(), ClusterMember::new("node-1".to_string(), "127.0.0.1:7980".parse().unwrap()));

        let pm = PartitionManager::new(members, "node-1".to_string());

        let flow_ids = vec!["flow-a".to_string(), "flow-b".to_string()];
        pm.compute_assignments(&flow_ids).await;

        assert!(pm.owns_flow("flow-a").await);
        assert!(pm.owns_flow("flow-b").await);
        assert!(!pm.owns_flow("flow-nonexistent").await);
    }

    /// ClusterPartitionerBridge implements ClusterFlowPartitioner correctly.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bridge_implements_partitioner_trait() {
        let members = Arc::new(DashMap::new());
        members
            .insert("node-1".to_string(), ClusterMember::new("node-1".to_string(), "127.0.0.1:7980".parse().unwrap()));

        let pm = Arc::new(PartitionManager::new(members, "node-1".to_string()));
        let bridge = ClusterPartitionerBridge::new(pm.clone());

        let flow_ids = vec!["flow-test".to_string()];
        pm.compute_assignments(&flow_ids).await;

        assert!(bridge.is_enabled());
        assert_eq!(bridge.local_node_id(), "node-1");
        assert!(bridge.owns_flow("flow-test"));
    }

    /// PartitionManager should rebalance flows when a node fails.
    #[tokio::test]
    async fn test_partition_manager_rebalance_after_failure() {
        let members = Arc::new(DashMap::new());
        members
            .insert("node-1".to_string(), ClusterMember::new("node-1".to_string(), "127.0.0.1:7980".parse().unwrap()));
        members
            .insert("node-2".to_string(), ClusterMember::new("node-2".to_string(), "127.0.0.1:7981".parse().unwrap()));

        let pm = PartitionManager::new(Arc::clone(&members), "node-1".to_string());

        let flow_ids: Vec<String> = (0..4).map(|i| format!("flow-{i}")).collect();
        let assignments = pm.compute_assignments(&flow_ids).await;
        assert_eq!(assignments.len(), 4);

        // Mark node-2 as dead
        members.get_mut("node-2").unwrap().mark_dead();

        let rebalanced = pm.rebalance_after_failure("node-2").await;

        // All flows should now be assigned to node-1
        for a in &rebalanced {
            assert_ne!(a.assigned_to, "node-2", "Found flow still assigned to dead node");
        }
    }

    /// ClusterManager can be created with disabled config without errors.
    #[test]
    fn test_cluster_manager_creation_disabled() {
        let cfg = ClusterConfig::default();
        assert!(!cfg.enabled);

        let cancel = CancellationToken::new();
        let mgr = ClusterManager::new(cfg, cancel);
        assert!(!mgr.local_id().is_empty());
        assert!(!mgr.is_enabled());
    }

    /// ClusterMember::is_alive should return true for Alive and Self_ states.
    #[test]
    fn test_member_is_alive() {
        let mut m = ClusterMember::new("n1".to_string(), "127.0.0.1:7980".parse().unwrap());
        assert!(m.is_alive());

        m.mark_dead();
        assert!(!m.is_alive());

        m.refute();
        assert!(m.is_alive());
    }
}
