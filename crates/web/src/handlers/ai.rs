//! Axum handlers for AI assistant endpoints.
//!
//! Provides REST API endpoints for:
//! - Chat with streaming (SSE) responses
//! - Node suggestions based on flow context
//! - Node/flow explanation
//! - Listing available AI providers

use std::sync::Arc;

use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{
        IntoResponse, Json,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures_util::stream::Stream;
use rust_red_core::runtime::ai::provider::{AiProvider, ChatChunk, ChatMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Shared AI state
// ---------------------------------------------------------------------------

/// Shared AI state that holds the configured provider and chat history.
pub struct AiState {
    /// The active AI provider.
    pub provider: RwLock<Option<Box<dyn AiProvider>>>,
    /// Chat history per session (session_id -> messages).
    pub chat_history: RwLock<Vec<ChatMessage>>,
    /// Whether AI is enabled.
    pub enabled: bool,
}

impl AiState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { provider: RwLock::new(None), chat_history: RwLock::new(Vec::new()), enabled: false })
    }

    pub fn with_provider(provider: Box<dyn AiProvider>, enabled: bool) -> Arc<Self> {
        Arc::new(Self { provider: RwLock::new(Some(provider)), chat_history: RwLock::new(Vec::new()), enabled })
    }

    pub async fn set_provider(&self, provider: Box<dyn AiProvider>) {
        let mut guard = self.provider.write().await;
        *guard = Some(provider);
    }

    pub async fn clear_history(&self) {
        let mut guard = self.chat_history.write().await;
        guard.clear();
    }
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    /// Optional flow JSON to include as context.
    pub flow_context: Option<String>,
    /// Optional provider override.
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponseRest {
    pub message: String,
    pub usage: Option<TokenUsageRest>,
}

#[derive(Debug, Serialize)]
pub struct TokenUsageRest {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    pub context: String,
    pub flow_json: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestResponse {
    pub suggestions: Vec<NodeSuggestionRest>,
}

#[derive(Debug, Serialize)]
pub struct NodeSuggestionRest {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub default_config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    pub target: String,
    pub flow_context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainResponse {
    pub explanation: String,
}

#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub enabled: bool,
    pub default_provider: String,
    pub available_providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub available: bool,
}

#[derive(Debug, Deserialize)]
pub struct ClearHistoryQuery {
    pub session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST `/ai/chat` - Send a chat message and get a complete (non-streaming) response.
pub async fn chat(
    Extension(ai_state): Extension<Arc<AiState>>,
    Json(body): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if !ai_state.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI assistant is not enabled. Configure [ai] in rust-red.toml"
            })),
        ));
    }

    let provider_guard = ai_state.provider.read().await;
    let provider = provider_guard.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "No AI provider configured"})))
    })?;

    // Build system prompt with flow context
    let system_prompt = build_system_prompt(body.flow_context.as_deref());

    // Append user message to history
    {
        let mut history = ai_state.chat_history.write().await;
        history.push(ChatMessage { role: "user".to_string(), content: body.message.clone() });
        // Keep history bounded
        if history.len() > 50 {
            let drain = history.len() - 50;
            history.drain(..drain);
        }
    }

    let history = ai_state.chat_history.read().await;
    let messages: Vec<ChatMessage> = history.clone();

    match provider.chat(&messages, Some(&system_prompt)).await {
        Ok(resp) => {
            // Append assistant response to history
            drop(history);
            {
                let mut hist = ai_state.chat_history.write().await;
                hist.push(resp.message.clone());
            }

            let chat_resp = ChatResponseRest {
                message: resp.message.content,
                usage: resp.usage.map(|u| TokenUsageRest {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
            };
            Ok(Json(chat_resp))
        }
        Err(e) => {
            log::error!("AI chat error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("AI provider error: {e}")})),
            ))
        }
    }
}

/// POST `/ai/chat/stream` - Send a chat message and get a streaming SSE response.
pub async fn chat_stream(
    Extension(ai_state): Extension<Arc<AiState>>,
    Json(body): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if !ai_state.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "AI assistant is not enabled. Configure [ai] in rust-red.toml"
            })),
        ));
    }

    let provider_guard = ai_state.provider.read().await;
    let provider = provider_guard.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "No AI provider configured"})))
    })?;

    let system_prompt = build_system_prompt(body.flow_context.as_deref());

    // Append user message to history
    {
        let mut history = ai_state.chat_history.write().await;
        history.push(ChatMessage { role: "user".to_string(), content: body.message.clone() });
        if history.len() > 50 {
            let drain = history.len() - 50;
            history.drain(..drain);
        }
    }

    let history = ai_state.chat_history.read().await;
    let messages: Vec<ChatMessage> = history.clone();

    match provider.chat_stream(&messages, Some(&system_prompt)).await {
        Ok(stream) => {
            // We need to clone the Arc<AiState> into the stream to append the
            // collected assistant response when the stream completes.
            let ai_state_clone = ai_state.clone();
            let stream_stream = convert_stream_to_sse(stream, ai_state_clone);

            Ok(Sse::new(stream_stream).keep_alive(KeepAlive::default()).into_response())
        }
        Err(e) => {
            log::error!("AI chat stream error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("AI provider error: {e}")})),
            ))
        }
    }
}

