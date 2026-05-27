//! AI configuration loaded from the `[ai]` section of `rust-red.toml`.

use serde::{Deserialize, Serialize};

/// Top-level AI configuration.
///
/// Example TOML:
/// ```toml
/// [ai]
/// enabled = true
/// default_provider = "openai-compatible"
///
/// [ai.providers.openai_compatible]
/// base_url = "http://localhost:11434/v1"
/// model = "llama3"
/// api_key = ""
///
/// [ai.providers.anthropic]
/// api_key = ""
/// model = "claude-sonnet-4-20250514"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Whether the AI assistant feature is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Which provider to use by default. Must match a key in `providers`.
    /// Accepted values: `"openai-compatible"`, `"anthropic"`, `"local-wasm"`.
    #[serde(default = "default_provider")]
    pub default_provider: String,

    /// Per-provider configuration. Only the entries that are present are
    /// considered "available" by the runtime.
    #[serde(default)]
    pub providers: AiProvidersConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self { enabled: false, default_provider: default_provider(), providers: AiProvidersConfig::default() }
    }
}

impl AiConfig {
    /// Load the `[ai]` section from a `config::Config`.  Returns the default
    /// (disabled) config when the section is absent.
    pub fn load(cfg: &config::Config) -> Self {
        match cfg.get::<Self>("ai") {
            Ok(c) => c,
            Err(config::ConfigError::NotFound(_)) => {
                log::info!("[ai] config section not found, AI assistant disabled");
                Self::default()
            }
            Err(e) => {
                log::warn!("[ai] config parse error: {e}, using defaults (disabled)");
                Self::default()
            }
        }
    }

    /// Return a list of provider names that have non-empty configuration.
    pub fn available_providers(&self) -> Vec<&str> {
        let mut providers = Vec::new();
        if self.providers.openai_compatible.is_configured() {
            providers.push("openai-compatible");
        }
        if self.providers.anthropic.is_configured() {
            providers.push("anthropic");
        }
        if self.providers.local_wasm.is_configured() {
            providers.push("local-wasm");
        }
        providers
    }
}

/// Per-provider configuration block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvidersConfig {
    #[serde(default)]
    pub openai_compatible: OpenAiCompatibleConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    #[serde(default)]
    pub local_wasm: LocalWasmConfig,
}

impl Default for AiProvidersConfig {
    fn default() -> Self {
        Self {
            openai_compatible: OpenAiCompatibleConfig::default(),
            anthropic: AnthropicConfig::default(),
            local_wasm: LocalWasmConfig::default(),
        }
    }
}

/// OpenAI-compatible provider configuration (Ollama, LM Studio, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleConfig {
    /// Base URL for the OpenAI-compatible API.
    /// Default: `http://localhost:11434/v1` (Ollama)
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,

    /// Model name to use.
    #[serde(default = "default_openai_model")]
    pub model: String,

    /// Optional API key.  Leave empty for local servers that don't require one.
    #[serde(default)]
    pub api_key: String,

    /// Maximum tokens to generate in a single response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Temperature for generation (0.0 - 2.0).
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for OpenAiCompatibleConfig {
    fn default() -> Self {
        Self {
            base_url: default_openai_base_url(),
            model: default_openai_model(),
            api_key: String::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

impl OpenAiCompatibleConfig {
    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty()
    }
}

/// Anthropic Claude provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Anthropic API key.
    #[serde(default)]
    pub api_key: String,

    /// Model to use.
    #[serde(default = "default_anthropic_model")]
    pub model: String,

    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// API version header.
    #[serde(default = "default_anthropic_version")]
    pub api_version: String,

    /// Base URL (override for proxies).
    #[serde(default = "default_anthropic_base_url")]
    pub base_url: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_anthropic_model(),
            max_tokens: default_max_tokens(),
            api_version: default_anthropic_version(),
            base_url: default_anthropic_base_url(),
        }
    }
}

impl AnthropicConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// Local WASM provider configuration (placeholder for future WASM-based inference).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalWasmConfig {
    /// Whether the local WASM provider is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Path to the WASM model file.
    #[serde(default)]
    pub model_path: String,
}

impl Default for LocalWasmConfig {
    fn default() -> Self {
        Self { enabled: false, model_path: String::new() }
    }
}

impl LocalWasmConfig {
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.model_path.is_empty()
    }
}

// --- Default value helpers ---

fn default_provider() -> String {
    "openai-compatible".to_string()
}

fn default_openai_base_url() -> String {
    "http://localhost:11434/v1".to_string()
}

fn default_openai_model() -> String {
    "llama3".to_string()
}

fn default_anthropic_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_anthropic_version() -> String {
    "2023-06-01".to_string()
}

fn default_anthropic_base_url() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}
