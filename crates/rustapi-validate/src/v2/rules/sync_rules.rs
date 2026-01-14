//! Synchronous validation rules.
//!
//! These rules perform validation without requiring async operations.

use crate::v2::error::RuleError;
use crate::v2::traits::ValidationRule;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// Pre-compiled regex patterns
static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
static URL_REGEX: OnceLock<Regex> = OnceLock::new();

fn email_regex() -> &'static Regex {
    EMAIL_REGEX.get_or_init(|| {
        // RFC 5322 simplified email regex
        Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).unwrap()
    })
}

fn url_regex() -> &'static Regex {
    URL_REGEX.get_or_init(|| Regex::new(r"^(https?|ftp)://[^\s/$.?#].[^\s]*$").unwrap())
}

/// Email format validation rule.
///
/// Validates that a string is a valid email address according to RFC 5322.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EmailRule {
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl EmailRule {
    /// Create a new email rule with default message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an email rule with a custom message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}

impl ValidationRule<str> for EmailRule {
    fn validate(&self, value: &str) -> Result<(), RuleError> {
        if email_regex().is_match(value) {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| "Invalid email format".to_string());
            Err(RuleError::new("email", message))
        }
    }

    fn rule_name(&self) -> &'static str {
        "email"
    }
}

impl ValidationRule<String> for EmailRule {
    fn validate(&self, value: &String) -> Result<(), RuleError> {
        <Self as ValidationRule<str>>::validate(self, value.as_str())
    }

    fn rule_name(&self) -> &'static str {
        "email"
    }
}

/// String length validation rule.
///
/// Validates that a string's length is within specified bounds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LengthRule {
    /// Minimum length (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<usize>,
    /// Maximum length (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LengthRule {
    /// Create a length rule with min and max bounds.
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            message: None,
        }
    }

    /// Create a length rule with only a minimum.
    pub fn min(min: usize) -> Self {
        Self {
            min: Some(min),
            max: None,
            message: None,
        }
    }

    /// Create a length rule with only a maximum.
    pub fn max(max: usize) -> Self {
        Self {
            min: None,
            max: Some(max),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl ValidationRule<str> for LengthRule {
    fn validate(&self, value: &str) -> Result<(), RuleError> {
        let len = value.chars().count();

        if let Some(min) = self.min {
            if len < min {
                let message = self
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Length must be at least {min} characters"));
                return Err(RuleError::new("length", message)
                    .param("min", min)
                    .param("max", self.max)
                    .param("actual", len));
            }
        }

        if let Some(max) = self.max {
            if len > max {
                let message = self
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Length must be at most {max} characters"));
                return Err(RuleError::new("length", message)
                    .param("min", self.min)
                    .param("max", max)
                    .param("actual", len));
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "length"
    }
}

impl ValidationRule<String> for LengthRule {
    fn validate(&self, value: &String) -> Result<(), RuleError> {
        <Self as ValidationRule<str>>::validate(self, value.as_str())
    }

    fn rule_name(&self) -> &'static str {
        "length"
    }
}

/// Numeric range validation rule.
///
/// Validates that a number is within specified bounds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RangeRule<T> {
    /// Minimum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    /// Maximum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> RangeRule<T> {
    /// Create a range rule with min and max bounds.
    pub fn new(min: T, max: T) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            message: None,
        }
    }

    /// Create a range rule with only a minimum.
    pub fn min(min: T) -> Self {
        Self {
            min: Some(min),
            max: None,
            message: None,
        }
    }

    /// Create a range rule with only a maximum.
    pub fn max(max: T) -> Self {
        Self {
            min: None,
            max: Some(max),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl<T> ValidationRule<T> for RangeRule<T>
where
    T: PartialOrd + std::fmt::Display + Copy + Send + Sync + std::fmt::Debug + Serialize,
{
    fn validate(&self, value: &T) -> Result<(), RuleError> {
        if let Some(ref min) = self.min {
            if value < min {
                let message = self
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Value must be at least {min}"));
                return Err(RuleError::new("range", message)
                    .param("min", *min)
                    .param("max", self.max)
                    .param("actual", *value));
            }
        }

        if let Some(ref max) = self.max {
            if value > max {
                let message = self
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Value must be at most {max}"));
                return Err(RuleError::new("range", message)
                    .param("min", self.min)
                    .param("max", *max)
                    .param("actual", *value));
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "range"
    }
}

/// Regex pattern validation rule.
///
/// Validates that a string matches a regex pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexRule {
    /// The regex pattern
    pub pattern: String,
    /// Compiled regex (not serialized)
    #[serde(skip)]
    compiled: OnceLock<Regex>,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl PartialEq for RegexRule {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.message == other.message
    }
}

impl RegexRule {
    /// Create a new regex rule.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            compiled: OnceLock::new(),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    fn get_regex(&self) -> Result<&Regex, RuleError> {
        self.compiled.get_or_init(|| {
            Regex::new(&self.pattern).unwrap_or_else(|_| Regex::new("^$").unwrap())
        });

        // Verify the pattern is valid
        if Regex::new(&self.pattern).is_err() {
            return Err(RuleError::new(
                "regex",
                format!("Invalid regex pattern: {}", self.pattern),
            ));
        }

        Ok(self.compiled.get().unwrap())
    }
}

impl ValidationRule<str> for RegexRule {
    fn validate(&self, value: &str) -> Result<(), RuleError> {
        let regex = self.get_regex()?;

        if regex.is_match(value) {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| format!("Value does not match pattern: {}", self.pattern));
            Err(RuleError::new("regex", message).param("pattern", self.pattern.clone()))
        }
    }

    fn rule_name(&self) -> &'static str {
        "regex"
    }
}

impl ValidationRule<String> for RegexRule {
    fn validate(&self, value: &String) -> Result<(), RuleError> {
        <Self as ValidationRule<str>>::validate(self, value.as_str())
    }

    fn rule_name(&self) -> &'static str {
        "regex"
    }
}

