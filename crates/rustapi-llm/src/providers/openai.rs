//! OpenAI-compatible LLM provider.
//!
//! Supports the OpenAI Chat Completions API, including function calling,
//! streaming, and structured outputs.  Also works with any OpenAI-compatible
//! endpoint (Azure OpenAI, Together AI, Groq, etc.) by setting a custom
//! `base_url`.
//!
//! # Example
//! ```no_run
//! use rustapi_llm::{LlmRouter, CompletionRequest};
//! use rustapi_llm::providers::OpenAiProvider;
//!
//! # async fn example() {
//! let provider = OpenAiProvider::builder()
//!     .api_key("sk-...")
//!     .model("gpt-4o")
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

/// Builder for [`OpenAiProvider`].
#[derive(Debug, Clone)]
pub struct OpenAiProviderBuilder {
    api_key: Option<String>,
    base_url: String,
    model: String,
    organization: Option<String>,
    max_retries: u32,
    timeout_secs: u64,
}

impl Default for OpenAiProviderBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            organization: None,
            max_retries: 2,
            timeout_secs: 60,
        }
    }
}

impl OpenAiProviderBuilder {
    /// Set the API key.  Falls back to `OPENAI_API_KEY` env var.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the base URL (for Azure, Together, Groq, etc.).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// OpenAI organization header.
    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
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

