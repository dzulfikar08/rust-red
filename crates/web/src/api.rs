use axum::{
    Router,
    routing::{delete, get, post, put},
};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::auth::UserStore;
use crate::auth::handlers::*;
use crate::auth::jwt::JwtConfig;
use crate::auth::middleware::{AuthState, auth_middleware};
use crate::handlers::dashboard::{self, DashboardStore};
use crate::handlers::library::*;
use crate::handlers::web_state::WebState;
use crate::handlers::*;
use crate::health::*;
use crate::security::AuditLogLayer;
use crate::security::SecurityHeadersLayer;
use crate::security::rate_limit::RateLimitLayer;
use std::sync::Arc;

/// `GET /api/frontend/plugins` - list all registered frontend plugins.
async fn list_frontend_plugins(
    axum::Extension(state): axum::Extension<Arc<WebState>>,
) -> axum::Json<serde_json::Value> {
    let descriptors = state.frontend_plugins.descriptors();
    axum::Json(serde_json::json!({
        "plugins": descriptors
    }))
}

#[cfg(feature = "ai")]
use crate::handlers::ai as ai_handlers;

/// Axum middleware that creates an OpenTelemetry span for each HTTP request.
/// When the `otel` feature is not enabled this compiles to a no-op pass-through.
#[cfg(feature = "otel")]
async fn otel_http_trace(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    use opentelemetry::trace::{Span, SpanKind, Status, Tracer};

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let tracer = opentelemetry::global::tracer("rust-red");
    let mut span = tracer
        .span_builder("http_request")
        .with_kind(SpanKind::Server)
        .with_attributes(vec![
            opentelemetry::KeyValue::new("http.method", method.to_string()),
            opentelemetry::KeyValue::new("http.route", path.clone()),
        ])
        .start(&tracer);

    // Intentionally avoid ContextGuard — it is !Send and cannot live across an await.
    let response = next.run(req).await;

    let status_code = response.status().as_u16() as i64;
    span.set_attribute(opentelemetry::KeyValue::new("http.status_code", status_code));
    if response.status().is_server_error() {
        span.set_status(Status::Error { description: format!("HTTP {status_code}").into() });
    }
    span.end();

    response
}

#[cfg(not(feature = "otel"))]
async fn otel_http_trace(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
    next.run(req).await
}

/// Create Node-RED compatible API routes
/// These routes directly mimic Node-RED's path structure
fn create_node_red_api_routes() -> Router {
    Router::new()
        // Flows management (Node-RED compatible paths)
        .route("/flows", get(get_flows).post(post_flows))
        .route("/flows/state", get(get_flows_state).post(post_flows_state))
        // Credentials (Node-RED editor loads these for config nodes with credentials)
        .route("/credentials/{type}/{id}", get(get_credentials))
        // Single flow management
        .route("/flow/{id}", get(get_flow).put(put_flow).delete(delete_flow))
        .route("/flow", post(post_flow))
        // Node management (supports complex regex paths)
        .route("/nodes", get(get_nodes).post(install_node_module))
        .route("/nodes/messages", get(get_nodes_locale))
        .route("/nodes/{module}", get(get_node_module).put(toggle_node_module).delete(uninstall_node_module))
        .route("/nodes/{module}/{set}", get(get_node_set).put(toggle_node_set))
        .route("/nodes/{module}/{set}/messages", get(get_node_set_messages))
        // Library API (Node-RED compatible)
        .route("/library/{type}", get(get_library_entries))
        .route("/library/{type}/{*name}", get(get_library_entry).post(post_library_entry))
        // Plugin management (Node-RED expected paths)
        .route("/plugins", get(get_plugins))
        .route("/plugins/messages", get(get_plugin_messages))
        // System settings (Node-RED compatible)
        .route("/settings", get(get_settings))
        // Icons
        .route("/icons", get(get_icons))
        // Theme
        .route("/theme", get(get_theme))
        // Context management
        .route("/context/global", get(get_global_context))
        .route("/context/global/{key}", get(get_global_context_key).delete(delete_global_context_key))
        .route("/context/flow/{id}", get(get_flow_context))
        .route("/context/flow/{id}/{key}", get(get_flow_context_key).delete(delete_flow_context_key))
        .route("/context/node/{id}", get(get_node_context))
        .route("/context/node/{id}/{key}", get(get_node_context_key).delete(delete_node_context_key))
        // Versioning API
        .route("/versioning/versions", get(list_versions))
        .route("/versioning/versions/{id}", get(get_version))
        .route("/versioning/rollback/{id}", post(rollback_version))
        .route("/versioning/diff", get(diff_versions))
}

