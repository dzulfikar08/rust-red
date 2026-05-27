//! Role and permission definitions for RBAC.
//!
//! Defines three roles — admin, editor, viewer — with a clear permission
//! matrix controlling access to every action category in Rust-Red.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Permission enum
// ---------------------------------------------------------------------------

/// Fine-grained permission tokens used by the authorisation middleware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Deploy flows (POST /flows, flow state changes)
    Deploy,
    /// Create, update, or delete flows and individual flow nodes
    EditFlows,
    /// Read flows and context data
    ViewFlows,
    /// View debug output (websocket /comms, /debug/*)
    ViewDebug,
    /// Install, enable/disable, or uninstall node modules
    ManageNodes,
    /// Read runtime settings
    ViewSettings,
    /// Modify runtime settings
    EditSettings,
    /// Create, list, and remove users
    ManageUsers,
    /// Access the library API (read/write)
    LibraryAccess,
    /// Access the versioning API (list, diff, rollback)
    VersioningAccess,
}

// ---------------------------------------------------------------------------
// Role enum
// ---------------------------------------------------------------------------

/// RBAC roles recognised by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Editor => write!(f, "editor"),
            Role::Viewer => write!(f, "viewer"),
        }
    }
}

impl Role {
    /// Return the set of permissions granted to this role.
    pub fn permissions(&self) -> &'static [Permission] {
        // Permission matrix:
        //   admin  — full access
        //   editor — edit flows, deploy, view everything, library, versioning
        //   viewer — read-only: view flows, debug, settings
        match self {
            Role::Admin => &[
                Permission::Deploy,
                Permission::EditFlows,
                Permission::ViewFlows,
                Permission::ViewDebug,
                Permission::ManageNodes,
                Permission::ViewSettings,
                Permission::EditSettings,
                Permission::ManageUsers,
                Permission::LibraryAccess,
                Permission::VersioningAccess,
            ],
            Role::Editor => &[
                Permission::Deploy,
                Permission::EditFlows,
                Permission::ViewFlows,
                Permission::ViewDebug,
                Permission::ManageNodes,
                Permission::ViewSettings,
                Permission::LibraryAccess,
                Permission::VersioningAccess,
            ],
            Role::Viewer => &[Permission::ViewFlows, Permission::ViewDebug, Permission::ViewSettings],
        }
    }

    /// Check whether this role grants the given permission.
    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions().contains(&perm)
    }

    /// The default role assigned to newly created users when none is specified.
    pub fn default_role() -> Self {
        Role::Viewer
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_all_permissions() {
        let all = [
            Permission::Deploy,
            Permission::EditFlows,
            Permission::ViewFlows,
            Permission::ViewDebug,
            Permission::ManageNodes,
            Permission::ViewSettings,
            Permission::EditSettings,
            Permission::ManageUsers,
            Permission::LibraryAccess,
            Permission::VersioningAccess,
        ];
        for p in &all {
            assert!(Role::Admin.has_permission(*p), "admin missing {p:?}");
        }
    }

    #[test]
    fn editor_cannot_manage_users_or_edit_settings() {
        assert!(!Role::Editor.has_permission(Permission::ManageUsers));
        assert!(!Role::Editor.has_permission(Permission::EditSettings));
        assert!(Role::Editor.has_permission(Permission::Deploy));
        assert!(Role::Editor.has_permission(Permission::EditFlows));
    }

    #[test]
    fn viewer_is_read_only() {
        assert!(Role::Viewer.has_permission(Permission::ViewFlows));
        assert!(Role::Viewer.has_permission(Permission::ViewDebug));
        assert!(!Role::Viewer.has_permission(Permission::EditFlows));
        assert!(!Role::Viewer.has_permission(Permission::Deploy));
    }
}
