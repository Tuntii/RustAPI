//! Integration tests for rustapi-llm
//!
//! Tests MockProvider, LlmRouter (fallback, routing, streaming),
//! StructuredOutput, CompletionRequest builder, and error types.

use futures_util::StreamExt;
use rustapi_llm::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// CompletionRequest builder
// ===========================================================================

#[test]
fn test_completion_request_builder() {
    let req = CompletionRequest::simple("Hello, world!")
        .with_model("gpt-4o")
        .with_max_tokens(1000)
        .with_temperature(0.7)
        .with_system("You are a helpful assistant")
        .with_tools(vec![FunctionDef {
            name: "web_search".into(),
            description: "Search the web".into(),
            parameters: json!({"type": "object"}),
        }])
        .with_response_schema(json!({"type": "object", "properties": {"answer": {"type": "string"}}}));

    assert_eq!(req.messages.len(), 2); // system + user
    assert_eq!(req.model, Some("gpt-4o".to_string()));
    assert_eq!(req.max_tokens, Some(1000));
    assert_eq!(req.tools.len(), 1);
    assert!(req.response_schema.is_some());
}

#[test]
fn test_completion_request_simple() {
    let req = CompletionRequest::simple("What is Rust?");
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.messages[0].role, MessageRole::User);
    assert_eq!(req.messages[0].content, "What is Rust?");
}

// ===========================================================================
// Message constructors
// ===========================================================================

#[test]
fn test_message_constructors() {
    let sys = Message::system("You are helpful");
    assert_eq!(sys.role, MessageRole::System);

    let usr = Message::user("Hello");
    assert_eq!(usr.role, MessageRole::User);

    let ast = Message::assistant("Hi there");
    assert_eq!(ast.role, MessageRole::Assistant);

    let tool = Message::tool_result("42", "call-123");
    assert_eq!(tool.role, MessageRole::Tool);
    assert_eq!(tool.tool_call_id, Some("call-123".to_string()));
}

// ===========================================================================
// TokenUsage
// ===========================================================================

#[test]
fn test_token_usage_default() {
    let usage = TokenUsage::default();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}

// ===========================================================================
// ModelInfo cost estimation
// ===========================================================================

#[test]
fn test_model_info_estimate_cost() {
    let model = ModelInfo {
        model_id: "gpt-4o".into(),
        provider: "openai".into(),
        context_window: 128_000,
        max_output_tokens: 4096,
        cost_per_m_input: 5.0,   // $5 per million input tokens
        cost_per_m_output: 15.0, // $15 per million output tokens
        supports_tools: true,
        supports_structured_output: true,
        supports_streaming: true,
    };

    // 1000 input + 500 output
    let cost = model.estimate_cost_micros(1000, 500);
    // (1000 * 5.0 / 1_000_000 + 500 * 15.0 / 1_000_000) * 1_000_000 = 5 + 7.5 = 12.5 → 12 micros
    assert!(cost > 0);
}

// ===========================================================================
// MockProvider
// ===========================================================================

#[tokio::test]
async fn test_mock_provider_with_default_content() {
    let provider = MockProvider::new("test")
        .with_default_content("Custom default");

    let response = provider.complete(CompletionRequest::simple("Hi")).await.unwrap();
    assert_eq!(response.content, "Custom default");
    assert_eq!(response.provider, "test");
}

#[tokio::test]
async fn test_mock_provider_queue_and_fallback() {
    let provider = MockProvider::new("mock");

    provider.enqueue_response(CompletionResponse {
        model: "m".into(),
        content: "Queued".into(),
        tool_calls: Vec::new(),
        usage: TokenUsage::default(),
        finish_reason: FinishReason::Stop,
        provider: "mock".into(),
    });

    let r1 = provider.complete(CompletionRequest::simple("a")).await.unwrap();
    assert_eq!(r1.content, "Queued");

    // Fallback to default.
    let r2 = provider.complete(CompletionRequest::simple("b")).await.unwrap();
    assert_eq!(r2.content, "Mock response");
}

#[tokio::test]
async fn test_mock_provider_streaming_default() {
    let provider = MockProvider::new("stream-test")
        .with_default_content("streamed");

    // Default streaming impl should return single chunk.
    let stream = provider.complete_stream(CompletionRequest::simple("Hi")).await.unwrap();
    let chunks: Vec<_> = stream.collect::<Vec<_>>().await;

    assert_eq!(chunks.len(), 1);
    let chunk = chunks[0].as_ref().unwrap();
    assert!(chunk.done);
    assert_eq!(chunk.content_delta, "streamed");
}

