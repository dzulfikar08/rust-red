pub mod models;

use crate::broker::{BrokerMetrics, MqttBroker};
use axum::{extract::State, routing::get, Json, Router};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    pub broker: Arc<MqttBroker>,
    pub metrics: Arc<BrokerMetrics>,
}

pub fn create_api_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/mqtt/status", get(get_status))
        .route("/api/mqtt/connections", get(get_connections))
        .route("/api/mqtt/subscriptions", get(get_subscriptions))
        .with_state(state)
}

async fn get_status(State(state): State<ApiState>) -> Json<models::BrokerStatus> {
    let m = &state.metrics;
    Json(models::BrokerStatus {
        enabled: true,
        active_connections: m.active_connections.load(Ordering::Relaxed),
        total_connections: m.total_connections.load(Ordering::Relaxed),
        messages_received: m.messages_received.load(Ordering::Relaxed),
        messages_sent: m.messages_sent.load(Ordering::Relaxed),
        bytes_received: m.bytes_received.load(Ordering::Relaxed),
        bytes_sent: m.bytes_sent.load(Ordering::Relaxed),
        subscriptions_count: m.subscriptions_count.load(Ordering::Relaxed),
    })
}

async fn get_connections(State(state): State<ApiState>) -> Json<models::ConnectionsResponse> {
    Json(models::ConnectionsResponse { connections: state.broker.get_sessions_info().await })
}

async fn get_subscriptions(State(state): State<ApiState>) -> Json<models::SubscriptionsResponse> {
    Json(models::SubscriptionsResponse { subscriptions: state.broker.get_subscriptions_info().await })
}
