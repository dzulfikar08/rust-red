use dashmap::DashMap;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SessionData {
    pub client_id: String,
    pub subscriptions: HashMap<String, u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default)]
pub struct MemorySessionStore {
    sessions: DashMap<String, SessionData>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn put(&self, session: SessionData) {
        self.sessions.insert(session.client_id.clone(), session);
    }
    pub fn get(&self, client_id: &str) -> Option<SessionData> {
        self.sessions.get(client_id).map(|r| r.value().clone())
    }
    pub fn remove(&self, client_id: &str) {
        self.sessions.remove(client_id);
    }
    pub fn len(&self) -> usize {
        self.sessions.len()
    }
}
