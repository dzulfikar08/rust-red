//! Built-in frontend plugins.
//!
//! This module contains the default set of frontend plugins that ship with
//! EdgeLinkd.  Each plugin is registered at compile time via `inventory`.

use axum::{Router, extract::Extension, response::IntoResponse, routing::get};
use rust_red_core::web::frontend_plugin::{FrontendPlugin, FrontendPluginEntry, PluginDescriptor};
use std::path::PathBuf;
use std::sync::Arc;

use crate::handlers::WebState;

// ---------------------------------------------------------------------------
// Flow Editor Plugin (modern React frontend)
// ---------------------------------------------------------------------------

/// The built-in flow editor plugin serves the modern React-based flow editor
/// at `/editor/` and exposes an API for listing loaded plugins.
pub struct FlowEditorPlugin {
    descriptor: PluginDescriptor,
}

impl FlowEditorPlugin {
    pub fn new() -> Self {
        let static_dir = std::env::var("RUST_RED_EDITOR_STATIC_DIR").ok().map(PathBuf::from).or_else(|| {
            // Default: look for the frontend dist relative to the binary
            let base = rust_red_core::runtime::paths::ui_static_dir();
            let editor_dir = base.join("editor");
            if editor_dir.is_dir() { Some(editor_dir) } else { None }
        });

        Self {
            descriptor: PluginDescriptor {
                id: "flow-editor".to_string(),
                name: "Flow Editor (Modern)".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Modern React-based flow editor with drag-and-drop node canvas".to_string(),
                route_prefix: "/editor".to_string(),
                static_dir,
            },
        }
    }
}

impl Default for FlowEditorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl FrontendPlugin for FlowEditorPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn routes(&self) -> Router {
        // Note: /api/plugins listing is available via the authenticated
        // endpoint GET /api/frontend/plugins (defined in api.rs).
        // Only health/status endpoints are exposed here (no auth required).
        Router::new().route("/api/status", get(editor_status))
    }

    fn static_dir(&self) -> Option<&PathBuf> {
        self.descriptor.static_dir.as_ref()
    }
}

// ---------------------------------------------------------------------------
// API handlers for the flow editor plugin
// ---------------------------------------------------------------------------

/// `GET /editor/api/status` - basic editor status.
async fn editor_status(Extension(state): Extension<Arc<WebState>>) -> impl IntoResponse {
    let engine_guard = state.engine.read().await;
    let running = engine_guard.as_ref().is_some_and(|e| e.is_running());
    let engine_status = if running { "running" } else { "stopped" };
    drop(engine_guard);

    let body = serde_json::json!({
        "status": "ok",
        "engine": engine_status,
        "version": env!("CARGO_PKG_VERSION"),
    });
    axum::Json(body)
}

// ---------------------------------------------------------------------------
// Compile-time registration via inventory
// ---------------------------------------------------------------------------

fn create_flow_editor_plugin() -> Box<dyn FrontendPlugin> {
    Box::new(FlowEditorPlugin::new())
}

inventory::submit! {
    FrontendPluginEntry {
        factory: create_flow_editor_plugin,
    }
}
