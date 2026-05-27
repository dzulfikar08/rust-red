//! In-memory user store with Argon2 password hashing.
//!
//! Provides a thread-safe (`Arc<RwLock<...>>`) user database that can be
//! shared across Axum handlers. A default admin user is seeded on creation.

use std::collections::HashMap;
use std::sync::Arc;

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::roles::Role;

// ---------------------------------------------------------------------------
// User record
// ---------------------------------------------------------------------------

/// A single user in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Login username.
    pub username: String,
    /// Argon2 PHC-format password hash.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Assigned role.
    pub role: Role,
    /// Whether the account is active.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// API key record
// ---------------------------------------------------------------------------

/// An API key for programmatic access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// The key prefix (first 8 chars) used for identification.
    pub prefix: String,
    /// SHA-256 hash of the full key for secure comparison.
    pub key_hash: String,
    /// Human-readable label.
    pub label: String,
    /// User id this key belongs to.
    pub user_id: String,
    /// Whether this key is active.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// User store
// ---------------------------------------------------------------------------

/// Thread-safe in-memory user store.
#[derive(Debug, Clone)]
pub struct UserStore {
    inner: Arc<RwLock<UserStoreInner>>,
}

#[derive(Debug)]
struct UserStoreInner {
    users: HashMap<String, User>,            // id -> User
    username_index: HashMap<String, String>, // username -> user id
    api_keys: HashMap<String, ApiKey>,       // prefix -> ApiKey
}

impl UserStore {
    /// Create a new empty user store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(UserStoreInner {
                users: HashMap::new(),
                username_index: HashMap::new(),
                api_keys: HashMap::new(),
            })),
        }
    }

    /// Create a store pre-seeded with a default admin user.
    ///
    /// The default admin credentials are `admin` / `admin`.
    /// **Change the password immediately after first login in production.**
    pub fn with_default_admin() -> Self {
        let store = Self::new();
        // We synchronously hash the password; this only runs at startup.
        let hash = hash_password("admin").expect("failed to hash default admin password");
        let admin = User {
            id: Uuid::new_v4().to_string(),
            username: "admin".to_string(),
            password_hash: hash,
            role: Role::Admin,
            active: true,
        };
        // Block on inserting the admin user (store is new, no contention).
        let inner = Arc::clone(&store.inner);
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut guard = inner.write().await;
            guard.username_index.insert("admin".to_string(), admin.id.clone());
            guard.users.insert(admin.id.clone(), admin);
        });
        store
    }

    // -- User management --------------------------------------------------

    /// Create a new user. Returns the user id on success.
    pub async fn create_user(&self, username: &str, password: &str, role: Role) -> Result<String, UserStoreError> {
        let hash = hash_password(password)?;

        let mut guard = self.inner.write().await;

        if guard.username_index.contains_key(username) {
            return Err(UserStoreError::UsernameExists);
        }

        let id = Uuid::new_v4().to_string();
        let user = User { id: id.clone(), username: username.to_string(), password_hash: hash, role, active: true };
        guard.username_index.insert(username.to_string(), id.clone());
        guard.users.insert(id.clone(), user);

        Ok(id)
    }

    /// Look up a user by username and verify the password.
    /// Returns the user on success.
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<User, UserStoreError> {
        let guard = self.inner.read().await;

        let user_id = guard.username_index.get(username).ok_or(UserStoreError::InvalidCredentials)?;

        let user = guard.users.get(user_id).ok_or(UserStoreError::InvalidCredentials)?;

        if !user.active {
            return Err(UserStoreError::AccountDisabled);
        }

        verify_password(password, &user.password_hash)?;

        Ok(user.clone())
    }

    /// Get a user by id.
    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        let guard = self.inner.read().await;
        guard.users.get(user_id).cloned()
    }

    /// List all users (password hashes excluded from serialization).
    pub async fn list_users(&self) -> Vec<User> {
        let guard = self.inner.read().await;
        guard.users.values().cloned().collect()
    }

    /// Update a user's role.
    pub async fn set_role(&self, user_id: &str, role: Role) -> Result<(), UserStoreError> {
        let mut guard = self.inner.write().await;
        let user = guard.users.get_mut(user_id).ok_or(UserStoreError::NotFound)?;
        user.role = role;
        Ok(())
    }

    /// Update a user's password.
    pub async fn set_password(&self, user_id: &str, new_password: &str) -> Result<(), UserStoreError> {
        let hash = hash_password(new_password)?;
        let mut guard = self.inner.write().await;
        let user = guard.users.get_mut(user_id).ok_or(UserStoreError::NotFound)?;
        user.password_hash = hash;
        Ok(())
    }

    /// Delete a user by id.
    pub async fn delete_user(&self, user_id: &str) -> Result<(), UserStoreError> {
        let mut guard = self.inner.write().await;
        let user = guard.users.remove(user_id).ok_or(UserStoreError::NotFound)?;
        guard.username_index.remove(&user.username);
        Ok(())
    }

    // -- API key management ------------------------------------------------

    /// Create a new API key for a user. Returns the raw key string
    /// (only shown once).
    pub async fn create_api_key(&self, user_id: &str, label: &str) -> Result<String, UserStoreError> {
        // Verify user exists
        {
            let guard = self.inner.read().await;
            if !guard.users.contains_key(user_id) {
                return Err(UserStoreError::NotFound);
            }
        }

        // Generate a random API key: "rrk_" prefix + 32 random bytes hex-encoded
        let raw_key = format!("rrk_{}", Uuid::new_v4().to_string().replace('-', ""));
        let prefix = raw_key[..12].to_string();
        let key_hash = sha256_hex(raw_key.as_bytes());

        let api_key = ApiKey {
            prefix: prefix.clone(),
            key_hash,
            label: label.to_string(),
            user_id: user_id.to_string(),
            active: true,
        };

        let mut guard = self.inner.write().await;
        guard.api_keys.insert(prefix, api_key);

        Ok(raw_key)
    }

    /// Authenticate using an API key. Returns the user if the key is valid.
    pub async fn authenticate_api_key(&self, raw_key: &str) -> Result<User, UserStoreError> {
        if raw_key.len() < 12 {
            return Err(UserStoreError::InvalidCredentials);
        }

        let prefix = &raw_key[..12];
        let key_hash = sha256_hex(raw_key.as_bytes());

        let guard = self.inner.read().await;

        let api_key = guard.api_keys.get(prefix).ok_or(UserStoreError::InvalidCredentials)?;

        if !api_key.active {
            return Err(UserStoreError::InvalidCredentials);
        }

        if api_key.key_hash != key_hash {
            return Err(UserStoreError::InvalidCredentials);
        }

        let user = guard.users.get(&api_key.user_id).ok_or(UserStoreError::NotFound)?;

        if !user.active {
            return Err(UserStoreError::AccountDisabled);
        }

        Ok(user.clone())
    }

    /// List API keys for a user (prefixes and labels only).
    pub async fn list_api_keys(&self, user_id: &str) -> Vec<ApiKey> {
        let guard = self.inner.read().await;
        guard.api_keys.values().filter(|k| k.user_id == user_id).cloned().collect()
    }

    /// Revoke an API key by prefix.
    pub async fn revoke_api_key(&self, prefix: &str) -> Result<(), UserStoreError> {
        let mut guard = self.inner.write().await;
        guard.api_keys.remove(prefix).ok_or(UserStoreError::NotFound).map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum UserStoreError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account disabled")]
    AccountDisabled,
    #[error("username already exists")]
    UsernameExists,
    #[error("user not found")]
    NotFound,
    #[error("password hashing error: {0}")]
    HashError(String),
}

