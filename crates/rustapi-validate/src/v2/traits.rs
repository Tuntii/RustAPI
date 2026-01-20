//! Core validation traits for the v2 validation engine.

use crate::v2::context::ValidationContext;
use crate::v2::error::{RuleError, ValidationErrors};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait for synchronous validation of a struct.
///
/// Implement this trait to enable validation on your types.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::v2::prelude::*;
///
/// struct User {
///     email: String,
///     age: u8,
/// }
///
/// impl Validate for User {
///     fn validate(&self) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         
///         if let Err(e) = EmailRule::default().validate(&self.email) {
///             errors.add("email", e);
///         }
///         
///         if let Err(e) = RangeRule::new(18, 120).validate(&self.age) {
///             errors.add("age", e);
///         }
///         
///         errors.into_result()
///     }
/// }
/// ```
pub trait Validate {
    /// Validate the struct synchronously with the default group.
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.validate_with_group(crate::v2::group::ValidationGroup::Default)
    }

    /// Validate the struct with a specific validation group.
    fn validate_with_group(
        &self,
        group: crate::v2::group::ValidationGroup,
    ) -> Result<(), ValidationErrors>;

    /// Validate and return the struct if valid.
    fn validated(self) -> Result<Self, ValidationErrors>
    where
        Self: Sized,
    {
        self.validate()?;
        Ok(self)
    }

    /// Validate and return the struct if valid (with group).
    fn validated_with_group(
        self,
        group: crate::v2::group::ValidationGroup,
    ) -> Result<Self, ValidationErrors>
    where
        Self: Sized,
    {
        self.validate_with_group(group)?;
        Ok(self)
    }
}

/// Trait for asynchronous validation of a struct.
///
/// Use this trait when validation requires async operations like database checks or API calls.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::v2::prelude::*;
///
/// struct CreateUser {
///     email: String,
/// }
///
/// #[async_trait]
/// impl AsyncValidate for CreateUser {
///     async fn validate_async_with_group(&self, ctx: &ValidationContext, group: ValidationGroup) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         
///         // Check email uniqueness in database
///         if let Some(db) = ctx.database() {
///             let rule = AsyncUniqueRule::new("users", "email");
///             if let Err(e) = rule.validate_async(&self.email, ctx).await {
///                 errors.add("email", e);
///             }
///         }
///         
///         errors.into_result()
///     }
/// }
/// ```
#[async_trait]
pub trait AsyncValidate: Validate + Send + Sync {
    /// Validate the struct asynchronously with the default group.
    async fn validate_async(&self, ctx: &ValidationContext) -> Result<(), ValidationErrors> {
        self.validate_async_with_group(ctx, crate::v2::group::ValidationGroup::Default)
            .await
    }

    /// Validate the struct asynchronously with a specific group.
    async fn validate_async_with_group(
        &self,
        ctx: &ValidationContext,
        group: crate::v2::group::ValidationGroup,
    ) -> Result<(), ValidationErrors>;

    /// Perform full validation (sync + async) with default group.
    async fn validate_full(&self, ctx: &ValidationContext) -> Result<(), ValidationErrors> {
        self.validate_full_with_group(ctx, crate::v2::group::ValidationGroup::Default)
            .await
    }

    /// Perform full validation (sync + async) with specific group.
    async fn validate_full_with_group(
        &self,
        ctx: &ValidationContext,
        group: crate::v2::group::ValidationGroup,
    ) -> Result<(), ValidationErrors> {
        // First run sync validation
        self.validate_with_group(group.clone())?;
        // Then run async validation
        self.validate_async_with_group(ctx, group).await
    }

    /// Validate and return the struct if valid (async version).
    async fn validated_async(self, ctx: &ValidationContext) -> Result<Self, ValidationErrors>
    where
        Self: Sized,
    {
        self.validate_full(ctx).await?;
        Ok(self)
    }

    /// Validate and return the struct if valid (async version with group).
    async fn validated_async_with_group(
        self,
        ctx: &ValidationContext,
        group: crate::v2::group::ValidationGroup,
    ) -> Result<Self, ValidationErrors>
    where
        Self: Sized,
    {
        self.validate_full_with_group(ctx, group).await?;
        Ok(self)
    }
}

