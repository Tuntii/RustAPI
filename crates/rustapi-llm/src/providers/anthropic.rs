//! Anthropic Messages API provider.
//!
//! Implements the [`LlmProvider`] trait for Anthropic's Claude models.
//! Supports function calling (tool_use), streaming, and structured output.
//!
//! # Example
//! ```no_run
//! use rustapi_llm::{LlmRouter, CompletionRequest};
//! use rustapi_llm::providers::AnthropicProvider;
//!
//! # async fn example() {
//! let provider = AnthropicProvider::builder()
//!     .api_key("sk-ant-...")
//!     .model("claude-sonnet-4-20250514")
//!     .build();
//!
//! let router = LlmRouter::builder()
//!     .provider(provider)
//!     .build();
//!
//! let resp = router.complete(CompletionRequest::simple("Hello")).await.unwrap();
//! println!("{}", resp.content);
//! # }
//! ```

use crate::{
    CompletionRequest, CompletionResponse, FinishReason, LlmError, LlmProvider, ModelInfo,
    StreamChunk, TokenUsage, ToolCall, ToolCallDelta,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`AnthropicProvider`].
#[derive(Debug, Clone)]
pub struct AnthropicProviderBuilder {
    api_key: Option<String>,
    base_url: String,
    model: String,
    max_tokens: u32,
    max_retries: u32,
    timeout_secs: u64,
    api_version: String,
}

impl Default for AnthropicProviderBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            max_retries: 2,
            timeout_secs: 60,
            api_version: "2023-06-01".to_string(),
        }
    }
}

impl AnthropicProviderBuilder {
    /// Set the API key.  Falls back to `ANTHROPIC_API_KEY` env var.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Default max tokens for completions (required by Anthropic API).
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Maximum retry count for transient errors (default: 2).
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Request timeout in seconds (default: 60).
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Anthropic API version header (default: "2023-06-01").
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Build the provider.  Reads `ANTHROPIC_API_KEY` from env if not set.
    pub fn build(self) -> AnthropicProvider {
        let api_key = self
            .api_key
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .unwrap_or_default();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .expect("failed to build reqwest client");

        AnthropicProvider {
            api_key,
            base_url: self.base_url,
            default_model: self.model,
            default_max_tokens: self.max_tokens,
            max_retries: self.max_retries,
            api_version: self.api_version,
            client,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// Anthropic Messages API provider.
///
/// Implements the Claude Messages API with support for function calling
/// (tool_use), streaming, and structured output.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    default_model: String,
    default_max_tokens: u32,
    max_retries: u32,
    api_version: String,
    client: Client,
}

impl AnthropicProvider {
    /// Start building a new provider.
    pub fn builder() -> AnthropicProviderBuilder {
        AnthropicProviderBuilder::default()
    }

    /// Create with just an API key (uses defaults for everything else).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder().api_key(api_key).build()
    }

    /// Resolve the model: request-level override or provider default.
    fn resolve_model(&self, request: &CompletionRequest) -> String {
        request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone())
    }