    /// Build the provider.  Reads `OPENAI_API_KEY` from env if not set explicitly.
    pub fn build(self) -> OpenAiProvider {
        let api_key = self
            .api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .unwrap_or_default();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .expect("failed to build reqwest client");

        OpenAiProvider {
            api_key,
            base_url: self.base_url,
            default_model: self.model,
            organization: self.organization,
            max_retries: self.max_retries,
            client,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// OpenAI-compatible LLM provider.
///
/// Works with the official OpenAI API and any endpoint that implements the
/// same Chat Completions protocol (Azure OpenAI, Together, Groq, Fireworks…).
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    default_model: String,
    organization: Option<String>,
    max_retries: u32,
    client: Client,
}

impl OpenAiProvider {
    /// Start building a new provider.
    pub fn builder() -> OpenAiProviderBuilder {
        OpenAiProviderBuilder::default()
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

    /// Build the request body from a [`CompletionRequest`].
    fn build_body(&self, request: &CompletionRequest) -> OaiRequestBody {
        let model = self.resolve_model(request);

        let messages: Vec<OaiMessage> = request
            .messages
            .iter()
            .map(|m| OaiMessage {
                role: match m.role {
                    crate::MessageRole::System => "system".to_string(),
                    crate::MessageRole::User => "user".to_string(),
                    crate::MessageRole::Assistant => "assistant".to_string(),
                    crate::MessageRole::Tool => "tool".to_string(),
                },
                content: Some(m.content.clone()),
                tool_call_id: m.tool_call_id.clone(),
                tool_calls: if m.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        m.tool_calls
                            .iter()
                            .map(|tc| OaiToolCall {
                                id: tc.id.clone(),
                                r#type: "function".to_string(),
                                function: OaiFunction {
                                    name: tc.name.clone(),
                                    arguments: tc.arguments.to_string(),
                                },
                            })
                            .collect(),
                    )
                },
            })
            .collect();

        let tools: Option<Vec<OaiToolDef>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| OaiToolDef {
                        r#type: "function".to_string(),
                        function: OaiToolFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.parameters.clone(),
                        },
                    })
                    .collect(),
            )
        };

        let response_format = request.response_schema.as_ref().map(|schema| {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "response",
                    "strict": true,
                    "schema": schema,
                }
            })
        });

        OaiRequestBody {
            model,
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: if request.stop.is_empty() {
                None
            } else {
                Some(request.stop.clone())
            },
            tools,
            response_format,
            stream: None,
        }
    }

    /// Build HTTP request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse()
                .expect("invalid api key header"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if let Some(ref org) = self.organization {
            headers.insert(
                "OpenAI-Organization",
                org.parse().expect("invalid org header"),
            );
        }
        headers
    }

    /// Parse an API-level error response.
    fn parse_error_response(status: reqwest::StatusCode, body: &str) -> LlmError {
        // Try to extract the error message from the JSON response
        if let Ok(err) = serde_json::from_str::<OaiErrorResponse>(body) {
            let msg = format!(
                "{} ({})",
                err.error.message,
                err.error.r#type.as_deref().unwrap_or("unknown")
            );

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return LlmError::RateLimited {
                    provider: "openai".to_string(),
                    retry_after_secs: 30, // sensible default
                };
            }

            return LlmError::provider("openai", msg);
        }

        LlmError::provider(
            "openai",
            format!("HTTP {status}: {body}"),
        )
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        // Common OpenAI models with known specs
        vec![
            ModelInfo {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                context_window: 128_000,
                max_output_tokens: 16_384,
                cost_per_m_input: 2.50,
                cost_per_m_output: 10.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
            ModelInfo {
                model_id: "gpt-4o-mini".to_string(),
                provider: "openai".to_string(),
                context_window: 128_000,
                max_output_tokens: 16_384,
                cost_per_m_input: 0.15,
                cost_per_m_output: 0.60,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
            ModelInfo {
                model_id: "o1".to_string(),
                provider: "openai".to_string(),
                context_window: 200_000,
                max_output_tokens: 100_000,
                cost_per_m_input: 15.0,
                cost_per_m_output: 60.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
            ModelInfo {
                model_id: "o3-mini".to_string(),
                provider: "openai".to_string(),
                context_window: 200_000,
                max_output_tokens: 100_000,
                cost_per_m_input: 1.10,
                cost_per_m_output: 4.40,
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
        let url = format!("{}/chat/completions", self.base_url);
        let body = self.build_body(&request);
        let headers = self.build_headers();
        let model = self.resolve_model(&request);

        debug!(model = %model, url = %url, "Sending OpenAI completion request");

        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                debug!(attempt, "Retrying OpenAI request");
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
                        LlmError::provider("openai", e.to_string())
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

                // Retry on 429 (rate limit) or 5xx (server error)
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status.is_server_error()
                {
                    last_error =
                        Some(Self::parse_error_response(status, &body_text));
                    continue;
                }

                return Err(Self::parse_error_response(status, &body_text));
            }

            let oai_response: OaiChatCompletion = response
                .json()
                .await
                .map_err(|e| LlmError::provider("openai", format!("JSON parse error: {e}")))?;

            let choice = oai_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::provider("openai", "No choices returned"))?;

            let tool_calls = choice
                .message
                .tool_calls
                .unwrap_or_default()
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::String(tc.function.arguments)),
                })
                .collect();

            let finish_reason = match choice.finish_reason.as_deref() {
                Some("stop") => FinishReason::Stop,
                Some("length") => FinishReason::Length,
                Some("tool_calls") => FinishReason::ToolCalls,
                Some("content_filter") => FinishReason::ContentFilter,
                Some(other) => FinishReason::Other(other.to_string()),
                None => FinishReason::Stop,
            };

            let usage = if let Some(u) = oai_response.usage {
                TokenUsage {
                    input_tokens: u.prompt_tokens,
                    output_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }
            } else {
                TokenUsage::default()
            };

            return Ok(CompletionResponse {
                model: oai_response.model,
                content: choice.message.content.unwrap_or_default(),
                tool_calls,
                usage,
                finish_reason,
                provider: "openai".to_string(),
            });
        }

        Err(last_error.unwrap_or_else(|| {
            LlmError::provider("openai", "Unknown error after retries")
        }))
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<
        Pin<Box<dyn futures_util::Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = self.build_body(&request);
        body.stream = Some(true);
        let headers = self.build_headers();

        debug!("Sending OpenAI streaming completion request");

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
                    LlmError::provider("openai", e.to_string())
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
                            "openai",
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
                    if data == "[DONE]" {
                        results.push(Ok(StreamChunk {
                            content_delta: String::new(),
                            tool_call_deltas: Vec::new(),
                            done: true,
                            usage: None,
                        }));
                        break;
                    }

                    match serde_json::from_str::<OaiStreamChunk>(data) {
                        Ok(chunk) => {
                            if let Some(choice) = chunk.choices.into_iter().next() {
                                let delta = choice.delta;

                                let tool_call_deltas: Vec<ToolCallDelta> = delta
                                    .tool_calls
                                    .unwrap_or_default()
                                    .into_iter()
                                    .map(|tc| ToolCallDelta {
                                        id: tc.id.unwrap_or_default(),
                                        name: tc.function.as_ref().and_then(|f| {
                                            f.name.clone()
                                        }),
                                        arguments_delta: tc
                                            .function
                                            .map(|f| f.arguments.unwrap_or_default())
                                            .unwrap_or_default(),
                                    })
                                    .collect();

                                let usage = chunk.usage.map(|u| TokenUsage {
                                    input_tokens: u.prompt_tokens,
                                    output_tokens: u.completion_tokens,
                                    total_tokens: u.total_tokens,
                                });

                                results.push(Ok(StreamChunk {
                                    content_delta: delta.content.unwrap_or_default(),
                                    tool_call_deltas,
                                    done: choice.finish_reason.is_some(),
                                    usage,
                                }));
                            }
                        }
                        Err(e) => {
                            warn!(data, error = %e, "Failed to parse OpenAI stream chunk");
                        }
                    }
                }
                results
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(stream))
    }

    async fn embeddings(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let url = format!("{}/embeddings", self.base_url);
        let headers = self.build_headers();

        let body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": input,
        });

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::provider("openai", e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(Self::parse_error_response(status, &body_text));
        }

        let result: OaiEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LlmError::provider("openai", format!("JSON parse error: {e}")))?;

        Ok(result
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/models", self.base_url);
        let headers = self.build_headers();

        self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| LlmError::provider("openai", format!("Health check failed: {e}")))?
            .error_for_status()
            .map_err(|e| LlmError::provider("openai", format!("Health check failed: {e}")))?;

        Ok(())
    }
}

