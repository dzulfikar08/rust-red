use crate::handlers::WebState;
use crate::models::*;
use axum::{Extension, extract::Path, http::StatusCode, response::Json};
use rust_red_core::runtime::credentials::{
    CredentialError, CredentialSettings, CredentialStore, apply_credentials, editor_view, strip_credentials,
};
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

/// Save flows the way Node-RED does: the `credentials` of each node go to the credentials file
/// next to the flows file (see `rust_red_core::runtime::credentials`), the flows file never holds
/// them. `current` is what the flows file contains now, so credentials written inline there by
/// hand are moved over instead of lost. Fails without writing anything when the stored credentials
/// cannot be read.
pub(crate) async fn save_flows_extracting_credentials(
    settings: &CredentialSettings,
    flows: &mut [serde_json::Value],
    current: &[serde_json::Value],
    file_path: &StdPath,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    CredentialStore::for_flows_file(file_path, settings).extract_for_deploy(flows, current)?;
    save_flows_to_file(flows, file_path).await
}

/// The flows as the engine runs them: stored credentials merged back into each node.
pub(crate) fn flows_with_credentials(
    settings: &CredentialSettings,
    flows: Vec<serde_json::Value>,
    file_path: &StdPath,
) -> Result<serde_json::Value, CredentialError> {
    let mut flows = serde_json::Value::Array(flows);
    CredentialStore::for_flows_file(file_path, settings).merge_into_flows(&mut flows)?;
    Ok(flows)
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
        vec![]
    };
    // Node-RED never sends credentials with the flows, the editor fetches them per node.
    strip_credentials(&mut flows);

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
    let mut parsed_payload: FlowsPayload = match serde_json::from_str(&payload) {
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
            if let Some(flows_path) = flows_path_guard_v.as_ref()
                && flows_path.exists()
            {
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

    // Save flows to file if path is available
    let flows_path_guard = state.flows_file_path.read().await;
    if let Some(flows_path) = flows_path_guard.as_ref() {
        let current_flows = load_flows_from_file(flows_path).await.unwrap_or_default();
        let saved = save_flows_extracting_credentials(
            &state.credential_settings,
            &mut parsed_payload.flows,
            &current_flows,
            flows_path,
        )
        .await;
        match saved {
            Ok(_) => {
                log::info!("Flows saved to file: {}", flows_path.display());

                // Redeploy flows using event-driven approach
                let engine_guard = state.engine.read().await;
                if let Some(_engine) = engine_guard.as_ref() {
                    let flows_json =
                        match flows_with_credentials(&state.credential_settings, parsed_payload.flows, flows_path) {
                            Ok(flows) => flows,
                            Err(e) => {
                                log::error!("Failed to merge credentials into the deployed flows: {e}");
                                state.comms.send_deploy_notification(false, Some("0")).await;
                                state
                                    .comms
                                    .send_notification("error", &format!("Failed to load credentials: {e}"))
                                    .await;
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        };
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

    let current_flows = flows.clone();
    flows.push(payload.clone());

    // Save back to file
    if let Some(flows_path) = flows_path_guard.as_ref() {
        if let Err(e) =
            save_flows_extracting_credentials(&state.credential_settings, &mut flows, &current_flows, flows_path).await
        {
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

    let current_flows = flows.clone();
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
        if let Err(e) =
            save_flows_extracting_credentials(&state.credential_settings, &mut flows, &current_flows, flows_path).await
        {
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

    let current_flows = flows.clone();
    // Find and delete the specified flow
    let initial_len = flows.len();
    flows.retain(|flow| flow.get("id").and_then(|v| v.as_str()).is_none_or(|flow_id| flow_id != id));

    if flows.len() < initial_len {
        // Save back to file
        if let Some(flows_path) = flows_path_guard.as_ref() {
            if let Err(e) =
                save_flows_extracting_credentials(&state.credential_settings, &mut flows, &current_flows, flows_path)
                    .await
            {
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
/// Node-RED's editor calls this when opening a node's edit dialog for types that declare
/// `credentials` in their `registerType()`. The values come from the credentials file next to
/// the flows file, with any `credentials` written inline in the flows file layered on top.
/// Like Node-RED, password values are never returned, only `has_<name>: true`. `{}` when the
/// node has none (a new node).
pub async fn get_credentials(
    Extension(state): Extension<Arc<WebState>>,
    Path((_node_type, node_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let flows_path_guard = state.flows_file_path.read().await;
    let Some(path) = flows_path_guard.as_ref() else {
        return Ok(Json(serde_json::json!({})));
    };
    let flows = match load_flows_from_file(path).await {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to load flows for credentials: {e}");
            return Ok(Json(serde_json::json!({})));
        }
    };
    let stored = match CredentialStore::for_flows_file(path, &state.credential_settings).load() {
        Ok(stored) => stored,
        Err(e) => {
            log::error!("{e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let inline = flows
        .iter()
        .find(|node| node.get("id").and_then(Value::as_str) == Some(node_id.as_str()))
        .and_then(|node| node.get("credentials").cloned());
    let mut probe = serde_json::json!([{ "id": node_id }]);
    if let (Some(inline), Some(node)) = (inline, probe[0].as_object_mut()) {
        node.insert("credentials".to_owned(), inline);
    }
    apply_credentials(&mut probe, &stored);
    match probe[0].get("credentials") {
        Some(Value::Object(creds)) => Ok(Json(editor_view(creds))),
        _ => Ok(Json(serde_json::json!({}))),
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    use rust_red_core::runtime::credentials::{SecretSetting, SecretString};
    use serde_json::json;

    fn state_for(flows_path: &StdPath, secret: SecretSetting) -> Arc<WebState> {
        let mut state = WebState::new();
        let inner = Arc::get_mut(&mut state).expect("the state is not shared yet");
        inner.credential_settings = CredentialSettings { secret, ..Default::default() };
        *inner.flows_file_path.get_mut() = Some(flows_path.to_owned());
        state
    }

    async fn deploy(state: &Arc<WebState>, flows: Value) -> Value {
        let body = json!({ "flows": flows, "rev": "1" }).to_string();
        post_flows(Extension(state.clone()), body).await.expect("the deploy succeeds").0
    }

    fn broker(credentials: Value) -> Value {
        json!([
            {"id": "t1", "type": "tab", "label": "Flow 1"},
            {"id": "b1", "type": "mqtt-broker", "broker": "127.0.0.1", "port": "1883", "credentials": credentials},
        ])
    }

    #[tokio::test]
    async fn editor_deploy_moves_credentials_into_the_encrypted_file() {
        let tmp = tempfile::tempdir().unwrap();
        let flows_path = tmp.path().join("flows.json");
        let secret = SecretSetting::Secret(SecretString::new("deploy secret"));
        let state = state_for(&flows_path, secret.clone());

        deploy(&state, broker(json!({"user": "plant", "password": "pw-1"}))).await;

        let saved = std::fs::read_to_string(&flows_path).unwrap();
        assert!(!saved.contains("credentials") && !saved.contains("pw-1"), "flows.json holds no credentials");
        let cred_text = std::fs::read_to_string(tmp.path().join("flows_cred.json")).unwrap();
        assert!(cred_text.contains("\"$\"") && !cred_text.contains("pw-1"), "the file is encrypted");
        let store = CredentialStore::for_flows_file(&flows_path, &CredentialSettings { secret, ..Default::default() });
        assert_eq!(store.load().unwrap()["b1"], json!({"user": "plant", "password": "pw-1"}));

        // GET /flows leaves credentials out, GET /credentials hides the password.
        let flows = get_flows(Extension(state.clone())).await.unwrap().0;
        assert!(!flows.to_string().contains("pw-1"));
        let view =
            get_credentials(Extension(state.clone()), Path(("mqtt-broker".into(), "b1".into()))).await.unwrap().0;
        assert_eq!(view, json!({"user": "plant", "has_password": true}));
        let none =
            get_credentials(Extension(state.clone()), Path(("mqtt-broker".into(), "nope".into()))).await.unwrap().0;
        assert_eq!(none, json!({}));

        // The editor changes the user and leaves the password alone, then rotates the password.
        deploy(&state, broker(json!({"user": "plant2", "password": "__PWRD__"}))).await;
        assert_eq!(store.load().unwrap()["b1"], json!({"user": "plant2", "password": "pw-1"}));
        deploy(&state, broker(json!({"password": "pw-2"}))).await;
        assert_eq!(store.load().unwrap()["b1"], json!({"user": "plant2", "password": "pw-2"}));

        // A deploy without credentials keeps them, and deleting the node drops them.
        deploy(&state, broker(json!(null))).await;
        assert_eq!(store.load().unwrap()["b1"]["password"], "pw-2");
        deploy(&state, json!([{"id": "t1", "type": "tab", "label": "Flow 1"}])).await;
        assert!(store.load().unwrap().is_empty());
    }

    #[tokio::test]
    async fn inline_credentials_in_the_current_flows_file_are_migrated_on_deploy() {
        let tmp = tempfile::tempdir().unwrap();
        let flows_path = tmp.path().join("flows.json");
        std::fs::write(&flows_path, broker(json!({"user": "old", "password": "inline-pw"})).to_string()).unwrap();
        let state = state_for(&flows_path, SecretSetting::Disabled);

        // The editor never saw the inline credentials, so it deploys the nodes without them.
        deploy(&state, broker(json!(null))).await;

        let saved = std::fs::read_to_string(&flows_path).unwrap();
        assert!(!saved.contains("inline-pw"));
        let cred: Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join("flows_cred.json")).unwrap()).unwrap();
        assert_eq!(cred["b1"], json!({"user": "old", "password": "inline-pw"}));
    }

    #[tokio::test]
    async fn deploy_with_a_wrong_secret_fails_and_keeps_both_files() {
        let tmp = tempfile::tempdir().unwrap();
        let flows_path = tmp.path().join("flows.json");
        let good = SecretSetting::Secret(SecretString::new("right"));
        deploy(&state_for(&flows_path, good), broker(json!({"user": "plant", "password": "pw-1"}))).await;
        let flows_before = std::fs::read_to_string(&flows_path).unwrap();
        let cred_before = std::fs::read_to_string(tmp.path().join("flows_cred.json")).unwrap();

        let wrong = state_for(&flows_path, SecretSetting::Secret(SecretString::new("wrong")));
        let body = json!({ "flows": broker(json!({"password": "evil"})), "rev": "1" }).to_string();
        let status = post_flows(Extension(wrong.clone()), body).await.expect_err("the deploy fails");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(std::fs::read_to_string(&flows_path).unwrap(), flows_before);
        assert_eq!(std::fs::read_to_string(tmp.path().join("flows_cred.json")).unwrap(), cred_before);
        let status = get_credentials(Extension(wrong), Path(("mqtt-broker".into(), "b1".into()))).await.err();
        assert_eq!(status, Some(StatusCode::INTERNAL_SERVER_ERROR));
    }
}
