//! Integration tests for rustapi-agent
//!
//! Tests AgentEngine execution, ClosureStep, planning, branching,
//! yields, replay, error conversions, and cross-crate integration
//! with tools and memory via AgentContext.

use async_trait::async_trait;
use rustapi_agent::*;
use rustapi_context::{ContextError, RequestContextBuilder};
use rustapi_memory::backend::InMemoryStore;
use rustapi_memory::MemoryStore;
use rustapi_tools::{Tool, ToolError, ToolOutput, ToolRegistry};
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// Test helpers
// ===========================================================================

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes the input" }
    fn parameters_schema(&self) -> serde_json::Value { json!({}) }
    async fn execute(
        &self,
        _ctx: &rustapi_context::RequestContext,
        input: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::value(input))
    }
}

fn make_agent_context() -> AgentContext {
    let ctx = RequestContextBuilder::new()
        .method("POST")
        .path("/agent/test")
        .build();
    let tools = ToolRegistry::new().register(EchoTool);
    let memory: Arc<dyn MemoryStore> = Arc::new(InMemoryStore::new());
    AgentContext::new(ctx, tools, memory)
}

// ===========================================================================
// ClosureStep tests
// ===========================================================================

#[tokio::test]
async fn test_closure_step_basic() {
    let step = ClosureStep::new("greet", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"greeting": "hello"})))
        })
    });

    assert_eq!(step.name(), "greet");

    let mut ctx = make_agent_context();
    let result = step.execute(&mut ctx).await.unwrap();
    assert!(result.is_terminal());
}

#[tokio::test]
async fn test_closure_step_continue() {
    let step = ClosureStep::new("step1", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::cont(json!({"progress": 50})))
        })
    });

    let mut ctx = make_agent_context();
    let result = step.execute(&mut ctx).await.unwrap();
    assert!(!result.is_terminal());
}

// ===========================================================================
// AgentEngine: basic plan execution
// ===========================================================================

#[tokio::test]
async fn test_engine_simple_two_step_plan() {
    let config = EngineConfig {
        max_steps: 10,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    let step1: Arc<dyn Step> = Arc::new(ClosureStep::new("analyze", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::cont(json!({"analysis": "done"})))
        })
    }));

    let step2: Arc<dyn Step> = Arc::new(ClosureStep::new("respond", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"response": "final answer"})))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("analyze", "Analyze the question"),
        PlannedStep::new("respond", "Generate response"),
    ]).with_summary("Simple two-step plan");

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[step1, step2], &mut ctx).await.unwrap();

    assert_eq!(result.steps_executed, 2);
    assert_eq!(result.value, json!({"response": "final answer"}));
    assert!(result.duration_ms < 1000); // Should be fast
}

// ===========================================================================
// AgentEngine: yields / streaming
// ===========================================================================

#[tokio::test]
async fn test_engine_yield_collects_partial_results() {
    let config = EngineConfig {
        max_steps: 10,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    let yielder: Arc<dyn Step> = Arc::new(ClosureStep::new("stream", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::yield_partial(json!({"chunk": "partial data"})))
        })
    }));

    let finisher: Arc<dyn Step> = Arc::new(ClosureStep::new("finish", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"done": true})))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("stream", "Stream some data"),
        PlannedStep::new("finish", "Finish up"),
    ]);

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[yielder, finisher], &mut ctx).await.unwrap();

    assert_eq!(result.steps_executed, 2);
    assert!(!result.yields.is_empty());
}

// ===========================================================================
// AgentEngine: max steps exceeded
// ===========================================================================

#[tokio::test]
async fn test_engine_max_steps_exceeded() {
    let config = EngineConfig {
        max_steps: 3,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    // A step that always continues (infinite loop prevention).
    let looper: Arc<dyn Step> = Arc::new(ClosureStep::new("loop", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::cont(json!({"iteration": true})))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("loop", "Step 1"),
        PlannedStep::new("loop", "Step 2"),
        PlannedStep::new("loop", "Step 3"),
        PlannedStep::new("loop", "Step 4"), // Should not reach here
    ]);

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[looper], &mut ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::MaxStepsExceeded { max_steps } => assert_eq!(max_steps, 3),
        other => panic!("Expected MaxStepsExceeded, got {other:?}"),
    }
}

