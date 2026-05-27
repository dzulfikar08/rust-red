//! Axum middleware for extracting and validating authenticated users.
//!
//! Provides:
//! - `auth_middleware` — axum `from_fn` middleware that validates JWT or API
//!   key from the `Authorization` header and injects an `AuthenticatedUser`
//!   into request extensions.
//! - `AuthenticatedUser` — can be extracted directly in handlers via Axum's
//!   `FromRequestParts` mechanism.
//!
//! The `AuthState` is provided via `Extension` layer, consistent with how
//! `WebState` is provided in this codebase.

use std::sync::Arc;

use axum::extract::{FromRequestParts, Request};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::request::Parts;

use super::jwt::validate_access_token;
use super::roles::{Permission, Role};
use super::users::UserStore;

// ---------------------------------------------------------------------------
// Authenticated user — extension data added to the request
// ---------------------------------------------------------------------------

/// Represents an authenticated user that has been extracted from the request.
///
/// Inserted into request extensions by the auth middleware so that downstream
/// handlers can access the identity. Can also be extracted directly as an
/// Axum handler parameter.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub role: Role,
}

/// Allow `AuthenticatedUser` to be used as an Axum extractor.
///
/// ```ignore
/// async fn my_handler(user: AuthenticatedUser) -> impl IntoResponse { ... }
/// ```
impl<S: Send + Sync> FromRequestParts<S> for AuthenticatedUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<AuthenticatedUser>().cloned().ok_or((
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error":"unauthorized","message":"Not authenticated"}).to_string(),
        ))
    }
}

impl AuthenticatedUser {
    /// Check whether the authenticated user holds the given permission.
    /// Returns an HTTP 403 response if not.
    pub fn require_permission(&self, perm: Permission) -> Result<(), Response> {
        if self.role.has_permission(perm) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                serde_json::json!({
                    "error": "forbidden",
                    "message": format!("Requires permission: {perm:?}")
                })
                .to_string(),
            )
                .into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Auth state — shared between middleware and handlers
// ---------------------------------------------------------------------------

/// Shared state needed by the auth middleware and auth handlers.
/// Stored as `Extension<Arc<AuthState>>` on the Axum router.
#[derive(Debug, Clone)]
pub struct AuthState {
    pub user_store: UserStore,
    pub jwt_config: super::jwt::JwtConfig,
    pub auth_enabled: bool,
}

// ---------------------------------------------------------------------------
// Auth middleware (axum `from_fn` style, reads AuthState from Extension)
// ---------------------------------------------------------------------------

/// Axum middleware function that extracts the authenticated user from the
/// `Authorization` header.
///
/// Accepts two schemes:
/// - `Bearer <jwt-token>` — validated as a JWT access token.
/// - `Bearer <api-key>` — validated as an API key (prefixed `rrk_`).
///
/// When authentication is disabled (`auth_enabled = false`) the middleware
/// injects a synthetic admin user so that all routes remain accessible.
///
/// Certain paths are always treated as public (no auth required):
/// - `/auth/login`, `/auth/refresh`, `/auth/logout`
/// - `/api/health`, `/api/info`
pub async fn auth_middleware(
    axum::Extension(auth_state): axum::Extension<Arc<AuthState>>,
    mut req: Request,
    next: Next,
) -> Response {
    // When auth is disabled, inject a synthetic admin user and continue.
    if !auth_state.auth_enabled {
        let synthetic =
            AuthenticatedUser { user_id: "__system".to_string(), username: "anonymous".to_string(), role: Role::Admin };
        req.extensions_mut().insert(synthetic);
        return next.run(req).await;
    }

    let path = req.uri().path();

    // Public paths that never require authentication
    if is_public_path(path) {
        // Still inject anonymous user for consistency
        let synthetic = AuthenticatedUser {
            user_id: "__anonymous".to_string(),
            username: "anonymous".to_string(),
            role: Role::Admin,
        };
        req.extensions_mut().insert(synthetic);
        return next.run(req).await;
    }

    let auth_header = req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok());

    let Some(auth_value) = auth_header else {
        return unauthorized("Missing Authorization header");
    };

    let Some(token) = auth_value.strip_prefix("Bearer ") else {
        return unauthorized("Invalid Authorization scheme; expected Bearer");
    };

    // Try API key first (starts with "rrk_")
    if token.starts_with("rrk_") {
        match auth_state.user_store.authenticate_api_key(token).await {
            Ok(user) => {
                let auth_user = AuthenticatedUser { user_id: user.id, username: user.username, role: user.role };
                req.extensions_mut().insert(auth_user);
                return next.run(req).await;
            }
            Err(e) => {
                log::warn!("API key authentication failed: {e}");
                return unauthorized("Invalid API key");
            }
        }
    }

    // Otherwise treat as JWT
    match validate_access_token(token, &auth_state.jwt_config) {
        Ok(claims) => {
            // Verify the user still exists and is active
            match auth_state.user_store.get_user(&claims.sub).await {
                Some(user) if user.active => {
                    let auth_user = AuthenticatedUser {
                        user_id: user.id,
                        username: user.username,
                        // Use the role from the user store (fresh) rather than
                        // the stale token claim, so role changes take effect
                        // immediately.
                        role: user.role,
                    };
                    req.extensions_mut().insert(auth_user);
                    next.run(req).await
                }
                Some(_) => unauthorized("Account disabled"),
                None => unauthorized("User not found"),
            }
        }
        Err(e) => {
            log::debug!("JWT validation failed: {e}");
            unauthorized("Invalid or expired token")
        }
    }
}

/// Check if a path should be publicly accessible without authentication.
fn is_public_path(path: &str) -> bool {
    matches!(path, "/auth/login" | "/auth/refresh" | "/auth/logout")
        || path.starts_with("/api/health")
        || path.starts_with("/api/info")
}

fn unauthorized(msg: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        serde_json::json!({
            "error": "unauthorized",
            "message": msg
        })
        .to_string(),
    )
        .into_response()
}
