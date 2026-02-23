use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors produced by the agent execution engine.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum AgentError {
    /// Planning failed.
    #[error("Planning failed: {message}")]
    PlanningFailed { message: String },

    /// A step execution failed.
    #[error("Step failed [{step}]: {message}")]
    StepFailed { step: String, message: String },

    /// Maximum step count exceeded (runaway loop protection).
    #[error("Max steps exceeded: {max_steps} steps reached")]
    MaxStepsExceeded { max_steps: usize },

    /// A branch target was not found in the execution plan.
    #[error("Branch not found: {branch_name}")]
    BranchNotFound { branch_name: String },

    /// Budget exceeded (delegated from CostTracker).
    #[error("Budget exceeded: {message}")]
    BudgetExceeded { message: String },

    /// Replay divergence detected.
    #[error("Replay divergence at step {step_index}: {message}")]
    ReplayDivergence { step_index: usize, message: String },

    /// Tool error.
    #[error("Tool error: {message}")]
    ToolError { message: String },

    /// Memory error.
    #[error("Memory error: {message}")]
    MemoryError { message: String },

    /// Generic internal error.
    #[error("Agent error: {message}")]
    Internal { message: String },
}

impl AgentError {
    pub fn planning_failed(msg: impl fmt::Display) -> Self {
        Self::PlanningFailed {
            message: msg.to_string(),
        }
    }

    pub fn step_failed(step: impl fmt::Display, msg: impl fmt::Display) -> Self {
        Self::StepFailed {
            step: step.to_string(),
            message: msg.to_string(),
        }
    }

    pub fn branch_not_found(name: impl fmt::Display) -> Self {
        Self::BranchNotFound {
            branch_name: name.to_string(),
        }
    }

    pub fn internal(msg: impl fmt::Display) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
}

impl From<rustapi_context::ContextError> for AgentError {
    fn from(e: rustapi_context::ContextError) -> Self {
        match e {
            rustapi_context::ContextError::BudgetExceeded { message } => {
                Self::BudgetExceeded { message }
            }
            other => Self::Internal {
                message: other.to_string(),
            },
        }
    }
}

impl From<rustapi_tools::ToolError> for AgentError {
    fn from(e: rustapi_tools::ToolError) -> Self {
        Self::ToolError {
            message: e.to_string(),
        }
    }
}

impl From<rustapi_memory::MemoryError> for AgentError {
    fn from(e: rustapi_memory::MemoryError) -> Self {
        Self::MemoryError {
            message: e.to_string(),
        }
    }
}
