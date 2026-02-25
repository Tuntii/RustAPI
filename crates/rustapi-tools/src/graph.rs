use crate::{ToolError, ToolRegistry};
use rustapi_context::{RequestContext, TraceNodeKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ToolNode — nodes in the execution graph
// ---------------------------------------------------------------------------

/// A node in the tool execution graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolNode {
    /// Invoke a single tool.
    Call {
        /// Unique id for this node within the graph.
        id: String,
        /// Name of the tool to call (must exist in ToolRegistry).
        tool_name: String,
        /// Input to the tool (may reference outputs of dependency nodes).
        input: serde_json::Value,
        /// IDs of nodes that must complete before this one.
        #[serde(default)]
        depends_on: Vec<String>,
    },

    /// Execute multiple nodes in parallel (fork-join).
    Parallel {
        id: String,
        /// Nodes to run concurrently.
        nodes: Vec<ToolNode>,
    },

    /// Execute nodes sequentially.
    Sequence {
        id: String,
        /// Nodes to run in order.
        nodes: Vec<ToolNode>,
    },

    /// Conditional branching.
    Conditional {
        id: String,
        /// A JSON path expression or simple condition on the context.
        condition: String,
        /// Node to execute if condition is true.
        if_true: Box<ToolNode>,
        /// Node to execute if condition is false.
        #[serde(skip_serializing_if = "Option::is_none")]
        if_false: Option<Box<ToolNode>>,
    },
}

impl ToolNode {
    /// Get the node id.
    pub fn id(&self) -> &str {
        match self {
            Self::Call { id, .. }
            | Self::Parallel { id, .. }
            | Self::Sequence { id, .. }
            | Self::Conditional { id, .. } => id,
        }
    }

    /// Create a simple tool call node.
    pub fn call(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::Call {
            id: id.into(),
            tool_name: tool_name.into(),
            input,
            depends_on: Vec::new(),
        }
    }

    /// Create a call node with dependencies.
    pub fn call_with_deps(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        input: serde_json::Value,
        depends_on: Vec<String>,
    ) -> Self {
        Self::Call {
            id: id.into(),
            tool_name: tool_name.into(),
            input,
            depends_on,
        }
    }
}

// ---------------------------------------------------------------------------
// GraphOutput — result of executing a tool graph
// ---------------------------------------------------------------------------

/// Aggregated output from a complete tool graph execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphOutput {
    /// Outputs keyed by node id.
    pub node_outputs: HashMap<String, serde_json::Value>,
    /// Total execution time in milliseconds.
    pub total_duration_ms: u64,
    /// Number of tool calls made.
    pub tool_calls: usize,
}

// ---------------------------------------------------------------------------
// ToolGraph — the execution DAG
// ---------------------------------------------------------------------------

/// A directed acyclic graph of tool invocations.
///
/// The graph resolves dependencies between nodes and executes them
/// with maximum parallelism where possible.
#[derive(Debug, Clone)]
pub struct ToolGraph {
    /// Name for identification / tracing.
    name: String,
    /// The root node of the graph.
    pub(crate) root: ToolNode,
    /// Global timeout for the entire graph execution.
    timeout_ms: Option<u64>,
}

impl ToolGraph {
    /// Create a new tool graph.
    pub fn new(name: impl Into<String>, root: ToolNode) -> Self {
        Self {
            name: name.into(),
            root,
            timeout_ms: None,
        }
    }

    /// Graph name for identification / tracing.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set a global timeout for graph execution.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Execute the graph against a tool registry.
    pub async fn execute(
        &self,
        registry: &ToolRegistry,
        ctx: &RequestContext,
    ) -> Result<GraphOutput, ToolError> {
        let start = std::time::Instant::now();
        let mut outputs = HashMap::new();

        let result = if let Some(timeout_ms) = self.timeout_ms {
            let duration = std::time::Duration::from_millis(timeout_ms);
            match tokio::time::timeout(
                duration,
                self.execute_node(&self.root, registry, ctx, &mut outputs),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(ToolError::Timeout {
                    timeout_ms,
                }),
            }
        } else {
            self.execute_node(&self.root, registry, ctx, &mut outputs)
                .await
        };

        result?;

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(GraphOutput {
            tool_calls: outputs.len(),
            node_outputs: outputs,
            total_duration_ms: elapsed,
        })
    }

