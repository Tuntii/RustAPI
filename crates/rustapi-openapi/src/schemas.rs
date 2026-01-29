//! Standard error schemas for OpenAPI documentation
//!
//! These schemas match the error response format used by RustAPI.
//! 
//! Both native `ToOpenApiSchema` and optional `utoipa::ToSchema` implementations
//! are provided for maximum flexibility.

use crate::native::ToOpenApiSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;

// Conditionally derive utoipa::ToSchema when the feature is enabled
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Standard error response body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ErrorSchema {
    /// The error details
    pub error: ErrorBodySchema,
    /// Optional request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Error body details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct FieldErrorSchema {
    /// Field name (supports nested paths like "address.city")
    pub field: String,
    /// Error code (e.g., "email", "length", "required")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Validation error response (422)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ValidationErrorSchema {
    /// Error wrapper
    pub error: ValidationErrorBodySchema,
}

/// Validation error body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ValidationErrorBodySchema {
    /// Always "validation_error" for validation errors
    #[serde(rename = "type")]
    pub error_type: String,
    /// Always "Request validation failed"
    pub message: String,
    /// List of field-level errors
    pub fields: Vec<FieldErrorSchema>,
}

// ============================================================================
// Native ToOpenApiSchema implementations
// ============================================================================

impl ToOpenApiSchema for FieldErrorSchema {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "FieldError".into(),
            serde_json::json!({
                "type": "object",
                "description": "Field-level validation error",
                "properties": {
                    "field": {
                        "type": "string",
                        "description": "Field name (supports nested paths like 'address.city')"
                    },
                    "code": {
                        "type": "string",
                        "description": "Error code (e.g., 'email', 'length', 'required')"
                    },
                    "message": {
                        "type": "string",
                        "description": "Human-readable error message"
                    }
                },
                "required": ["field", "code", "message"]
            }),
        )
    }
}

impl ToOpenApiSchema for ErrorBodySchema {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "ErrorBody".into(),
            serde_json::json!({
                "type": "object",
                "description": "Error body details",
                "properties": {
                    "type": {
                        "type": "string",
                        "description": "Error type identifier (e.g., 'validation_error', 'not_found')"
                    },
                    "message": {
                        "type": "string",
                        "description": "Human-readable error message"
                    },
                    "fields": {
                        "type": "array",
                        "nullable": true,
                        "description": "Field-level errors (for validation errors)",
                        "items": { "$ref": "#/components/schemas/FieldError" }
                    }
                },
                "required": ["type", "message"]
            }),
        )
    }
}

impl ToOpenApiSchema for ErrorSchema {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "Error".into(),
            serde_json::json!({
                "type": "object",
                "description": "Standard error response body",
                "properties": {
                    "error": { "$ref": "#/components/schemas/ErrorBody" },
                    "request_id": {
                        "type": "string",
                        "nullable": true,
                        "description": "Optional request ID for tracing"
                    }
                },
                "required": ["error"]
            }),
        )
    }
}

impl ToOpenApiSchema for ValidationErrorBodySchema {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "ValidationErrorBody".into(),
            serde_json::json!({
                "type": "object",
                "description": "Validation error body",
                "properties": {
                    "type": {
                        "type": "string",
                        "description": "Always 'validation_error' for validation errors",
                        "example": "validation_error"
                    },
                    "message": {
                        "type": "string",
                        "description": "Always 'Request validation failed'",
                        "example": "Request validation failed"
                    },
                    "fields": {
                        "type": "array",
                        "description": "List of field-level errors",
                        "items": { "$ref": "#/components/schemas/FieldError" }
                    }
                },
                "required": ["type", "message", "fields"]
            }),
        )
    }
}

impl ToOpenApiSchema for ValidationErrorSchema {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "ValidationError".into(),
            serde_json::json!({
                "type": "object",
                "description": "Validation error response (422)",
                "properties": {
                    "error": { "$ref": "#/components/schemas/ValidationErrorBody" }
                },
                "required": ["error"]
            }),
        )
    }
}

// ============================================================================
// Helper methods
// ============================================================================

impl ValidationErrorSchema {
    /// Create a sample validation error for documentation
    pub fn example() -> Self {
        Self {
            error: ValidationErrorBodySchema {
                error_type: "validation_error".to_string(),
                message: "Request validation failed".to_string(),
                fields: vec![FieldErrorSchema {
                    field: "email".to_string(),
                    code: "email".to_string(),
                    message: "Invalid email format".to_string(),
                }],
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