    /// Build the request body for the Anthropic Messages API.
    fn build_body(&self, request: &CompletionRequest) -> AnthropicRequestBody {
        let model = self.resolve_model(request);

        // Anthropic separates system messages from the conversation.
        let mut system: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();

        for m in &request.messages {
            match m.role {
                crate::MessageRole::System => {
                    // Anthropic takes system as a top-level string.
                    system = Some(m.content.clone());
                }
                crate::MessageRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Text(m.content.clone()),
                    });
                }
                crate::MessageRole::Assistant => {
                    // If the assistant message contains tool_use results, we
                    // need to encode them as content blocks. For simplicity,
                    // if there are tool_calls we serialize them; otherwise text.
                    if m.tool_calls.is_empty() {
                        messages.push(AnthropicMessage {
                            role: "assistant".to_string(),
                            content: AnthropicContent::Text(m.content.clone()),
                        });
                    } else {
                        let blocks: Vec<AnthropicContentBlock> = m
                            .tool_calls
                            .iter()
                            .map(|tc| AnthropicContentBlock::ToolUse {
                                id: tc.id.clone(),
                                name: tc.name.clone(),
                                input: tc.arguments.clone(),
                            })
                            .collect();
                        messages.push(AnthropicMessage {
                            role: "assistant".to_string(),
                            content: AnthropicContent::Blocks(blocks),
                        });
                    }
                }
                crate::MessageRole::Tool => {
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Blocks(vec![
                            AnthropicContentBlock::ToolResult {
                                tool_use_id: m.tool_call_id.clone().unwrap_or_default(),
                                content: m.content.clone(),
                            },
                        ]),
                    });
                }
            }
        }

        let max_tokens = request.max_tokens.unwrap_or(self.default_max_tokens);

        let tools: Option<Vec<AnthropicToolDef>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| AnthropicToolDef {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t.parameters.clone(),
                    })
                    .collect(),
            )
        };

        AnthropicRequestBody {
            model,
            max_tokens,
            system,
            messages,
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: if request.stop.is_empty() {
                None
            } else {
                Some(request.stop.clone())
            },
            tools,
            stream: None,
        }
    }

    /// Build HTTP request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            self.api_key.parse().expect("invalid api key header"),
        );
        headers.insert(
            "anthropic-version",
            self.api_version.parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers
    }

    /// Parse an Anthropic error response.
    fn parse_error_response(status: reqwest::StatusCode, body: &str) -> LlmError {
        if let Ok(err) = serde_json::from_str::<AnthropicErrorResponse>(body) {
            let msg = format!(
                "{} ({})",
                err.error.message,
                err.error.r#type
            );

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return LlmError::RateLimited {
                    provider: "anthropic".to_string(),
                    retry_after_secs: 30,
                };
            }

            return LlmError::provider("anthropic", msg);
        }

        LlmError::provider("anthropic", format!("HTTP {status}: {body}"))
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                model_id: "claude-sonnet-4-20250514".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                max_output_tokens: 64_000,
                cost_per_m_input: 3.0,
                cost_per_m_output: 15.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
            ModelInfo {
                model_id: "claude-opus-4-20250514".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                max_output_tokens: 32_000,
                cost_per_m_input: 15.0,
                cost_per_m_output: 75.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
            ModelInfo {
                model_id: "claude-3-5-haiku-20241022".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                max_output_tokens: 8_192,
                cost_per_m_input: 0.80,
                cost_per_m_output: 4.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
        ]
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = self.build_body(&request);
        let headers = self.build_headers();
        let model = self.resolve_model(&request);

        debug!(model = %model, url = %url, "Sending Anthropic completion request");

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                debug!(attempt, "Retrying Anthropic request");
                tokio::time::sleep(std::time::Duration::from_millis(
                    500 * 2u64.pow(attempt - 1),
                ))
                .await;
            }

            let response = self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout { timeout_ms: 60_000 }
                    } else {
                        LlmError::provider("anthropic", e.to_string())
                    }
                });

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let body_text = response.text().await.unwrap_or_default();

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status.is_server_error()
                {
                    last_error =
                        Some(Self::parse_error_response(status, &body_text));
                    continue;
                }

                return Err(Self::parse_error_response(status, &body_text));
            }

            let anth_response: AnthropicResponse = response
                .json()
                .await
                .map_err(|e| {
                    LlmError::provider("anthropic", format!("JSON parse error: {e}"))
                })?;

            // Extract text content and tool_use blocks.
            let mut content = String::new();
            let mut tool_calls = Vec::new();

            for block in &anth_response.content {
                match block {
                    AnthropicResponseBlock::Text { text } => {
                        content.push_str(text);
                    }
                    AnthropicResponseBlock::ToolUse { id, name, input } => {
                        tool_calls.push(ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: input.clone(),
                        });
                    }
                }
            }

            let finish_reason = match anth_response.stop_reason.as_deref() {
                Some("end_turn") => FinishReason::Stop,
                Some("max_tokens") => FinishReason::Length,
                Some("tool_use") => FinishReason::ToolCalls,
                Some(other) => FinishReason::Other(other.to_string()),
                None => FinishReason::Stop,
            };

            let usage = TokenUsage {
                input_tokens: anth_response.usage.input_tokens,
                output_tokens: anth_response.usage.output_tokens,
                total_tokens: anth_response.usage.input_tokens
                    + anth_response.usage.output_tokens,
            };

            return Ok(CompletionResponse {
                model: anth_response.model,
                content,
                tool_calls,
                usage,
                finish_reason,
                provider: "anthropic".to_string(),
            });
        }

        Err(last_error.unwrap_or_else(|| {
            LlmError::provider("anthropic", "Unknown error after retries")
        }))
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let url = format!("{}/v1/messages", self.base_url);
        let mut body = self.build_body(&request);
        body.stream = Some(true);
        let headers = self.build_headers();

        debug!("Sending Anthropic streaming completion request");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout { timeout_ms: 60_000 }
                } else {
                    LlmError::provider("anthropic", e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(Self::parse_error_response(status, &body_text));
        }

        let byte_stream = response.bytes_stream();

        let stream = byte_stream
            .map(move |chunk_result| {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        return vec![Err(LlmError::provider(
                            "anthropic",
                            format!("Stream read error: {e}"),
                        ))];
                    }
                };

                let text = String::from_utf8_lossy(&bytes);
                let mut results = Vec::new();

                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];

                    match serde_json::from_str::<AnthropicStreamEvent>(data) {
                        Ok(event) => match event {
                            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                match delta {
                                    AnthropicDelta::TextDelta { text } => {
                                        results.push(Ok(StreamChunk {
                                            content_delta: text,
                                            tool_call_deltas: Vec::new(),
                                            done: false,
                                            usage: None,
                                        }));
                                    }
                                    AnthropicDelta::InputJsonDelta {
                                        partial_json,
                                    } => {
                                        // This is a streaming tool call argument
                                        results.push(Ok(StreamChunk {
                                            content_delta: String::new(),
                                            tool_call_deltas: vec![ToolCallDelta {
                                                id: String::new(),
                                                name: None,
                                                arguments_delta: partial_json,
                                            }],
                                            done: false,
                                            usage: None,
                                        }));
                                    }
                                }
                            }
                            AnthropicStreamEvent::MessageDelta { usage, .. } => {
                                let usage = usage.map(|u| TokenUsage {
                                    input_tokens: u.input_tokens.unwrap_or(0),
                                    output_tokens: u.output_tokens.unwrap_or(0),
                                    total_tokens: u
                                        .input_tokens
                                        .unwrap_or(0)
                                        + u.output_tokens.unwrap_or(0),
                                });
                                results.push(Ok(StreamChunk {
                                    content_delta: String::new(),
                                    tool_call_deltas: Vec::new(),
                                    done: true,
                                    usage,
                                }));
                            }
                            AnthropicStreamEvent::MessageStop => {
                                results.push(Ok(StreamChunk {
                                    content_delta: String::new(),
                                    tool_call_deltas: Vec::new(),
                                    done: true,
                                    usage: None,
                                }));
                            }
                            _ => {} // Ignore other events
                        },
                        Err(e) => {
                            warn!(data, error = %e, "Failed to parse Anthropic stream event");
                        }
                    }
                }
                results
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        // Anthropic doesn't have a /models endpoint. We do a minimal
        // completion as health check.
        let url = format!("{}/v1/messages", self.base_url);
        let headers = self.build_headers();

        let body = serde_json::json!({
            "model": self.default_model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "ping"}]
        });

        self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                LlmError::provider("anthropic", format!("Health check failed: {e}"))
            })?
            .error_for_status()
            .map_err(|e| {
                LlmError::provider("anthropic", format!("Health check failed: {e}"))
            })?;

        Ok(())
    }
}

