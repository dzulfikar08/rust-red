use crate::handlers::WebState;
use crate::models::*;
use axum::{Extension, extract::Path, http::StatusCode, response::Json};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path as StdPath;
use std::sync::Arc;

/// Load flows from a JSON file
async fn load_flows_from_file(
    file_path: &StdPath,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
    if !file_path.exists() {
        // Return empty flows if file doesn't exist
        return Ok(vec![]);
    }

    let content = tokio::fs::read_to_string(file_path).await?;
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let flows: Vec<serde_json::Value> = serde_json::from_str(&content)?;
    Ok(flows)
}

/// Save flows to a JSON file
async fn save_flows_to_file(
    flows: &[serde_json::Value],
    file_path: &StdPath,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let flows_json = serde_json::to_string_pretty(flows)?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(file_path, flows_json).await?;
    Ok(())
}

/// Trigger engine restart after flows change
async fn restart_engine_if_available(state: &WebState) {
    let restart_callback_guard = state.restart_callback.read().await;
    let flows_path_guard = state.flows_file_path.read().await;
    if let (Some(restart_callback), Some(flows_path)) = (restart_callback_guard.as_ref(), flows_path_guard.as_ref()) {
        log::info!("Triggering flow engine restart...");
        restart_callback(flows_path.clone());
        state.comms.send_notification("info", "Flow engine restart initiated").await;
    } else {
        log::warn!("No restart callback or flows path available");
    }
}

