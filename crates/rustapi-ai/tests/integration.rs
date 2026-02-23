//! Integration tests for rustapi-ai
//!
//! End-to-end tests for the AiRuntime facade: builder, tools,
//! memory, LLM router, and full agent execution pipeline.

use async_trait::async_trait;
use rustapi_ai::prelude::*;
use rustapi_agent::AgentError;
use rustapi_context::RequestContextBuilder;
use rustapi_tools::ToolError;
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// Test helpers
// ===========================================================================

struct UppercaseTool;

#[async_trait]
impl Tool for UppercaseTool {
    fn name(&self) -> &str { "uppercase" }
    fn description(&self) -> &str { "Converts to uppercase" }
    fn parameters_schema(&self) -> serde_json::Value { json!({"type": "object"}) }
    async fn execute(
        &self,
        _ctx: &rustapi_context::RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let text = input["text"].as_str().unwrap_or("");
        Ok(ToolOutput::value(json!({"result": text.to_uppercase()})))
    }
}

struct ReverseTool;

#[async_trait]
impl Tool for ReverseTool {
    fn name(&self) -> &str { "reverse" }
    fn description(&self) -> &str { "Reverses a string" }
    fn parameters_schema(&self) -> serde_json::Value { json!({"type": "object"}) }
    async fn execute(
        &self,
        _ctx: &rustapi_context::RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let text = input["text"].as_str().unwrap_or("");
        let reversed: String = text.chars().rev().collect();
        Ok(ToolOutput::value(json!({"result": reversed})))
    }
}

// ===========================================================================
// AiRuntime builder: defaults
// ===========================================================================

#[test]
fn test_runtime_builder_defaults() {
    let runtime = AiRuntime::builder().build();

    // Default LLM router should have mock models.
    assert!(!runtime.llm().available_models().is_empty());

    // Default tools registry should be empty.
    assert!(runtime.tools().is_empty());

    // Default engine config.
    assert_eq!(runtime.engine_config().max_steps, 50);
}

// ===========================================================================
// AiRuntime builder: custom config
// ===========================================================================

#[test]
fn test_runtime_builder_custom_engine_config() {
    let config = EngineConfig {
        max_steps: 5,
        emit_events: false,
        trace_step_io: false,
    };

    let runtime = AiRuntime::builder()
        .engine_config(config)
        .build();

    assert_eq!(runtime.engine_config().max_steps, 5);
    assert!(!runtime.engine_config().emit_events);
}

// ===========================================================================
// AiRuntime builder: tools
// ===========================================================================

#[test]
fn test_runtime_builder_with_tools() {
    let runtime = AiRuntime::builder()
        .tool(UppercaseTool)
        .tool(ReverseTool)
        .build();

    assert_eq!(runtime.tools().len(), 2);
    assert!(runtime.tools().contains("uppercase"));
    assert!(runtime.tools().contains("reverse"));
}

#[test]
fn test_runtime_builder_tool_arc() {
    let tool: Arc<dyn Tool> = Arc::new(UppercaseTool);
    let runtime = AiRuntime::builder().tool_arc(tool).build();
    assert!(runtime.tools().contains("uppercase"));
}

// ===========================================================================
// AiRuntime builder: custom memory
// ===========================================================================

#[tokio::test]
async fn test_runtime_builder_custom_memory() {
    let store = InMemoryStore::new();

    // Pre-populate memory.
    use rustapi_memory::MemoryStore;
    store.store(rustapi_memory::MemoryEntry::new("key", json!("value")))
        .await
        .unwrap();

    let runtime = AiRuntime::builder()
        .memory(store)
        .build();

    // Verify memory persists through the runtime.
    let entry = runtime.memory().get("key").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().value, json!("value"));
}

// ===========================================================================
// AiRuntime builder: custom LLM router
// ===========================================================================

#[tokio::test]
async fn test_runtime_builder_custom_llm() {
    let router = LlmRouter::builder()
        .provider(MockProvider::new("custom").with_default_content("Custom LLM"))
        .build();

    let runtime = AiRuntime::builder()
        .llm(router)
        .build();

    let response = runtime.llm()
        .complete(CompletionRequest::simple("Hi"))
        .await
        .unwrap();

    assert_eq!(response.content, "Custom LLM");
}

// ===========================================================================
// AiRuntime: Clone + Send + Sync
// ===========================================================================

#[test]
fn test_runtime_is_clone_send_sync() {
    fn assert_clone_send_sync<T: Clone + Send + Sync>() {}
    assert_clone_send_sync::<AiRuntime>();
}

#[test]
fn test_runtime_clone_shares_state() {
    let runtime = AiRuntime::builder()
        .tool(UppercaseTool)
        .build();

    let cloned = runtime.clone();
    assert_eq!(cloned.tools().len(), 1);
    assert!(cloned.tools().contains("uppercase"));
}

// ===========================================================================
// AiRuntime: Arc accessors
// ===========================================================================

#[test]
fn test_runtime_arc_accessors() {
    let runtime = AiRuntime::builder().build();

    let _memory_arc = runtime.memory_arc();
    let _tools_arc = runtime.tools_arc();
    let _llm_arc = runtime.llm_arc();
}

