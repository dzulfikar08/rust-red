//! Integration tests for the web API layer and frontend plugin system.

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use rust_red_core::web::frontend_plugin::{FrontendPlugin, FrontendPluginRegistry, PluginDescriptor};
    use std::path::PathBuf;
    use tower::ServiceExt;

    // -----------------------------------------------------------------------
    // Dummy plugin for testing
    // -----------------------------------------------------------------------

    struct TestPlugin {
        desc: PluginDescriptor,
    }

    impl TestPlugin {
        fn new(id: &str, prefix: &str) -> Self {
            Self {
                desc: PluginDescriptor {
                    id: id.to_string(),
                    name: format!("Test {id}"),
                    version: "0.0.1".to_string(),
                    description: "A test plugin".to_string(),
                    route_prefix: prefix.to_string(),
                    static_dir: None,
                },
            }
        }
    }

    impl FrontendPlugin for TestPlugin {
        fn descriptor(&self) -> &PluginDescriptor {
            &self.desc
        }
    }

    // -----------------------------------------------------------------------
    // FrontendPluginRegistry tests
    // -----------------------------------------------------------------------

    #[test]
    fn registry_starts_empty() {
        let reg = FrontendPluginRegistry::new();
        assert_eq!(reg.iter().count(), 0);
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn register_and_get_plugin() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(TestPlugin::new("test1", "/test1")));

        let plugin = reg.get("test1").expect("plugin should exist");
        assert_eq!(plugin.descriptor().id, "test1");
        assert_eq!(plugin.descriptor().route_prefix, "/test1");
        assert!(plugin.static_dir().is_none());
    }

    #[test]
    fn register_multiple_and_unregister() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(TestPlugin::new("a", "/a")));
        reg.register(Box::new(TestPlugin::new("b", "/b")));
        reg.register(Box::new(TestPlugin::new("c", "/c")));

        assert_eq!(reg.iter().count(), 3);

        // Unregister middle
        assert!(reg.unregister("b"));
        assert_eq!(reg.iter().count(), 2);
        assert!(reg.get("b").is_none());

        // Double unregister returns false
        assert!(!reg.unregister("b"));
    }

    #[test]
    fn descriptors_returns_all() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(TestPlugin::new("x", "/x")));
        reg.register(Box::new(TestPlugin::new("y", "/y")));

        let descs = reg.descriptors();
        assert_eq!(descs.len(), 2);
        let ids: Vec<&str> = descs.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"x"));
        assert!(ids.contains(&"y"));
    }

    #[test]
    fn build_router_from_plugins() {
        let mut reg = FrontendPluginRegistry::new();
        reg.register(Box::new(TestPlugin::new("r1", "/r1")));
        reg.register(Box::new(TestPlugin::new("r2", "/r2")));

        let _router = reg.build_router();
        // Router construction succeeds without panic
    }

    #[test]
    fn plugin_descriptor_serialize() {
        let desc = PluginDescriptor {
            id: "editor".to_string(),
            name: "Flow Editor".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            route_prefix: "/editor".to_string(),
            static_dir: Some(PathBuf::from("/tmp/editor")),
        };

        let json = serde_json::to_value(&desc).expect("should serialize");
        assert_eq!(json["id"], "editor");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["route_prefix"], "/editor");
    }

    // -----------------------------------------------------------------------
    // PluginDescriptor default values
    // -----------------------------------------------------------------------

    #[test]
    fn plugin_with_static_dir() {
        let desc = PluginDescriptor {
            id: "with-static".to_string(),
            name: "With Static".to_string(),
            version: "0.1.0".to_string(),
            description: "Has static files".to_string(),
            route_prefix: "/app".to_string(),
            static_dir: Some(PathBuf::from("/some/path")),
        };

        let plugin = TestPluginForStatic { desc };
        assert_eq!(plugin.static_dir(), Some(&PathBuf::from("/some/path")));
    }

    struct TestPluginForStatic {
        desc: PluginDescriptor,
    }

    impl FrontendPlugin for TestPluginForStatic {
        fn descriptor(&self) -> &PluginDescriptor {
            &self.desc
        }
    }

    // -----------------------------------------------------------------------
    // Health check endpoint test (using axum test helpers)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn health_check_returns_healthy() {
        use crate::health::health_check;

        // Build a minimal router with the health check handler
        let app = Router::new().route("/health", axum::routing::get(health_check));

        let response = app.oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], "rust-red-web");
    }

    #[tokio::test]
    async fn api_info_returns_metadata() {
        use crate::health::api_info;

        let app = Router::new().route("/info", axum::routing::get(api_info));

        let response = app.oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["name"], "Rust-Red Web API");
    }
}
