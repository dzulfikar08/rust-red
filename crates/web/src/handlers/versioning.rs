//! API handlers for flow versioning.
//!
//! Endpoints:
//! - GET  /versioning/versions           -> list versions (paginated)
//! - GET  /versioning/versions/{id}      -> get specific version
//! - POST /versioning/rollback/{id}      -> rollback to version
//! - GET  /versioning/diff?from=A&to=B   -> diff two versions

use crate::handlers::WebState;
use crate::handlers::flows::{flows_with_credentials, save_flows_extracting_credentials};
use crate::versioning::FlowVersionStore;
use axum::{
    Extension,
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ListVersionsQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub from: String,
    pub to: String,
}

async fn make_store(state: &WebState) -> Option<FlowVersionStore> {
    let config = &state.red_settings.versioning;
    let flows_path_guard = state.flows_file_path.read().await;
    flows_path_guard.as_ref().map(|p| FlowVersionStore::new(p, config))
}

pub async fn list_versions(
    Extension(state): Extension<Arc<WebState>>,
    Query(params): Query<ListVersionsQuery>,
) -> Result<Json<Value>, StatusCode> {
    let store = make_store(&state).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let page = params.page.unwrap_or(0);
    let per_page = params.per_page.unwrap_or(20);

    let versions = store.list_versions(page, per_page).await.map_err(|e| {
        log::error!("Failed to list versions: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total = store.total_versions().await;

    Ok(Json(serde_json::json!({
        "versions": versions,
        "page": page,
        "per_page": per_page,
        "total": total,
    })))
}

pub async fn get_version(
    Extension(state): Extension<Arc<WebState>>,
    Path(version_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let store = make_store(&state).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = store.get_version(&version_id).await.map_err(|e| {
        log::error!("Failed to get version {version_id}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match result {
        Some((meta, flows)) => Ok(Json(serde_json::json!({
            "version": meta,
            "flows": flows,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn rollback_version(
    Extension(state): Extension<Arc<WebState>>,
    Path(version_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let store = make_store(&state).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut flows = store.load_version_flows(&version_id).await.map_err(|e| {
        log::error!("Rollback failed for version {version_id}: {e}");
        StatusCode::NOT_FOUND
    })?;

    let version_meta = store.get_version(&version_id).await.map_err(|e| {
        log::error!("Failed to get version metadata for {version_id}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let meta = match version_meta {
        Some((m, _)) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };

    {
        let flows_path_guard = state.flows_file_path.read().await;
        if let Some(flows_path) = flows_path_guard.as_ref() {
            let current_flows = match tokio::fs::read_to_string(flows_path).await {
                Ok(text) => serde_json::from_str::<Vec<Value>>(&text).unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            if let Err(e) =
                save_flows_extracting_credentials(&state.credential_settings, &mut flows, &current_flows, flows_path)
                    .await
            {
                log::error!("Failed to write flows during rollback: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            let engine_guard = state.engine.read().await;
            if let Some(_engine) = engine_guard.as_ref() {
                let flows_value = match flows_with_credentials(&state.credential_settings, flows.clone(), flows_path) {
                    Ok(value) => value,
                    Err(e) => {
                        log::error!("Failed to load credentials during rollback: {e}");
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };
                match state.redeploy_flows(flows_value).await {
                    Ok(_) => {
                        log::info!("Rollback to version {version_id} deployed successfully");
                        state.comms.send_notification("info", &format!("Rolled back to version {version_id}")).await;
                    }
                    Err(e) => {
                        log::error!("Failed to redeploy during rollback: {e}");
                        state.comms.send_notification("error", &format!("Rollback deploy failed: {e}")).await;
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        } else {
            log::error!("No flows file path configured during rollback");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "rolled_back_to": meta,
    })))
}

pub async fn diff_versions(
    Extension(state): Extension<Arc<WebState>>,
    Query(params): Query<DiffQuery>,
) -> Result<Json<Value>, StatusCode> {
    let store = make_store(&state).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let diff = store.diff_versions(&params.from, &params.to).await.map_err(|e| {
        log::error!("Diff failed between {} and {}: {}", params.from, params.to, e);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(serde_json::to_value(diff).unwrap_or_default()))
}
