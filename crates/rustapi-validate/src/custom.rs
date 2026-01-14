use crate::error::{FieldError, ValidationError};
use std::collections::HashMap;

/// A validation rule trait.
pub trait Rule<T: ?Sized> {
    /// Validate the value.
    fn validate(&self, value: &T) -> Result<(), FieldError>;
}

/// Type alias for validation rule functions to reduce complexity.
type ValidationRuleFn<T> = Box<dyn Fn(&T) -> Result<(), FieldError> + Send + Sync>;

/// A functional validator builder.
pub struct Validator<T> {
    rules: Vec<ValidationRuleFn<T>>,
}

impl<T> Default for Validator<T> {
    fn default() -> Self {
        Self { rules: Vec::new() }
    }
}

impl<T> Validator<T> {
    /// Create a new validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validation rule.
    pub fn rule<F>(mut self, rule: F) -> Self
    where
        F: Fn(&T) -> Result<(), FieldError> + Send + Sync + 'static,
    {
        self.rules.push(Box::new(rule));
        self
    }

    /// Validate the value.
    pub fn validate(&self, value: &T) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        for rule in &self.rules {
            if let Err(e) = rule(value) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::new(errors))
        }
    }
}

/// Common validation rules.
pub mod rules {
    use super::*;

    /// Create a required field rule (not null/empty).
    pub fn required<T: AsRef<str>>(
        message: impl Into<String>,
    ) -> impl Fn(&Option<T>) -> Result<(), FieldError> {
        let message = message.into();
        move |value: &Option<T>| match value {
            Some(v) if !v.as_ref().is_empty() => Ok(()),
            _ => Err(FieldError::new("required", "required", message.clone())),
        }
    }

    /// Create a string length rule.
    pub fn length<T: AsRef<str> + ?Sized>(
        min: Option<usize>,
        max: Option<usize>,
        message: impl Into<String>,
    ) -> impl Fn(&T) -> Result<(), FieldError> {
        let message = message.into();
        move |value: &T| {
            let value = value.as_ref();
            let len = value.len();
            if let Some(min) = min {
                if len < min {
                    let mut params = HashMap::new();
                    params.insert("min".to_string(), serde_json::json!(min));
                    params.insert("max".to_string(), serde_json::json!(max));
                    params.insert("value".to_string(), serde_json::json!(len));
                    return Err(FieldError::with_params(
                        "length",
                        "length",
                        message.clone(),
                        params,
                    ));
                }
            }
            if let Some(max) = max {
                if len > max {
                    let mut params = HashMap::new();
                    params.insert("min".to_string(), serde_json::json!(min));
                    params.insert("max".to_string(), serde_json::json!(max));
                    params.insert("value".to_string(), serde_json::json!(len));
                    return Err(FieldError::with_params(
                        "length",
                        "length",
                        message.clone(),
                        params,
                    ));
                }
            }
            Ok(())
        }
    }

    /// Create a numeric range rule.
    pub fn range<T: PartialOrd + Copy + serde::Serialize>(
        min: Option<T>,
        max: Option<T>,
        message: impl Into<String>,
    ) -> impl Fn(&T) -> Result<(), FieldError> {
        let message = message.into();
        move |value: &T| {
            if let Some(min) = min {
                if *value < min {
                    let mut params = HashMap::new();
                    params.insert("min".to_string(), serde_json::json!(min));
                    params.insert("max".to_string(), serde_json::json!(max));
                    params.insert("value".to_string(), serde_json::json!(value));
                    return Err(FieldError::with_params(
                        "range",
                        "range",
                        message.clone(),
                        params,
                    ));
                }
            }
            if let Some(max) = max {
                if *value > max {
                    let mut params = HashMap::new();
                    params.insert("min".to_string(), serde_json::json!(min));
                    params.insert("max".to_string(), serde_json::json!(max));
                    params.insert("value".to_string(), serde_json::json!(value));
                    return Err(FieldError::with_params(
                        "range",
                        "range",
                        message.clone(),
                        params,
                    ));
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_validator() {
        let validator = Validator::<String>::new().rule(rules::length(
            Some(3),
            Some(10),
            "Must be between 3 and 10 chars",
        ));

        assert!(validator.validate(&"ab".to_string()).is_err());
        assert!(validator.validate(&"abc".to_string()).is_ok());
        assert!(validator.validate(&"abcdefghijk".to_string()).is_err());
    }

    #[test]
    fn test_range_validator() {
        let validator = Validator::<i32>::new().rule(rules::range(
            Some(18),
            Some(100),
            "Must be between 18 and 100",
        ));

        assert!(validator.validate(&17).is_err());
        assert!(validator.validate(&18).is_ok());
        assert!(validator.validate(&100).is_ok());
        assert!(validator.validate(&101).is_err());
    }
}
