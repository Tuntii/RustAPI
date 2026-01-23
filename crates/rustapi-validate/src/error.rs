//! Validation error types and JSON error format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fmt;

/// Trait for translating validation errors.
pub trait Translator {
    /// Translate a validation error message.
    ///
    /// # Arguments
    ///
    /// * `code` - The validation rule code (e.g., "email", "length")
    /// * `field` - The field name
    /// * `params` - Optional parameters for the validation rule
    fn translate(
        &self,
        code: &str,
        field: &str,
        params: Option<&HashMap<String, serde_json::Value>>,
    ) -> Option<String>;
}

/// A single field validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// The field name that failed validation
    pub field: String,
    /// The validation rule code (e.g., "email", "length", "range")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional additional parameters (e.g., min/max values)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

impl FieldError {
    /// Create a new field error.
    pub fn new(
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
            message: message.into(),
            params: None,
        }
    }

    /// Create a field error with parameters.
    pub fn with_params(
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        params: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
            message: message.into(),
            params: Some(params),
        }
    }
}

/// Internal error structure for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorBody {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
    fields: Vec<FieldError>,
}

/// Wrapper for the error response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorWrapper {
    error: ErrorBody,
}

/// Validation error containing all field errors.
///
/// This type serializes to the standard RustAPI error format:
///
/// ```json
/// {
///   "error": {
///     "type": "validation_error",
///     "message": "Validation failed",
///     "fields": [...]
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Collection of field-level validation errors
    pub fields: Vec<FieldError>,
    /// Custom error message (default: "Validation failed")
    pub message: String,
}

impl ValidationError {
    /// Create a new validation error with field errors.
    pub fn new(fields: Vec<FieldError>) -> Self {
        Self {
            fields,
            message: "Validation failed".to_string(),
        }
    }

    /// Create a validation error with a custom message.
    pub fn with_message(fields: Vec<FieldError>, message: impl Into<String>) -> Self {
        Self {
            fields,
            message: message.into(),
        }
    }

    /// Create a validation error for a single field.
    pub fn field(
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(vec![FieldError::new(field, code, message)])
    }

    /// Check if there are any validation errors.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get the number of field errors.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Add a field error.
    pub fn add(&mut self, error: FieldError) {
        self.fields.push(error);
    }

    /// Localize validation errors using a translator.
    pub fn localize<T: Translator>(&self, translator: &T) -> Self {
        let fields = self
            .fields
            .iter()
            .map(|f| {
                let message = translator
                    .translate(&f.code, &f.field, f.params.as_ref())
                    .unwrap_or_else(|| f.message.clone());

                FieldError {
                    field: f.field.clone(),
                    code: f.code.clone(),
                    message,
                    params: f.params.clone(),
                }
            })
            .collect();

        Self {
            fields,
            message: self.message.clone(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} field error(s)", self.message, self.fields.len())
    }
}

impl std::error::Error for ValidationError {}

impl Serialize for ValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wrapper = ErrorWrapper {
            error: ErrorBody {
                error_type: "validation_error".to_string(),
                message: self.message.clone(),
                fields: self.fields.clone(),
            },
        };
        wrapper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ValidationError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wrapper = ErrorWrapper::deserialize(deserializer)?;
        Ok(Self {
            fields: wrapper.error.fields,
            message: wrapper.error.message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_error_creation() {
        let error = FieldError::new("email", "email", "Invalid email format");
        assert_eq!(error.field, "email");
        assert_eq!(error.code, "email");
        assert_eq!(error.message, "Invalid email format");
        assert!(error.params.is_none());
    }

    #[test]
    fn validation_error_serialization() {
        let error = ValidationError::new(vec![FieldError::new(
            "email",
            "email",
            "Invalid email format",
        )]);

        let json = serde_json::to_value(&error).unwrap();

        assert_eq!(json["error"]["type"], "validation_error");
        assert_eq!(json["error"]["message"], "Validation failed");
        assert_eq!(json["error"]["fields"][0]["field"], "email");
    }

    #[test]
    fn validation_error_display() {
        let error = ValidationError::new(vec![
            FieldError::new("email", "email", "Invalid email"),
            FieldError::new("age", "range", "Out of range"),
        ]);

        assert_eq!(error.to_string(), "Validation failed: 2 field error(s)");
    }
}
