//! AI provider trait and concrete implementations.
//!
//! The [`AiProvider`] trait abstracts over different LLM backends.  Each
//! implementation handles the specifics of communicating with its API.

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::config::{AiConfig, AnthropicConfig, LocalWasmConfig, OpenAiCompatibleConfig};

/// A single message in the chat history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// The text delta for this chunk.
    pub delta: String,
    /// Whether this is the final chunk.
    pub done: bool,
}

/// Complete (non-streaming) response from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// A suggestion returned by the `/ai/suggest` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSuggestion {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub default_config: serde_json::Value,
}

/// Error type for AI provider operations.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response from provider: {0}")]
    InvalidResponse(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Streaming error: {0}")]
    Streaming(String),

    #[error("Provider not configured: {0}")]
    NotConfigured(String),
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        AiError::RequestFailed(e.to_string())
    }
}

/// Type alias for a boxed streaming response.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, AiError>> + Send>>;

/// Trait that all AI providers must implement.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// The human-readable name of this provider.
    fn name(&self) -> &str;

    /// Whether this provider is properly configured and ready to use.
    fn is_available(&self) -> bool;

    /// Send a chat message and get a complete response.
    async fn chat(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatResponse, AiError>;

    /// Send a chat message and get a streaming response.
    async fn chat_stream(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatStream, AiError>;

    /// Get node suggestions based on the given context.
    async fn suggest_nodes(&self, context: &str, current_flow: &str) -> Result<Vec<NodeSuggestion>, AiError>;

    /// Explain a node or flow segment.
    async fn explain(&self, target: &str, context: &str) -> Result<String, AiError>;
}

// ---------------------------------------------------------------------------
// OpenAI-compatible provider (Ollama, LM Studio, etc.)
// ---------------------------------------------------------------------------

/// Provider that talks to an OpenAI-compatible chat completions API.
pub struct OpenAiCompatibleProvider {
    config: OpenAiCompatibleConfig,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiCompatibleConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
        if !config.api_key.is_empty() {
            headers.insert(reqwest::header::AUTHORIZATION, format!("Bearer {}", config.api_key).parse().unwrap());
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        "openai-compatible"
    }

    fn is_available(&self) -> bool {
        self.config.is_configured()
    }

    async fn chat(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatResponse, AiError> {
        let mut api_messages = Vec::new();
        if let Some(prompt) = system_prompt {
            api_messages.push(OpenAiMessage { role: "system".to_string(), content: prompt.to_string() });
        }
        for msg in messages {
            api_messages.push(OpenAiMessage { role: msg.role.clone(), content: msg.content.clone() });
        }

        let body = OpenAiChatRequest {
            model: self.config.model.clone(),
            messages: api_messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
            stream: None,
        };

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let resp =
            self.client.post(&url).json(&body).send().await.map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!("API returned {status}: {text}")));
        }

        let chat_resp: OpenAiChatResponse = resp.json().await.map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let choice = chat_resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AiError::InvalidResponse("No choices in response".to_string()))?;

        let usage = chat_resp.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ChatResponse { message: ChatMessage { role: choice.message.role, content: choice.message.content }, usage })
    }

    async fn chat_stream(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatStream, AiError> {
        let mut api_messages = Vec::new();
        if let Some(prompt) = system_prompt {
            api_messages.push(OpenAiMessage { role: "system".to_string(), content: prompt.to_string() });
        }
        for msg in messages {
            api_messages.push(OpenAiMessage { role: msg.role.clone(), content: msg.content.clone() });
        }

        let body = OpenAiChatRequest {
            model: self.config.model.clone(),
            messages: api_messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
            stream: Some(true),
        };

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        let resp =
            self.client.post(&url).json(&body).send().await.map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!("API returned {status}: {text}")));
        }

        let stream = Self::parse_sse_stream(resp);
        Ok(Box::pin(stream))
    }

    async fn suggest_nodes(&self, context: &str, current_flow: &str) -> Result<Vec<NodeSuggestion>, AiError> {
        let system = "You are a Node-RED / Rust-Red flow assistant. \
            Given the current flow context and a description of what the user wants to accomplish, \
            suggest nodes that should be added. Respond with a JSON array of objects with fields: \
            node_type, label, description, confidence (0.0-1.0), default_config (object). \
            Only respond with valid JSON, no other text.";

        let user_msg = format!("Current flow:\n{current_flow}\n\nUser context:\n{context}\n\nSuggest nodes to add.");

        let messages = vec![ChatMessage { role: "user".to_string(), content: user_msg }];

        let resp = self.chat(&messages, Some(system)).await?;
        let suggestions: Vec<NodeSuggestion> = serde_json::from_str(&resp.message.content).unwrap_or_else(|e| {
            log::warn!("Failed to parse node suggestions: {e}");
            Vec::new()
        });
        Ok(suggestions)
    }

    async fn explain(&self, target: &str, context: &str) -> Result<String, AiError> {
        let system = "You are a Node-RED / Rust-Red expert. Explain the given node or flow segment \
            clearly, including what it does, how it works, and any common patterns or pitfalls. \
            Format your response in Markdown.";

        let user_msg = format!("Explain this:\n\n{target}\n\nFlow context:\n{context}");

        let messages = vec![ChatMessage { role: "user".to_string(), content: user_msg }];

        let resp = self.chat(&messages, Some(system)).await?;
        Ok(resp.message.content)
    }
}