/// Trait for individual validation rules.
///
/// Each rule validates a single value and returns a `RuleError` on failure.
/// Rules should be serializable for configuration and pretty-printing.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::v2::prelude::*;
///
/// struct PositiveRule;
///
/// impl ValidationRule<i32> for PositiveRule {
///     fn validate(&self, value: &i32) -> Result<(), RuleError> {
///         if *value > 0 {
///             Ok(())
///         } else {
///             Err(RuleError::new("positive", "Value must be positive"))
///         }
///     }
///     
///     fn rule_name(&self) -> &'static str {
///         "positive"
///     }
/// }
/// ```
pub trait ValidationRule<T: ?Sized>: Debug + Send + Sync {
    /// Validate the value against this rule.
    fn validate(&self, value: &T) -> Result<(), RuleError>;

    /// Get the rule name/code for error reporting.
    fn rule_name(&self) -> &'static str;

    /// Get the default error message for this rule.
    fn default_message(&self) -> String {
        format!("Validation failed for rule '{}'", self.rule_name())
    }
}

/// Trait for async validation rules.
///
/// Use this for rules that require async operations like database or API checks.
#[async_trait]
pub trait AsyncValidationRule<T: ?Sized + Sync>: Debug + Send + Sync {
    /// Validate the value asynchronously.
    async fn validate_async(&self, value: &T, ctx: &ValidationContext) -> Result<(), RuleError>;

    /// Get the rule name/code for error reporting.
    fn rule_name(&self) -> &'static str;

    /// Get the default error message for this rule.
    fn default_message(&self) -> String {
        format!("Async validation failed for rule '{}'", self.rule_name())
    }
}