// ===========================================================================
// OpenAI API wire types (private)
// ===========================================================================

#[derive(Serialize)]
struct OaiRequestBody {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OaiToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Serialize, Deserialize)]
struct OaiToolCall {
    id: String,
    r#type: String,
    function: OaiFunction,
}

#[derive(Serialize, Deserialize)]
struct OaiFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OaiToolDef {
    r#type: String,
    function: OaiToolFunctionDef,
}

#[derive(Serialize)]
struct OaiToolFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// --- Response types ---

#[derive(Deserialize)]
struct OaiChatCompletion {
    model: String,
    choices: Vec<OaiChoice>,
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiChoiceMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiChoiceMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

// --- Error response ---

#[derive(Deserialize)]
struct OaiErrorResponse {
    error: OaiErrorDetail,
}

#[derive(Deserialize)]
struct OaiErrorDetail {
    message: String,
    r#type: Option<String>,
}

// --- Streaming types ---

#[derive(Deserialize)]
struct OaiStreamChunk {
    choices: Vec<OaiStreamChoice>,
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct OaiStreamChoice {
    delta: OaiStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiStreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OaiStreamToolCall>>,
}

#[derive(Deserialize)]
struct OaiStreamToolCall {
    id: Option<String>,
    function: Option<OaiStreamFunction>,
}

#[derive(Deserialize)]
struct OaiStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

// --- Embeddings ---

#[derive(Deserialize)]
struct OaiEmbeddingResponse {
    data: Vec<OaiEmbeddingData>,
}

#[derive(Deserialize)]
struct OaiEmbeddingData {
    embedding: Vec<f32>,
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let provider = OpenAiProvider::builder()
            .api_key("test-key")
            .build();

        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model, "gpt-4o");
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
        assert_eq!(provider.max_retries, 2);
    }