#[tokio::test]
async fn test_mock_provider_health_check() {
    let provider = MockProvider::new("healthy");
    assert!(provider.health_check().await.is_ok());
}

#[tokio::test]
async fn test_mock_provider_embeddings_not_supported() {
    let provider = MockProvider::new("test");
    let result = provider.embeddings(vec!["hello".into()]).await;
    assert!(result.is_err());
}

#[test]
fn test_mock_provider_available_models() {
    let provider = MockProvider::new("test");
    let models = provider.available_models();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].model_id, "mock-model");
    assert_eq!(models[0].provider, "test");
}

// ===========================================================================
// LlmRouter: basic routing
// ===========================================================================

#[tokio::test]
async fn test_router_single_provider() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("primary").with_default_content("Hello from primary"))
        .build();

    let response = router.complete(CompletionRequest::simple("Hi")).await.unwrap();
    assert_eq!(response.content, "Hello from primary");
}

#[tokio::test]
async fn test_router_no_providers() {
    let router = LlmRouter::builder().build();
    let result = router.complete(CompletionRequest::simple("Hi")).await;
    assert!(result.is_err());
}

// ===========================================================================
// LlmRouter: fallback
// ===========================================================================

#[tokio::test]
async fn test_router_fallback_on_failure() {
    // Primary always fails, fallback succeeds.
    let failing_provider = FailingProvider;
    let fallback = MockProvider::new("fallback").with_default_content("Fallback response");

    let router = LlmRouter::builder()
        .provider_arc(Arc::new(failing_provider))
        .provider(fallback)
        .max_retries(5)
        .build();

    let response = router.complete(CompletionRequest::simple("Hi")).await.unwrap();
    assert_eq!(response.content, "Fallback response");
}

/// A provider that always fails.
struct FailingProvider;

#[async_trait::async_trait]
impl LlmProvider for FailingProvider {
    fn name(&self) -> &str { "failing" }
    fn available_models(&self) -> Vec<ModelInfo> { vec![] }
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::provider("failing", "intentional failure"))
    }
}

// ===========================================================================
// LlmRouter: available_models aggregation
// ===========================================================================

#[test]
fn test_router_available_models() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("p1"))
        .provider(MockProvider::new("p2"))
        .build();

    let models = router.available_models();
    assert_eq!(models.len(), 2);
}

// ===========================================================================
// LlmRouter: health check
// ===========================================================================

#[tokio::test]
async fn test_router_has_healthy_provider() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("healthy"))
        .build();

    assert!(router.has_healthy_provider().await);
}

#[tokio::test]
async fn test_router_no_healthy_providers() {
    let router = LlmRouter::builder().build();
    assert!(!router.has_healthy_provider().await);
}

// ===========================================================================
// LlmRouter: streaming
// ===========================================================================

#[tokio::test]
async fn test_router_streaming() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("stream").with_default_content("stream response"))
        .build();

    let stream = router.complete_stream(CompletionRequest::simple("Hi")).await.unwrap();
    let chunks: Vec<_> = stream.collect::<Vec<_>>().await;

    assert!(!chunks.is_empty());
    let last = chunks.last().unwrap().as_ref().unwrap();
    assert!(last.done);
}

// ===========================================================================
// LlmRouter: default model
// ===========================================================================

#[tokio::test]
async fn test_router_default_model() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("test"))
        .default_model("gpt-4o-mini")
        .build();

    // Request without specifying model should use default.
    let response = router.complete(CompletionRequest::simple("Hi")).await.unwrap();
    assert!(!response.content.is_empty());
}

// ===========================================================================
// StructuredOutput
// ===========================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct Answer {
    text: String,
    confidence: f32,
}

#[tokio::test]
async fn test_structured_output_success() {
    let mock = MockProvider::new("test");
    mock.enqueue_response(CompletionResponse {
        model: "m".into(),
        content: r#"{"text": "Rust is great", "confidence": 0.95}"#.into(),
        tool_calls: Vec::new(),
        usage: TokenUsage::default(),
        finish_reason: FinishReason::Stop,
        provider: "test".into(),
    });

    let router = LlmRouter::builder().provider(mock).build();
    let result: Answer = router
        .structured()
        .extract(CompletionRequest::simple("Review Rust"))
        .await
        .unwrap();

    assert_eq!(result.text, "Rust is great");
    assert!((result.confidence - 0.95).abs() < 0.01);
}