// ---------------------------------------------------------------------------
// Password helpers
// ---------------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, UserStoreError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| UserStoreError::HashError(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> Result<(), UserStoreError> {
    let parsed = PasswordHash::new(hash).map_err(|e| UserStoreError::HashError(e.to_string()))?;
    Argon2::default().verify_password(password.as_bytes(), &parsed).map_err(|_| UserStoreError::InvalidCredentials)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_authenticate_user() {
        let store = UserStore::new();
        let id = store.create_user("alice", "password123", Role::Editor).await.unwrap();

        let user = store.authenticate("alice", "password123").await.unwrap();
        assert_eq!(user.id, id);
        assert_eq!(user.role, Role::Editor);
    }

    #[tokio::test]
    async fn wrong_password_fails() {
        let store = UserStore::new();
        store.create_user("bob", "correct", Role::Viewer).await.unwrap();
        assert!(store.authenticate("bob", "wrong").await.is_err());
    }

    #[tokio::test]
    async fn duplicate_username_fails() {
        let store = UserStore::new();
        store.create_user("carol", "pass1", Role::Admin).await.unwrap();
        assert!(store.create_user("carol", "pass2", Role::Editor).await.is_err());
    }

    #[tokio::test]
    async fn api_key_roundtrip() {
        let store = UserStore::new();
        let uid = store.create_user("dave", "pass", Role::Admin).await.unwrap();
        let raw = store.create_api_key(&uid, "test-key").await.unwrap();
        assert!(raw.starts_with("rrk_"));

        let user = store.authenticate_api_key(&raw).await.unwrap();
        assert_eq!(user.id, uid);
    }

    #[tokio::test]
    async fn revoke_api_key() {
        let store = UserStore::new();
        let uid = store.create_user("eve", "pass", Role::Admin).await.unwrap();
        let raw = store.create_api_key(&uid, "test-key").await.unwrap();
        let prefix = &raw[..12];

        store.revoke_api_key(prefix).await.unwrap();
        assert!(store.authenticate_api_key(&raw).await.is_err());
    }
}
