use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors produced by the tool subsystem.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ToolError {
    /// Tool was not found in the registry.
    #[error("Tool not found: {name}")]
    NotFound { name: String },

    /// Tool execution failed.
    #[error("Tool execution failed [{tool}]: {message}")]
    ExecutionFailed { tool: String, message: String },

    /// A cycle was detected in the tool graph.
    #[error("Cycle detected in tool graph: {details}")]
    CycleDetected { details: String },

    /// Tool graph execution timed out.
    #[error("Tool graph execution timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Input validation failed.
    #[error("Tool input validation error [{tool}]: {message}")]
    InputValidation { tool: String, message: String },

    /// A dependency node failed, blocking this node.
    #[error("Dependency failed: node {node_id} depends on failed node {dependency_id}")]
    DependencyFailed {
        node_id: String,
        dependency_id: String,
    },

    /// Generic internal error.
    #[error("Tool error: {message}")]
    Internal { message: String },
}

impl ToolError {
    pub fn not_found(name: impl fmt::Display) -> Self {
        Self::NotFound {
            name: name.to_string(),
        }
    }

    pub fn execution_failed(tool: impl fmt::Display, msg: impl fmt::Display) -> Self {
        Self::ExecutionFailed {
            tool: tool.to_string(),
            message: msg.to_string(),
        }
    }

    pub fn cycle_detected(details: impl fmt::Display) -> Self {
        Self::CycleDetected {
            details: details.to_string(),
        }
    }

    pub fn input_validation(tool: impl fmt::Display, msg: impl fmt::Display) -> Self {
        Self::InputValidation {
            tool: tool.to_string(),
            message: msg.to_string(),
        }
    }

    pub fn internal(msg: impl fmt::Display) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
}