/// URL format validation rule.
///
/// Validates that a string is a valid URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UrlRule {
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl UrlRule {
    /// Create a new URL rule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a URL rule with a custom message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}

impl ValidationRule<str> for UrlRule {
    fn validate(&self, value: &str) -> Result<(), RuleError> {
        if url_regex().is_match(value) {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| "Invalid URL format".to_string());
            Err(RuleError::new("url", message))
        }
    }

    fn rule_name(&self) -> &'static str {
        "url"
    }
}

impl ValidationRule<String> for UrlRule {
    fn validate(&self, value: &String) -> Result<(), RuleError> {
        <Self as ValidationRule<str>>::validate(self, value.as_str())
    }

    fn rule_name(&self) -> &'static str {
        "url"
    }
}

/// Required (non-empty) validation rule.
///
/// Validates that a value is not empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RequiredRule {
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl RequiredRule {
    /// Create a new required rule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a required rule with a custom message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }
}

impl ValidationRule<str> for RequiredRule {
    fn validate(&self, value: &str) -> Result<(), RuleError> {
        if !value.trim().is_empty() {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| "This field is required".to_string());
            Err(RuleError::new("required", message))
        }
    }

    fn rule_name(&self) -> &'static str {
        "required"
    }
}

impl ValidationRule<String> for RequiredRule {
    fn validate(&self, value: &String) -> Result<(), RuleError> {
        <Self as ValidationRule<str>>::validate(self, value.as_str())
    }

    fn rule_name(&self) -> &'static str {
        "required"
    }
}

impl<T> ValidationRule<Option<T>> for RequiredRule
where
    T: std::fmt::Debug + Send + Sync,
{
    fn validate(&self, value: &Option<T>) -> Result<(), RuleError> {
        if value.is_some() {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| "This field is required".to_string());
            Err(RuleError::new("required", message))
        }
    }

    fn rule_name(&self) -> &'static str {
        "required"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_rule_valid() {
        let rule = EmailRule::new();
        assert!(rule.validate("test@example.com").is_ok());
        assert!(rule.validate("user.name+tag@domain.co.uk").is_ok());
    }

    #[test]
    fn email_rule_invalid() {
        let rule = EmailRule::new();
        assert!(rule.validate("invalid").is_err());
        assert!(rule.validate("@domain.com").is_err());
        assert!(rule.validate("user@").is_err());
    }

    #[test]
    fn email_rule_custom_message() {
        let rule = EmailRule::with_message("Please enter a valid email");
        let err = rule.validate("invalid").unwrap_err();
        assert_eq!(err.message, "Please enter a valid email");
    }

    #[test]
    fn length_rule_valid() {
        let rule = LengthRule::new(3, 10);
        assert!(rule.validate("abc").is_ok());
        assert!(rule.validate("abcdefghij").is_ok());
    }

    #[test]
    fn length_rule_too_short() {
        let rule = LengthRule::new(3, 10);
        let err = rule.validate("ab").unwrap_err();
        assert_eq!(err.code, "length");
    }

    #[test]
    fn length_rule_too_long() {
        let rule = LengthRule::new(3, 10);
        let err = rule.validate("abcdefghijk").unwrap_err();
        assert_eq!(err.code, "length");
    }

    #[test]
    fn range_rule_valid() {
        let rule = RangeRule::new(18, 120);
        assert!(rule.validate(&18).is_ok());
        assert!(rule.validate(&50).is_ok());
        assert!(rule.validate(&120).is_ok());
    }

    #[test]
    fn range_rule_too_low() {
        let rule = RangeRule::new(18, 120);
        let err = rule.validate(&17).unwrap_err();
        assert_eq!(err.code, "range");
    }

    #[test]
    fn range_rule_too_high() {
        let rule = RangeRule::new(18, 120);
        let err = rule.validate(&121).unwrap_err();
        assert_eq!(err.code, "range");
    }

    #[test]
    fn regex_rule_valid() {
        let rule = RegexRule::new(r"^\d{3}-\d{4}$");
        assert!(rule.validate("123-4567").is_ok());
    }

    #[test]
    fn regex_rule_invalid() {
        let rule = RegexRule::new(r"^\d{3}-\d{4}$");
        assert!(rule.validate("1234567").is_err());
    }

    #[test]
    fn url_rule_valid() {
        let rule = UrlRule::new();
        assert!(rule.validate("https://example.com").is_ok());
        assert!(rule.validate("http://example.com/path?query=1").is_ok());
    }

    #[test]
    fn url_rule_invalid() {
        let rule = UrlRule::new();
        assert!(rule.validate("not-a-url").is_err());
        assert!(rule.validate("ftp://").is_err());
    }

    #[test]
    fn required_rule_valid() {
        let rule = RequiredRule::new();
        assert!(rule.validate("value").is_ok());
        assert!(rule.validate("  value  ").is_ok());
    }

    #[test]
    fn required_rule_empty() {
        let rule = RequiredRule::new();
        assert!(rule.validate("").is_err());
        assert!(rule.validate("   ").is_err());
    }

    #[test]
    fn required_rule_option() {
        let rule = RequiredRule::new();
        assert!(ValidationRule::<Option<i32>>::validate(&rule, &Some(42)).is_ok());
        assert!(ValidationRule::<Option<i32>>::validate(&rule, &None).is_err());
    }

    #[test]
    fn rule_serialization_roundtrip() {
        let rule = LengthRule::new(3, 50).with_message("Custom message");
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: LengthRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, parsed);
    }
}