// ===========================================================================
// AiRuntime: create_engine
// ===========================================================================

#[test]
fn test_runtime_create_engine() {
    let runtime = AiRuntime::builder()
        .engine_config(EngineConfig {
            max_steps: 25,
            emit_events: true,
            trace_step_io: true,
        })
        .build();

    let _engine = runtime.create_engine();
}

// ===========================================================================
// End-to-end: AiRuntime → AgentEngine → Tools + Memory + LLM
// ===========================================================================

#[tokio::test]
async fn test_end_to_end_agent_execution() {
    // Build runtime with tools + memory + LLM.
    let runtime = AiRuntime::builder()
        .tool(UppercaseTool)
        .tool(ReverseTool)
        .engine_config(EngineConfig {
            max_steps: 10,
            emit_events: false,
            trace_step_io: false,
        })
        .build();

    // Create agent engine from runtime.
    let engine = runtime.create_engine();

    // Create agent context.
    let req_ctx = RequestContextBuilder::new()
        .method("POST")
        .path("/agent/run")
        .build();
    let tools = runtime.tools().clone();
    let memory = runtime.memory_arc();
    let mut agent_ctx = AgentContext::new(req_ctx, tools, memory);

    // Define steps.
    let tool_step: Arc<dyn Step> = Arc::new(ClosureStep::new("use_tool", |ctx| {
        Box::pin(async move {
            let output = ctx.call_tool("uppercase", json!({"text": "hello"})).await
                .map_err(|e| AgentError::internal(e.to_string()))?;
            Ok(StepResult::cont(output.value))
        })
    }));

    let memory_step: Arc<dyn Step> = Arc::new(ClosureStep::new("save_result", |ctx| {
        Box::pin(async move {
            // Get previous result from state.
            let prev = ctx.get_state("previous_result")
                .cloned()
                .unwrap_or(json!(null));

            // Save to memory.
            ctx.remember("agent_result", prev.clone()).await
                .map_err(|e| AgentError::internal(e.to_string()))?;

            Ok(StepResult::complete(prev))
        })
    }));

    // Create execution plan.
    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("use_tool", "Call uppercase tool"),
        PlannedStep::new("save_result", "Save result to memory"),
    ]);

    // Execute.
    let result = engine.run(&plan, &[tool_step, memory_step], &mut agent_ctx).await.unwrap();

    assert_eq!(result.steps_executed, 2);
    assert!(result.duration_ms < 5000);

    // Verify memory was written.
    let recalled = agent_ctx.recall("agent_result").await.unwrap();
    assert!(recalled.is_some());
}

#[tokio::test]
async fn test_end_to_end_with_llm() {
    // Mock LLM that returns a specific response.
    let mock = MockProvider::new("test-llm");
    mock.enqueue_response(CompletionResponse {
        model: "mock".into(),
        content: "The answer is 42".into(),
        tool_calls: Vec::new(),
        usage: TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
        },
        finish_reason: FinishReason::Stop,
        provider: "test-llm".into(),
    });

    let router = LlmRouter::builder().provider(mock).build();

    let runtime = AiRuntime::builder()
        .llm(router)
        .engine_config(EngineConfig {
            max_steps: 5,
            emit_events: false,
            trace_step_io: false,
        })
        .build();

    // Use LLM from within a step.
    let llm_arc = runtime.llm_arc();
    let llm_step: Arc<dyn Step> = Arc::new(ClosureStep::new("ask_llm", move |_ctx| {
        let llm = llm_arc.clone();
        Box::pin(async move {
            let response = llm.complete(CompletionRequest::simple("What is the meaning of life?"))
                .await
                .map_err(|e| AgentError::internal(e.to_string()))?;
            Ok(StepResult::complete(json!({"answer": response.content})))
        })
    }));

    let engine = runtime.create_engine();
    let req_ctx = RequestContextBuilder::new().method("POST").path("/ask").build();
    let tools = ToolRegistry::new();
    let memory = runtime.memory_arc();
    let mut agent_ctx = AgentContext::new(req_ctx, tools, memory);

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("ask_llm", "Ask the LLM a question"),
    ]);

    let result = engine.run(&plan, &[llm_step], &mut agent_ctx).await.unwrap();
    assert_eq!(result.value, json!({"answer": "The answer is 42"}));
    assert_eq!(result.steps_executed, 1);
}

// ===========================================================================
// Prelude re-exports verification
// ===========================================================================

#[test]
fn test_prelude_types_accessible() {
    // Just verify these types are accessible from the prelude.
    let _: Option<AiRuntime> = None;
    let _: Option<InMemoryStore> = None;
    let _: Option<ToolRegistry> = None;
    let _: Option<AgentEngine> = None;
    let _: Option<EngineConfig> = None;
    let _: Option<ExecutionPlan> = None;
    let _: Option<PlannedStep> = None;
    let _: Option<LlmRouter> = None;
    let _: Option<MockProvider> = None;
    let _: Option<CompletionRequest> = None;
    let _: Option<CompletionResponse> = None;
    #[allow(clippy::type_complexity)]
    let _: Option<ClosureStep<fn(&mut AgentContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult, AgentError>> + Send + '_>>>> = None;
}