#[tokio::test]
async fn test_structured_output_strips_markdown_fences() {
    let mock = MockProvider::new("test");
    mock.enqueue_response(CompletionResponse {
        model: "m".into(),
        content: "```json\n{\"text\": \"fenced\", \"confidence\": 0.5}\n```".into(),
        tool_calls: Vec::new(),
        usage: TokenUsage::default(),
        finish_reason: FinishReason::Stop,
        provider: "test".into(),
    });

    let router = LlmRouter::builder().provider(mock).build();
    let result: Answer = router
        .structured()
        .extract(CompletionRequest::simple("test"))
        .await
        .unwrap();

    assert_eq!(result.text, "fenced");
}

#[tokio::test]
async fn test_structured_output_retry_on_invalid_json() {
    let mock = MockProvider::new("test");
    // First: invalid, second: valid.
    mock.enqueue_response(CompletionResponse {
        model: "m".into(),
        content: "not valid json at all".into(),
        tool_calls: Vec::new(),
        usage: TokenUsage::default(),
        finish_reason: FinishReason::Stop,
        provider: "test".into(),
    });
    mock.enqueue_response(CompletionResponse {
        model: "m".into(),
        content: r#"{"text": "retry worked", "confidence": 0.8}"#.into(),
        tool_calls: Vec::new(),
        usage: TokenUsage::default(),
        finish_reason: FinishReason::Stop,
        provider: "test".into(),
    });

    let router = LlmRouter::builder().provider(mock).build();
    let result: Answer = router
        .structured()
        .with_max_retries(3)
        .extract(CompletionRequest::simple("test"))
        .await
        .unwrap();

    assert_eq!(result.text, "retry worked");
}

#[tokio::test]
async fn test_structured_output_all_retries_exhausted() {
    let mock = MockProvider::new("test")
        .with_default_content("always invalid json");

    let router = LlmRouter::builder().provider(mock).build();
    let result = router
        .structured::<Answer>()
        .with_max_retries(2)
        .extract(CompletionRequest::simple("test"))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LlmError::StructuredOutputError { .. } => {} // expected
        other => panic!("Expected StructuredOutputError, got {other:?}"),
    }
}

// ===========================================================================
// FinishReason
// ===========================================================================

#[test]
fn test_finish_reason_serialization() {
    let reasons = vec![
        FinishReason::Stop,
        FinishReason::Length,
        FinishReason::ToolCalls,
        FinishReason::ContentFilter,
        FinishReason::Other("custom".into()),
    ];

    for reason in &reasons {
        let json_str = serde_json::to_string(reason).unwrap();
        let _: FinishReason = serde_json::from_str(&json_str).unwrap();
    }
}

// ===========================================================================
// LlmError factory methods
// ===========================================================================

#[test]
fn test_llm_error_factories() {
    let e1 = LlmError::provider("openai", "rate limited");
    assert!(format!("{e1}").contains("openai"));

    let e2 = LlmError::all_failed("no healthy providers");
    assert!(format!("{e2}").contains("no healthy"));

    let e3 = LlmError::structured_output("parse failed");
    assert!(format!("{e3}").contains("parse"));

    let e4 = LlmError::config("missing API key");
    assert!(format!("{e4}").contains("API key"));

    let e5 = LlmError::internal("unexpected");
    assert!(format!("{e5}").contains("unexpected"));
}

// ===========================================================================
// RoutingStrategy debug
// ===========================================================================

#[test]
fn test_routing_strategy_debug() {
    assert_eq!(format!("{:?}", RoutingStrategy::CostOptimized), "CostOptimized");
    assert_eq!(format!("{:?}", RoutingStrategy::LatencyOptimized), "LatencyOptimized");
    assert_eq!(format!("{:?}", RoutingStrategy::QualityFirst), "QualityFirst");
    assert_eq!(format!("{:?}", RoutingStrategy::RoundRobin), "RoundRobin");
}

#[test]
fn test_routing_strategy_default() {
    let strategy = RoutingStrategy::default();
    assert!(matches!(strategy, RoutingStrategy::CostOptimized));
}

// ===========================================================================
// ToolCall and ToolCallDelta serialization
// ===========================================================================

#[test]
fn test_tool_call_serialization() {
    let tc = ToolCall {
        id: "call-1".into(),
        name: "web_search".into(),
        arguments: json!({"query": "rust async"}),
    };

    let json_str = serde_json::to_string(&tc).unwrap();
    let deserialized: ToolCall = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.name, "web_search");
}

#[test]
fn test_stream_chunk_serialization() {
    let chunk = StreamChunk {
        content_delta: "Hello".into(),
        tool_call_deltas: vec![],
        done: false,
        usage: None,
    };

    let json_str = serde_json::to_string(&chunk).unwrap();
    let _: StreamChunk = serde_json::from_str(&json_str).unwrap();
}