    #[test]
    fn test_builder_custom() {
        let provider = OpenAiProvider::builder()
            .api_key("sk-test")
            .base_url("https://custom.endpoint.com/v1")
            .model("gpt-4o-mini")
            .organization("org-123")
            .max_retries(5)
            .timeout_secs(120)
            .build();

        assert_eq!(provider.default_model, "gpt-4o-mini");
        assert_eq!(provider.base_url, "https://custom.endpoint.com/v1");
        assert_eq!(provider.organization.as_deref(), Some("org-123"));
        assert_eq!(provider.max_retries, 5);
    }

    #[test]
    fn test_available_models() {
        let provider = OpenAiProvider::builder().api_key("test").build();
        let models = provider.available_models();
        assert!(models.len() >= 3);
        assert!(models.iter().any(|m| m.model_id == "gpt-4o"));
        assert!(models.iter().any(|m| m.model_id == "gpt-4o-mini"));
        assert!(models.iter().all(|m| m.supports_tools));
    }

    #[test]
    fn test_build_body_simple() {
        let provider = OpenAiProvider::builder().api_key("test").build();
        let request = CompletionRequest::simple("Hello");
        let body = provider.build_body(&request);

        assert_eq!(body.model, "gpt-4o");
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
        assert_eq!(body.messages[0].content.as_deref(), Some("Hello"));
        assert!(body.tools.is_none());
        assert!(body.stream.is_none());
    }

    #[test]
    fn test_build_body_with_options() {
        let provider = OpenAiProvider::builder()
            .api_key("test")
            .model("gpt-4o-mini")
            .build();

        let request = CompletionRequest::simple("Hi")
            .with_model("o1")
            .with_max_tokens(100)
            .with_temperature(0.7)
            .with_system("You are helpful.");

        let body = provider.build_body(&request);

        assert_eq!(body.model, "o1"); // request-level override
        assert_eq!(body.max_tokens, Some(100));
        assert_eq!(body.temperature, Some(0.7));
        assert_eq!(body.messages.len(), 2);
        assert_eq!(body.messages[0].role, "system");
    }

    #[test]
    fn test_build_body_with_tools() {
        let provider = OpenAiProvider::builder().api_key("test").build();
        let request = CompletionRequest::simple("Call a function").with_tools(vec![
            crate::FunctionDef {
                name: "get_weather".to_string(),
                description: "Get weather for a city".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    }
                }),
            },
        ]);

        let body = provider.build_body(&request);
        let tools = body.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "get_weather");
    }

    #[test]
    fn test_build_body_with_response_schema() {
        let provider = OpenAiProvider::builder().api_key("test").build();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string" }
            },
            "required": ["answer"]
        });
        let request = CompletionRequest::simple("Answer").with_response_schema(schema);

        let body = provider.build_body(&request);
        let fmt = body.response_format.unwrap();
        assert_eq!(fmt["type"], "json_schema");
        assert!(fmt["json_schema"]["strict"].as_bool().unwrap());
    }

    #[test]
    fn test_parse_error_response_json() {
        let body = r#"{"error": {"message": "Invalid API key", "type": "authentication_error"}}"#;
        let err = OpenAiProvider::parse_error_response(
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
        let body = r#"{"error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}}"#;
        let err = OpenAiProvider::parse_error_response(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            body,
        );
        assert!(matches!(err, LlmError::RateLimited { .. }));
    }

    #[test]
    fn test_parse_error_response_non_json() {
        let body = "Internal Server Error";
        let err = OpenAiProvider::parse_error_response(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            body,
        );
        match err {
            LlmError::ProviderError { message, .. } => {
                assert!(message.contains("500"));
            }
            other => panic!("Expected ProviderError, got: {other:?}"),
        }
    }

    #[test]
    fn test_headers_include_auth() {
        let provider = OpenAiProvider::builder()
            .api_key("sk-test123")
            .build();
        let headers = provider.build_headers();

        assert!(headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("sk-test123"));
    }

    #[test]
    fn test_headers_include_org() {
        let provider = OpenAiProvider::builder()
            .api_key("sk-test")
            .organization("org-abc")
            .build();
        let headers = provider.build_headers();

        assert_eq!(
            headers.get("OpenAI-Organization").unwrap().to_str().unwrap(),
            "org-abc"
        );
    }
}
