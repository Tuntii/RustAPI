//! Asynchronous validation rules.
//!
//! These rules require async operations like database queries or API calls.

use crate::v2::context::ValidationContext;
use crate::v2::error::RuleError;
use crate::v2::traits::AsyncValidationRule;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Database uniqueness validation rule.
///
/// Validates that a value is unique in a database table column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsyncUniqueRule {
    /// Database table name
    pub table: String,
    /// Column name to check
    pub column: String,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AsyncUniqueRule {
    /// Create a new uniqueness rule.
    pub fn new(table: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[async_trait]
impl AsyncValidationRule<str> for AsyncUniqueRule {
    async fn validate_async(&self, value: &str, ctx: &ValidationContext) -> Result<(), RuleError> {
        let db = ctx.database().ok_or_else(|| {
            RuleError::new(
                "async_unique",
                "Database validator not configured in context",
            )
        })?;

        let is_unique = if let Some(exclude_id) = ctx.exclude_id() {
            db.is_unique_except(&self.table, &self.column, value, exclude_id)
                .await
                .map_err(|e| RuleError::new("async_unique", format!("Database error: {}", e)))?
        } else {
            db.is_unique(&self.table, &self.column, value)
                .await
                .map_err(|e| RuleError::new("async_unique", format!("Database error: {}", e)))?
        };

        if is_unique {
            Ok(())
        } else {
            let message = self.message.clone().unwrap_or_else(|| {
                format!("Value already exists in {}.{}", self.table, self.column)
            });
            Err(RuleError::new("async_unique", message)
                .param("table", self.table.clone())
                .param("column", self.column.clone()))
        }
    }

    fn rule_name(&self) -> &'static str {
        "async_unique"
    }
}

#[async_trait]
impl AsyncValidationRule<String> for AsyncUniqueRule {
    async fn validate_async(
        &self,
        value: &String,
        ctx: &ValidationContext,
    ) -> Result<(), RuleError> {
        <Self as AsyncValidationRule<str>>::validate_async(self, value.as_str(), ctx).await
    }

    fn rule_name(&self) -> &'static str {
        "async_unique"
    }
}

/// Database existence validation rule.
///
/// Validates that a value exists in a database table column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsyncExistsRule {
    /// Database table name
    pub table: String,
    /// Column name to check
    pub column: String,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AsyncExistsRule {
    /// Create a new existence rule.
    pub fn new(table: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[async_trait]
impl AsyncValidationRule<str> for AsyncExistsRule {
    async fn validate_async(&self, value: &str, ctx: &ValidationContext) -> Result<(), RuleError> {
        let db = ctx.database().ok_or_else(|| {
            RuleError::new(
                "async_exists",
                "Database validator not configured in context",
            )
        })?;

        let exists = db
            .exists(&self.table, &self.column, value)
            .await
            .map_err(|e| RuleError::new("async_exists", format!("Database error: {}", e)))?;

        if exists {
            Ok(())
        } else {
            let message = self.message.clone().unwrap_or_else(|| {
                format!("Value does not exist in {}.{}", self.table, self.column)
            });
            Err(RuleError::new("async_exists", message)
                .param("table", self.table.clone())
                .param("column", self.column.clone()))
        }
    }

    fn rule_name(&self) -> &'static str {
        "async_exists"
    }
}

#[async_trait]
impl AsyncValidationRule<String> for AsyncExistsRule {
    async fn validate_async(
        &self,
        value: &String,
        ctx: &ValidationContext,
    ) -> Result<(), RuleError> {
        <Self as AsyncValidationRule<str>>::validate_async(self, value.as_str(), ctx).await
    }

    fn rule_name(&self) -> &'static str {
        "async_exists"
    }
}

/// External API validation rule.
///
/// Validates a value against an external API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsyncApiRule {
    /// API endpoint URL
    pub endpoint: String,
    /// Custom error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AsyncApiRule {
    /// Create a new API validation rule.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            message: None,
        }
    }

    /// Set a custom error message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[async_trait]
