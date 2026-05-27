//! HTTP handlers for authentication endpoints.
//!
//! Provides:
//! - `POST /auth/login` — exchange credentials for access/refresh tokens
//! - `POST /auth/refresh` — exchange a refresh token for a new access token
//! - `POST /auth/logout` — acknowledge logout (client-side token discard)
//! - `GET  /auth/me` — return the current user's profile
//! - `GET  /auth/users` — list all users (admin only)
//! - `POST /auth/users` — create a user (admin only)
//! - `DELETE /auth/users/:id` — delete a user (admin only)
//! - `PUT /auth/users/:id/role` — update a user's role (admin only)
//! - `PUT /auth/users/:id/password` — update a user's password (admin or self)
//! - `POST /auth/api-keys` — create an API key
//! - `GET  /auth/api-keys` — list current user's API keys
//! - `DELETE /auth/api-keys/:prefix` — revoke an API key

use std::sync::Arc;

use axum::Extension;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

use super::jwt::{create_access_token, create_refresh_token, validate_refresh_token};
use super::middleware::{AuthState, AuthenticatedUser};
use super::roles::{Permission, Role};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub role: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "Role::default_role")]
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub key: String,
    pub prefix: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub prefix: String,
    pub label: String,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /auth/login`
pub async fn login(
    Extension(auth_state): Extension<Arc<AuthState>>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = match auth_state.user_store.authenticate(&body.username, &body.password).await {
        Ok(u) => u,
        Err(e) => {
            log::warn!("Login failed for '{}': {e}", body.username);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "invalid_credentials",
                    "message": "Invalid username or password"
                })),
            )
                .into_response();
        }
    };

    let access_token = match create_access_token(&user.id, user.role, &auth_state.jwt_config) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create access token: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let refresh_token = match create_refresh_token(&user.id, &auth_state.jwt_config) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create refresh token: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    log::info!("User '{}' logged in successfully", user.username);

    Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: auth_state.jwt_config.access_ttl_secs,
    })
    .into_response()
}

/// `POST /auth/refresh`
pub async fn refresh_token(
    Extension(auth_state): Extension<Arc<AuthState>>,
    Json(body): Json<RefreshRequest>,
) -> impl IntoResponse {
    let claims = match validate_refresh_token(&body.refresh_token, &auth_state.jwt_config) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "invalid_token",
                    "message": format!("Refresh token is invalid: {e}")
                })),
            )
                .into_response();
        }
    };

    // Look up user to get current role
    let user = match auth_state.user_store.get_user(&claims.sub).await {
        Some(u) if u.active => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "invalid_token",
                    "message": "User not found or disabled"
                })),
            )
                .into_response();
        }
    };

    let access_token = match create_access_token(&user.id, user.role, &auth_state.jwt_config) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create access token: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let new_refresh = match create_refresh_token(&user.id, &auth_state.jwt_config) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create refresh token: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(TokenResponse {
        access_token,
        refresh_token: new_refresh,
        token_type: "Bearer".to_string(),
        expires_in: auth_state.jwt_config.access_ttl_secs,
    })
    .into_response()
}

/// `POST /auth/logout`
///
/// JWT is stateless, so "logout" is simply a client-side token discard.
/// This endpoint returns 200 to acknowledge the request.
pub async fn logout() -> impl IntoResponse {
    Json(serde_json::json!({"message": "Logged out successfully"}))
}

/// `GET /auth/me`
pub async fn me(user: AuthenticatedUser) -> impl IntoResponse {
    Json(UserProfile { id: user.user_id, username: user.username, role: user.role.to_string(), active: true })
}

/// `GET /auth/users` — admin only
pub async fn list_users(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
) -> impl IntoResponse {
    if let Err(resp) = user.require_permission(Permission::ManageUsers) {
        return resp;
    }

    let users = auth_state.user_store.list_users().await;
    let profiles: Vec<UserProfile> = users
        .into_iter()
        .map(|u| UserProfile { id: u.id, username: u.username, role: u.role.to_string(), active: u.active })
        .collect();

    Json(profiles).into_response()
}

/// `POST /auth/users` — admin only
pub async fn create_user(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Json(body): Json<CreateUserRequest>,
) -> impl IntoResponse {
    if let Err(resp) = user.require_permission(Permission::ManageUsers) {
        return resp;
    }

    match auth_state.user_store.create_user(&body.username, &body.password, body.role).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "id": id,
                "username": body.username,
                "role": body.role.to_string()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "user_exists",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `DELETE /auth/users/:id` — admin only
pub async fn delete_user(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Path(target_id): Path<String>,
) -> impl IntoResponse {
    if let Err(resp) = user.require_permission(Permission::ManageUsers) {
        return resp;
    }

    // Prevent self-deletion
    if user.user_id == target_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "bad_request",
                "message": "Cannot delete your own account"
            })),
        )
            .into_response();
    }

    match auth_state.user_store.delete_user(&target_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `PUT /auth/users/:id/role` — admin only
pub async fn update_user_role(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Path(target_id): Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    if let Err(resp) = user.require_permission(Permission::ManageUsers) {
        return resp;
    }

    match auth_state.user_store.set_role(&target_id, body.role).await {
        Ok(()) => Json(serde_json::json!({"message": "Role updated"})).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `PUT /auth/users/:id/password` — admin or self
pub async fn update_user_password(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Path(target_id): Path<String>,
    Json(body): Json<UpdatePasswordRequest>,
) -> impl IntoResponse {
    // Admins can change anyone's password; users can only change their own
    let is_admin = user.role.has_permission(Permission::ManageUsers);
    let is_self = user.user_id == target_id;

    if !is_admin && !is_self {
        return StatusCode::FORBIDDEN.into_response();
    }

    match auth_state.user_store.set_password(&target_id, &body.password).await {
        Ok(()) => Json(serde_json::json!({"message": "Password updated"})).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `POST /auth/api-keys`
pub async fn create_api_key(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Json(body): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    match auth_state.user_store.create_api_key(&user.user_id, &body.label).await {
        Ok(raw_key) => {
            let prefix = raw_key[..12].to_string();
            Json(ApiKeyResponse { key: raw_key, prefix, label: body.label }).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "internal_error",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /auth/api-keys`
pub async fn list_api_keys(
    user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
) -> impl IntoResponse {
    let keys = auth_state.user_store.list_api_keys(&user.user_id).await;
    let infos: Vec<ApiKeyInfo> =
        keys.into_iter().map(|k| ApiKeyInfo { prefix: k.prefix, label: k.label, active: k.active }).collect();
    Json(infos).into_response()
}

/// `DELETE /auth/api-keys/:prefix`
pub async fn revoke_api_key(
    _user: AuthenticatedUser,
    Extension(auth_state): Extension<Arc<AuthState>>,
    Path(prefix): Path<String>,
) -> impl IntoResponse {
    match auth_state.user_store.revoke_api_key(&prefix).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}