/// Create editor routes (for frontend file service)
fn create_editor_routes() -> Router {
    Router::new()
        .route("/icons/{module}/{icon}", get(get_icon_file))
        .route("/locales/nodes", get(get_nodes_locale))
        .route("/locales/editor", get(get_editor_locale))
        .route("/locales/available", get(get_available_locales))
        .route("/locales/{namespace}", get(get_namespace_locale))
        .route("/settings/user", get(get_user_settings).post(update_user_settings))
        .route("/comms", get(websocket_handler))
        // Debug node specific routes (Node-RED compatible)
        .route("/debug/view/view.html", get(serve_debug_view_html))
        .route("/debug/view/{resource}", get(serve_debug_view_resource))
        // Debug node root path support (for main editor)
        .route("/debug.js", get(serve_debug_js))
        .route("/debug-utils.js", get(serve_debug_utils_js))
        // General core module resource routes
        .route("/core/{resource_path}", get(serve_core_lib_resource))
}

/// Create debug and health check routes
fn create_debug_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/info", get(api_info))
        .route("/frontend/plugins", get(list_frontend_plugins))
}

/// Create dashboard CRUD routes.
///
/// Dashboard state is kept in an in-memory `DashboardStore` (HashMap + RwLock)
/// that is injected via Axum's `State` extractor.
fn create_dashboard_routes(store: DashboardStore) -> Router {
    Router::new()
        .route("/dashboard", get(dashboard::list_dashboards).post(dashboard::create_dashboard))
        .route(
            "/dashboard/{id}",
            get(dashboard::get_dashboard).put(dashboard::update_dashboard).delete(dashboard::delete_dashboard),
        )
        .with_state(store)
}

/// Create authentication API routes.
///
/// These routes are public (no auth middleware) except for `/auth/me` and
/// user management endpoints which are protected individually by the
/// handler code itself via the `AuthenticatedUser` extractor.
fn create_auth_routes() -> Router {
    Router::new()
        // Public routes (no token required)
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh_token))
        .route("/auth/logout", post(logout))
        // Authenticated routes (token required, extracted via Axum State)
        .route("/auth/me", get(me))
        // User management (admin-only checks in handlers)
        .route("/auth/users", get(list_users).post(create_user))
        .route("/auth/users/{id}/role", put(update_user_role))
        .route("/auth/users/{id}/password", put(update_user_password))
        .route("/auth/users/{id}", delete(delete_user))
        // API key management
        .route("/auth/api-keys", get(list_api_keys).post(create_api_key))
        .route("/auth/api-keys/{prefix}", delete(revoke_api_key))
}

/// Build a CORS layer based on the security configuration.
/// - Empty `cors_origins` (default): no CORS headers (same-origin only).
/// - `cors_origins = ["*"]`: allow all origins (insecure, for dev only).
/// - Otherwise: allow only the specified origins.
fn build_cors_layer(cors_origins: &[String]) -> Option<CorsLayer> {
    if cors_origins.is_empty() {
        // Same-origin only: do not add CORS headers at all.
        // This is the secure default.
        log::info!("CORS: same-origin only (no CORS headers)");
        None
    } else if cors_origins.len() == 1 && cors_origins[0] == "*" {
        // Allow all origins - NOT recommended for production
        log::warn!("CORS: allowing all origins -- this is insecure for production use");
        Some(
            CorsLayer::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods([
                    http::Method::GET,
                    http::Method::POST,
                    http::Method::PUT,
                    http::Method::DELETE,
                    http::Method::OPTIONS,
                ])
                .allow_headers([CONTENT_TYPE, AUTHORIZATION]),
        )
    } else {
        // Allow specific origins
        let origins: Vec<http::HeaderValue> = cors_origins
            .iter()
            .filter_map(|o| match o.parse() {
                Ok(v) => Some(v),
                Err(e) => {
                    log::warn!("CORS: ignoring invalid origin '{o}': {e}");
                    None
                }
            })
            .collect();

        if origins.is_empty() {
            log::warn!("CORS: no valid origins configured, falling back to same-origin only");
            None
        } else {
            log::info!("CORS: allowing {} specific origin(s)", origins.len());
            Some(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list(origins))
                    .allow_methods([
                        http::Method::GET,
                        http::Method::POST,
                        http::Method::PUT,
                        http::Method::DELETE,
                        http::Method::OPTIONS,
                    ])
                    .allow_headers([CONTENT_TYPE, AUTHORIZATION]),
            )
        }
    }
}

