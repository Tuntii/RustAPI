//! Integration tests for rustapi-tools
//!
//! Tests ClosureTool, ToolRegistry, ToolGraph (parallel, sequence,
//! conditional, timeout), and combined registry+graph execution.

use async_trait::async_trait;
use rustapi_context::{RequestContext, RequestContextBuilder};
use rustapi_tools::*;
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// Test helpers
// ===========================================================================

struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str { "add" }
    fn description(&self) -> &str { "Adds two numbers" }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    }
    async fn execute(&self, _ctx: &RequestContext, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let a = input["a"].as_f64().unwrap_or(0.0);
        let b = input["b"].as_f64().unwrap_or(0.0);
        Ok(ToolOutput::value(json!({"result": a + b})))
    }
}

struct FailTool;

#[async_trait]
impl Tool for FailTool {
    fn name(&self) -> &str { "fail" }
    fn description(&self) -> &str { "Always fails" }
    fn parameters_schema(&self) -> serde_json::Value { json!({}) }
    async fn execute(&self, _ctx: &RequestContext, _input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        Err(ToolError::execution_failed("fail", "intentional failure"))
    }
}

struct SlowTool;

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str { "slow" }
    fn description(&self) -> &str { "Takes a while" }
    fn parameters_schema(&self) -> serde_json::Value { json!({}) }
    async fn execute(&self, _ctx: &RequestContext, _input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        Ok(ToolOutput::value(json!("slow done")))
    }
}

fn make_ctx() -> RequestContext {
    RequestContextBuilder::new().method("POST").path("/test").build()
}

// ===========================================================================
// ClosureTool tests
// ===========================================================================

#[tokio::test]
async fn test_closure_tool_basic() {
    let tool = ClosureTool::new(
        "greet",
        "Greets someone",
        json!({"type": "object", "properties": {"name": {"type": "string"}}}),
        |_ctx, input| {
            Box::pin(async move {
                let name = input["name"].as_str().unwrap_or("World");
                Ok(ToolOutput::value(json!({"greeting": format!("Hello, {name}!")})))
            })
        },
    );

    assert_eq!(tool.name(), "greet");
    assert_eq!(tool.description(), "Greets someone");

    let ctx = make_ctx();
    let output = tool.execute(&ctx, json!({"name": "Alice"})).await.unwrap();
    assert_eq!(output.value, json!({"greeting": "Hello, Alice!"}));
}

#[tokio::test]
async fn test_closure_tool_in_registry() {
    let tool = ClosureTool::new(
        "multiply",
        "Multiplies two numbers",
        json!({}),
        |_ctx, input| {
            Box::pin(async move {
                let a = input["a"].as_f64().unwrap_or(0.0);
                let b = input["b"].as_f64().unwrap_or(0.0);
                Ok(ToolOutput::value(json!(a * b)))
            })
        },
    );

    let registry = ToolRegistry::new().register(tool);
    let ctx = make_ctx();

    let output = registry.execute("multiply", &ctx, json!({"a": 3, "b": 7})).await.unwrap();
    assert_eq!(output.value, json!(21.0));
}

// ===========================================================================
// ToolRegistry: describe_all, function_definitions
// ===========================================================================

#[test]
fn test_registry_describe_all() {
    let registry = ToolRegistry::new()
        .register(AddTool)
        .register(FailTool);

    let descriptions = registry.describe_all();
    assert_eq!(descriptions.len(), 2);

    let names: Vec<_> = descriptions.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"fail"));
}

#[test]
fn test_registry_register_arc() {
    let tool: Arc<dyn Tool> = Arc::new(AddTool);
    let registry = ToolRegistry::new().register_arc(tool);
    assert!(registry.contains("add"));
    assert_eq!(registry.len(), 1);
}

#[test]
fn test_registry_overwrite() {
    // Registering a tool with the same name should overwrite.
    let registry = ToolRegistry::new()
        .register(AddTool)
        .register(FailTool);

    // AddTool and FailTool have different names, both should exist.
    assert_eq!(registry.len(), 2);

    // Register another AddTool — should overwrite.
    let registry2 = registry.register(AddTool);
    assert_eq!(registry2.len(), 2);
}

// ===========================================================================
// ToolOutput builder
// ===========================================================================

