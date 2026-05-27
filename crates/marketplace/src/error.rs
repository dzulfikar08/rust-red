//! Marketplace error types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Error type for marketplace operations.
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("version not found: {0}@{1}")]
    VersionNotFound(String, String),

    #[error("version already exists: {0}@{1}")]
    VersionConflict(String, String),

    #[error("invalid semver: {0}")]
    InvalidSemver(String),

    #[error("version {0} is not newer than existing {1}")]
    VersionNotNewer(String, String),

    #[error("plugin too large: {0} bytes (max {1})")]
    PluginTooLarge(u64, u64),

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON error body returned by all marketplace API errors.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub code: &'static str,
}

impl IntoResponse for MarketplaceError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            MarketplaceError::NotFound(_) | MarketplaceError::VersionNotFound(_, _) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND")
            }
            MarketplaceError::VersionConflict(_, _) => (StatusCode::CONFLICT, "VERSION_CONFLICT"),
            MarketplaceError::InvalidSemver(_) => (StatusCode::BAD_REQUEST, "INVALID_SEMVER"),
            MarketplaceError::VersionNotNewer(_, _) => (StatusCode::CONFLICT, "VERSION_NOT_NEWER"),
            MarketplaceError::PluginTooLarge(_, _) => (StatusCode::PAYLOAD_TOO_LARGE, "PLUGIN_TOO_LARGE"),
            MarketplaceError::VerificationFailed(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VERIFICATION_FAILED"),
            MarketplaceError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            MarketplaceError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = ErrorBody { error: self.to_string(), code };

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for MarketplaceError {
    fn from(e: anyhow::Error) -> Self {
        MarketplaceError::Internal(e.to_string())
    }
}
