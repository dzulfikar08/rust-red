use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BrokerConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,
    #[serde(default = "default_keep_alive")]
    pub default_keep_alive_secs: u16,
    #[serde(default = "default_session_expiry")]
    pub session_expiry_secs: u32,
    #[serde(default = "default_max_qos")]
    pub max_qos: u8,
    #[serde(default = "default_true")]
    pub retain_available: bool,
    #[serde(default = "default_true")]
    pub shared_subscriptions_available: bool,
    #[serde(default = "default_true")]
    pub wildcard_subscriptions_available: bool,
    #[serde(default = "default_dispatch_capacity")]
    pub dispatch_channel_capacity: usize,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub persistence: PersistenceConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersistenceConfig {
    #[serde(default = "default_persistence_backend")]
    pub backend: String,
}

fn default_enabled() -> bool {
    false
}
fn default_bind() -> String {
    "127.0.0.1:1883".into()
}
fn default_max_connections() -> usize {
    10000
}
fn default_max_packet_size() -> usize {
    268435
}
fn default_keep_alive() -> u16 {
    60
}
fn default_session_expiry() -> u32 {
    1800
}
fn default_max_qos() -> u8 {
    2
}
fn default_true() -> bool {
    true
}
fn default_dispatch_capacity() -> usize {
    256
}
fn default_persistence_backend() -> String {
    "memory".into()
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            bind: default_bind(),
            max_connections: default_max_connections(),
            max_packet_size: default_max_packet_size(),
            default_keep_alive_secs: default_keep_alive(),
            session_expiry_secs: default_session_expiry(),
            max_qos: default_max_qos(),
            retain_available: default_true(),
            shared_subscriptions_available: default_true(),
            wildcard_subscriptions_available: default_true(),
            dispatch_channel_capacity: default_dispatch_capacity(),
            auth: AuthConfig::default(),
            persistence: PersistenceConfig { backend: default_persistence_backend() },
        }
    }
}

impl BrokerConfig {
    pub fn from_config(cfg: &config::Config) -> Self {
        cfg.get("mqtt_broker").unwrap_or_default()
    }
}
