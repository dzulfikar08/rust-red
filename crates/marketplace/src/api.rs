//! Axum routes for the marketplace API.

use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use tokio::sync::RwLock;

use crate::error::MarketplaceError;
use crate::models::*;
use crate::store::PluginStore;
use crate::verify;

/// Shared application state for the marketplace router.
pub type AppState = Arc<RwLock<PluginStore>>;

/// Build the marketplace sub-router, meant to be nested at `/marketplace`.
pub fn marketplace_router(state: AppState) -> Router {
    Router::new()
        .route("/plugins", get(list_plugins).post(publish_plugin))
        .route("/plugins/{id}", get(get_plugin_detail))
        .route("/plugins/{id}/versions", get(list_versions))
        .route("/plugins/{id}/download/{version}", get(download_plugin))
        .route("/plugins/{id}/versions/{version}", delete(unpublish_version))
        .route("/plugins/{id}/rate", post(rate_plugin))
        .with_state(state)
}

// ---- Handlers ----

/// GET `/marketplace/plugins` -- list/search plugins.
async fn list_plugins(
    State(state): State<AppState>,
    Query(query): Query<ListPluginsQuery>,
) -> Result<Json<PluginListResponse>, MarketplaceError> {
    let store = state.read().await;
    let result = store.list_plugins(&query);
    Ok(Json(result))
}

/// GET `/marketplace/plugins/{id}` -- plugin detail.
async fn get_plugin_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginDetail>, MarketplaceError> {
    let store = state.read().await;

    // Try as UUID first; fall back to name lookup
    let record = if let Some(r) = store.get_plugin(&id) {
        r
    } else if let Some(pid) = store.get_plugin_id_by_name(&id) {
        store.get_plugin(&pid).ok_or_else(|| MarketplaceError::NotFound(id.clone()))?
    } else {
        return Err(MarketplaceError::NotFound(id));
    };

    let detail = store.record_to_detail(&record);
    Ok(Json(detail))
}

/// GET `/marketplace/plugins/{id}/versions` -- list versions for a plugin.
async fn list_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<VersionSummary>>, MarketplaceError> {
    let store = state.read().await;

    let record = resolve_record(&store, &id)?;
    let versions: Vec<VersionSummary> = record
        .versions
        .values()
        .map(|v| VersionSummary {
            version: v.version.clone(),
            checksum: v.checksum.clone(),
            size_bytes: v.size_bytes,
            published_at: v.published_at,
        })
        .collect();
    Ok(Json(versions))
}

/// GET `/marketplace/plugins/{id}/download/{version}` -- download WASM binary.
async fn download_plugin(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<impl IntoResponse, MarketplaceError> {
    let store = state.read().await;

    // Resolve plugin_id (could be UUID or name)
    let plugin_id = if store.get_plugin(&id).is_some() {
        id.clone()
    } else if let Some(pid) = store.get_plugin_id_by_name(&id) {
        pid
    } else {
        return Err(MarketplaceError::NotFound(id));
    };

    let binary = store.download(&plugin_id, &version)?;

    let disposition = format!("attachment; filename=\"{}.wasm\"", plugin_id);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("application/wasm")),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&disposition).unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            ),
        ],
        binary,
    ))
}

/// POST `/marketplace/plugins` -- publish a new plugin.
///
/// Accepts `multipart/form-data` with two fields:
/// - `metadata` -- JSON `PluginMetadata`
/// - `wasm` -- the `.wasm` binary
async fn publish_plugin(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<serde_json::Value>, MarketplaceError> {
    let (metadata, wasm_bytes) = parse_publish_multipart(multipart).await?;

    // Verification (if enabled)
    {
        let store = state.read().await;
        if store.config().require_verification {
            let report = verify::verify_wasm_plugin(&wasm_bytes)?;
            log::info!(
                "Verification passed: {} exports, {} imports, sandbox={}",
                report.export_count,
                report.import_count,
                report.sandbox_passed,
            );
        }
    }

    // Publish
    let store = state.write().await;
    let plugin_id = store.publish(&metadata, wasm_bytes)?;

    Ok(Json(serde_json::json!({
        "id": plugin_id,
        "status": "published"
    })))
}

/// DELETE `/marketplace/plugins/{id}/versions/{version}` -- unpublish a version.
async fn unpublish_version(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<StatusCode, MarketplaceError> {
    let store = state.read().await;

    // Resolve plugin_id
    let plugin_id = if store.get_plugin(&id).is_some() {
        id.clone()
    } else if let Some(pid) = store.get_plugin_id_by_name(&id) {
        pid
    } else {
        return Err(MarketplaceError::NotFound(id));
    };

    drop(store);

    let store = state.write().await;
    store.unpublish_version(&plugin_id, &version)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST `/marketplace/plugins/{id}/rate` -- rate a plugin.
async fn rate_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RateRequest>,
) -> Result<Json<serde_json::Value>, MarketplaceError> {
    let store = state.read().await;

    let plugin_id = if store.get_plugin(&id).is_some() {
        id.clone()
    } else if let Some(pid) = store.get_plugin_id_by_name(&id) {
        pid
    } else {
        return Err(MarketplaceError::NotFound(id));
    };

    drop(store);

    let store = state.read().await;
    let avg = store.rate(&plugin_id, body.rating)?;

    Ok(Json(serde_json::json!({
        "rating_avg": (avg * 100.0).round() / 100.0,
    })))
}

// ---- Helpers ----

fn resolve_record(store: &PluginStore, id: &str) -> Result<PluginRecord, MarketplaceError> {
    if let Some(r) = store.get_plugin(id) {
        return Ok(r);
    }
    if let Some(pid) = store.get_plugin_id_by_name(id) {
        return store.get_plugin(&pid).ok_or_else(|| MarketplaceError::NotFound(id.to_string()));
    }
    Err(MarketplaceError::NotFound(id.to_string()))
}

/// Extract metadata and wasm bytes from a multipart upload.
async fn parse_publish_multipart(mut multipart: Multipart) -> Result<(PluginMetadata, Vec<u8>), MarketplaceError> {
    let mut metadata: Option<PluginMetadata> = None;
    let mut wasm_bytes: Option<Vec<u8>> = None;

    while let Some(field) =
        multipart.next_field().await.map_err(|e| MarketplaceError::BadRequest(format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "metadata" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| MarketplaceError::BadRequest(format!("metadata read error: {e}")))?;
                let meta: PluginMetadata = serde_json::from_str(&text)
                    .map_err(|e| MarketplaceError::BadRequest(format!("invalid metadata JSON: {e}")))?;
                metadata = Some(meta);
            }
            "wasm" => {
                let data =
                    field.bytes().await.map_err(|e| MarketplaceError::BadRequest(format!("wasm read error: {e}")))?;
                wasm_bytes = Some(data.to_vec());
            }
            other => {
                log::warn!("publish: ignoring unknown multipart field: {other}");
            }
        }
    }

    let meta = metadata.ok_or_else(|| MarketplaceError::BadRequest("missing 'metadata' field".into()))?;
    let wasm = wasm_bytes.ok_or_else(|| MarketplaceError::BadRequest("missing 'wasm' field".into()))?;

    Ok((meta, wasm))
}
