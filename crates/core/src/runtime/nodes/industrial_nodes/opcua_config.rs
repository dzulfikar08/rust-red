use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::runtime::engine::Engine;
use crate::runtime::model::Variant;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct OpcUaConfig {
    endpoint: String,
    #[serde(default = "default_security_mode")]
    security_mode: String,
    #[serde(default = "default_security_policy")]
    security_policy: String,
    username: Option<String>,
    password: Option<String>,
    /// Path to the client certificate PEM file (for SignAndEncrypt security).
    #[serde(default)]
    cert_path: Option<String>,
    /// Path to the client private key PEM file.
    #[serde(default)]
    key_path: Option<String>,
    /// Path to the trusted CA PEM or directory.
    #[serde(default)]
    trusted_certs_path: Option<String>,
    /// Session timeout in milliseconds (default: 30000).
    #[serde(default = "default_session_timeout_ms")]
    session_timeout_ms: u32,
    /// Authentication method: "anonymous", "credentials", or "certificate".
    #[serde(default)]
    auth_method: Option<String>,
}

fn default_security_mode() -> String {
    "None".to_string()
}
fn default_security_policy() -> String {
    "None".to_string()
}
fn default_session_timeout_ms() -> u32 {
    30000
}

#[derive(Debug)]
pub(crate) struct OpcUaConnection {
    endpoint: String,
    security_mode: String,
    security_policy: String,
    connected: bool,
}

impl OpcUaConnection {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[derive(Debug)]
#[global_node("opcua-config", red_name = "opcua-config", module = "node-red")]
pub(crate) struct OpcUaConfigNode {
    base: BaseGlobalNodeState,
    config: OpcUaConfig,
    pub(crate) connection: Arc<RwLock<Option<OpcUaConnection>>>,
}

impl OpcUaConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let opcua_config = OpcUaConfig::deserialize(&config.rest)?;

        // Validate certificate paths if security requires them
        if opcua_config.security_mode != "None" {
            if let Some(ref cert) = opcua_config.cert_path {
                let p = PathBuf::from(cert);
                if !p.exists() {
                    log::warn!("[opcua-config] Certificate file not found: {}", cert);
                }
            }
            if let Some(ref key) = opcua_config.key_path {
                let p = PathBuf::from(key);
                if !p.exists() {
                    log::warn!("[opcua-config] Private key file not found: {}", key);
                }
            }
        }

        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "opcua-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(OpcUaConfigNode { base: state, config: opcua_config, connection: Arc::new(RwLock::new(None)) }))
    }

    pub async fn ensure_connected(&self) -> crate::Result<()> {
        {
            let guard = self.connection.read().await;
            if guard.is_some() {
                return Ok(());
            }
        }
        let mut guard = self.connection.write().await;
        if guard.is_none() {
            log::info!(
                "[opcua-config:{}] Connecting to {} (security={}, policy={}, sessionTimeout={}ms)",
                self.name(),
                self.config.endpoint,
                self.config.security_mode,
                self.config.security_policy,
                self.config.session_timeout_ms,
            );

            // TODO: When opcua client is fully wired, establish real session here.
            // For now, record that a connection was attempted.
            *guard = Some(OpcUaConnection {
                endpoint: self.config.endpoint.clone(),
                security_mode: self.config.security_mode.clone(),
                security_policy: self.config.security_policy.clone(),
                connected: true,
            });
        }
        Ok(())
    }

    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    pub fn security_mode(&self) -> &str {
        &self.config.security_mode
    }

    pub fn security_policy(&self) -> &str {
        &self.config.security_policy
    }

    pub fn cert_path(&self) -> Option<&str> {
        self.config.cert_path.as_deref()
    }

    pub fn key_path(&self) -> Option<&str> {
        self.config.key_path.as_deref()
    }

    pub fn session_timeout_ms(&self) -> u32 {
        self.config.session_timeout_ms
    }
}

#[async_trait]
impl GlobalNodeBehavior for OpcUaConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
