use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// TraceNodeKind — what happened
// ---------------------------------------------------------------------------

/// Classification of a trace node — what kind of operation it represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceNodeKind {
    /// The root HTTP request was received.
    HttpReceived,
    /// An agent execution step.
    AgentStep,
    /// A planning / reasoning step.
    PlanningStep,
    /// A tool invocation.
    ToolCall,
    /// An LLM completion or chat call.
    LlmCall,
    /// A memory read / write / search operation.
    MemoryOp,
    /// A branching decision point.
    BranchDecision,
    /// A retry attempt.
    Retry,
    /// User-defined custom node.
    Custom(String),
}

// ---------------------------------------------------------------------------
// TraceStatus — outcome
// ---------------------------------------------------------------------------

/// Outcome of a traced operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    /// Completed successfully.
    Ok,
    /// Failed with an error message.
    Error(String),
    /// Skipped (e.g. conditional branch not taken).
    Skipped,
    /// Currently executing.
    InProgress,
}

// ---------------------------------------------------------------------------
// TraceNode — single node in the execution tree
// ---------------------------------------------------------------------------

/// A single node in the hierarchical execution trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceNode {
    /// Unique identifier for this node.
    pub id: String,
    /// What kind of operation this represents.
    pub kind: TraceNodeKind,
    /// Human-readable label (e.g. tool name, model name).
    pub label: String,
    /// Input data (serialised).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Output data (serialised).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Outcome of this node.
    pub status: TraceStatus,
    /// Wall-clock start time.
    pub started_at: DateTime<Utc>,
    /// Duration in milliseconds (set on completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Ordered child nodes.
    pub children: Vec<TraceNode>,
    /// Arbitrary key-value metadata.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl TraceNode {
    /// Create a new in-progress trace node.
    pub fn new(kind: TraceNodeKind, label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            label: label.into(),
            input: None,
            output: None,
            status: TraceStatus::InProgress,
            started_at: Utc::now(),
            duration_ms: None,
            children: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Attach input data.
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    /// Mark as completed successfully and record duration.
    pub fn complete(&mut self, output: Option<serde_json::Value>) {
        self.status = TraceStatus::Ok;
        self.output = output;
        self.duration_ms = Some(
            (Utc::now() - self.started_at)
                .num_milliseconds()
                .max(0) as u64,
        );
    }

    /// Mark as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TraceStatus::Error(error.into());
        self.duration_ms = Some(
            (Utc::now() - self.started_at)
                .num_milliseconds()
                .max(0) as u64,
        );
    }

    /// Mark as skipped.
    pub fn skip(&mut self) {
        self.status = TraceStatus::Skipped;
        self.duration_ms = Some(0);
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: TraceNode) {
        self.children.push(child);
    }

    /// Add metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Get total duration including children (if self duration is missing,
    /// sums children).
    pub fn total_duration_ms(&self) -> u64 {
        self.duration_ms.unwrap_or_else(|| {
            self.children.iter().map(|c| c.total_duration_ms()).sum()
        })
    }

    /// Count all nodes in this subtree (including self).
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// TraceTree — the full execution trace (thread-safe)
// ---------------------------------------------------------------------------

/// Thread-safe, append-only execution trace tree.
///
/// The tree is wrapped in `Arc<RwLock<..>>` so that concurrent agent steps,
/// tool calls, and LLM invocations can all record their traces.
#[derive(Debug, Clone)]
pub struct TraceTree {
    inner: Arc<RwLock<TraceNode>>,
}

impl TraceTree {
    /// Create a new trace tree with an HTTP root node.
    pub fn new_http(method: &str, path: &str) -> Self {
        let mut root = TraceNode::new(TraceNodeKind::HttpReceived, format!("{method} {path}"));
        root.set_metadata(
            "http_method",
            serde_json::Value::String(method.to_string()),
        );
        root.set_metadata(
            "http_path",
            serde_json::Value::String(path.to_string()),
        );
        Self {
            inner: Arc::new(RwLock::new(root)),
        }
    }

    /// Create a trace tree with a custom root node.
    pub fn new(root: TraceNode) -> Self {
        Self {
            inner: Arc::new(RwLock::new(root)),
        }
    }

    /// Add a child node to the root.
    pub fn add_root_child(&self, child: TraceNode) {
        if let Ok(mut root) = self.inner.write() {
            root.add_child(child);
        }
    }

    /// Start a new child span under the root, returning a [`SpanGuard`] that
    /// auto-completes on drop.
    pub fn start_span(&self, kind: TraceNodeKind, label: impl Into<String>) -> SpanGuard {
        SpanGuard {
            tree: self.clone(),
            node: TraceNode::new(kind, label),
            _parent_path: Vec::new(), // root-level
        }
    }

