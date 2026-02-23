use crate::{
    CompletionRequest, CompletionResponse, LlmError, ModelInfo, StreamChunk,
};
use async_trait::async_trait;
use std::pin::Pin;

/// Model-agnostic LLM provider abstraction.
///
/// Every LLM backend (OpenAI, Anthropic, local Ollama, mock) implements
/// this trait, enabling the [`LlmRouter`](crate::LlmRouter) to dispatch
/// requests transparently.
#[async_trait]
pub trait LlmProvider: Send + Sync + 'static {
    /// Provider name (e.g. "openai", "anthropic", "ollama").
    fn name(&self) -> &str;

    /// List of models available from this provider.
    fn available_models(&self) -> Vec<ModelInfo>;

    /// Send a completion request and wait for the full response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Send a completion request and receive a streaming response.
    ///
    /// The default implementation calls `complete()` and wraps the result
    /// in a single-chunk stream.
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError>
    {
        let response = self.complete(request).await?;
        let chunk = StreamChunk {
            content_delta: response.content.clone(),
            tool_call_deltas: Vec::new(),
            done: true,
            usage: Some(response.usage.clone()),
        };
        Ok(Box::pin(futures_util::stream::once(async move {
            Ok(chunk)
        })))
    }

    /// Generate embeddings for a list of texts.
    ///
    /// Not all providers support this; the default returns an error.
    async fn embeddings(&self, _input: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        Err(LlmError::provider(self.name(), "Embeddings not supported"))
    }

    /// Health check — is this provider reachable?
    async fn health_check(&self) -> Result<(), LlmError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MockProvider — deterministic responses for testing
// ---------------------------------------------------------------------------

/// A mock LLM provider that returns predetermined responses.
///
/// Useful for testing, benchmarking, and deterministic replay.
#[derive(Debug, Clone)]
pub struct MockProvider {
    /// Name of this mock provider instance.
    name: String,
    /// Queue of responses to return (FIFO).
    responses: std::sync::Arc<std::sync::Mutex<Vec<CompletionResponse>>>,
    /// Default response when the queue is empty.
    default_response: CompletionResponse,
    /// Model info to report.
    model_info: ModelInfo,
}

impl MockProvider {
    /// Create a mock provider with a default response.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            responses: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            default_response: CompletionResponse {
                model: "mock-model".to_string(),
                content: "Mock response".to_string(),
                tool_calls: Vec::new(),
                usage: crate::TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                },
                finish_reason: crate::FinishReason::Stop,
                provider: name.clone(),
            },
            model_info: ModelInfo {
                model_id: "mock-model".to_string(),
                provider: name,
                context_window: 128_000,
                max_output_tokens: 4096,
                cost_per_m_input: 0.0,
                cost_per_m_output: 0.0,
                supports_tools: true,
                supports_structured_output: true,
                supports_streaming: true,
            },
        }
    }

    /// Enqueue a response to be returned by the next `complete()` call.
    pub fn enqueue_response(&self, response: CompletionResponse) {
        self.responses.lock().unwrap().push(response);
    }

    /// Set the default response for when the queue is empty.
    pub fn with_default_content(mut self, content: impl Into<String>) -> Self {
        self.default_response.content = content.into();
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        vec![self.model_info.clone()]
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut queue = self.responses.lock().unwrap();
        if queue.is_empty() {
            Ok(self.default_response.clone())
        } else {
            Ok(queue.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_default_response() {
        let provider = MockProvider::new("test-mock")
            .with_default_content("Hello from mock!");

        let response = provider
            .complete(CompletionRequest::simple("Hi"))
            .await
            .unwrap();

        assert_eq!(response.content, "Hello from mock!");
        assert_eq!(response.provider, "test-mock");
    }

    #[tokio::test]
    async fn test_mock_provider_queued_responses() {
        let provider = MockProvider::new("test-mock");

        provider.enqueue_response(CompletionResponse {
            model: "mock".to_string(),
            content: "First".to_string(),
            tool_calls: Vec::new(),
            usage: crate::TokenUsage::default(),
            finish_reason: crate::FinishReason::Stop,
            provider: "test-mock".to_string(),
        });

        provider.enqueue_response(CompletionResponse {
            model: "mock".to_string(),
            content: "Second".to_string(),
            tool_calls: Vec::new(),
            usage: crate::TokenUsage::default(),
            finish_reason: crate::FinishReason::Stop,
            provider: "test-mock".to_string(),
        });

        let r1 = provider.complete(CompletionRequest::simple("a")).await.unwrap();
        let r2 = provider.complete(CompletionRequest::simple("b")).await.unwrap();
        let r3 = provider.complete(CompletionRequest::simple("c")).await.unwrap();

        assert_eq!(r1.content, "First");
        assert_eq!(r2.content, "Second");
        assert_eq!(r3.content, "Mock response"); // falls back to default
    }
}
