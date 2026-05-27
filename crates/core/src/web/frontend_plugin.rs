//! Pluggable frontend architecture for EdgeLinkd.
//!
//! A `FrontendPlugin` provides a complete frontend experience: routes, static
//! assets, and optionally WebSocket handlers.  The default plugin serves the
//! legacy Node-RED-compatible editor.  Additional plugins (e.g. a modern
//! React-based flow editor, a dashboard, a monitoring UI) can be registered
//! at compile-time via `inventory` or at runtime via
//! `FrontendPluginRegistry::register`.

use axum::Router;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata describing a frontend plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    /// Unique identifier, e.g. `"flow-editor"`, `"dashboard"`.
    pub id: String,
    /// Human-readable name shown in the admin UI.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Short description of what the plugin provides.
    pub description: String,
    /// URL route prefix where the plugin is mounted, e.g. `"/editor"`.
    pub route_prefix: String,
    /// Absolute path to the directory containing static assets.
    pub static_dir: Option<PathBuf>,
}

/// Trait that every frontend plugin must implement.
///
/// Implementations are `Send + Sync` so they can be shared across async tasks.
pub trait FrontendPlugin: Send + Sync {
    /// Return metadata about this plugin.
    fn descriptor(&self) -> &PluginDescriptor;

    /// Return additional axum routes that this plugin contributes.
    ///
    /// Routes are merged into the main router under the plugin's `route_prefix`.
    /// Return `Router::new()` if the plugin does not need custom routes.
    fn routes(&self) -> Router {
        Router::new()
    }

    /// Return the path to this plugin's static assets directory.
    ///
    /// If the plugin has no static assets, return `None`.
    fn static_dir(&self) -> Option<&PathBuf> {
        self.descriptor().static_dir.as_ref()
    }

    /// Called once after the plugin is registered, allowing it to perform
    /// initialisation that requires the full server context.
    fn on_register(&self) {
        // Default no-op
    }
}

// ---------------------------------------------------------------------------
// Compile-time registration via `inventory`
// ---------------------------------------------------------------------------

/// Wrapper used with `inventory::collect!` for compile-time plugin discovery.
pub struct FrontendPluginEntry {
    pub factory: fn() -> Box<dyn FrontendPlugin>,
}

inventory::collect!(FrontendPluginEntry);

// ---------------------------------------------------------------------------
// Runtime plugin registry
// ---------------------------------------------------------------------------

/// Manages all loaded frontend plugins.
pub struct FrontendPluginRegistry {
    plugins: Vec<Box<dyn FrontendPlugin>>,
}

impl FrontendPluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Collect all plugins registered at compile time via `inventory`.
    pub fn collect_static_plugins() -> Self {
        let mut reg = Self::new();
        log::info!("Collecting static frontend plugins...");
        for entry in inventory::iter::<FrontendPluginEntry> {
            let plugin = (entry.factory)();
            let desc = plugin.descriptor();
            log::info!("  - {} v{} ({}) at {}", desc.name, desc.version, desc.id, desc.route_prefix,);
            plugin.on_register();
            reg.plugins.push(plugin);
        }
        reg
    }

    /// Register an additional plugin at runtime.
    pub fn register(&mut self, plugin: Box<dyn FrontendPlugin>) {
        let desc = plugin.descriptor();
        log::info!(
            "Registering frontend plugin: {} v{} ({}) at {}",
            desc.name,
            desc.version,
            desc.id,
            desc.route_prefix,
        );
        plugin.on_register();
        self.plugins.push(plugin);
    }

    /// Unregister a plugin by id. Returns `true` if a plugin was removed.
    pub fn unregister(&mut self, id: &str) -> bool {
        let before = self.plugins.len();
        self.plugins.retain(|p| p.descriptor().id != id);
        self.plugins.len() < before
    }

    /// Iterate over all registered plugins.
    pub fn iter(&self) -> impl Iterator<Item = &dyn FrontendPlugin> {
        self.plugins.iter().map(|p| p.as_ref())
    }

    /// Find a plugin by its id.
    pub fn get(&self, id: &str) -> Option<&dyn FrontendPlugin> {
        self.plugins.iter().find(|p| p.descriptor().id == id).map(|p| p.as_ref())
    }

    /// Build a merged axum Router from all plugin routes.
    ///
    /// Each plugin's routes are nested under its `route_prefix`.
    pub fn build_router(&self) -> Router {
        let mut router = Router::new();
        for plugin in &self.plugins {
            let prefix = &plugin.descriptor().route_prefix;
            let plugin_routes = plugin.routes();
            // Only nest if the plugin provides routes (empty router check is
            // tricky; just always nest – axum handles empty routers fine).
            router = router.nest(prefix, plugin_routes);
        }
        router
    }

    /// Return descriptors for all registered plugins.
    pub fn descriptors(&self) -> Vec<PluginDescriptor> {
        self.plugins.iter().map(|p| p.descriptor().clone()).collect()
    }
}

impl Default for FrontendPluginRegistry {
    fn default() -> Self {
        Self::collect_static_plugins()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPlugin {
        desc: PluginDescriptor,
    }

    impl DummyPlugin {
        fn new(id: &str, prefix: &str) -> Self {
            Self {
                desc: PluginDescriptor {
                    id: id.to_string(),
                    name: format!("Dummy {id}"),
                    version: "0.1.0".to_string(),
                    description: "A test plugin".to_string(),
                    route_prefix: prefix.to_string(),
                    static_dir: None,
                },
            }
        }
    }

    impl FrontendPlugin for DummyPlugin {
        fn descriptor(&self) -> &PluginDescriptor {
            &self.desc
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(DummyPlugin::new("test", "/test")));
        assert!(reg.get("test").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_unregister() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(DummyPlugin::new("a", "/a")));
        reg.register(Box::new(DummyPlugin::new("b", "/b")));
        assert_eq!(reg.iter().count(), 2);
        assert!(reg.unregister("a"));
        assert_eq!(reg.iter().count(), 1);
        assert!(!reg.unregister("a")); // already removed
    }

    #[test]
    fn test_descriptors() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(DummyPlugin::new("x", "/x")));
        let descs = reg.descriptors();
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].id, "x");
    }

    #[test]
    fn test_build_router() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(DummyPlugin::new("r", "/r")));
        let _router = reg.build_router();
        // Router built without panic is sufficient
    }
}
