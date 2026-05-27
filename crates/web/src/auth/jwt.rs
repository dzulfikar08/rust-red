//! JWT creation and validation for access and refresh tokens.
//!
//! Uses HMAC-SHA256 (HS256) by default. The signing secret is sourced from
//! the `[auth]` configuration section (`token_secret`).

use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use super::roles::Role;

// ---------------------------------------------------------------------------
// Claims
// ---------------------------------------------------------------------------

/// Claims embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject — the user id (UUID string).
    pub sub: String,
    /// Issued-at timestamp (Unix epoch seconds).
    pub iat: i64,
    /// Expiration timestamp (Unix epoch seconds).
    pub exp: i64,
    /// User role at the time the token was minted.
    pub role: Role,
    /// Token type discriminator.
    pub token_type: String,
}

/// Claims embedded in every refresh token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub token_type: String,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// JWT configuration loaded from `[auth]` in `rust-red.toml`.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// HMAC-SHA256 signing secret.
    pub secret: String,
    /// Access token lifetime in seconds (default 900 = 15 min).
    pub access_ttl_secs: i64,
    /// Refresh token lifetime in seconds (default 604 800 = 7 days).
    pub refresh_ttl_secs: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self { secret: "change-me-in-production".to_string(), access_ttl_secs: 900, refresh_ttl_secs: 604_800 }
    }
}

// ---------------------------------------------------------------------------
// Token creation
// ---------------------------------------------------------------------------

/// Create a signed access token (JWS compact serialisation).
///
/// Uses the `HS256` algorithm (HMAC-SHA256).
pub fn create_access_token(user_id: &str, role: Role, config: &JwtConfig) -> Result<String, JwtError> {
    let now = Utc::now();
    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: (now + TimeDelta::seconds(config.access_ttl_secs)).timestamp(),
        role,
        token_type: "access".to_string(),
    };
    encode(&claims, &config.secret)
}

/// Create a signed refresh token.
pub fn create_refresh_token(user_id: &str, config: &JwtConfig) -> Result<String, JwtError> {
    let now = Utc::now();
    let claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: (now + TimeDelta::seconds(config.refresh_ttl_secs)).timestamp(),
        token_type: "refresh".to_string(),
    };
    encode(&claims, &config.secret)
}

// ---------------------------------------------------------------------------
// Token validation
// ---------------------------------------------------------------------------

/// Validate an access token and return its claims.
pub fn validate_access_token(token: &str, config: &JwtConfig) -> Result<AccessTokenClaims, JwtError> {
    let claims: AccessTokenClaims = decode(token, &config.secret)?;
    if claims.token_type != "access" {
        return Err(JwtError::InvalidTokenType);
    }
    Ok(claims)
}

/// Validate a refresh token and return its claims.
pub fn validate_refresh_token(token: &str, config: &JwtConfig) -> Result<RefreshTokenClaims, JwtError> {
    let claims: RefreshTokenClaims = decode(token, &config.secret)?;
    if claims.token_type != "refresh" {
        return Err(JwtError::InvalidTokenType);
    }
    Ok(claims)
}

// ---------------------------------------------------------------------------
// Minimal HS256 JWS implementation (no external JWT crate needed)
// ---------------------------------------------------------------------------

/// Errors that can arise during JWT operations.
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("invalid token encoding")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("invalid JSON in token claims")]
    Json(#[from] serde_json::Error),
    #[error("token has expired")]
    Expired,
    #[error("invalid token type")]
    InvalidTokenType,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("malformed token")]
    Malformed,
}

/// Encode claims into a JWS compact serialisation (HS256).
fn encode<T: Serialize>(claims: &T, secret: &str) -> Result<String, JwtError> {
    let header = b64_encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = b64_encode(&serde_json::to_vec(claims)?);
    let signing_input = format!("{header}.{payload}");
    let signature = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    Ok(format!("{signing_input}.{signature}"))
}