// ===========================================================================
// AgentEngine: branching
// ===========================================================================

#[tokio::test]
async fn test_engine_branching() {
    let config = EngineConfig {
        max_steps: 10,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    let decide: Arc<dyn Step> = Arc::new(ClosureStep::new("decide", |_ctx| {
        Box::pin(async move {
            // Branch to "fast_path".
            Ok(StepResult::branch("fast_path", json!({"decision": "fast"})))
        })
    }));

    let slow: Arc<dyn Step> = Arc::new(ClosureStep::new("slow_step", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"path": "slow"})))
        })
    }));

    let fast: Arc<dyn Step> = Arc::new(ClosureStep::new("fast_step", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"path": "fast"})))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("decide", "Decide path"),  // index 0
        PlannedStep::new("slow_step", "Slow path"),  // index 1
        PlannedStep::new("fast_step", "Fast finish"), // index 2
    ])
    .with_branch("fast_path", 2); // Jump to fast_step

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[decide, slow, fast], &mut ctx).await.unwrap();

    assert_eq!(result.value, json!({"path": "fast"}));
    assert_eq!(result.steps_executed, 2); // decide + fast_step
}

// ===========================================================================
// AgentEngine: branch not found
// ===========================================================================

#[tokio::test]
async fn test_engine_branch_not_found() {
    let config = EngineConfig {
        max_steps: 10,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    let brancher: Arc<dyn Step> = Arc::new(ClosureStep::new("brancher", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::branch("nonexistent", json!(null)))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("brancher", "Try to branch"),
    ]);
    // No branches defined.

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[brancher], &mut ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::BranchNotFound { branch_name } => assert_eq!(branch_name, "nonexistent"),
        other => panic!("Expected BranchNotFound, got {other:?}"),
    }
}

// ===========================================================================
// AgentEngine: step not found in implementations
// ===========================================================================

#[tokio::test]
async fn test_engine_missing_step_impl() {
    let config = EngineConfig {
        max_steps: 10,
        emit_events: false,
        trace_step_io: false,
    };
    let engine = AgentEngine::new(config);

    // Plan references "unknown_step" but no impl provided.
    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("unknown_step", "Does not exist"),
    ]);

    let mut ctx = make_agent_context();
    let result = engine.run(&plan, &[], &mut ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::StepFailed { step, .. } => assert_eq!(step, "unknown_step"),
        other => panic!("Expected StepFailed, got {other:?}"),
    }
}

// ===========================================================================
// AgentContext: tools integration
// ===========================================================================

#[tokio::test]
async fn test_agent_context_call_tool() {
    let ctx = make_agent_context();

    let output = ctx.call_tool("echo", json!({"data": "hello"})).await.unwrap();
    assert_eq!(output.value, json!({"data": "hello"}));
}

#[tokio::test]
async fn test_agent_context_call_tool_not_found() {
    let ctx = make_agent_context();
    let result = ctx.call_tool("nonexistent", json!({})).await;
    assert!(result.is_err());
}

// ===========================================================================
// AgentContext: memory integration
// ===========================================================================

#[tokio::test]
async fn test_agent_context_remember_recall() {
    let ctx = make_agent_context();

    ctx.remember("user_preference", json!({"theme": "dark"})).await.unwrap();

    let recalled = ctx.recall("user_preference").await.unwrap();
    assert_eq!(recalled, Some(json!({"theme": "dark"})));
}

#[tokio::test]
async fn test_agent_context_recall_missing() {
    let ctx = make_agent_context();
    let recalled = ctx.recall("nonexistent").await.unwrap();
    assert!(recalled.is_none());
}

