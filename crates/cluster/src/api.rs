use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::ClusterManager;
use crate::partition::FlowAssignment;
use crate::sync::{DeployAck, DeployRequest};

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ClusterStatusResponse {
    pub enabled: bool,
    pub local_node_id: String,
    pub leader_id: Option<String>,
    pub total_nodes: usize,
    pub alive_nodes: usize,
    pub members: Vec<MemberSummary>,
}

#[derive(Debug, Serialize)]
pub struct MemberSummary {
    pub node_id: String,
    pub addr: String,
    pub state: String,
    pub incarnation: u64,
    pub last_heartbeat_ago_ms: i64,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct ClusterNodesResponse {
    pub nodes: Vec<MemberSummary>,
}

#[derive(Debug, Deserialize)]
pub struct DeployPayload {
    pub flows: serde_json::Value,
    pub revision: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub deploy_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FlowDistributionResponse {
    pub assignments: Vec<FlowAssignment>,
}

// ---------------------------------------------------------------------------
// Shared state for the API handlers
// ---------------------------------------------------------------------------

/// Reference-counted state shared across all cluster API handlers.
#[derive(Clone)]
pub struct ClusterApiState {
    pub manager: Arc<ClusterManager>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn get_cluster_status(State(state): State<ClusterApiState>) -> Json<ClusterStatusResponse> {
    let members = state.manager.members();
    let now = chrono::Utc::now();
    let alive_count = members.iter().filter(|m| m.is_alive()).count();

    let leader_id = state.manager.partition_manager.leader_id();

    let member_summaries: Vec<MemberSummary> = members
        .iter()
        .map(|m| {
            let ago = now.signed_duration_since(m.last_heartbeat).num_milliseconds().max(0);
            MemberSummary {
                node_id: m.node_id.clone(),
                addr: m.addr.to_string(),
                state: m.state.to_string(),
                incarnation: m.incarnation,
                last_heartbeat_ago_ms: ago,
                joined_at: m.joined_at.to_rfc3339(),
            }
        })
        .collect();

    Json(ClusterStatusResponse {
        enabled: true,
        local_node_id: state.manager.local_id().to_string(),
        leader_id,
        total_nodes: members.len(),
        alive_nodes: alive_count,
        members: member_summaries,
    })
}

pub async fn get_cluster_nodes(State(state): State<ClusterApiState>) -> Json<ClusterNodesResponse> {
    let members = state.manager.members();
    let now = chrono::Utc::now();

    let nodes: Vec<MemberSummary> = members
        .iter()
        .map(|m| {
            let ago = now.signed_duration_since(m.last_heartbeat).num_milliseconds().max(0);
            MemberSummary {
                node_id: m.node_id.clone(),
                addr: m.addr.to_string(),
                state: m.state.to_string(),
                incarnation: m.incarnation,
                last_heartbeat_ago_ms: ago,
                joined_at: m.joined_at.to_rfc3339(),
            }
        })
        .collect();

    Json(ClusterNodesResponse { nodes })
}

pub async fn post_cluster_deploy(
    State(state): State<ClusterApiState>,
    Json(payload): Json<DeployPayload>,
) -> Json<DeployResponse> {
    let deploy_id = uuid::Uuid::new_v4().to_string();

    let req = DeployRequest {
        deploy_id: deploy_id.clone(),
        flows: payload.flows,
        revision: payload.revision.unwrap_or_else(|| "0".to_string()),
    };

    // Apply locally first.
    let local_ok = state.manager.sync_manager.handle_deploy(req.clone()).await;

    if !local_ok {
        return Json(DeployResponse {
            deploy_id,
            status: "error".to_string(),
            message: "local deploy failed".to_string(),
        });
    }

    // Record self ack.
    state
        .manager
        .sync_manager
        .record_deploy_ack(DeployAck {
            deploy_id: deploy_id.clone(),
            node_id: state.manager.local_id().to_string(),
            success: true,
            error: None,
        })
        .await;

    // In a full implementation we would fan-out the deploy to all peers
    // over TCP/gossip here. For now the local deploy succeeds and the
    // request can be picked up by other nodes via gossip sync.
    log::info!("cluster: deploy {} applied locally, pending fan-out", deploy_id);

    Json(DeployResponse {
        deploy_id,
        status: "accepted".to_string(),
        message: "deploy accepted locally, propagating to cluster".to_string(),
    })
}

pub async fn get_cluster_flows(State(state): State<ClusterApiState>) -> Json<FlowDistributionResponse> {
    let assignments = state.manager.partition_manager.get_assignments().await;
    Json(FlowDistributionResponse { assignments })
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the cluster API sub-router.
pub fn cluster_router(state: ClusterApiState) -> Router {
    Router::new()
        .route("/status", get(get_cluster_status))
        .route("/nodes", get(get_cluster_nodes))
        .route("/deploy", post(post_cluster_deploy))
        .route("/flows", get(get_cluster_flows))
        .with_state(state)
}
