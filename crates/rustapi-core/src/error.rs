//! Error types for RustAPI

use http::StatusCode;
use serde::Serialize;
use std::fmt;

/// Result type alias for RustAPI operations
pub type Result<T, E = ApiError> = std::result::Result<T, E>;

/// Standard API error type
///
/// Provides structured error responses following a consistent JSON format.
#[derive(Debug, Clone)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error type identifier
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Optional field-level validation errors
    pub fields: Option<Vec<FieldError>>,
    /// Internal details (hidden in production)
    pub(crate) internal: Option<String>,
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    /// Field name (supports nested: "address.city")
    pub field: String,
    /// Error code (e.g., "email", "length", "required")
    pub code: String,
    /// Human-readable message
    pub message: String,
}

impl ApiError {
    /// Create a new API error
    pub fn new(status: StatusCode, error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            error_type: error_type.into(),
            message: message.into(),
            fields: None,
            internal: None,
        }
    }

    /// Create a validation error with field details
    pub fn validation(fields: Vec<FieldError>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error_type: "validation_error".to_string(),
            message: "Request validation failed".to_string(),
            fields: Some(fields),
            internal: None,
        }
    }

    /// Create a 400 Bad Request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    /// Create a 401 Unauthorized error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    /// Create a 403 Forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// Create a 404 Not Found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    /// Create a 409 Conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// Create a 500 Internal Server Error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }

    /// Add internal details (for logging, hidden from response in prod)
    pub fn with_internal(mut self, details: impl Into<String>) -> Self {
        self.internal = Some(details.into());
        self
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for ApiError {}

/// JSON representation of API error response
#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    pub error: ErrorBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldError>>,
}

impl From<ApiError> for ErrorResponse {
    fn from(err: ApiError) -> Self {
        Self {
            error: ErrorBody {
                error_type: err.error_type,
                message: err.message,
                fields: err.fields,
            },
            request_id: None, // TODO: inject from request context
        }
    }
}

// Conversion from common error types
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::bad_request(format!("Invalid JSON: {}", err))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::internal("I/O error").with_internal(err.to_string())
    }
}

impl From<hyper::Error> for ApiError {
    fn from(err: hyper::Error) -> Self {
        ApiError::internal("HTTP error").with_internal(err.to_string())
    }
}

impl From<rustapi_validate::ValidationError> for ApiError {
    fn from(err: rustapi_validate::ValidationError) -> Self {
        let fields = err.fields.into_iter().map(|f| FieldError {
            field: f.field,
            code: f.code,
            message: f.message,
        }).collect();
        
        ApiError::validation(fields)
    }
}

impl ApiError {
    /// Create a validation error from a ValidationError
    pub fn from_validation_error(err: rustapi_validate::ValidationError) -> Self {
        err.into()
    }
}

