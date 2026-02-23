use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors produced by the AI context subsystem.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ContextError {
    /// The cost budget for the current execution has been exceeded.
    #[error("Budget exceeded: {message}")]
    BudgetExceeded { message: String },

    /// A trace node could not be found by the given id.
    #[error("Trace node not found: {node_id}")]
    TraceNodeNotFound { node_id: String },

    /// An event could not be published.
    #[error("Event bus error: {message}")]
    EventBusError { message: String },

    /// Generic internal error.
    #[error("Context error: {message}")]
    Internal { message: String },
}

impl ContextError {
    pub fn budget_exceeded(msg: impl fmt::Display) -> Self {
        Self::BudgetExceeded {
            message: msg.to_string(),
        }
    }

    pub fn trace_node_not_found(id: impl fmt::Display) -> Self {
        Self::TraceNodeNotFound {
            node_id: id.to_string(),
        }
    }

    pub fn event_bus(msg: impl fmt::Display) -> Self {
        Self::EventBusError {
            message: msg.to_string(),
        }
    }

    pub fn internal(msg: impl fmt::Display) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
}
