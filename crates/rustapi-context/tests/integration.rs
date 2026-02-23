//! Integration tests for rustapi-context
//!
//! Tests cross-module interactions between RequestContext, TraceTree,
//! CostTracker, EventBus, and ObservabilityCtx.

use chrono::Utc;
use rustapi_context::*;
use serde_json::json;

// ===========================================================================
// ObservabilityCtx tests
// ===========================================================================

#[test]
fn test_observability_ctx_child_of() {
    let parent = ObservabilityCtx::new();
    let child = ObservabilityCtx::child_of(&parent.trace_id, &parent.span_id);

    // Child inherits parent trace_id.
    assert_eq!(child.trace_id, parent.trace_id);
    // Child has new span_id.
    assert_ne!(child.span_id, parent.span_id);
    // Child references parent span.
    assert_eq!(child.parent_span_id.as_deref(), Some(parent.span_id.as_str()));
}

#[test]
fn test_observability_ctx_default() {
    let obs = ObservabilityCtx::default();
    assert!(!obs.trace_id.is_empty());
    assert!(!obs.span_id.is_empty());
    assert!(obs.parent_span_id.is_none());
    assert!(obs.baggage.is_empty());
}

// ===========================================================================
// SpanGuard advanced tests
// ===========================================================================

#[test]
fn test_span_guard_add_child() {
    let tree = TraceTree::new_http("POST", "/agent/run");

    let mut span = tree.start_span(TraceNodeKind::AgentStep, "plan");
    let child1 = TraceNode::new(TraceNodeKind::LlmCall, "gpt-4o");
    let child2 = TraceNode::new(TraceNodeKind::ToolCall, "web_search");
    span.add_child(child1);
    span.add_child(child2);
    span.complete(Some(json!({"plan": "done"})));

    let snapshot = tree.snapshot().unwrap();
    assert_eq!(snapshot.children.len(), 1); // plan span
    assert_eq!(snapshot.children[0].children.len(), 2); // gpt-4o + web_search
    assert_eq!(snapshot.children[0].children[0].label, "gpt-4o");
    assert_eq!(snapshot.children[0].children[1].label, "web_search");
}

#[test]
fn test_span_guard_fail() {
    let tree = TraceTree::new_http("POST", "/agent/run");
    let span = tree.start_span(TraceNodeKind::ToolCall, "db_query");
    span.fail("connection timeout");

    let snapshot = tree.snapshot().unwrap();
    assert_eq!(snapshot.children.len(), 1);
    match &snapshot.children[0].status {
        TraceStatus::Error(msg) => assert!(msg.contains("connection timeout")),
        other => panic!("Expected Error, got {other:?}"),
    }
    assert!(snapshot.children[0].duration_ms.is_some());
}

#[test]
fn test_trace_tree_fail_root() {
    let tree = TraceTree::new_http("GET", "/fail");
    tree.fail_root("internal server error");

    let snapshot = tree.snapshot().unwrap();
    match &snapshot.status {
        TraceStatus::Error(msg) => assert!(msg.contains("internal server error")),
        other => panic!("Expected Error on root, got {other:?}"),
    }
}

#[test]
fn test_trace_tree_complete_root() {
    let tree = TraceTree::new_http("GET", "/ok");
    tree.complete_root(Some(json!({"status": 200})));

    let snapshot = tree.snapshot().unwrap();
    assert_eq!(snapshot.status, TraceStatus::Ok);
    assert!(snapshot.output.is_some());
}

// ===========================================================================
// CostBudget builder tests
// ===========================================================================

#[test]
fn test_cost_budget_chained_builders() {
    let budget = CostBudget::unlimited()
        .with_max_tokens(5000)
        .with_max_cost_usd(0.25)
        .with_max_api_calls(10);

    assert_eq!(budget.max_tokens, Some(5000));
    assert_eq!(budget.max_cost_micros, Some(250_000));
    assert_eq!(budget.max_api_calls, Some(10));
}

