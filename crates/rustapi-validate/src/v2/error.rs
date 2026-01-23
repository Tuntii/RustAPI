//! Error types for the v2 validation engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Error from a single validation rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleError {
    /// The validation rule code (e.g., "email", "length", "range")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional parameters for message interpolation
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, serde_json::Value>,
}

impl RuleError {
    /// Create a new rule error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            params: HashMap::new(),
        }
    }

    /// Create a rule error with parameters.
    pub fn with_params(
        code: impl Into<String>,
        message: impl Into<String>,
        params: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            params,
        }
    }

    /// Add a parameter to the error.
    pub fn param(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.params.insert(key.into(), v);
        }
        self
    }

    /// Interpolate parameters into the message, optionally localized.
    ///
    /// If a locale is provided, attempts to translate the message key.
    /// Replaces `{param_name}` placeholders with actual values.
    pub fn interpolate_with_locale(&self, locale: Option<&str>) -> String {
        let msg = crate::v2::i18n::translate(&self.message, locale);
        let mut result = msg;

        for (key, value) in &self.params {
            let placeholder = format!("{{{}}}", key);
            let p_placeholder = format!("%{{{}}}", key); // Support ruby style %{param} often used in i18n

            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => value.to_string(),
            };
            // Replace both {param} and %{param}
            result = result.replace(&p_placeholder, &replacement);
            result = result.replace(&placeholder, &replacement);
        }
        result
    }

    /// Interpolate parameters into the message (uses default locale).
    pub fn interpolate_message(&self) -> String {
        self.interpolate_with_locale(None)
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.interpolate_message())
    }
}

impl std::error::Error for RuleError {}

/// Collection of validation errors for multiple fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// Map of field name to list of errors for that field
    #[serde(flatten)]
    pub fields: HashMap<String, Vec<RuleError>>,
}

impl ValidationErrors {
    /// Create an empty validation errors collection.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Add an error for a field.
    pub fn add(&mut self, field: impl Into<String>, error: RuleError) {
        self.fields.entry(field.into()).or_default().push(error);
    }

    /// Add multiple errors for a field.
    pub fn add_all(&mut self, field: impl Into<String>, errors: Vec<RuleError>) {
        let field = field.into();
        for error in errors {
            self.add(field.clone(), error);
        }
    }

    /// Merge another ValidationErrors into this one.
    pub fn merge(&mut self, other: ValidationErrors) {
        for (field, errors) in other.fields {
            self.add_all(field, errors);
        }
    }

    /// Check if there are any errors.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get the total number of errors.
    pub fn len(&self) -> usize {
        self.fields.values().map(|v| v.len()).sum()
    }

    /// Get errors for a specific field.
    pub fn get(&self, field: &str) -> Option<&Vec<RuleError>> {
        self.fields.get(field)
    }

    /// Convert to Result - Ok if no errors, Err otherwise.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }

    /// Get all field names with errors.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }

    /// Convert to the standard RustAPI error format with localization.
    pub fn to_api_error_with_locale(&self, locale: Option<&str>) -> ApiValidationError {
        let fields: Vec<FieldErrorResponse> = self
            .fields
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| FieldErrorResponse {
                    field: field.clone(),
                    code: e.code.clone(),
                    message: e.interpolate_with_locale(locale),
                    params: if e.params.is_empty() {
                        None
                    } else {
                        Some(e.params.clone())
                    },
                })
            })
            .collect();

        ApiValidationError {
            error: ErrorBody {
                error_type: "validation_error".to_string(),
                message: "Validation failed".to_string(),
                fields,
            },
        }
    }

    /// Convert to the standard RustAPI error format.
    pub fn to_api_error(&self) -> ApiValidationError {
        self.to_api_error_with_locale(None)
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation failed: {} error(s)", self.len())
    }
}

impl std::error::Error for ValidationErrors {}

/// API response format for validation errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiValidationError {
    pub error: ErrorBody,
}

/// Error body in API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub fields: Vec<FieldErrorResponse>,
}

/// Single field error in API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldErrorResponse {
    pub field: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_error_creation() {
        let error = RuleError::new("email", "Invalid email format");
        assert_eq!(error.code, "email");
        assert_eq!(error.message, "Invalid email format");
        assert!(error.params.is_empty());
    }

    #[test]
    fn rule_error_with_params() {
        let error = RuleError::new("length", "Must be between {min} and {max} characters")
            .param("min", 3)
            .param("max", 50);

        assert_eq!(
            error.interpolate_message(),
            "Must be between 3 and 50 characters"
        );
    }

    #[test]
    fn validation_errors_add_and_get() {
        let mut errors = ValidationErrors::new();
        errors.add("email", RuleError::new("email", "Invalid email"));
        errors.add("email", RuleError::new("required", "Email is required"));
        errors.add("age", RuleError::new("range", "Age out of range"));

        assert_eq!(errors.len(), 3);
        assert_eq!(errors.get("email").unwrap().len(), 2);
        assert_eq!(errors.get("age").unwrap().len(), 1);
    }

    #[test]
    fn validation_errors_into_result() {
        let errors = ValidationErrors::new();
        assert!(errors.into_result().is_ok());

        let mut errors = ValidationErrors::new();
        errors.add("field", RuleError::new("code", "message"));
        assert!(errors.into_result().is_err());
    }

    #[test]
    fn validation_errors_to_api_error() {
        let mut errors = ValidationErrors::new();
        errors.add("email", RuleError::new("email", "Invalid email format"));

        let api_error = errors.to_api_error();
        assert_eq!(api_error.error.error_type, "validation_error");
        assert_eq!(api_error.error.fields.len(), 1);
        assert_eq!(api_error.error.fields[0].field, "email");
    }

    #[test]
    fn validation_errors_merge() {
        let mut errors1 = ValidationErrors::new();
        errors1.add("email", RuleError::new("email", "Invalid"));

        let mut errors2 = ValidationErrors::new();
        errors2.add("age", RuleError::new("range", "Out of range"));

        errors1.merge(errors2);
        assert_eq!(errors1.len(), 2);
        assert!(errors1.get("email").is_some());
        assert!(errors1.get("age").is_some());
    }
}
