use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct BacnetConfig {
    device_id: u32,
    target_host: Option<String>,
    #[serde(default = "default_target_port")]
    target_port: u16,
    #[serde(default = "default_interface")]
    interface: String,
    #[serde(default = "default_port")]
    port: u16,
    /// COV (Change of Value) subscription lifetime in seconds.
    /// Default: 0 (no auto-subscribe from config; node-level only).
    #[serde(default)]
    cov_lifetime: u32,
    /// APDU timeout in milliseconds (default: 3000).
    #[serde(default = "default_apdu_timeout_ms")]
    apdu_timeout_ms: u32,
    /// Number of retries for confirmed requests (default: 3).
    #[serde(default = "default_retries")]
    retries: u32,
}

fn default_interface() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    47808
}
fn default_target_port() -> u16 {
    47808
}
fn default_apdu_timeout_ms() -> u32 {
    3000
}
fn default_retries() -> u32 {
    3
}

pub(crate) struct BacnetConnection {
    config: BacnetConfig,
}

impl std::fmt::Debug for BacnetConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BacnetConnection")
            .field("device_id", &self.config.device_id)
            .field(
                "target",
                &format!("{}:{}", self.config.target_host.as_deref().unwrap_or("-"), self.config.target_port),
            )
            .finish()
    }
}

#[allow(dead_code)]
impl BacnetConnection {
    pub(crate) fn new(config: BacnetConfig) -> Self {
        Self { config }
    }

    pub fn target_addr(&self) -> String {
        format!("{}:{}", self.config.target_host.as_deref().unwrap_or("127.0.0.1"), self.config.target_port)
    }

    pub fn device_id(&self) -> u32 {
        self.config.device_id
    }

    pub fn interface(&self) -> &str {
        &self.config.interface
    }

    pub fn port(&self) -> u16 {
        self.config.port
    }

    pub fn cov_lifetime(&self) -> u32 {
        self.config.cov_lifetime
    }

    pub fn apdu_timeout_ms(&self) -> u32 {
        self.config.apdu_timeout_ms
    }

    pub fn retries(&self) -> u32 {
        self.config.retries
    }
}

#[derive(Debug)]
#[global_node("bacnet-config", red_name = "bacnet-config", module = "node-red")]
pub(crate) struct BacnetConfigNode {
    base: BaseGlobalNodeState,
    config: BacnetConfig,
    pub(crate) connection: Arc<Mutex<BacnetConnection>>,
}

#[allow(dead_code)]
impl BacnetConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let bacnet_config = BacnetConfig::deserialize(&config.rest)?;
        let connection = BacnetConnection::new(bacnet_config.clone());
        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "bacnet-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(BacnetConfigNode {
            base: state,
            config: bacnet_config,
            connection: Arc::new(Mutex::new(connection)),
        }))
    }

    pub fn device_id(&self) -> u32 {
        self.config.device_id
    }

    pub fn interface(&self) -> &str {
        &self.config.interface
    }

    pub fn port(&self) -> u16 {
        self.config.port
    }

    pub fn cov_lifetime(&self) -> u32 {
        self.config.cov_lifetime
    }
}

#[async_trait]
impl GlobalNodeBehavior for BacnetConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
