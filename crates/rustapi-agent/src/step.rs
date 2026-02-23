use crate::AgentError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The result of executing a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepResult {
    /// Continue to the next step in the plan, carrying a value forward.
    Continue { value: serde_json::Value },
    /// Jump to a named branch in the execution plan.
    Branch {
        branch_name: String,
        value: serde_json::Value,
    },
    /// Yield a partial result (for streaming responses).
    Yield { value: serde_json::Value },
    /// The agent has completed its work.
    Complete { value: serde_json::Value },
    /// The step failed with an error.
    Error { message: String },
}

impl StepResult {
    pub fn cont(value: serde_json::Value) -> Self {
        Self::Continue { value }
    }

    pub fn branch(name: impl Into<String>, value: serde_json::Value) -> Self {
        Self::Branch {
            branch_name: name.into(),
            value,
        }
    }

    pub fn yield_partial(value: serde_json::Value) -> Self {
        Self::Yield { value }
    }

    pub fn complete(value: serde_json::Value) -> Self {
        Self::Complete { value }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error {
            message: msg.into(),
        }
    }

    /// Whether this result terminates the execution.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete { .. } | Self::Error { .. })
    }
}

/// A single executable step in the agent's plan.
///
/// Steps are the fundamental unit of work. They receive an
/// [`AgentContext`](crate::AgentContext) with access to tools, memory,
/// and the running state, and produce a [`StepResult`] that drives
/// the execution forward.
#[async_trait]
pub trait Step: Send + Sync + 'static {
    /// Human-readable name for tracing / logging.
    fn name(&self) -> &str;

    /// Execute this step.
    async fn execute(&self, ctx: &mut crate::AgentContext) -> Result<StepResult, AgentError>;
}

// ---------------------------------------------------------------------------
// ClosureStep — inline step from a closure
// ---------------------------------------------------------------------------

/// A step created from an async closure, for quick prototyping.
pub struct ClosureStep<F>
where
    F: Fn(
            &mut crate::AgentContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult, AgentError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    name: String,
    handler: F,
}

impl<F> ClosureStep<F>
where
    F: Fn(
            &mut crate::AgentContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult, AgentError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }
}

#[async_trait]
impl<F> Step for ClosureStep<F>
where
    F: Fn(
            &mut crate::AgentContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult, AgentError>> + Send + '_>>
        + Send
        + Sync
        + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut crate::AgentContext) -> Result<StepResult, AgentError> {
        (self.handler)(ctx).await
    }
}
