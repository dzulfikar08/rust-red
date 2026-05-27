use axum::{
    Extension,
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use rust_red_core::runtime::model::propex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::handlers::WebState;

fn format_context_response(
    data: std::collections::HashMap<String, rust_red_core::runtime::model::Variant>,
    store_name: &str,
) -> Value {
    let mut entries = serde_json::Map::new();
    for (key, variant) in data {
        let json_val = variant.to_json_value();
        let format = match json_val {
            serde_json::Value::Null => "undefined",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        };
        let msg = serde_json::to_string(&json_val).unwrap_or_else(|_| "null".into());
        entries.insert(key, serde_json::json!({"msg": msg, "format": format}));
    }
    let mut store_map = serde_json::Map::new();
    store_map.insert(store_name.to_string(), Value::Object(entries));
    Value::Object(store_map)
}

fn format_single_entry(variant: &rust_red_core::runtime::model::Variant) -> Value {
    let json_val = variant.to_json_value();
    let format = match &json_val {
        serde_json::Value::Null => "undefined",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    };
    let msg = serde_json::to_string(&json_val).unwrap_or_else(|_| "null".into());
    serde_json::json!({"msg": msg, "format": format})
}

/// Get global context
pub async fn get_global_context(Extension(state): Extension<Arc<WebState>>) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        if let Ok(data) = store.get_all("global").await {
            let store_name = store.name().await;
            return Ok(Json(format_context_response(data, store_name)));
        }
    }
    Ok(Json(serde_json::json!({"default": {}})))
}

/// Get global context key-value
pub async fn get_global_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path(key): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store_name = params.get("store").map(|s| s.as_str()).unwrap_or("default");
        let store = cm.get_context_store(store_name).unwrap_or(cm.get_default_store());
        if let Ok(path) = propex::parse(&key)
            && let Ok(variant) = store.get_one("global", &path).await
        {
            return Ok(Json(format_single_entry(&variant)));
        }
    }
    Ok(Json(serde_json::json!({"msg": "undefined", "format": "undefined"})))
}

/// Delete global context key-value
pub async fn delete_global_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        if let Ok(path) = propex::parse(&key) {
            let _ = store.remove_one("global", &path).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Get flow context
pub async fn get_flow_context(
    Extension(state): Extension<Arc<WebState>>,
    Path(flow_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        let scope = format!("flow:{}", flow_id);
        if let Ok(data) = store.get_all(&scope).await {
            let store_name = store.name().await;
            return Ok(Json(format_context_response(data, store_name)));
        }
    }
    Ok(Json(serde_json::json!({"default": {}})))
}

/// Get flow context key-value
pub async fn get_flow_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path((flow_id, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store_name = params.get("store").map(|s| s.as_str()).unwrap_or("default");
        let store = cm.get_context_store(store_name).unwrap_or(cm.get_default_store());
        let scope = format!("flow:{}", flow_id);
        if let Ok(path) = propex::parse(&key)
            && let Ok(variant) = store.get_one(&scope, &path).await
        {
            return Ok(Json(format_single_entry(&variant)));
        }
    }
    Ok(Json(serde_json::json!({"msg": "undefined", "format": "undefined"})))
}

/// Delete flow context key-value
pub async fn delete_flow_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path((flow_id, key)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        let scope = format!("flow:{}", flow_id);
        if let Ok(path) = propex::parse(&key) {
            let _ = store.remove_one(&scope, &path).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Get node context
pub async fn get_node_context(
    Extension(state): Extension<Arc<WebState>>,
    Path(node_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        if let Ok(data) = store.get_all(&node_id).await {
            let store_name = store.name().await;
            return Ok(Json(format_context_response(data, store_name)));
        }
    }
    Ok(Json(serde_json::json!({"default": {}})))
}

/// Get node context key-value
pub async fn get_node_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path((node_id, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store_name = params.get("store").map(|s| s.as_str()).unwrap_or("default");
        let store = cm.get_context_store(store_name).unwrap_or(cm.get_default_store());
        if let Ok(path) = propex::parse(&key)
            && let Ok(variant) = store.get_one(&node_id, &path).await
        {
            return Ok(Json(format_single_entry(&variant)));
        }
    }
    Ok(Json(serde_json::json!({"msg": "undefined", "format": "undefined"})))
}

/// Delete node context key-value
pub async fn delete_node_context_key(
    Extension(state): Extension<Arc<WebState>>,
    Path((node_id, key)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        let cm = engine.get_context_manager();
        let store = cm.get_default_store();
        if let Ok(path) = propex::parse(&key) {
            let _ = store.remove_one(&node_id, &path).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
