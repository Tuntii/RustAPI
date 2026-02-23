use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors produced by the LLM subsystem.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum LlmError {
    /// The provider returned an error.
    #[error("LLM provider error [{provider}]: {message}")]
    ProviderError { provider: String, message: String },

    /// All providers in the fallback chain failed.
    #[error("All LLM providers failed: {message}")]
    AllProvidersFailed { message: String },

    /// Rate limit hit.
    #[error("Rate limited by {provider}: retry after {retry_after_secs}s")]
    RateLimited {
        provider: String,
        retry_after_secs: u64,
    },

    /// Structured output could not be parsed.
    #[error("Structured output parse error: {message}")]
    StructuredOutputError { message: String },

    /// Request timeout.
    #[error("LLM request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Invalid configuration.
    #[error("LLM configuration error: {message}")]
    ConfigError { message: String },

    /// Generic internal error.
    #[error("LLM error: {message}")]
    Internal { message: String },
}

impl LlmError {
    pub fn provider(provider: impl fmt::Display, msg: impl fmt::Display) -> Self {
        Self::ProviderError {
            provider: provider.to_string(),
            message: msg.to_string(),
        }
    }

    pub fn all_failed(msg: impl fmt::Display) -> Self {
        Self::AllProvidersFailed {
            message: msg.to_string(),
        }
    }

    pub fn structured_output(msg: impl fmt::Display) -> Self {
        Self::StructuredOutputError {
            message: msg.to_string(),
        }
    }

    pub fn config(msg: impl fmt::Display) -> Self {
        Self::ConfigError {
            message: msg.to_string(),
        }
    }

    pub fn internal(msg: impl fmt::Display) -> Self {
        Self::Internal {
            message: msg.to_string(),
        }
    }
}