/// POST `/ai/suggest` - Get node suggestions based on context.
pub async fn suggest(
    Extension(ai_state): Extension<Arc<AiState>>,
    Json(body): Json<SuggestRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if !ai_state.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "AI assistant is not enabled"})),
        ));
    }

    let provider_guard = ai_state.provider.read().await;
    let provider = provider_guard.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "No AI provider configured"})))
    })?;

    let flow_json = body.flow_json.as_deref().unwrap_or("{}");

    match provider.suggest_nodes(&body.context, flow_json).await {
        Ok(suggestions) => {
            let resp_suggestions: Vec<NodeSuggestionRest> = suggestions
                .into_iter()
                .map(|s| NodeSuggestionRest {
                    node_type: s.node_type,
                    label: s.label,
                    description: s.description,
                    confidence: s.confidence,
                    default_config: s.default_config,
                })
                .collect();

            Ok(Json(SuggestResponse { suggestions: resp_suggestions }))
        }
        Err(e) => {
            log::error!("AI suggest error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("AI provider error: {e}")})),
            ))
        }
    }
}

/// POST `/ai/explain` - Explain a selected node or flow segment.
pub async fn explain(
    Extension(ai_state): Extension<Arc<AiState>>,
    Json(body): Json<ExplainRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if !ai_state.enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "AI assistant is not enabled"})),
        ));
    }

    let provider_guard = ai_state.provider.read().await;
    let provider = provider_guard.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "No AI provider configured"})))
    })?;

    let flow_context = body.flow_context.as_deref().unwrap_or("");

    match provider.explain(&body.target, flow_context).await {
        Ok(explanation) => Ok(Json(ExplainResponse { explanation })),
        Err(e) => {
            log::error!("AI explain error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("AI provider error: {e}")})),
            ))
        }
    }
}

/// GET `/ai/providers` - List available AI providers.
pub async fn providers(
    Extension(ai_state): Extension<Arc<AiState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let provider_guard = ai_state.provider.read().await;

    let (default_name, available) = match provider_guard.as_ref() {
        Some(p) => {
            (p.name().to_string(), vec![ProviderInfo { name: p.name().to_string(), available: p.is_available() }])
        }
        None => (String::new(), Vec::new()),
    };

    Ok(Json(ProvidersResponse {
        enabled: ai_state.enabled,
        default_provider: default_name,
        available_providers: available,
    }))
}

/// DELETE `/ai/history` - Clear chat history.
pub async fn clear_history(
    Extension(ai_state): Extension<Arc<AiState>>,
    Query(_params): Query<ClearHistoryQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    ai_state.clear_history().await;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_system_prompt(flow_context: Option<&str>) -> String {
    let mut prompt = String::from(
        "You are an AI assistant for Rust-Red, a Rust reimplementation of Node-RED. \
         You help users build, debug, and understand their visual flow-based programs. \
         Be concise and practical. When suggesting flows, provide valid Node-RED JSON. \
         Reference specific node types by their exact type names.",
    );

    if let Some(ctx) = flow_context {
        if !ctx.is_empty() {
            prompt.push_str("\n\n## Current Flow Context\n");
            prompt.push_str(ctx);
        }
    }

    prompt
}

/// Convert a provider stream into an axum SSE stream.
fn convert_stream_to_sse(
    stream: std::pin::Pin<
        Box<dyn Stream<Item = Result<ChatChunk, rust_red_core::runtime::ai::provider::AiError>> + Send>,
    >,
    ai_state: Arc<AiState>,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Event, std::convert::Infallible>> + Send>> {
    use futures_util::StreamExt;

    Box::pin(futures_util::stream::unfold(
        (stream, ai_state, String::new()),
        |(mut stream, ai_state, mut collected)| async move {
            match stream.next().await {
                Some(Ok(chunk)) => {
                    if !chunk.delta.is_empty() {
                        collected.push_str(&chunk.delta);
                    }
                    if chunk.done {
                        // Append final assistant message to history
                        let msg = ChatMessage { role: "assistant".to_string(), content: collected.clone() };
                        let mut history = ai_state.chat_history.write().await;
                        history.push(msg);
                    }
                    let event = Event::default().data(
                        serde_json::json!({
                            "delta": chunk.delta,
                            "done": chunk.done
                        })
                        .to_string(),
                    );
                    Some((Ok(event), (stream, ai_state, collected)))
                }
                Some(Err(e)) => {
                    let event = Event::default().data(
                        serde_json::json!({
                            "error": e.to_string(),
                            "done": true
                        })
                        .to_string(),
                    );
                    Some((Ok(event), (stream, ai_state, collected)))
                }
                None => None,
            }
        },
    ))
}
