use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models_dashboard::*;

/// In-memory dashboard store
pub type DashboardStore = Arc<RwLock<HashMap<String, Dashboard>>>;

/// Shared dashboard store singleton
pub fn new_dashboard_store() -> DashboardStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// GET /dashboard - list all dashboards
pub async fn list_dashboards(State(store): State<DashboardStore>) -> Json<Vec<Dashboard>> {
    let map = store.read().await;
    let list: Vec<Dashboard> = map.values().cloned().collect();
    Json(list)
}

/// POST /dashboard - create a new dashboard
pub async fn create_dashboard(
    State(store): State<DashboardStore>,
    Json(payload): Json<CreateDashboardPayload>,
) -> Result<(StatusCode, Json<Dashboard>), StatusCode> {
    if payload.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let dashboard = Dashboard {
        id,
        name: payload.name,
        description: payload.description,
        widgets: vec![],
        created_at: now.clone(),
        updated_at: now,
    };

    let mut map = store.write().await;
    map.insert(dashboard.id.clone(), dashboard.clone());

    Ok((StatusCode::CREATED, Json(dashboard)))
}

/// GET /dashboard/:id - get a single dashboard with widgets
pub async fn get_dashboard(
    State(store): State<DashboardStore>,
    Path(id): Path<String>,
) -> Result<Json<Dashboard>, StatusCode> {
    let map = store.read().await;
    match map.get(&id) {
        Some(d) => Ok(Json(d.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// PUT /dashboard/:id - update dashboard layout/widgets
pub async fn update_dashboard(
    State(store): State<DashboardStore>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDashboardPayload>,
) -> Result<Json<Dashboard>, StatusCode> {
    let mut map = store.write().await;
    match map.get_mut(&id) {
        Some(d) => {
            if let Some(name) = payload.name {
                d.name = name;
            }
            if let Some(description) = payload.description {
                d.description = description;
            }
            if let Some(widgets) = payload.widgets {
                d.widgets = widgets;
            }
            d.updated_at = chrono::Utc::now().to_rfc3339();
            Ok(Json(d.clone()))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /dashboard/:id - delete a dashboard
pub async fn delete_dashboard(State(store): State<DashboardStore>, Path(id): Path<String>) -> StatusCode {
    let mut map = store.write().await;
    if map.remove(&id).is_some() { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
}