/// Wrapper for serializable validation rules.
///
/// This enum allows rules to be serialized/deserialized for configuration files
/// and pretty-printing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableRule {
    /// Email format validation
    Email {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// String length validation
    Length {
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Numeric range validation
    Range {
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Regex pattern validation
    Regex {
        pattern: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// URL format validation
    Url {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Required (non-empty) validation
    Required {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Database uniqueness check (async)
    AsyncUnique {
        table: String,
        column: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Database existence check (async)
    AsyncExists {
        table: String,
        column: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// External API validation (async)
    AsyncApi {
        endpoint: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Credit Card validation
    CreditCard {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// IP Address validation
    Ip {
        #[serde(skip_serializing_if = "Option::is_none")]
        v4: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        v6: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Phone number validation
    Phone {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Contains substring validation
    Contains {
        needle: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Custom async validation function
    CustomAsync {
        function: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

impl SerializableRule {
    /// Pretty print the rule definition.
    pub fn pretty_print(&self) -> String {
        match self {
            SerializableRule::Email { message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(email{})]", msg)
            }
            SerializableRule::Length { min, max, message } => {
                let mut parts = Vec::new();
                if let Some(min) = min {
                    parts.push(format!("min = {}", min));
                }
                if let Some(max) = max {
                    parts.push(format!("max = {}", max));
                }
                if let Some(msg) = message {
                    parts.push(format!("message = \"{}\"", msg));
                }
                format!("#[validate(length({}))]", parts.join(", "))
            }
            SerializableRule::Range { min, max, message } => {
                let mut parts = Vec::new();
                if let Some(min) = min {
                    parts.push(format!("min = {}", min));
                }
                if let Some(max) = max {
                    parts.push(format!("max = {}", max));
                }
                if let Some(msg) = message {
                    parts.push(format!("message = \"{}\"", msg));
                }
                format!("#[validate(range({}))]", parts.join(", "))
            }
            SerializableRule::Regex { pattern, message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(regex = \"{}\"{})]", pattern, msg)
            }
            SerializableRule::Url { message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(url{})]", msg)
            }
            SerializableRule::Required { message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(required{})]", msg)
            }
            SerializableRule::AsyncUnique {
                table,
                column,
                message,
            } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!(
                    "#[validate(async_unique(table = \"{}\", column = \"{}\"{}))]",
                    table, column, msg
                )
            }
            SerializableRule::AsyncExists {
                table,
                column,
                message,
            } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!(
                    "#[validate(async_exists(table = \"{}\", column = \"{}\"{}))]",
                    table, column, msg
                )
            }
            SerializableRule::AsyncApi { endpoint, message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(async_api(endpoint = \"{}\"{}))]", endpoint, msg)
            }
            SerializableRule::CreditCard { message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(credit_card{})]", msg)
            }
            SerializableRule::Ip { v4, v6, message } => {
                let mut parts = Vec::new();
                if let Some(true) = v4 {
                    parts.push("v4".to_string());
                }
                if let Some(true) = v6 {
                    parts.push("v6".to_string());
                }
                if let Some(msg) = message {
                    parts.push(format!("message = \"{}\"", msg));
                }
                if parts.is_empty() {
                    "#[validate(ip)]".to_string()
                } else {
                    format!("#[validate(ip({}))]", parts.join(", "))
                }
            }
            SerializableRule::Phone { message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(phone{})]", msg)
            }
            SerializableRule::Contains { needle, message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(contains(needle = \"{}\"{}))]", needle, msg)
            }
            SerializableRule::CustomAsync { function, message } => {
                let msg = message
                    .as_ref()
                    .map(|m| format!(", message = \"{}\"", m))
                    .unwrap_or_default();
                format!("#[validate(custom_async = \"{}\"{})]", function, msg)
            }
        }
    }

    /// Parse a SerializableRule from a pretty-printed string.
    ///
    /// This is the inverse of `pretty_print()` and enables round-trip
    /// serialization of validation rules.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Must start with #[validate( and end with )]
        if !s.starts_with("#[validate(") || !s.ends_with(")]") {
            return None;
        }

        // Extract the inner content
        let inner = &s[11..s.len() - 2];

        // Parse based on rule type
        if inner == "email" || inner.starts_with("email,") {
            let message = Self::extract_message(inner);
            return Some(SerializableRule::Email { message });
        }

        if inner == "url" || inner.starts_with("url,") {
            let message = Self::extract_message(inner);
            return Some(SerializableRule::Url { message });
        }

        if inner == "required" || inner.starts_with("required,") {
            let message = Self::extract_message(inner);
            return Some(SerializableRule::Required { message });
        }

        if inner.starts_with("length(") {
            return Self::parse_length(inner);
        }

        if inner.starts_with("range(") {
            return Self::parse_range(inner);
        }

        if inner.starts_with("regex") {
            return Self::parse_regex(inner);
        }

        if inner.starts_with("async_unique(") {
            return Self::parse_async_unique(inner);
        }

        if inner.starts_with("async_exists(") {
            return Self::parse_async_exists(inner);
        }

        if inner.starts_with("async_api(") {
            return Self::parse_async_api(inner);
        }

        if inner == "credit_card" || inner.starts_with("credit_card,") {
            let message = Self::extract_message(inner);
            return Some(SerializableRule::CreditCard { message });
        }

        if inner == "ip" {
            return Some(SerializableRule::Ip {
                v4: None,
                v6: None,
                message: None,
            });
        }

        if inner.starts_with("ip(") {
            return Self::parse_ip(inner);
        }

        if inner == "phone" || inner.starts_with("phone,") {
            let message = Self::extract_message(inner);
            return Some(SerializableRule::Phone { message });
        }

        if inner.starts_with("contains(") {
            return Self::parse_contains(inner);
        }

        if inner.starts_with("custom_async") {
            return Self::parse_custom_async(inner);
        }

        None
    }

    fn extract_message(s: &str) -> Option<String> {
        if let Some(idx) = s.find("message = \"") {
            let start = idx + 11;
            if let Some(end) = s[start..].find('"') {
                return Some(s[start..start + end].to_string());
            }
        }
        None
    }

    fn extract_param(s: &str, param: &str) -> Option<String> {
        let pattern = format!("{} = ", param);
        if let Some(idx) = s.find(&pattern) {
            let start = idx + pattern.len();
            let rest = &s[start..];

            // Check if it's a quoted string
            if let Some(stripped) = rest.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    return Some(stripped[..end].to_string());
                }
            } else {
                // It's a number or other value
                let end = rest.find([',', ')']).unwrap_or(rest.len());
                return Some(rest[..end].trim().to_string());
            }
        }
        None
    }

    fn parse_length(s: &str) -> Option<Self> {
        let min = Self::extract_param(s, "min").and_then(|v| v.parse().ok());
        let max = Self::extract_param(s, "max").and_then(|v| v.parse().ok());
        let message = Self::extract_message(s);
        Some(SerializableRule::Length { min, max, message })
    }

    fn parse_range(s: &str) -> Option<Self> {
        let min = Self::extract_param(s, "min").and_then(|v| v.parse().ok());
        let max = Self::extract_param(s, "max").and_then(|v| v.parse().ok());
        let message = Self::extract_message(s);
        Some(SerializableRule::Range { min, max, message })
    }

    fn parse_regex(s: &str) -> Option<Self> {
        let pattern =
            Self::extract_param(s, "regex").or_else(|| Self::extract_param(s, "pattern"))?;
        let message = Self::extract_message(s);
        Some(SerializableRule::Regex { pattern, message })
    }

    fn parse_async_unique(s: &str) -> Option<Self> {
        let table = Self::extract_param(s, "table")?;
        let column = Self::extract_param(s, "column")?;
        let message = Self::extract_message(s);
        Some(SerializableRule::AsyncUnique {
            table,
            column,
            message,
        })
    }

    fn parse_async_exists(s: &str) -> Option<Self> {
        let table = Self::extract_param(s, "table")?;
        let column = Self::extract_param(s, "column")?;
        let message = Self::extract_message(s);
        Some(SerializableRule::AsyncExists {
            table,
            column,
            message,
        })
    }

    fn parse_async_api(s: &str) -> Option<Self> {
        let endpoint = Self::extract_param(s, "endpoint")?;
        let message = Self::extract_message(s);
        Some(SerializableRule::AsyncApi { endpoint, message })
    }

    fn parse_ip(s: &str) -> Option<Self> {
        let v4 = if s.contains("v4") { Some(true) } else { None };
        let v6 = if s.contains("v6") { Some(true) } else { None };
        let message = Self::extract_message(s);
        Some(SerializableRule::Ip { v4, v6, message })
    }

    fn parse_contains(s: &str) -> Option<Self> {
        let needle = Self::extract_param(s, "needle")?;
        let message = Self::extract_message(s);
        Some(SerializableRule::Contains { needle, message })
    }

    fn parse_custom_async(s: &str) -> Option<Self> {
        // Handle both simple 'custom_async = "func"' and logical 'custom_async(function = "func")'
        let function = Self::extract_param(s, "custom_async")
            .or_else(|| Self::extract_param(s, "function"))?;
        let message = Self::extract_message(s);
        Some(SerializableRule::CustomAsync { function, message })
    }
}

// Conversion implementations from concrete rules to SerializableRule
use crate::v2::rules::{
    AsyncApiRule, AsyncExistsRule, AsyncUniqueRule, ContainsRule, CreditCardRule, EmailRule,
    IpRule, LengthRule, PhoneRule, RegexRule, RequiredRule, UrlRule,
};

impl From<EmailRule> for SerializableRule {
    fn from(rule: EmailRule) -> Self {
        SerializableRule::Email {
            message: rule.message,
        }
    }
}

impl From<LengthRule> for SerializableRule {
    fn from(rule: LengthRule) -> Self {
        SerializableRule::Length {
            min: rule.min,
            max: rule.max,
            message: rule.message,
        }
    }
}

impl From<RegexRule> for SerializableRule {
    fn from(rule: RegexRule) -> Self {
        SerializableRule::Regex {
            pattern: rule.pattern,
            message: rule.message,
        }
    }
}

impl From<UrlRule> for SerializableRule {
    fn from(rule: UrlRule) -> Self {
        SerializableRule::Url {
            message: rule.message,
        }
    }
}

impl From<RequiredRule> for SerializableRule {
    fn from(rule: RequiredRule) -> Self {
        SerializableRule::Required {
            message: rule.message,
        }
    }
}

impl From<AsyncUniqueRule> for SerializableRule {
    fn from(rule: AsyncUniqueRule) -> Self {
        SerializableRule::AsyncUnique {
            table: rule.table,
            column: rule.column,
            message: rule.message,
        }
    }
}

impl From<AsyncExistsRule> for SerializableRule {
    fn from(rule: AsyncExistsRule) -> Self {
        SerializableRule::AsyncExists {
            table: rule.table,
            column: rule.column,
            message: rule.message,
        }
    }
}

impl From<AsyncApiRule> for SerializableRule {
    fn from(rule: AsyncApiRule) -> Self {
        SerializableRule::AsyncApi {
            endpoint: rule.endpoint,
            message: rule.message,
        }
    }
}

impl From<CreditCardRule> for SerializableRule {
    fn from(rule: CreditCardRule) -> Self {
        SerializableRule::CreditCard {
            message: rule.message,
        }
    }
}

impl From<IpRule> for SerializableRule {
    fn from(rule: IpRule) -> Self {
        SerializableRule::Ip {
            v4: rule.v4,
            v6: rule.v6,
            message: rule.message,
        }
    }
}

impl From<PhoneRule> for SerializableRule {
    fn from(rule: PhoneRule) -> Self {
        SerializableRule::Phone {
            message: rule.message,
        }
    }
}

impl From<ContainsRule> for SerializableRule {
    fn from(rule: ContainsRule) -> Self {
        SerializableRule::Contains {
            needle: rule.needle,
            message: rule.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializable_rule_email_pretty_print() {
        let rule = SerializableRule::Email { message: None };
        assert_eq!(rule.pretty_print(), "#[validate(email)]");

        let rule = SerializableRule::Email {
            message: Some("Invalid email".to_string()),
        };
        assert_eq!(
            rule.pretty_print(),
            "#[validate(email, message = \"Invalid email\")]"
        );
    }

    #[test]
    fn serializable_rule_length_pretty_print() {
        let rule = SerializableRule::Length {
            min: Some(3),
            max: Some(50),
            message: None,
        };
        assert_eq!(
            rule.pretty_print(),
            "#[validate(length(min = 3, max = 50))]"
        );
    }

    #[test]
    fn serializable_rule_roundtrip() {
        let rule = SerializableRule::Range {
            min: Some(18.0),
            max: Some(120.0),
            message: Some("Age must be between 18 and 120".to_string()),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let parsed: SerializableRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_email() {
        let rule = SerializableRule::Email { message: None };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);

        let rule = SerializableRule::Email {
            message: Some("Invalid email".to_string()),
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_length() {
        let rule = SerializableRule::Length {
            min: Some(3),
            max: Some(50),
            message: None,
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_range() {
        let rule = SerializableRule::Range {
            min: Some(18.0),
            max: Some(120.0),
            message: None,
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_url() {
        let rule = SerializableRule::Url { message: None };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_required() {
        let rule = SerializableRule::Required { message: None };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_async_unique() {
        let rule = SerializableRule::AsyncUnique {
            table: "users".to_string(),
            column: "email".to_string(),
            message: None,
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_async_exists() {
        let rule = SerializableRule::AsyncExists {
            table: "categories".to_string(),
            column: "id".to_string(),
            message: Some("Category not found".to_string()),
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn serializable_rule_pretty_print_roundtrip_async_api() {
        let rule = SerializableRule::AsyncApi {
            endpoint: "https://api.example.com/validate".to_string(),
            message: None,
        };
        let pretty = rule.pretty_print();
        let parsed = SerializableRule::parse(&pretty).unwrap();
        assert_eq!(rule, parsed);
    }

    #[test]
    fn from_email_rule() {
        let rule = EmailRule::new().with_message("Invalid email");
        let serializable: SerializableRule = rule.into();
        assert_eq!(
            serializable,
            SerializableRule::Email {
                message: Some("Invalid email".to_string())
            }
        );
    }

    #[test]
    fn from_length_rule() {
        let rule = LengthRule::new(3, 50).with_message("Invalid length");
        let serializable: SerializableRule = rule.into();
        assert_eq!(
            serializable,
            SerializableRule::Length {
                min: Some(3),
                max: Some(50),
                message: Some("Invalid length".to_string())
            }
        );
    }

    #[test]
    fn from_async_unique_rule() {
        let rule = AsyncUniqueRule::new("users", "email").with_message("Email taken");
        let serializable: SerializableRule = rule.into();
        assert_eq!(
            serializable,
            SerializableRule::AsyncUnique {
                table: "users".to_string(),
                column: "email".to_string(),
                message: Some("Email taken".to_string())
            }
        );
    }
}
