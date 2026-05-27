//! Authentication and Role-Based Access Control (RBAC) for Rust-Red.
//!
//! This module provides a comprehensive auth system with:
//! - JWT-based authentication (access + refresh tokens)
//! - Three roles: admin, editor, viewer
//! - Permission-gated middleware for all routes
//! - In-memory user store with Argon2 password hashing
//! - API key support for programmatic access
//! - Configurable via `[auth]` section in `rust-red.toml`
//!
//! # Feature flag
//!
//! The entire module is gated behind the `auth` feature flag in
//! `crates/web/Cargo.toml`. When the feature is disabled, a no-op
//! passthrough is used instead.

pub mod config;
pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod roles;
pub mod users;

pub use config::AuthConfig;
pub use handlers::*;
pub use middleware::{AuthState, AuthenticatedUser};
pub use roles::{Permission, Role};
pub use users::UserStore;
