//! View error types

use std::fmt;

/// Error type for view/template operations
#[derive(Debug)]
pub enum ViewError {
    /// Template not found
    TemplateNotFound(String),
    /// Template rendering failed
    RenderError(String),
    /// Template parsing failed
    ParseError(String),
    /// Context serialization failed
    SerializationError(String),
    /// Template engine not initialized
    NotInitialized,
    /// IO error
    IoError(std::io::Error),
    /// Tera error
    Tera(tera::Error),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemplateNotFound(name) => write!(f, "Template not found: {}", name),
            Self::RenderError(msg) => write!(f, "Template rendering failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Template parsing failed: {}", msg),
            Self::SerializationError(msg) => write!(f, "Context serialization failed: {}", msg),
            Self::NotInitialized => write!(f, "Template engine not initialized"),
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::Tera(e) => write!(f, "Tera error: {}", e),
        }
    }
}

impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::Tera(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ViewError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<tera::Error> for ViewError {
    fn from(e: tera::Error) -> Self {
        Self::Tera(e)
    }
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
