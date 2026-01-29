//! Standard error schemas for OpenAPI documentation
//!
//! These schemas match the error response format used by RustAPI.

use serde::{Deserialize, Serialize};
// use crate::ToSchema; // TODO: Re-enable once macro is implemented in rustapi-macros

/// Standard error response body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSchema {
    /// The error details
    pub error: ErrorBodySchema,
    /// Optional request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Error body details
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ValidationErrorSchema {
    /// Error wrapper
    pub error: ValidationErrorBodySchema,
}

/// Validation error body
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Manual ToSchema implementations

impl crate::schema::ToSchema for ErrorSchema {
    fn name() -> String {
        "ErrorSchema".to_string()
    }

    fn schema() -> (String, crate::schema::RefOr<crate::schema::Schema>) {
        use crate::schema::{Schema, SchemaType};
        let mut props = std::collections::HashMap::new();
        props.insert(
            "error".to_string(),
            <ErrorBodySchema as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "request_id".to_string(),
            <Option<String> as crate::schema::ToSchema>::schema().1,
        );

        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Object),
                description: Some("Standard error response body".to_string()),
                properties: Some(props),
                required: Some(vec!["error".to_string()]),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl crate::schema::ToSchema for ErrorBodySchema {
    fn name() -> String {
        "ErrorBodySchema".to_string()
    }

    fn schema() -> (String, crate::schema::RefOr<crate::schema::Schema>) {
        use crate::schema::{Schema, SchemaType};
        let mut props = std::collections::HashMap::new();
        props.insert(
            "type".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "message".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "fields".to_string(),
            <Option<Vec<FieldErrorSchema>> as crate::schema::ToSchema>::schema().1,
        );

        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Object),
                description: Some("Error body details".to_string()),
                properties: Some(props),
                required: Some(vec!["type".to_string(), "message".to_string()]),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl crate::schema::ToSchema for FieldErrorSchema {
    fn name() -> String {
        "FieldErrorSchema".to_string()
    }

    fn schema() -> (String, crate::schema::RefOr<crate::schema::Schema>) {
        use crate::schema::{Schema, SchemaType};
        let mut props = std::collections::HashMap::new();
        props.insert(
            "field".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "code".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "message".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );

        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Object),
                description: Some("Field-level validation error".to_string()),
                properties: Some(props),
                required: Some(vec![
                    "field".to_string(),
                    "code".to_string(),
                    "message".to_string(),
                ]),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl crate::schema::ToSchema for ValidationErrorSchema {
    fn name() -> String {
        "ValidationErrorSchema".to_string()
    }

    fn schema() -> (String, crate::schema::RefOr<crate::schema::Schema>) {
        use crate::schema::{Schema, SchemaType};
        let mut props = std::collections::HashMap::new();
        props.insert(
            "error".to_string(),
            <ValidationErrorBodySchema as crate::schema::ToSchema>::schema().1,
        );

        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Object),
                description: Some("Validation error response".to_string()),
                properties: Some(props),
                required: Some(vec!["error".to_string()]),
                ..Default::default()
            }
            .into(),
        )
    }
}

impl crate::schema::ToSchema for ValidationErrorBodySchema {
    fn name() -> String {
        "ValidationErrorBodySchema".to_string()
    }

    fn schema() -> (String, crate::schema::RefOr<crate::schema::Schema>) {
        use crate::schema::{Schema, SchemaType};
        let mut props = std::collections::HashMap::new();
        props.insert(
            "type".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "message".to_string(),
            <String as crate::schema::ToSchema>::schema().1,
        );
        props.insert(
            "fields".to_string(),
            <Vec<FieldErrorSchema> as crate::schema::ToSchema>::schema().1,
        );

        (
            Self::name(),
            Schema {
                schema_type: Some(SchemaType::Object),
                description: Some("Validation error body".to_string()),
                properties: Some(props),
                required: Some(vec![
                    "type".to_string(),
                    "message".to_string(),
                    "fields".to_string(),
                ]),
                ..Default::default()
            }
            .into(),
        )
    }
}
