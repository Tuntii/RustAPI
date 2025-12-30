//! Standard error schemas for OpenAPI documentation
//!
//! These schemas match the error response format used by RustAPI.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard error response body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorSchema {
    /// The error details
    pub error: ErrorBodySchema,
    /// Optional request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Error body details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorBodySchema {
    /// Error type identifier (e.g., "validation_error", "not_found")
    #[serde(rename = "type")]
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Field-level errors (for validation errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldErrorSchema>>,
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldErrorSchema {
    /// Field name (supports nested paths like "address.city")
    pub field: String,
    /// Error code (e.g., "email", "length", "required")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Validation error response (422)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorSchema {
    /// Error wrapper
    pub error: ValidationErrorBodySchema,
}

/// Validation error body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorBodySchema {
    /// Always "validation_error" for validation errors
    #[serde(rename = "type")]
    pub error_type: String,
    /// Always "Request validation failed"
    pub message: String,
    /// List of field-level errors
    pub fields: Vec<FieldErrorSchema>,
}

impl ValidationErrorSchema {
    /// Create a sample validation error for documentation
    pub fn example() -> Self {
        Self {
            error: ValidationErrorBodySchema {
                error_type: "validation_error".to_string(),
                message: "Request validation failed".to_string(),
                fields: vec![
                    FieldErrorSchema {
                        field: "email".to_string(),
                        code: "email".to_string(),
                        message: "Invalid email format".to_string(),
                    },
                ],
            },
        }
    }
}

impl ErrorSchema {
    /// Create a sample not found error
    pub fn not_found_example() -> Self {
        Self {
            error: ErrorBodySchema {
                error_type: "not_found".to_string(),
                message: "Resource not found".to_string(),
                fields: None,
            },
            request_id: None,
        }
    }
    
    /// Create a sample internal error
    pub fn internal_error_example() -> Self {
        Self {
            error: ErrorBodySchema {
                error_type: "internal_error".to_string(),
                message: "An internal error occurred".to_string(),
                fields: None,
            },
            request_id: None,
        }
    }
    
    /// Create a sample bad request error
    pub fn bad_request_example() -> Self {
        Self {
            error: ErrorBodySchema {
                error_type: "bad_request".to_string(),
                message: "Invalid request".to_string(),
                fields: None,
            },
            request_id: None,
        }
    }
}