// ===========================================================================
// Anthropic API wire types (private)
// ===========================================================================

#[derive(Serialize)]
struct AnthropicRequestBody {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

// --- Response types ---

#[derive(Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<AnthropicResponseBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

// --- Error response ---

#[derive(Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
    r#type: String,
}

// --- Streaming types ---

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {},

    #[serde(rename = "content_block_start")]
    ContentBlockStart {},

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        delta: AnthropicDelta,
    },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop {},

    #[serde(rename = "message_delta")]
    MessageDelta {
        usage: Option<AnthropicStreamUsage>,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping {},

    #[serde(rename = "error")]
    Error {
        #[allow(dead_code)]
        error: AnthropicErrorDetail,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize)]
struct AnthropicStreamUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let provider = AnthropicProvider::builder()
            .api_key("test-key")
            .build();

        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.default_model, "claude-sonnet-4-20250514");
        assert_eq!(provider.base_url, "https://api.anthropic.com");
        assert_eq!(provider.default_max_tokens, 4096);
        assert_eq!(provider.max_retries, 2);
        assert_eq!(provider.api_version, "2023-06-01");
    }

    #[test]
    fn test_builder_custom() {
        let provider = AnthropicProvider::builder()
            .api_key("sk-ant-test")
            .base_url("https://custom.endpoint.com")
            .model("claude-opus-4-20250514")
            .max_tokens(8192)
            .max_retries(5)
            .timeout_secs(120)
            .api_version("2024-01-01")
            .build();

        assert_eq!(provider.default_model, "claude-opus-4-20250514");
        assert_eq!(provider.base_url, "https://custom.endpoint.com");
        assert_eq!(provider.default_max_tokens, 8192);
        assert_eq!(provider.max_retries, 5);
        assert_eq!(provider.api_version, "2024-01-01");
    }

    #[test]
    fn test_available_models() {
        let provider = AnthropicProvider::builder().api_key("test").build();
        let models = provider.available_models();
        assert!(models.len() >= 3);
        assert!(models.iter().any(|m| m.model_id == "claude-sonnet-4-20250514"));
        assert!(models.iter().any(|m| m.model_id == "claude-opus-4-20250514"));
        assert!(models.iter().any(|m| m.model_id == "claude-3-5-haiku-20241022"));
        assert!(models.iter().all(|m| m.supports_tools));
        assert!(models.iter().all(|m| m.provider == "anthropic"));
    }

    #[test]
    fn test_build_body_simple() {
        let provider = AnthropicProvider::builder().api_key("test").build();
        let request = CompletionRequest::simple("Hello");
        let body = provider.build_body(&request);

        assert_eq!(body.model, "claude-sonnet-4-20250514");
        assert_eq!(body.max_tokens, 4096);
        assert!(body.system.is_none());
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
        assert!(body.tools.is_none());
        assert!(body.stream.is_none());
    }

    #[test]
    fn test_build_body_with_system_message() {
        let provider = AnthropicProvider::builder().api_key("test").build();
        let request = CompletionRequest::simple("Hi")
            .with_system("You are helpful.")
            .with_max_tokens(100);

        let body = provider.build_body(&request);

        // System message should be extracted to top-level field
        assert_eq!(body.system.as_deref(), Some("You are helpful."));
        assert_eq!(body.max_tokens, 100);
        // Only the user message should remain
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
    }

    #[test]
    fn test_build_body_with_tools() {
        let provider = AnthropicProvider::builder().api_key("test").build();
        let request = CompletionRequest::simple("Call a function").with_tools(vec![
            crate::FunctionDef {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": { "city": { "type": "string" } }
                }),
            },
        ]);

        let body = provider.build_body(&request);
        let tools = body.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_weather");
    }

    #[test]
    fn test_headers() {
        let provider = AnthropicProvider::builder()
            .api_key("sk-ant-test123")
            .build();
        let headers = provider.build_headers();

        assert_eq!(
            headers.get("x-api-key").unwrap().to_str().unwrap(),
            "sk-ant-test123"
        );
        assert_eq!(
            headers.get("anthropic-version").unwrap().to_str().unwrap(),
            "2023-06-01"
        );
    }

    #[test]
    fn test_parse_error_response_json() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;
        let err = AnthropicProvider::parse_error_response(
            reqwest::StatusCode::UNAUTHORIZED,
            body,
        );
        match err {
            LlmError::ProviderError { message, .. } => {
                assert!(message.contains("Invalid API key"));
                assert!(message.contains("authentication_error"));
            }
            other => panic!("Expected ProviderError, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_error_response_rate_limit() {
        let body =
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"Rate limit exceeded"}}"#;
        let err = AnthropicProvider::parse_error_response(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            body,
        );
        assert!(matches!(err, LlmError::RateLimited { .. }));
    }

    #[test]
    fn test_parse_error_response_non_json() {
        let body = "Service Unavailable";
        let err = AnthropicProvider::parse_error_response(
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            body,
        );
        match err {
            LlmError::ProviderError { message, .. } => {
                assert!(message.contains("503"));
            }
            other => panic!("Expected ProviderError, got: {other:?}"),
        }
    }

    #[test]
    fn test_anthropic_content_serialization() {
        // Text content
        let text = AnthropicContent::Text("Hello".to_string());
        let json = serde_json::to_string(&text).unwrap();
        assert_eq!(json, r#""Hello""#);

        // Block content
        let blocks = AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolResult {
            tool_use_id: "toolu_123".to_string(),
            content: "result".to_string(),
        }]);
        let json = serde_json::to_value(&blocks).unwrap();
        assert!(json.is_array());
    }
}