#[test]
fn test_tool_output_with_cost() {
    let output = ToolOutput::value(json!("result"))
        .with_cost(rustapi_context::CostDelta {
            input_tokens: 100,
            output_tokens: 50,
            cost_micros: 300,
            model: Some("gpt-4o".into()),
        })
        .with_side_effect(SideEffect::HttpRequest {
            url: "https://api.example.com".into(),
            method: "GET".into(),
        });

    assert!(output.cost.is_some());
    assert_eq!(output.side_effects.len(), 1);
}

// ===========================================================================
// ToolGraph: sequence execution
// ===========================================================================

#[tokio::test]
async fn test_graph_sequence_execution() {
    let registry = ToolRegistry::new().register(AddTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "add_sequence",
        ToolNode::Sequence {
            id: "seq".into(),
            nodes: vec![
                ToolNode::call("step1", "add", json!({"a": 1, "b": 2})),
                ToolNode::call("step2", "add", json!({"a": 10, "b": 20})),
            ],
        },
    );

    let output = graph.execute(&registry, &ctx).await.unwrap();
    assert_eq!(output.tool_calls, 2);
    assert_eq!(output.node_outputs["step1"], json!({"result": 3.0}));
    assert_eq!(output.node_outputs["step2"], json!({"result": 30.0}));
}

// ===========================================================================
// ToolGraph: parallel execution
// ===========================================================================

#[tokio::test]
async fn test_graph_parallel_execution() {
    let registry = ToolRegistry::new().register(AddTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "add_parallel",
        ToolNode::Parallel {
            id: "par".into(),
            nodes: vec![
                ToolNode::call("p1", "add", json!({"a": 1, "b": 1})),
                ToolNode::call("p2", "add", json!({"a": 2, "b": 2})),
                ToolNode::call("p3", "add", json!({"a": 3, "b": 3})),
            ],
        },
    );

    let output = graph.execute(&registry, &ctx).await.unwrap();
    assert_eq!(output.tool_calls, 3);
    assert_eq!(output.node_outputs["p1"], json!({"result": 2.0}));
    assert_eq!(output.node_outputs["p2"], json!({"result": 4.0}));
    assert_eq!(output.node_outputs["p3"], json!({"result": 6.0}));
}

// ===========================================================================
// ToolGraph: conditional execution
// ===========================================================================

#[tokio::test]
async fn test_graph_conditional_true_branch() {
    let registry = ToolRegistry::new().register(AddTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "cond_true",
        ToolNode::Conditional {
            id: "cond".into(),
            condition: "true".into(), // Always true.
            if_true: Box::new(ToolNode::call("yes", "add", json!({"a": 5, "b": 5}))),
            if_false: Some(Box::new(ToolNode::call("no", "add", json!({"a": 0, "b": 0})))),
        },
    );

    let output = graph.execute(&registry, &ctx).await.unwrap();
    assert!(output.node_outputs.contains_key("yes"));
    assert!(!output.node_outputs.contains_key("no"));
}

#[tokio::test]
async fn test_graph_conditional_false_branch() {
    let registry = ToolRegistry::new().register(AddTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "cond_false",
        ToolNode::Conditional {
            id: "cond".into(),
            condition: "false".into(),
            if_true: Box::new(ToolNode::call("yes", "add", json!({"a": 5, "b": 5}))),
            if_false: Some(Box::new(ToolNode::call("no", "add", json!({"a": 0, "b": 0})))),
        },
    );

    let output = graph.execute(&registry, &ctx).await.unwrap();
    assert!(!output.node_outputs.contains_key("yes"));
    assert!(output.node_outputs.contains_key("no"));
}

// ===========================================================================
// ToolGraph: timeout
// ===========================================================================

#[tokio::test]
async fn test_graph_timeout() {
    let registry = ToolRegistry::new().register(SlowTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "timeout_test",
        ToolNode::call("s", "slow", json!({})),
    )
    .with_timeout_ms(100); // 100ms timeout, tool takes 5s.

    let result = graph.execute(&registry, &ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::Timeout { timeout_ms } => assert_eq!(timeout_ms, 100),
        other => panic!("Expected Timeout, got {other:?}"),
    }
}

// ===========================================================================
// ToolGraph: error propagation from tool
// ===========================================================================