/// Create AI assistant API routes.
///
/// All AI endpoints are nested under `/ai/`.  The `#[cfg(feature = "ai")]`
/// attribute ensures these routes are only compiled when the `ai` feature is
/// enabled.
#[cfg(feature = "ai")]
fn create_ai_routes() -> Router {
    use axum::routing::delete;

    Router::new()
        .route("/ai/chat", post(ai_handlers::chat))
        .route("/ai/chat/stream", post(ai_handlers::chat_stream))
        .route("/ai/suggest", post(ai_handlers::suggest))
        .route("/ai/explain", post(ai_handlers::explain))
        .route("/ai/providers", get(ai_handlers::providers))
        .route("/ai/history", delete(ai_handlers::clear_history))
}

/// Create the complete API router
/// This function combines all routes and registers dynamic routes from WebHandlerRegistry
pub fn create_all_routes(web_state: &WebState) -> Router {
    let security = &web_state.red_settings.security;
    let auth_config = &web_state.auth_config;

    // Build auth state (shared across all handlers)
    let user_store = if auth_config.enabled { UserStore::with_default_admin() } else { UserStore::new() };

    let jwt_config = JwtConfig {
        secret: auth_config.token_secret.clone(),
        access_ttl_secs: auth_config.access_token_ttl_secs,
        refresh_ttl_secs: auth_config.refresh_token_ttl_secs,
    };

    let auth_state = AuthState { user_store, jwt_config, auth_enabled: auth_config.enabled };

    if auth_config.enabled {
        log::info!("Authentication enabled (RBAC active)");
    } else {
        log::info!("Authentication disabled (all routes open)");
    }

    // Build auth routes — these handle login/logout/token/user management.
    let auth_routes = create_auth_routes();

    let mut router = Router::new()
        .merge(create_node_red_api_routes())
        .merge(create_editor_routes())
        .nest("/api", create_debug_routes())
        .merge(create_dashboard_routes(dashboard::new_dashboard_store()))
        .merge(auth_routes);

    // AI assistant routes (feature-gated)
    #[cfg(feature = "ai")]
    {
        router = router.merge(create_ai_routes());
    }

    // Cluster API routes (feature-gated)
    #[cfg(feature = "cluster")]
    {
        let cm = web_state.cluster_manager.read().unwrap().clone();
        if let Some(cluster_mgr) = cm {
            let cluster_state = rust_red_cluster::api::ClusterApiState { manager: cluster_mgr };
            router = router.nest("/cluster", rust_red_cluster::api::cluster_router(cluster_state));
        }
    }

    // Apply auth middleware to all routes (including Node-RED API routes).
    // When auth is disabled, the middleware injects a synthetic admin user.
    // Public paths (login, refresh) are skipped by the middleware.
    router = router.layer(axum::middleware::from_fn(auth_middleware));

    // Provide AuthState as an Extension OUTSIDE the auth middleware
    // so the middleware can extract it. In Axum, layers execute outside-in,
    // so this layer (added after) wraps the middleware (added before).
    let auth_state_arc = Arc::new(auth_state);
    router = router.layer(axum::Extension(auth_state_arc.clone()));

    // Apply OpenTelemetry HTTP tracing middleware
    router = router.layer(axum::middleware::from_fn(otel_http_trace));

    // Apply CORS layer based on security config
    if let Some(cors_layer) = build_cors_layer(&security.cors_origins) {
        router = router.layer(cors_layer);
    }

    // Apply security headers if enabled (default: true)
    if security.security_headers {
        router = router.layer(SecurityHeadersLayer);
    }

    // Apply rate limiting
    let max_rpm = security.rate_limit_rpm;
    if max_rpm > 0 {
        let rate_limit_layer = RateLimitLayer::new(max_rpm);
        router = router.layer(rate_limit_layer);
        log::info!("Rate limiting enabled: {max_rpm} requests/minute per client");
    } else {
        log::info!("Rate limiting disabled (rate_limit_rpm = 0)");
    }

    // Apply audit logging middleware
    if let Some(ref audit_logger) = web_state.audit_logger {
        router = router.layer(AuditLogLayer::new(audit_logger.clone()));
        log::info!("Audit logging middleware enabled");
    }

    let wh = web_state.web_handlers.routes_handle();
    for desc in wh.lock().unwrap().iter() {
        log::info!("{}", desc.path);
    }

    web_state.register_web_routes(router)
}