impl OpenAiCompatibleProvider {
    /// Parse an SSE byte stream from the OpenAI-compatible API into ChatChunks.
    fn parse_sse_stream(resp: reqwest::Response) -> impl Stream<Item = Result<ChatChunk, AiError>> {
        use futures_util::StreamExt;

        let byte_stream = resp.bytes_stream();
        byte_stream
            .scan(String::new(), |buffer, chunk_result| {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        return futures_util::future::ready(Some(vec![Err(AiError::Streaming(e.to_string()))]));
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));
                let mut results = Vec::new();

                while let Some(pos) = buffer.find("\n\n") {
                    let event_text = buffer[..pos].to_string();
                    buffer.drain(..pos + 2);

                    for line in event_text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            let data = data.trim();
                            if data == "[DONE]" {
                                results.push(Ok(ChatChunk { delta: String::new(), done: true }));
                                return futures_util::future::ready(Some(results));
                            }

                            match serde_json::from_str::<OpenAiStreamChunk>(data) {
                                Ok(parsed) => {
                                    if let Some(choice) = parsed.choices.first() {
                                        let delta_text = choice.delta.content.clone().unwrap_or_default();
                                        let done = choice.finish_reason.is_some();
                                        if !delta_text.is_empty() || done {
                                            results.push(Ok(ChatChunk { delta: delta_text, done }));
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::debug!("Failed to parse SSE chunk: {e}");
                                }
                            }
                        }
                    }
                }

                futures_util::future::ready(Some(results))
            })
            .flat_map(futures_util::stream::iter)
    }
}

// ---------------------------------------------------------------------------
// Anthropic Claude provider
// ---------------------------------------------------------------------------

/// Provider that talks to the Anthropic Messages API.
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert("anthropic-version", config.api_version.parse().unwrap());
        if !config.api_key.is_empty() {
            headers.insert("x-api-key", config.api_key.parse().unwrap());
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }
}