#[test]
fn test_cost_budget_api_call_limit() {
    let budget = CostBudget::unlimited().with_max_api_calls(2);
    let tracker = CostTracker::with_budget(budget);

    let small_delta = CostDelta {
        input_tokens: 1,
        output_tokens: 1,
        cost_micros: 1,
        model: None,
    };

    // First two calls should succeed.
    assert!(tracker.record(&small_delta).is_ok());
    assert!(tracker.record(&small_delta).is_ok());

    // Third call exceeds limit.
    let result = tracker.record(&small_delta);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContextError::BudgetExceeded { message } => {
            assert!(message.contains("API call limit"));
        }
        other => panic!("Expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn test_cost_budget_cost_micros_limit() {
    let budget = CostBudget::per_request_usd(0.001); // 1000 micros
    let tracker = CostTracker::with_budget(budget);

    let delta = CostDelta {
        input_tokens: 10,
        output_tokens: 5,
        cost_micros: 1500, // Exceeds 1000 micro-USD
        model: Some("expensive-model".into()),
    };

    let result = tracker.record(&delta);
    assert!(result.is_err());
}

// ===========================================================================
// EventBus + EventSubscriber tests
// ===========================================================================

#[tokio::test]
async fn test_event_subscriber_try_recv() {
    let bus = EventBus::new(16);
    let mut sub = bus.subscribe();

    // Nothing available yet.
    assert!(sub.try_recv().is_none());

    bus.emit(ExecutionEvent::ContextCreated {
        context_id: "ctx-try".into(),
        timestamp: Utc::now(),
    });

    // Now available via try_recv.
    let event = sub.try_recv();
    assert!(event.is_some());
    assert_eq!(event.unwrap().context_id(), "ctx-try");

    // Queue is now empty again.
    assert!(sub.try_recv().is_none());
}

#[test]
fn test_event_bus_subscriber_count() {
    let bus = EventBus::new(16);
    assert_eq!(bus.subscriber_count(), 0);

    let _sub1 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);

    let _sub2 = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 2);

    drop(_sub1);
    assert_eq!(bus.subscriber_count(), 1);
}

#[tokio::test]
async fn test_event_bus_full_lifecycle() {
    let bus = EventBus::new(64);
    let mut sub = bus.subscribe();

    // Emit a sequence of events simulating a real request lifecycle.
    bus.emit(ExecutionEvent::RequestReceived {
        context_id: "req-1".into(),
        method: "POST".into(),
        path: "/agent/chat".into(),
        timestamp: Utc::now(),
    });
    bus.emit(ExecutionEvent::ContextCreated {
        context_id: "req-1".into(),
        timestamp: Utc::now(),
    });
    bus.emit(ExecutionEvent::AgentStepStarted {
        context_id: "req-1".into(),
        step_name: "think".into(),
        step_index: 0,
        timestamp: Utc::now(),
    });
    bus.emit(ExecutionEvent::LlmCallStarted {
        context_id: "req-1".into(),
        model: "gpt-4o".into(),
        timestamp: Utc::now(),
    });
    bus.emit(ExecutionEvent::LlmCallCompleted {
        context_id: "req-1".into(),
        model: "gpt-4o".into(),
        input_tokens: 500,
        output_tokens: 200,
        cost_micros: 1500,
        duration_ms: 850,
        timestamp: Utc::now(),
    });
    bus.emit(ExecutionEvent::ResponseGenerated {
        context_id: "req-1".into(),
        status_code: 200,
        total_duration_ms: 1200,
        timestamp: Utc::now(),
    });

    // Verify we can receive all events in order.
    let e1 = sub.recv().await.unwrap();
    assert_eq!(e1.context_id(), "req-1");

    let e2 = sub.recv().await.unwrap();
    assert_eq!(e2.context_id(), "req-1");

    // Drain remaining events.
    for _ in 0..4 {
        let e = sub.recv().await.unwrap();
        assert_eq!(e.context_id(), "req-1");
    }
}

// ===========================================================================
// Cross-module: RequestContext + TraceTree + CostTracker + EventBus
// ===========================================================================

#[tokio::test]
async fn test_full_context_lifecycle() {
    let bus = EventBus::new(64);
    let mut sub = bus.subscribe();

    let ctx = RequestContextBuilder::new()
        .method("POST")
        .path("/agent/research")
        .auth(AuthContext::new("user-42").with_role("admin").with_name("Bob"))
        .budget(CostBudget::per_request_usd(1.0).with_max_api_calls(100))
        .metadata("tenant", json!("acme"))
        .event_bus(bus.clone())
        .build();

    // Verify context fields.
    assert!(!ctx.id().is_empty());
    assert_eq!(ctx.auth().unwrap().subject, "user-42");
    assert!(ctx.auth().unwrap().has_role("admin"));
    assert_eq!(ctx.metadata_value("tenant"), Some(&json!("acme")));

    // Trace: add tool call span.
    let mut span = ctx.trace().start_span(TraceNodeKind::ToolCall, "web_search");
    span.set_input(json!({"query": "rust async"}));
    span.complete(Some(json!({"results": 3})));

    assert_eq!(ctx.trace().node_count(), 2);

    // Cost: record an LLM call.
    ctx.cost().record(&CostDelta {
        input_tokens: 500,
        output_tokens: 200,
        cost_micros: 1500,
        model: Some("gpt-4o".into()),
    }).unwrap();

    assert_eq!(ctx.cost().total_tokens(), 700);
    assert_eq!(ctx.cost().api_calls(), 1);

    // Event bus: emit and receive.
    ctx.event_bus().emit(ExecutionEvent::ToolCompleted {
        context_id: ctx.id().to_string(),
        tool_name: "web_search".into(),
        duration_ms: 150,
        success: true,
        timestamp: Utc::now(),
    });

    let event = sub.recv().await.unwrap();
    assert_eq!(event.context_id(), ctx.id());

    // Cost snapshot.
    let snap = ctx.cost().snapshot();
    assert_eq!(snap.input_tokens, 500);
    assert_eq!(snap.output_tokens, 200);
    assert_eq!(snap.total_tokens, 700);
    assert_eq!(snap.api_calls, 1);
}

