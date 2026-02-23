use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors produced by the memory subsystem.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum MemoryError {
    /// Entry was not found.
    #[error("Memory entry not found: {key}")]
    NotFound { key: String },

    /// A backend-specific error occurred.
    #[error("Memory backend error: {message}")]
    BackendError { message: String },

    /// Serialization / deserialization failure.
    #[error("Memory serialization error: {message}")]
    SerializationError { message: String },

    /// The store has reached its capacity limit.
    #[error("Memory store capacity exceeded")]
    CapacityExceeded,

    /// Generic internal error.
    #[error("Memory error: {message}")]
    Internal { message: String },
}

impl MemoryError {
    pub fn not_found(key: impl fmt::Display) -> Self {
        Self::NotFound {
            key: key.to_string(),
        }
    }

    pub fn backend(msg: impl fmt::Display) -> Self {
        Self::BackendError {
            message: msg.to_string(),
        }
    }

    pub fn serialization(msg: impl fmt::Display) -> Self {
        Self::SerializationError {
            message: msg.to_string(),
        }
    }

    pub fn internal(msg: impl fmt::Display) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
}