#[derive(Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicApiMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicChatResponse {
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicApiUsage,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicApiUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicStreamDelta>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AnthropicStreamDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    stop_reason: Option<String>,
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn is_available(&self) -> bool {
        self.config.is_configured()
    }

    async fn chat(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatResponse, AiError> {
        let api_messages: Vec<AnthropicApiMessage> =
            messages.iter().map(|m| AnthropicApiMessage { role: m.role.clone(), content: m.content.clone() }).collect();

        let body = AnthropicChatRequest {
            model: self.config.model.clone(),
            messages: api_messages,
            max_tokens: self.config.max_tokens,
            system: system_prompt.map(|s| s.to_string()),
            stream: None,
        };

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));
        let resp =
            self.client.post(&url).json(&body).send().await.map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!("Anthropic API returned {status}: {text}")));
        }

        let chat_resp: AnthropicChatResponse =
            resp.json().await.map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = chat_resp
            .content
            .into_iter()
            .filter_map(|block| if block.block_type == "text" { block.text } else { None })
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse {
            message: ChatMessage { role: "assistant".to_string(), content },
            usage: Some(TokenUsage {
                prompt_tokens: chat_resp.usage.input_tokens,
                completion_tokens: chat_resp.usage.output_tokens,
                total_tokens: chat_resp.usage.input_tokens.zip(chat_resp.usage.output_tokens).map(|(i, o)| i + o),
            }),
        })
    }

    async fn chat_stream(&self, messages: &[ChatMessage], system_prompt: Option<&str>) -> Result<ChatStream, AiError> {
        let api_messages: Vec<AnthropicApiMessage> =
            messages.iter().map(|m| AnthropicApiMessage { role: m.role.clone(), content: m.content.clone() }).collect();

        let body = AnthropicChatRequest {
            model: self.config.model.clone(),
            messages: api_messages,
            max_tokens: self.config.max_tokens,
            system: system_prompt.map(|s| s.to_string()),
            stream: Some(true),
        };

        let url = format!("{}/v1/messages", self.config.base_url.trim_end_matches('/'));
        let resp =
            self.client.post(&url).json(&body).send().await.map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!("Anthropic API returned {status}: {text}")));
        }

        let stream = Self::parse_sse_stream(resp);
        Ok(Box::pin(stream))
    }

    async fn suggest_nodes(&self, context: &str, current_flow: &str) -> Result<Vec<NodeSuggestion>, AiError> {
        let system = "You are a Node-RED / Rust-Red flow assistant. \
            Given the current flow context and a description of what the user wants, \
            suggest nodes. Respond with a JSON array: \
            [{\"node_type\": \"...\", \"label\": \"...\", \"description\": \"...\", \
            \"confidence\": 0.0, \"default_config\": {}}]. \
            Only respond with valid JSON.";

        let user_msg = format!("Current flow:\n{current_flow}\n\nUser context:\n{context}\n\nSuggest nodes.");

        let messages = vec![ChatMessage { role: "user".to_string(), content: user_msg }];

        let resp = self.chat(&messages, Some(system)).await?;
        let suggestions: Vec<NodeSuggestion> = serde_json::from_str(&resp.message.content).unwrap_or_else(|e| {
            log::warn!("Failed to parse node suggestions: {e}");
            Vec::new()
        });
        Ok(suggestions)
    }

    async fn explain(&self, target: &str, context: &str) -> Result<String, AiError> {
        let system = "You are a Node-RED / Rust-Red expert. Explain the given node or flow segment \
            clearly, including what it does, how it works, and any common patterns or pitfalls. \
            Format your response in Markdown.";

        let user_msg = format!("Explain this:\n\n{target}\n\nFlow context:\n{context}");

        let messages = vec![ChatMessage { role: "user".to_string(), content: user_msg }];

        let resp = self.chat(&messages, Some(system)).await?;
        Ok(resp.message.content)
    }
}

impl AnthropicProvider {
    fn parse_sse_stream(resp: reqwest::Response) -> impl Stream<Item = Result<ChatChunk, AiError>> {
        use futures_util::StreamExt;

        let byte_stream = resp.bytes_stream();
        byte_stream
            .scan(String::new(), |buffer, chunk_result| {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        return futures_util::future::ready(Some(vec![Err(AiError::Streaming(e.to_string()))]));
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));
                let mut results = Vec::new();

                while let Some(pos) = buffer.find("\n\n") {
                    let event_text = buffer[..pos].to_string();
                    buffer.drain(..pos + 2);

                    for line in event_text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            let data = data.trim();
                            if data == "[DONE]" {
                                results.push(Ok(ChatChunk { delta: String::new(), done: true }));
                                return futures_util::future::ready(Some(results));
                            }

                            match serde_json::from_str::<AnthropicStreamEvent>(data) {
                                Ok(event) => match event.event_type.as_str() {
                                    "content_block_delta" => {
                                        if let Some(delta) = event.delta {
                                            let text = delta.text.unwrap_or_default();
                                            if !text.is_empty() {
                                                results.push(Ok(ChatChunk { delta: text, done: false }));
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        results.push(Ok(ChatChunk { delta: String::new(), done: true }));
                                    }
                                    _ => {}
                                },
                                Err(e) => {
                                    log::debug!("Failed to parse Anthropic SSE chunk: {e}");
                                }
                            }
                        }
                    }
                }

                futures_util::future::ready(Some(results))
            })
            .flat_map(futures_util::stream::iter)
    }
}