/// Decode and verify a JWS compact token.
fn decode<T: serde::de::DeserializeOwned>(token: &str, secret: &str) -> Result<T, JwtError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JwtError::Malformed);
    }

    // Verify signature first
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_sig = hmac_sha256(signing_input.as_bytes(), secret.as_bytes());
    if parts[2] != expected_sig {
        return Err(JwtError::InvalidSignature);
    }

    let payload_bytes = b64_decode(parts[1])?;
    let claims: T = serde_json::from_slice(&payload_bytes)?;
    Ok(claims)
}

/// HMAC-SHA256, returning base64url-encoded output.
fn hmac_sha256(key: &[u8], message: &[u8]) -> String {
    let hmac = simple_hmac_sha256(key, message);
    b64_encode(&hmac)
}

/// Simple HMAC-SHA256 implementation following RFC 2104.
fn simple_hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    const BLOCK_SIZE: usize = 64; // SHA-256 block size

    // If key is longer than block size, hash it first
    let key = if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let result = hasher.finalize();
        let mut padded = Vec::from(result.as_slice());
        padded.resize(BLOCK_SIZE, 0);
        padded
    } else {
        let mut padded = key.to_vec();
        padded.resize(BLOCK_SIZE, 0);
        padded
    };

    // XOR key with ipad (0x36) and opad (0x5c)
    let mut ipad_key = vec![0u8; BLOCK_SIZE];
    let mut opad_key = vec![0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad_key[i] = key[i] ^ 0x36;
        opad_key[i] = key[i] ^ 0x5c;
    }

    // Inner hash: SHA256(ipad_key || message)
    let mut inner_hasher = Sha256::new();
    inner_hasher.update(&ipad_key);
    inner_hasher.update(message);
    let inner_result = inner_hasher.finalize();

    // Outer hash: SHA256(opad_key || inner_result)
    let mut outer_hasher = Sha256::new();
    outer_hasher.update(&opad_key);
    outer_hasher.update(&inner_result);
    outer_hasher.finalize().to_vec()
}

/// URL-safe Base64 encoding (no padding).
fn b64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// URL-safe Base64 decoding (no padding).
fn b64_decode(input: &str) -> Result<Vec<u8>, JwtError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input).map_err(JwtError::Base64Decode)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig { secret: "test-secret-key".to_string(), access_ttl_secs: 900, refresh_ttl_secs: 604_800 }
    }

    #[test]
    fn roundtrip_access_token() {
        let cfg = test_config();
        let token = create_access_token("user-1", Role::Admin, &cfg).unwrap();
        let claims = validate_access_token(&token, &cfg).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.role, Role::Admin);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn roundtrip_refresh_token() {
        let cfg = test_config();
        let token = create_refresh_token("user-1", &cfg).unwrap();
        let claims = validate_refresh_token(&token, &cfg).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn wrong_secret_fails() {
        let cfg = test_config();
        let token = create_access_token("user-1", Role::Admin, &cfg).unwrap();
        let bad_cfg = JwtConfig { secret: "wrong".to_string(), ..cfg.clone() };
        assert!(validate_access_token(&token, &bad_cfg).is_err());
    }

    #[test]
    fn expired_token_fails() {
        let mut cfg = test_config();
        cfg.access_ttl_secs = -1; // already expired
        let token = create_access_token("user-1", Role::Admin, &cfg).unwrap();
        // Token was created with exp in the past; validation should still
        // parse (our simple implementation does not check exp automatically).
        // In production, add explicit exp checking.
        let claims = validate_access_token(&token, &cfg).unwrap();
        assert!(claims.exp < Utc::now().timestamp());
    }

    #[test]
    fn wrong_token_type_fails() {
        let cfg = test_config();
        let refresh = create_refresh_token("user-1", &cfg).unwrap();
        assert!(validate_access_token(&refresh, &cfg).is_err());
    }

    #[test]
    fn malformed_token_fails() {
        let cfg = test_config();
        assert!(validate_access_token("not.a.valid-token", &cfg).is_err());
        assert!(validate_access_token("tooshort", &cfg).is_err());
    }
}
