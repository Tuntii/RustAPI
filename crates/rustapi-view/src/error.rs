//! View error types

use thiserror::Error;

/// Error type for view/template operations
#[derive(Error, Debug)]
pub enum ViewError {
    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Template rendering failed
    #[error("Template rendering failed: {0}")]
    RenderError(String),

    /// Template parsing failed
    #[error("Template parsing failed: {0}")]
    ParseError(String),

    /// Context serialization failed
    #[error("Context serialization failed: {0}")]
    SerializationError(String),

    /// Template engine not initialized
    #[error("Template engine not initialized")]
    NotInitialized,

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Tera error
    #[error("Tera error: {0}")]
    Tera(#[from] tera::Error),
}

impl ViewError {
    /// Create a template not found error
    pub fn not_found(template: impl Into<String>) -> Self {
        Self::TemplateNotFound(template.into())
    }

    /// Create a render error
    pub fn render_error(msg: impl Into<String>) -> Self {
        Self::RenderError(msg.into())
    }

    /// Create a parse error
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }

    /// Create a serialization error
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }
}

impl From<ViewError> for rustapi_core::ApiError {
    fn from(err: ViewError) -> Self {
        match err {
            ViewError::TemplateNotFound(name) => {
                rustapi_core::ApiError::internal(format!("Template not found: {}", name))
            }
            ViewError::NotInitialized => {
                rustapi_core::ApiError::internal("Template engine not initialized")
            }
            _ => rustapi_core::ApiError::internal(err.to_string()),
        }
    }
}