    /// Complete the root node.
    pub fn complete_root(&self, output: Option<serde_json::Value>) {
        if let Ok(mut root) = self.inner.write() {
            root.complete(output);
        }
    }

    /// Fail the root node.
    pub fn fail_root(&self, error: impl Into<String>) {
        if let Ok(mut root) = self.inner.write() {
            root.fail(error);
        }
    }

    /// Get a serialisable snapshot of the entire tree.
    pub fn snapshot(&self) -> Option<TraceNode> {
        self.inner.read().ok().map(|r| r.clone())
    }

    /// Serialise to JSON.
    pub fn to_json(&self) -> Option<serde_json::Value> {
        self.snapshot()
            .and_then(|n| serde_json::to_value(&n).ok())
    }

    /// Count all nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.inner.read().map(|r| r.node_count()).unwrap_or(0)
    }
}

impl Default for TraceTree {
    fn default() -> Self {
        Self::new(TraceNode::new(TraceNodeKind::Custom("root".into()), "root"))
    }
}

// ---------------------------------------------------------------------------
// SpanGuard — RAII trace span
// ---------------------------------------------------------------------------

/// RAII guard that records a [`TraceNode`] into the [`TraceTree`] when
/// completed or dropped.
pub struct SpanGuard {
    tree: TraceTree,
    node: TraceNode,
    _parent_path: Vec<String>,
}

impl SpanGuard {
    /// Set input data on this span.
    pub fn set_input(&mut self, input: serde_json::Value) {
        self.node.input = Some(input);
    }

    /// Add child node directly.
    pub fn add_child(&mut self, child: TraceNode) {
        self.node.add_child(child);
    }

    /// Mark as completed with optional output.
    pub fn complete(mut self, output: Option<serde_json::Value>) {
        self.node.complete(output);
        self.tree.add_root_child(self.node.clone());
        // Prevent Drop from double-inserting.
        std::mem::forget(self);
    }

    /// Mark as failed.
    pub fn fail(mut self, error: impl Into<String>) {
        self.node.fail(error);
        self.tree.add_root_child(self.node.clone());
        std::mem::forget(self);
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        // Panic-safety: if the span was not explicitly completed or failed
        // (e.g. the thread panicked during step execution), record it with an
        // error status so the trace tree is never left with a dangling
        // in-progress node.  Explicit complete()/fail() callers use
        // std::mem::forget to skip this path entirely.
        if self.node.status == TraceStatus::InProgress {
            self.node.fail("span dropped without completion (likely due to a panic)");
        }
        self.tree.add_root_child(self.node.clone());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_tree_basic() {
        let tree = TraceTree::new_http("POST", "/agent/run");
        assert_eq!(tree.node_count(), 1);

        let mut span = tree.start_span(TraceNodeKind::AgentStep, "think");
        span.set_input(serde_json::json!({"goal": "research"}));
        span.complete(Some(serde_json::json!({"result": "done"})));

        assert_eq!(tree.node_count(), 2);

        let snapshot = tree.snapshot().unwrap();
        assert_eq!(snapshot.children.len(), 1);
        assert_eq!(snapshot.children[0].label, "think");
        assert_eq!(snapshot.children[0].status, TraceStatus::Ok);
    }

    #[test]
    fn test_trace_node_nesting() {
        let mut parent = TraceNode::new(TraceNodeKind::AgentStep, "plan");
        let child = TraceNode::new(TraceNodeKind::LlmCall, "gpt-4o");
        parent.add_child(child);
        assert_eq!(parent.node_count(), 2);
    }

    #[test]
    fn test_span_drop_records_error() {
        let tree = TraceTree::new_http("GET", "/test");
        {
            let _span = tree.start_span(TraceNodeKind::ToolCall, "web_search");
            // dropped without complete/fail
        }
        let snapshot = tree.snapshot().unwrap();
        assert_eq!(snapshot.children.len(), 1);
        match &snapshot.children[0].status {
            TraceStatus::Error(msg) => assert!(msg.contains("dropped")),
            other => panic!("Expected Error, got {other:?}"),
        }
    }

    #[test]
    fn test_trace_serialization() {
        let tree = TraceTree::new_http("POST", "/chat");
        tree.start_span(TraceNodeKind::LlmCall, "claude")
            .complete(Some(serde_json::json!({"text": "hello"})));
        let json = tree.to_json().unwrap();
        assert!(json.is_object());
        let children = json.get("children").unwrap().as_array().unwrap();
        assert_eq!(children.len(), 1);
    }
}