impl AsyncValidationRule<str> for AsyncApiRule {
    async fn validate_async(&self, value: &str, ctx: &ValidationContext) -> Result<(), RuleError> {
        let http = ctx.http().ok_or_else(|| {
            RuleError::new("async_api", "HTTP validator not configured in context")
        })?;

        let is_valid = http
            .validate(&self.endpoint, value)
            .await
            .map_err(|e| RuleError::new("async_api", format!("API error: {}", e)))?;

        if is_valid {
            Ok(())
        } else {
            let message = self
                .message
                .clone()
                .unwrap_or_else(|| "API validation failed".to_string());
            Err(RuleError::new("async_api", message).param("endpoint", self.endpoint.clone()))
        }
    }

    fn rule_name(&self) -> &'static str {
        "async_api"
    }
}

#[async_trait]
impl AsyncValidationRule<String> for AsyncApiRule {
    async fn validate_async(
        &self,
        value: &String,
        ctx: &ValidationContext,
    ) -> Result<(), RuleError> {
        <Self as AsyncValidationRule<str>>::validate_async(self, value.as_str(), ctx).await
    }

    fn rule_name(&self) -> &'static str {
        "async_api"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::context::{DatabaseValidator, ValidationContextBuilder};

    struct MockDbValidator {
        unique_values: Vec<String>,
        existing_values: Vec<String>,
    }

    #[async_trait]
    impl DatabaseValidator for MockDbValidator {
        async fn exists(&self, _table: &str, _column: &str, value: &str) -> Result<bool, String> {
            Ok(self.existing_values.contains(&value.to_string()))
        }

        async fn is_unique(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
        ) -> Result<bool, String> {
            Ok(!self.unique_values.contains(&value.to_string()))
        }

        async fn is_unique_except(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
            _except_id: &str,
        ) -> Result<bool, String> {
            Ok(!self.unique_values.contains(&value.to_string()))
        }
    }

    #[tokio::test]
    async fn async_unique_rule_valid() {
        let db = MockDbValidator {
            unique_values: vec!["taken@example.com".to_string()],
            existing_values: vec![],
        };
        let ctx = ValidationContextBuilder::new().database(db).build();

        let rule = AsyncUniqueRule::new("users", "email");
        assert!(rule.validate_async("new@example.com", &ctx).await.is_ok());
    }

    #[tokio::test]
    async fn async_unique_rule_invalid() {
        let db = MockDbValidator {
            unique_values: vec!["taken@example.com".to_string()],
            existing_values: vec![],
        };
        let ctx = ValidationContextBuilder::new().database(db).build();

        let rule = AsyncUniqueRule::new("users", "email");
        let err = rule
            .validate_async("taken@example.com", &ctx)
            .await
            .unwrap_err();
        assert_eq!(err.code, "async_unique");
    }

    #[tokio::test]
    async fn async_exists_rule_valid() {
        let db = MockDbValidator {
            unique_values: vec![],
            existing_values: vec!["existing_id".to_string()],
        };
        let ctx = ValidationContextBuilder::new().database(db).build();

        let rule = AsyncExistsRule::new("users", "id");
        assert!(rule.validate_async("existing_id", &ctx).await.is_ok());
    }

    #[tokio::test]
    async fn async_exists_rule_invalid() {
        let db = MockDbValidator {
            unique_values: vec![],
            existing_values: vec!["existing_id".to_string()],
        };
        let ctx = ValidationContextBuilder::new().database(db).build();

        let rule = AsyncExistsRule::new("users", "id");
        let err = rule
            .validate_async("nonexistent_id", &ctx)
            .await
            .unwrap_err();
        assert_eq!(err.code, "async_exists");
    }

    #[tokio::test]
    async fn async_rule_without_context() {
        let ctx = ValidationContext::new();

        let rule = AsyncUniqueRule::new("users", "email");
        let err = rule
            .validate_async("test@example.com", &ctx)
            .await
            .unwrap_err();
        assert!(err.message.contains("not configured"));
    }

    #[test]
    fn async_rule_serialization() {
        let rule = AsyncUniqueRule::new("users", "email").with_message("Email already taken");
        let json = serde_json::to_string(&rule).unwrap();
        let parsed: AsyncUniqueRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, parsed);
    }
}