#[tokio::test]
async fn test_graph_tool_failure_propagation() {
    let registry = ToolRegistry::new().register(FailTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "fail_graph",
        ToolNode::call("f", "fail", json!({})),
    );

    let result = graph.execute(&registry, &ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ExecutionFailed { tool, .. } => assert_eq!(tool, "fail"),
        other => panic!("Expected ExecutionFailed, got {other:?}"),
    }
}

// ===========================================================================
// ToolGraph: tool not found
// ===========================================================================

#[tokio::test]
async fn test_graph_tool_not_found() {
    let registry = ToolRegistry::new(); // Empty registry.
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "missing_tool",
        ToolNode::call("x", "nonexistent", json!({})),
    );

    let result = graph.execute(&registry, &ctx).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::NotFound { name } => assert_eq!(name, "nonexistent"),
        other => panic!("Expected NotFound, got {other:?}"),
    }
}

// ===========================================================================
// ToolGraph: nested structures
// ===========================================================================

#[tokio::test]
async fn test_graph_nested_sequence_in_parallel() {
    let registry = ToolRegistry::new().register(AddTool);
    let ctx = make_ctx();

    let graph = ToolGraph::new(
        "nested",
        ToolNode::Parallel {
            id: "root_par".into(),
            nodes: vec![
                ToolNode::Sequence {
                    id: "branch_a".into(),
                    nodes: vec![
                        ToolNode::call("a1", "add", json!({"a": 1, "b": 1})),
                        ToolNode::call("a2", "add", json!({"a": 2, "b": 2})),
                    ],
                },
                ToolNode::call("b1", "add", json!({"a": 10, "b": 10})),
            ],
        },
    );

    let output = graph.execute(&registry, &ctx).await.unwrap();
    assert_eq!(output.tool_calls, 3);
    assert_eq!(output.node_outputs["a1"], json!({"result": 2.0}));
    assert_eq!(output.node_outputs["a2"], json!({"result": 4.0}));
    assert_eq!(output.node_outputs["b1"], json!({"result": 20.0}));
}

// ===========================================================================
// ToolNode constructors
// ===========================================================================

#[test]
fn test_tool_node_call_with_deps() {
    let node = ToolNode::call_with_deps(
        "step2",
        "process",
        json!({"data": "input"}),
        vec!["step1".into()],
    );

    assert_eq!(node.id(), "step2");
    if let ToolNode::Call { depends_on, .. } = &node {
        assert_eq!(depends_on, &vec!["step1".to_string()]);
    } else {
        panic!("Expected Call node");
    }
}

// ===========================================================================
// ToolError factory methods
// ===========================================================================

#[test]
fn test_tool_error_factories() {
    let e1 = ToolError::not_found("web_search");
    assert!(format!("{e1}").contains("web_search"));

    let e2 = ToolError::execution_failed("db_query", "connection refused");
    assert!(format!("{e2}").contains("connection refused"));

    let e3 = ToolError::input_validation("parse_json", "missing required field 'data'");
    assert!(format!("{e3}").contains("missing required field"));

    let e4 = ToolError::internal("unexpected panic");
    assert!(format!("{e4}").contains("unexpected panic"));
}

// ===========================================================================
// SideEffect serialization
// ===========================================================================

#[test]
fn test_side_effect_serialization() {
    let effects = vec![
        SideEffect::HttpRequest { url: "https://api.example.com".into(), method: "POST".into() },
        SideEffect::DataWrite { target: "postgres".into(), key: "user:42".into() },
        SideEffect::FileWrite { path: "/tmp/output.json".into() },
        SideEffect::MessageSent { channel: "email".into(), recipient: "alice@example.com".into() },
        SideEffect::Custom { kind: "audit_log".into(), details: json!({"action": "delete"}) },
    ];

    for effect in &effects {
        let json_str = serde_json::to_string(effect).unwrap();
        let _: SideEffect = serde_json::from_str(&json_str).unwrap();
    }
}

// ===========================================================================
// ToolGraph: graph name getter
// ===========================================================================

#[test]
fn test_graph_name() {
    let graph = ToolGraph::new("my_graph", ToolNode::call("n", "tool", json!({})));
    assert_eq!(graph.name(), "my_graph");
}