// ===========================================================================
// AgentContext: state management
// ===========================================================================

#[test]
fn test_agent_context_state() {
    let mut ctx = make_agent_context();

    assert!(ctx.get_state("key").is_none());

    ctx.set_state("key", json!("value"));
    assert_eq!(ctx.get_state("key"), Some(&json!("value")));

    let removed = ctx.remove_state("key");
    assert_eq!(removed, Some(json!("value")));
    assert!(ctx.get_state("key").is_none());
}

#[test]
fn test_agent_context_step_tracking() {
    let ctx = make_agent_context();
    assert_eq!(ctx.step_index(), 0);
    // advance_step is pub(crate), tested via engine execution
}

// ===========================================================================
// ExecutionPlan builder
// ===========================================================================

#[test]
fn test_execution_plan_builder() {
    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("init", "Initialize").with_tool("setup_tool").with_input(json!({})),
        PlannedStep::new("process", "Process data").with_branch_label("error_handler"),
        PlannedStep::new("error_handler", "Handle errors"),
    ])
    .with_branch("error_handler", 2)
    .with_summary("Test plan");

    assert_eq!(plan.len(), 3);
    assert!(!plan.is_empty());
    assert_eq!(plan.summary, Some("Test plan".to_string()));
    assert_eq!(plan.branches.get("error_handler"), Some(&2));
    assert_eq!(plan.steps[0].tool_name, Some("setup_tool".to_string()));
    assert_eq!(plan.steps[1].branch_label, Some("error_handler".to_string()));
}

#[test]
fn test_execution_plan_empty() {
    let plan = ExecutionPlan::new(vec![]);
    assert!(plan.is_empty());
    assert_eq!(plan.len(), 0);
}

// ===========================================================================
// StepResult variants
// ===========================================================================

#[test]
fn test_step_result_is_terminal() {
    assert!(!StepResult::cont(json!(null)).is_terminal());
    assert!(!StepResult::branch("x", json!(null)).is_terminal());
    assert!(!StepResult::yield_partial(json!(null)).is_terminal());
    assert!(StepResult::complete(json!(null)).is_terminal());
    assert!(StepResult::error("oops").is_terminal());
}

#[test]
fn test_step_result_serialization() {
    let results = vec![
        StepResult::cont(json!({"data": 1})),
        StepResult::branch("alt", json!(2)),
        StepResult::yield_partial(json!("chunk")),
        StepResult::complete(json!({"final": true})),
        StepResult::error("something failed"),
    ];

    for result in &results {
        let json_str = serde_json::to_string(result).unwrap();
        let _: StepResult = serde_json::from_str(&json_str).unwrap();
    }
}

// ===========================================================================
// Error conversions
// ===========================================================================