#[test]
fn test_context_clone_shares_trace_and_cost() {
    let ctx = RequestContextBuilder::new()
        .method("GET")
        .path("/test")
        .build();

    let ctx_clone = ctx.clone();

    // Both share the same trace tree (Arc).
    ctx.trace()
        .start_span(TraceNodeKind::LlmCall, "model-a")
        .complete(None);

    // Clone sees the change.
    assert_eq!(ctx_clone.trace().node_count(), 2);

    // Both share the same cost tracker.
    ctx.cost().record(&CostDelta {
        input_tokens: 100,
        output_tokens: 50,
        cost_micros: 300,
        model: None,
    }).unwrap();

    assert_eq!(ctx_clone.cost().total_tokens(), 150);
}

#[test]
fn test_context_with_custom_observability() {
    let parent_obs = ObservabilityCtx::new();
    let child_obs = ObservabilityCtx::child_of(&parent_obs.trace_id, &parent_obs.span_id);

    let ctx = RequestContextBuilder::new()
        .observability(child_obs)
        .build();

    assert_eq!(ctx.observability().trace_id, parent_obs.trace_id);
    assert!(ctx.observability().parent_span_id.is_some());
}

// ===========================================================================
// TraceNode metadata and total_duration_ms
// ===========================================================================

#[test]
fn test_trace_node_total_duration_sums_children() {
    let mut parent = TraceNode::new(TraceNodeKind::AgentStep, "orchestrate");
    // Parent has no explicit duration.

    let mut child1 = TraceNode::new(TraceNodeKind::ToolCall, "tool-a");
    child1.duration_ms = Some(100);

    let mut child2 = TraceNode::new(TraceNodeKind::LlmCall, "llm-b");
    child2.duration_ms = Some(200);

    parent.add_child(child1);
    parent.add_child(child2);

    // Parent's total_duration_ms should sum children when own duration is None.
    assert_eq!(parent.total_duration_ms(), 300);
}

#[test]
fn test_trace_node_skip() {
    let mut node = TraceNode::new(TraceNodeKind::BranchDecision, "check_cache");
    node.skip();

    assert_eq!(node.status, TraceStatus::Skipped);
    assert_eq!(node.duration_ms, Some(0));
}

#[test]
fn test_trace_node_metadata() {
    let mut node = TraceNode::new(TraceNodeKind::Custom("custom".into()), "my-op");
    node.set_metadata("retry_count", json!(3));
    node.set_metadata("provider", json!("openai"));

    assert_eq!(node.metadata.len(), 2);
    assert_eq!(node.metadata.get("retry_count"), Some(&json!(3)));
}

// ===========================================================================
// AuthContext serialization
// ===========================================================================

#[test]
fn test_auth_context_serialization() {
    let auth = AuthContext::new("svc-account")
        .with_name("CI Bot")
        .with_role("deploy");

    let json_str = serde_json::to_string(&auth).unwrap();
    let deserialized: AuthContext = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.subject, "svc-account");
    assert_eq!(deserialized.name, Some("CI Bot".to_string()));
    assert!(deserialized.has_role("deploy"));
}

// ===========================================================================
// Event serialization round-trip
// ===========================================================================

#[test]
fn test_execution_event_serialization_roundtrip() {
    let event = ExecutionEvent::CostUpdated {
        context_id: "ctx-roundtrip".into(),
        total_tokens: 1000,
        total_cost_micros: 5000,
        api_calls: 3,
        timestamp: Utc::now(),
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: ExecutionEvent = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.context_id(), "ctx-roundtrip");
}