    /// Recursively execute a node.
    #[allow(clippy::only_used_in_recursion)]
    fn execute_node<'a>(
        &'a self,
        node: &'a ToolNode,
        registry: &'a ToolRegistry,
        ctx: &'a RequestContext,
        outputs: &'a mut HashMap<String, serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send + 'a>>
    {
        Box::pin(async move {
            match node {
                ToolNode::Call {
                    id,
                    tool_name,
                    input,
                    ..
                } => {
                    let mut span =
                        ctx.trace()
                            .start_span(TraceNodeKind::ToolCall, tool_name.as_str());
                    span.set_input(input.clone());

                    match registry.execute(tool_name, ctx, input.clone()).await {
                        Ok(output) => {
                            outputs.insert(id.clone(), output.value.clone());
                            span.complete(Some(output.value));
                            Ok(())
                        }
                        Err(e) => {
                            span.fail(e.to_string());
                            Err(e)
                        }
                    }
                }

                ToolNode::Sequence { nodes, .. } => {
                    for child in nodes {
                        self.execute_node(child, registry, ctx, outputs).await?;
                    }
                    Ok(())
                }

                ToolNode::Parallel { id: _, nodes } => {
                    // Spawn all child nodes concurrently via JoinSet so results
                    // are collected as each task finishes rather than in
                    // submission order.  Any failure aborts the remaining tasks.
                    let mut set = tokio::task::JoinSet::new();
                    let registry = registry.clone();
                    let ctx = ctx.clone();

                    for child_node in nodes {
                        let reg = registry.clone();
                        let c = ctx.clone();
                        let node = child_node.clone();
                        set.spawn(async move {
                            let mut local_outputs = HashMap::new();
                            let graph = ToolGraph::new("parallel_child", node.clone());
                            match graph
                                .execute_node(&node, &reg, &c, &mut local_outputs)
                                .await
                            {
                                Ok(()) => Ok(local_outputs),
                                Err(e) => Err(e),
                            }
                        });
                    }

                    while let Some(result) = set.join_next().await {
                        match result {
                            Ok(Ok(local_outputs)) => {
                                outputs.extend(local_outputs);
                            }
                            Ok(Err(e)) => {
                                set.abort_all();
                                return Err(e);
                            }
                            Err(e) => {
                                return Err(ToolError::internal(format!("Join error: {e}")));
                            }
                        }
                    }
                    Ok(())
                }

                ToolNode::Conditional {
                    condition,
                    if_true,
                    if_false,
                    ..
                } => {
                    // Simple condition evaluation: check if a node output is truthy.
                    let condition_met = evaluate_condition(condition, outputs);

                    let span =
                        ctx.trace()
                            .start_span(TraceNodeKind::BranchDecision, condition.as_str());

                    if condition_met {
                        span.complete(Some(serde_json::json!({"branch": "if_true"})));
                        self.execute_node(if_true, registry, ctx, outputs).await
                    } else if let Some(ref false_node) = if_false {
                        span.complete(Some(serde_json::json!({"branch": "if_false"})));
                        self.execute_node(false_node, registry, ctx, outputs).await
                    } else {
                        span.complete(Some(serde_json::json!({"branch": "skipped"})));
                        Ok(())
                    }
                }
            }
        })
    }
}

/// Simple condition evaluator.
///
/// Supports:
/// - `"node_id"` — truthy check on a node's output
/// - `"node_id.field"` — truthy check on a field of a node's output
/// - `"true"` / `"false"` — literal booleans
fn evaluate_condition(condition: &str, outputs: &HashMap<String, serde_json::Value>) -> bool {
    match condition {
        "true" => true,
        "false" => false,
        _ => {
            if let Some((node_id, field)) = condition.split_once('.') {
                outputs
                    .get(node_id)
                    .and_then(|v| v.get(field))
                    .map(is_truthy)
                    .unwrap_or(false)
            } else {
                outputs
                    .get(condition)
                    .map(is_truthy)
                    .unwrap_or(false)
            }
        }
    }
}

fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Tool, ToolOutput};
    use async_trait::async_trait;
    use rustapi_context::RequestContextBuilder;

    struct AddTool;

    #[async_trait]
    impl Tool for AddTool {
        fn name(&self) -> &str {
            "add"
        }
        fn description(&self) -> &str {
            "Adds two numbers"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(
            &self,
            _ctx: &RequestContext,
            input: serde_json::Value,
        ) -> Result<ToolOutput, ToolError> {
            let a = input["a"].as_f64().unwrap_or(0.0);
            let b = input["b"].as_f64().unwrap_or(0.0);
            Ok(ToolOutput::value(serde_json::json!({"result": a + b})))
        }
    }

    #[tokio::test]
    async fn test_graph_single_call() {
        let registry = ToolRegistry::new().register(AddTool);
        let ctx = RequestContextBuilder::new().build();

        let graph = ToolGraph::new(
            "test",
            ToolNode::call("step1", "add", serde_json::json!({"a": 2, "b": 3})),
        );

        let output = graph.execute(&registry, &ctx).await.unwrap();
        assert_eq!(output.tool_calls, 1);
        assert_eq!(
            output.node_outputs["step1"],
            serde_json::json!({"result": 5.0})
        );
    }

    #[tokio::test]
    async fn test_graph_sequence() {
        let registry = ToolRegistry::new().register(AddTool);
        let ctx = RequestContextBuilder::new().build();

        let graph = ToolGraph::new(
            "seq_test",
            ToolNode::Sequence {
                id: "seq".into(),
                nodes: vec![
                    ToolNode::call("s1", "add", serde_json::json!({"a": 1, "b": 2})),
                    ToolNode::call("s2", "add", serde_json::json!({"a": 3, "b": 4})),
                ],
            },
        );

        let output = graph.execute(&registry, &ctx).await.unwrap();
        assert_eq!(output.tool_calls, 2);
    }

    #[tokio::test]
    async fn test_graph_conditional_true() {
        let registry = ToolRegistry::new().register(AddTool);
        let ctx = RequestContextBuilder::new().build();

        let graph = ToolGraph::new(
            "cond_test",
            ToolNode::Sequence {
                id: "seq".into(),
                nodes: vec![
                    ToolNode::call("check", "add", serde_json::json!({"a": 1, "b": 0})),
                    ToolNode::Conditional {
                        id: "branch".into(),
                        condition: "check.result".into(),
                        if_true: Box::new(ToolNode::call(
                            "yes",
                            "add",
                            serde_json::json!({"a": 10, "b": 20}),
                        )),
                        if_false: None,
                    },
                ],
            },
        );

        let output = graph.execute(&registry, &ctx).await.unwrap();
        assert!(output.node_outputs.contains_key("yes"));
    }

    #[test]
    fn test_evaluate_condition() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "node1".to_string(),
            serde_json::json!({"active": true, "count": 0}),
        );

        assert!(evaluate_condition("true", &outputs));
        assert!(!evaluate_condition("false", &outputs));
        assert!(evaluate_condition("node1.active", &outputs));
        assert!(!evaluate_condition("node1.count", &outputs));
        assert!(!evaluate_condition("nonexistent", &outputs));
    }
}