// ---------------------------------------------------------------------------
// Local WASM provider (placeholder)
// ---------------------------------------------------------------------------

/// Placeholder provider for local WASM-based inference.
/// Will be implemented when WASM inference is available.
pub struct LocalWasmProvider {
    _config: LocalWasmConfig,
}

impl LocalWasmProvider {
    pub fn new(config: LocalWasmConfig) -> Self {
        Self { _config: config }
    }
}

#[async_trait]
impl AiProvider for LocalWasmProvider {
    fn name(&self) -> &str {
        "local-wasm"
    }

    fn is_available(&self) -> bool {
        // Not yet implemented
        false
    }

    async fn chat(&self, _messages: &[ChatMessage], _system_prompt: Option<&str>) -> Result<ChatResponse, AiError> {
        Err(AiError::ProviderUnavailable("Local WASM inference is not yet implemented".to_string()))
    }

    async fn chat_stream(
        &self,
        _messages: &[ChatMessage],
        _system_prompt: Option<&str>,
    ) -> Result<ChatStream, AiError> {
        Err(AiError::ProviderUnavailable("Local WASM inference is not yet implemented".to_string()))
    }

    async fn suggest_nodes(&self, _context: &str, _current_flow: &str) -> Result<Vec<NodeSuggestion>, AiError> {
        Err(AiError::ProviderUnavailable("Local WASM inference is not yet implemented".to_string()))
    }

    async fn explain(&self, _target: &str, _context: &str) -> Result<String, AiError> {
        Err(AiError::ProviderUnavailable("Local WASM inference is not yet implemented".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Provider factory
// ---------------------------------------------------------------------------

/// Create the configured default AI provider based on the AI config.
pub fn create_default_provider(config: &AiConfig) -> Result<Box<dyn AiProvider>, AiError> {
    match config.default_provider.as_str() {
        "openai-compatible" => {
            if !config.providers.openai_compatible.is_configured() {
                return Err(AiError::NotConfigured("openai-compatible provider is not configured".to_string()));
            }
            Ok(Box::new(OpenAiCompatibleProvider::new(config.providers.openai_compatible.clone())))
        }
        "anthropic" => {
            if !config.providers.anthropic.is_configured() {
                return Err(AiError::NotConfigured(
                    "anthropic provider is not configured (missing API key)".to_string(),
                ));
            }
            Ok(Box::new(AnthropicProvider::new(config.providers.anthropic.clone())))
        }
        "local-wasm" => {
            if !config.providers.local_wasm.is_configured() {
                return Err(AiError::NotConfigured("local-wasm provider is not configured".to_string()));
            }
            Ok(Box::new(LocalWasmProvider::new(config.providers.local_wasm.clone())))
        }
        other => Err(AiError::Configuration(format!("Unknown AI provider: {other}"))),
    }
}

/// Create all configured providers.
pub fn create_all_providers(config: &AiConfig) -> Vec<(String, Box<dyn AiProvider>)> {
    let mut providers = Vec::new();

    if config.providers.openai_compatible.is_configured() {
        providers.push((
            "openai-compatible".to_string(),
            Box::new(OpenAiCompatibleProvider::new(config.providers.openai_compatible.clone())) as Box<dyn AiProvider>,
        ));
    }
    if config.providers.anthropic.is_configured() {
        providers.push((
            "anthropic".to_string(),
            Box::new(AnthropicProvider::new(config.providers.anthropic.clone())) as Box<dyn AiProvider>,
        ));
    }
    if config.providers.local_wasm.is_configured() {
        providers.push((
            "local-wasm".to_string(),
            Box::new(LocalWasmProvider::new(config.providers.local_wasm.clone())) as Box<dyn AiProvider>,
        ));
    }

    providers
}