#[test]
fn test_agent_error_from_context_error() {
    let ctx_err = ContextError::BudgetExceeded {
        message: "Token limit exceeded".into(),
    };
    let agent_err: AgentError = ctx_err.into();
    match agent_err {
        AgentError::BudgetExceeded { message } => {
            assert!(message.contains("Token limit"));
        }
        other => panic!("Expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn test_agent_error_from_tool_error() {
    let tool_err = rustapi_tools::ToolError::not_found("missing_tool");
    let agent_err: AgentError = tool_err.into();
    match agent_err {
        AgentError::ToolError { message } => {
            assert!(message.contains("missing_tool"));
        }
        other => panic!("Expected ToolError, got {other:?}"),
    }
}

#[test]
fn test_agent_error_from_memory_error() {
    let mem_err = rustapi_memory::MemoryError::CapacityExceeded;
    let agent_err: AgentError = mem_err.into();
    match agent_err {
        AgentError::MemoryError { message } => {
            assert!(!message.is_empty());
        }
        other => panic!("Expected MemoryError, got {other:?}"),
    }
}

// ===========================================================================
// ReplayEngine
// ===========================================================================

#[test]
fn test_replay_record_creates_session() {
    use rustapi_context::TraceNode;
    use rustapi_context::TraceNodeKind;
    use std::collections::HashMap;

    let trace = TraceNode::new(TraceNodeKind::HttpReceived, "POST /agent");
    let session = ReplayEngine::record(
        trace,
        HashMap::new(),
        HashMap::new(),
        json!({"answer": 42}),
    );

    assert!(!session.session_id.is_empty());
    assert_eq!(session.final_output, json!({"answer": 42}));
    assert_eq!(session.trace.label, "POST /agent");
}

#[test]
fn test_replay_compare_identical() {
    use rustapi_context::TraceNode;
    use rustapi_context::TraceNodeKind;
    use std::collections::HashMap;

    let trace = TraceNode::new(TraceNodeKind::HttpReceived, "POST /test");
    let output = json!({"status": "ok"});

    let session = ReplayEngine::record(
        trace.clone(),
        HashMap::new(),
        HashMap::new(),
        output.clone(),
    );

    let result = ReplayEngine::compare(&session, &trace, &output);
    assert!(result.matched);
    assert!(result.divergences.is_empty());
}

#[test]
fn test_replay_detect_output_divergence() {
    use rustapi_context::TraceNode;
    use rustapi_context::TraceNodeKind;
    use std::collections::HashMap;

    let trace = TraceNode::new(TraceNodeKind::HttpReceived, "POST /test");

    let session = ReplayEngine::record(
        trace.clone(),
        HashMap::new(),
        HashMap::new(),
        json!({"original": true}),
    );

    let result = ReplayEngine::compare(&session, &trace, &json!({"different": true}));
    assert!(!result.matched);
    assert!(!result.divergences.is_empty());
}

// ===========================================================================
// AgentError factory methods
// ===========================================================================

#[test]
fn test_agent_error_factories() {
    let e1 = AgentError::planning_failed("LLM returned invalid JSON");
    assert!(format!("{e1}").contains("invalid JSON"));

    let e2 = AgentError::step_failed("process_data", "timeout");
    assert!(format!("{e2}").contains("process_data"));

    let e3 = AgentError::branch_not_found("missing_branch");
    assert!(format!("{e3}").contains("missing_branch"));

    let e4 = AgentError::internal("unexpected state");
    assert!(format!("{e4}").contains("unexpected state"));
}

// ===========================================================================
// Engine with events and tracing
// ===========================================================================

#[tokio::test]
async fn test_engine_with_events_and_tracing() {
    let bus = rustapi_context::EventBus::new(64);
    let mut sub = bus.subscribe();

    let config = EngineConfig {
        max_steps: 10,
        emit_events: true,
        trace_step_io: true,
    };
    let engine = AgentEngine::new(config);

    let step: Arc<dyn Step> = Arc::new(ClosureStep::new("observe", |_ctx| {
        Box::pin(async move {
            Ok(StepResult::complete(json!({"observed": true})))
        })
    }));

    let plan = ExecutionPlan::new(vec![
        PlannedStep::new("observe", "Observable step"),
    ]).with_summary("Event test plan");

    let req_ctx = RequestContextBuilder::new()
        .method("POST")
        .path("/agent/test")
        .event_bus(bus.clone())
        .build();
    let tools = ToolRegistry::new();
    let memory: Arc<dyn MemoryStore> = Arc::new(InMemoryStore::new());
    let mut agent_ctx = AgentContext::new(req_ctx, tools, memory);

    let result = engine.run(&plan, &[step], &mut agent_ctx).await.unwrap();
    assert_eq!(result.steps_executed, 1);

    // Should have received events: PlanGenerated, AgentStepStarted, AgentStepCompleted.
    let e1 = sub.try_recv();
    assert!(e1.is_some(), "Expected PlanGenerated event");

    let e2 = sub.try_recv();
    assert!(e2.is_some(), "Expected AgentStepStarted event");

    let e3 = sub.try_recv();
    assert!(e3.is_some(), "Expected AgentStepCompleted event");
}
