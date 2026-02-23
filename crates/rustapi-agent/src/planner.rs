use crate::AgentError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A single step in an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    /// Human-readable name.
    pub name: String,
    /// Which tool to use (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Expected input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Description of what this step should achieve.
    pub description: String,
    /// Optional branch label (for conditional jumps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_label: Option<String>,
}

impl PlannedStep {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tool_name: None,
            input: None,
            description: description.into(),
            branch_label: None,
        }
    }

    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn with_branch_label(mut self, label: impl Into<String>) -> Self {
        self.branch_label = Some(label.into());
        self
    }
}

/// An execution plan produced by a [`Planner`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Ordered sequence of steps.
    pub steps: Vec<PlannedStep>,
    /// Named branch targets (step index → branch_name → target step index).
    #[serde(default)]
    pub branches: std::collections::HashMap<String, usize>,
    /// Optional plan summary (for tracing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl ExecutionPlan {
    pub fn new(steps: Vec<PlannedStep>) -> Self {
        Self {
            steps,
            branches: std::collections::HashMap::new(),
            summary: None,
        }
    }

    pub fn with_branch(mut self, name: impl Into<String>, target_index: usize) -> Self {
        self.branches.insert(name.into(), target_index);
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Number of steps in the plan.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// Trait for planning strategies that decompose goals into execution plans.
#[async_trait]
pub trait Planner: Send + Sync + 'static {
    /// Generate an execution plan from a goal description and optional context.
    async fn plan(
        &self,
        goal: &str,
        context: &crate::AgentContext,
    ) -> Result<ExecutionPlan, AgentError>;
}

// ---------------------------------------------------------------------------
// StaticPlanner — predefined plan (deterministic, testing)
// ---------------------------------------------------------------------------

/// A planner that always returns the same predefined plan.
/// Useful for testing, profiling, and deterministic workflows.
#[derive(Debug, Clone)]
pub struct StaticPlanner {
    plan: ExecutionPlan,
}

impl StaticPlanner {
    pub fn new(plan: ExecutionPlan) -> Self {
        Self { plan }
    }
}

#[async_trait]
impl Planner for StaticPlanner {
    async fn plan(
        &self,
        _goal: &str,
        _context: &crate::AgentContext,
    ) -> Result<ExecutionPlan, AgentError> {
        Ok(self.plan.clone())
    }
}

// ---------------------------------------------------------------------------
// ReActPlanner — Reasoning + Acting loop
// ---------------------------------------------------------------------------

/// Placeholder for a ReAct planner that uses an LLM to decompose goals.
///
/// The full implementation will be provided when the LLM crate is integrated.
/// For now, this creates a single-step plan that delegates to the LLM.
#[derive(Debug, Clone)]
pub struct ReActPlanner {
    /// Maximum reasoning iterations.
    pub max_iterations: usize,
}

impl ReActPlanner {
    pub fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
    }
}

impl Default for ReActPlanner {
    fn default() -> Self {
        Self {
            max_iterations: 10,
        }
    }
}

#[async_trait]
impl Planner for ReActPlanner {
    async fn plan(
        &self,
        goal: &str,
        _context: &crate::AgentContext,
    ) -> Result<ExecutionPlan, AgentError> {
        // Stub: creates a think → act → observe loop.
        // Full LLM integration will generate dynamic plans.
        let steps = vec![
            PlannedStep::new("think", format!("Reason about goal: {goal}"))
                .with_branch_label("think"),
            PlannedStep::new("act", "Execute the chosen action using tools")
                .with_branch_label("act"),
            PlannedStep::new("observe", "Observe results and decide next action")
                .with_branch_label("observe"),
            PlannedStep::new("complete", "Synthesize final answer"),
        ];

        Ok(ExecutionPlan::new(steps)
            .with_branch("think", 0)
            .with_branch("act", 1)
            .with_branch("observe", 2)
            .with_summary(format!(
                "ReAct loop (max {}) for: {goal}",
                self.max_iterations
            )))
    }
}
