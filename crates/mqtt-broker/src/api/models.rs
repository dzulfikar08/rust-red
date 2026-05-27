use crate::broker::{SessionInfo, SubscriptionInfo};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BrokerStatus {
    pub enabled: bool,
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub subscriptions_count: u64,
}

#[derive(Debug, Serialize)]
pub struct ConnectionsResponse {
    pub connections: Vec<SessionInfo>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionsResponse {
    pub subscriptions: Vec<SubscriptionInfo>,
}