/// Get all flows (Node-RED compatible)
pub async fn get_flows(Extension(state): Extension<Arc<WebState>>) -> Result<Json<Value>, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let flows = if let Some(flows_path) = flows_path_guard.as_ref() {
        match load_flows_from_file(flows_path).await {
            Ok(flows) => flows,
            Err(e) => {
                log::error!("Failed to load flows from file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::warn!("No flows file path configured");
        vec![]
    };

    let response = serde_json::json!({
        "flows": flows,
        "rev": "1"  // Simple revision for now
    });

    Ok(Json(response))
}

/// Deploy/update all flows (Node-RED compatible)
pub async fn post_flows(
    Extension(state): Extension<Arc<WebState>>,
    payload: String,
) -> Result<Json<Value>, StatusCode> {
    // Input size validation
    let max_flow_size = state.red_settings.security.max_flow_size;
    if payload.len() > max_flow_size {
        log::warn!("Flow payload too large: {} bytes (max: {} bytes)", payload.len(), max_flow_size);
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    log::debug!("Received raw POST payload ({} bytes)", payload.len());

    // Try to parse the payload
    let parsed_payload: FlowsPayload = match serde_json::from_str(&payload) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse flows payload: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // TODO: Check/validate rev field - appears to be SHA256 hash but algorithm unknown
    log::debug!("Received deployment request with rev: {:?}", parsed_payload.rev);
    log::debug!("Received deployment request with {} flows", parsed_payload.flows.len());

    // Validate config_node references before deploying
    if let Err(validation_errors) = validate_flow_config_nodes(&parsed_payload.flows) {
        log::warn!("Deploy rejected: {} invalid config reference(s)", validation_errors.len());
        state
            .comms
            .send_notification(
                "warning",
                &format!("Deploy rejected: {} node(s) have invalid config references", validation_errors.len()),
            )
            .await;
        // Return validation error as JSON - client must handle HTTP 200 with error payload
        // since Node-RED editor expects JSON response
        return Ok(Json(serde_json::json!({
            "error": "validation_failed",
            "message": "Some nodes reference config nodes that don't exist in this flow",
            "details": validation_errors
        })));
    }

    // Versioning: snapshot current flows before overwriting
    {
        let versioning_config = &state.red_settings.versioning;
        if versioning_config.enabled {
            let flows_path_guard_v = state.flows_file_path.read().await;
            if let Some(flows_path) = flows_path_guard_v.as_ref() {
                if flows_path.exists() {
                    match load_flows_from_file(flows_path).await {
                        Ok(current_flows) if !current_flows.is_empty() => {
                            let store = crate::versioning::FlowVersionStore::new(flows_path, versioning_config);
                            if let Err(e) = store.save_version(&current_flows, None).await {
                                log::warn!("Failed to save flow version snapshot: {e}");
                            }
                        }
                        Ok(_) => {} // empty file, skip
                        Err(e) => log::warn!("Failed to load current flows for versioning: {e}"),
                    }
                }
            }
        }
    }

    // Save flows to file if path is available
    let flows_path_guard = state.flows_file_path.read().await;
    if let Some(flows_path) = flows_path_guard.as_ref() {
        match save_flows_to_file(&parsed_payload.flows, flows_path).await {
            Ok(_) => {
                log::info!("Flows saved to file: {}", flows_path.display());

                // Redeploy flows using event-driven approach
                let engine_guard = state.engine.read().await;
                if let Some(_engine) = engine_guard.as_ref() {
                    let flows_json = serde_json::Value::Array(parsed_payload.flows);
                    match state.redeploy_flows(flows_json).await {
                        Ok(_) => {
                            log::info!("Flows redeployed successfully!");
                            // Send deploy success notification with actual revision
                            let engine_guard2 = state.engine.read().await;
                            let revision = if let Some(engine) = engine_guard2.as_ref() {
                                Some(engine.flows_rev().await)
                            } else {
                                None
                            };
                            state.comms.send_deploy_notification(true, revision.as_deref()).await;
                            // Note: other notifications will be sent automatically by event listeners
                        }
                        Err(e) => {
                            log::error!("Failed to redeploy flows: {e}");
                            state.comms.send_deploy_notification(false, Some("0")).await;
                            state.comms.send_notification("error", &format!("Failed to redeploy flows: {e}")).await;
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        }
                    }
                } else {
                    // Fall back to traditional restart method
                    log::warn!("Engine not available in AppState, falling back to traditional restart");
                    // Send deploy success notification with fallback revision
                    state.comms.send_deploy_notification(true, Some("1")).await;
                    state
                        .comms
                        .send_notification(
                            "success",
                            &format!("Successfully deployed {} flows", parsed_payload.flows.len()),
                        )
                        .await;
                    restart_engine_if_available(&state).await;
                }
            }
            Err(e) => {
                log::error!("Failed to save flows to file: {e}");
                state.comms.send_deploy_notification(false, Some("0")).await;
                state.comms.send_notification("error", &format!("Failed to save flows: {e}")).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::error!("No flows file path configured, cannot save flows");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let response = serde_json::json!({
        "rev": "1"
    });

    Ok(Json(response))
}

/// Get flows state
pub async fn get_flows_state(Extension(state): Extension<Arc<WebState>>) -> Result<Json<Value>, StatusCode> {
    // Check if engine is available and its running state
    let engine_guard = state.engine.read().await;
    let (started, state_str) = if let Some(engine) = engine_guard.as_ref() {
        let is_running = engine.is_running();
        if is_running { (true, "started") } else { (false, "stopped") }
    } else {
        // If no engine instance, return stopped state
        (false, "stopped")
    };

    let response = serde_json::json!({
        "started": started,
        "state": state_str
    });

    Ok(Json(response))
}

/// Set flows state
pub async fn post_flows_state(
    Extension(state): Extension<Arc<WebState>>,
    Json(payload): Json<FlowState>,
) -> Result<Json<Value>, StatusCode> {
    log::info!("Setting flows state to: {}", payload.state);

    // Check if state value is valid
    let engine_guard = state.engine.read().await;
    let (started, state_str) = match payload.state.as_str() {
        "start" => {
            // Start flows
            if let Some(engine) = engine_guard.as_ref() {
                match engine.start().await {
                    Ok(_) => {
                        log::info!("Engine started successfully");
                        state.comms.send_notification("success", "Flow engine started").await;
                        (true, "started")
                    }
                    Err(e) => {
                        log::error!("Failed to start engine: {e}");
                        state.comms.send_notification("error", &format!("Failed to start engine: {e}")).await;
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            } else {
                log::warn!("No engine available to start");
                state.comms.send_notification("warning", "No engine available to start").await;
                (false, "stopped")
            }
        }
        "stop" => {
            // Stop flows
            if let Some(engine) = engine_guard.as_ref() {
                match engine.stop().await {
                    Ok(_) => {
                        log::info!("Engine stopped successfully");
                        state.comms.send_notification("success", "Flow engine stopped").await;
                        (false, "stopped")
                    }
                    Err(e) => {
                        log::error!("Failed to stop engine: {e}");
                        state.comms.send_notification("error", &format!("Failed to stop engine: {e}")).await;
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            } else {
                log::warn!("No engine available to stop");
                (false, "stopped")
            }
        }
        _ => {
            log::error!("Invalid state value: {}", payload.state);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let response = serde_json::json!({
        "started": started,
        "state": state_str
    });

    Ok(Json(response))
}

/// Get single flow
pub async fn get_flow(
    Extension(state): Extension<Arc<WebState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let flows = if let Some(flows_path) = flows_path_guard.as_ref() {
        match load_flows_from_file(flows_path).await {
            Ok(flows) => flows,
            Err(e) => {
                log::error!("Failed to load flows from file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::warn!("No flows file path configured");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // Find the specified flow
    for flow in &flows {
        if let Some(flow_id) = flow.get("id").and_then(|v| v.as_str())
            && let Some(flow_type) = flow.get("type").and_then(|v| v.as_str())
            && flow_id == id
            && flow_type == "tab"
        {
            return Ok(Json(flow.clone()));
        }
    }

    Err(StatusCode::NOT_FOUND)
}

/// Create new flow
pub async fn post_flow(
    Extension(state): Extension<Arc<WebState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    // Input size validation
    let payload_str = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let max_flow_size = state.red_settings.security.max_flow_size;
    if payload_str.len() > max_flow_size {
        log::warn!("Single flow payload too large: {} bytes (max: {} bytes)", payload_str.len(), max_flow_size);
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let flows_path_guard = state.flows_file_path.read().await;
    let mut flows = if let Some(flows_path) = flows_path_guard.as_ref() {
        match load_flows_from_file(flows_path).await {
            Ok(flows) => flows,
            Err(e) => {
                log::error!("Failed to load flows from file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::warn!("No flows file path configured");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    flows.push(payload.clone());

    // Save back to file
    if let Some(flows_path) = flows_path_guard.as_ref() {
        if let Err(e) = save_flows_to_file(&flows, flows_path).await {
            log::error!("Failed to save flows to file: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Restart engine after modification
        restart_engine_if_available(&state).await;
    }

    Ok(Json(payload))
}

/// Update flow
pub async fn put_flow(
    Extension(state): Extension<Arc<WebState>>,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let mut flows = if let Some(flows_path) = flows_path_guard.as_ref() {
        match load_flows_from_file(flows_path).await {
            Ok(flows) => flows,
            Err(e) => {
                log::error!("Failed to load flows from file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::warn!("No flows file path configured");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // Find and update the specified flow
    let mut found = false;
    for flow in &mut flows {
        if let Some(flow_id) = flow.get("id").and_then(|v| v.as_str())
            && flow_id == id
        {
            *flow = payload.clone();
            found = true;
            break;
        }
    }

    if !found {
        return Err(StatusCode::NOT_FOUND);
    }

    // Save back to file
    if let Some(flows_path) = flows_path_guard.as_ref() {
        if let Err(e) = save_flows_to_file(&flows, flows_path).await {
            log::error!("Failed to save flows to file: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Restart engine after modification
        restart_engine_if_available(&state).await;
    }

    Ok(Json(payload))
}

/// Delete flow
pub async fn delete_flow(
    Extension(state): Extension<Arc<WebState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let mut flows = if let Some(flows_path) = flows_path_guard.as_ref() {
        match load_flows_from_file(flows_path).await {
            Ok(flows) => flows,
            Err(e) => {
                log::error!("Failed to load flows from file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        log::warn!("No flows file path configured");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // Find and delete the specified flow
    let initial_len = flows.len();
    flows.retain(|flow| flow.get("id").and_then(|v| v.as_str()).is_none_or(|flow_id| flow_id != id));

    if flows.len() < initial_len {
        // Save back to file
        if let Some(flows_path) = flows_path_guard.as_ref() {
            if let Err(e) = save_flows_to_file(&flows, flows_path).await {
                log::error!("Failed to save flows to file: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            // Restart engine after modification
            restart_engine_if_available(&state).await;
        }

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Validate that all config_node references point to existing nodes in the flow.
/// Returns Ok(()) if valid, Err(Vec<error details>) if invalid.
fn validate_flow_config_nodes(flows: &[Value]) -> Result<(), Vec<Value>> {
    // Collect all node IDs present in the flow
    let all_node_ids: HashSet<String> =
        flows.iter().filter_map(|node| node.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())).collect();

    let mut errors = Vec::new();

    for node in flows {
        // Only check nodes that have a config_node field
        if let Some(config_node_val) = node.get("configNode").or_else(|| node.get("config_node")) {
            let config_id = match config_node_val.as_str() {
                Some(s) => s,
                None => continue,
            };

            // Empty string means not configured yet — allow deploy (node will show error at runtime)
            if config_id.is_empty() {
                continue;
            }

            // Check if the referenced config node exists in the flow
            if !all_node_ids.contains(config_id) {
                let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                let node_name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
                errors.push(serde_json::json!({
                    "nodeId": node_id,
                    "nodeType": node_type,
                    "nodeName": node_name,
                    "configNode": config_id,
                    "error": format!("Config node '{}' not found in flow", config_id)
                }));
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// `GET /credentials/{type}/{id}`
///
/// Node-RED's editor calls this when opening a config node edit dialog for types that
/// declare `credentials` in their `registerType()`. Node-RED embeds credential values
/// as a nested `credentials` property on each config node in the flows array. We extract
/// and return them here, or `{}` if not found (e.g. new node).
pub async fn get_credentials(
    Extension(state): Extension<Arc<WebState>>,
    Path((_node_type, node_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let flows = match flows_path_guard.as_ref() {
        Some(path) => match load_flows_from_file(path).await {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to load flows for credentials: {e}");
                return Ok(Json(serde_json::json!({})));
            }
        },
        None => return Ok(Json(serde_json::json!({}))),
    };

    // Find the config node by ID and return its nested credentials object
    for node in &flows {
        if node.get("id").and_then(|v| v.as_str()) == Some(&node_id) {
            if let Some(creds) = node.get("credentials").cloned() {
                return Ok(Json(creds));
            }
        }
    }

    Ok(Json(serde_json::json!({})))
}
