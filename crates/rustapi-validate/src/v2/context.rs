//! Validation context for async operations.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for database validation operations.
#[async_trait]
pub trait DatabaseValidator: Send + Sync {
    /// Check if a value exists in a table column.
    async fn exists(&self, table: &str, column: &str, value: &str) -> Result<bool, String>;

    /// Check if a value is unique in a table column.
    async fn is_unique(&self, table: &str, column: &str, value: &str) -> Result<bool, String>;

    /// Check if a value is unique, excluding a specific ID (for updates).
    async fn is_unique_except(
        &self,
        table: &str,
        column: &str,
        value: &str,
        except_id: &str,
    ) -> Result<bool, String>;
}

/// Trait for HTTP/API validation operations.
#[async_trait]
pub trait HttpValidator: Send + Sync {
    /// Validate a value against an external API endpoint.
    async fn validate(&self, endpoint: &str, value: &str) -> Result<bool, String>;
}

/// Trait for custom async validators.
#[async_trait]
pub trait CustomValidator: Send + Sync {
    /// Validate a value with custom logic.
    async fn validate(&self, value: &str) -> Result<bool, String>;
}

/// Context for async validation operations.
///
/// Provides access to database, HTTP, and custom validators for async validation rules.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::v2::prelude::*;
///
/// let ctx = ValidationContextBuilder::new()
///     .database(my_db_validator)
///     .http(my_http_client)
///     .build();
///
/// user.validate_async(&ctx).await?;
/// ```
#[derive(Clone, Default)]
pub struct ValidationContext {
    database: Option<Arc<dyn DatabaseValidator>>,
    http: Option<Arc<dyn HttpValidator>>,
    custom: HashMap<String, Arc<dyn CustomValidator>>,
    /// ID to exclude from uniqueness checks (for updates)
    exclude_id: Option<String>,
    /// Locale for error messages (e.g. "en", "tr")
    locale: Option<String>,
}

impl ValidationContext {
    /// Create a new empty validation context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the database validator if configured.
    pub fn database(&self) -> Option<&Arc<dyn DatabaseValidator>> {
        self.database.as_ref()
    }

    /// Get the HTTP validator if configured.
    pub fn http(&self) -> Option<&Arc<dyn HttpValidator>> {
        self.http.as_ref()
    }

    /// Get a custom validator by name.
    pub fn custom(&self, name: &str) -> Option<&Arc<dyn CustomValidator>> {
        self.custom.get(name)
    }

    /// Get the locale.
    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    /// Get the ID to exclude from uniqueness checks.
    pub fn exclude_id(&self) -> Option<&str> {
        self.exclude_id.as_deref()
    }

    /// Create a builder for constructing a validation context.
    pub fn builder() -> ValidationContextBuilder {
        ValidationContextBuilder::new()
    }
}

impl std::fmt::Debug for ValidationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationContext")
            .field("has_database", &self.database.is_some())
            .field("has_http", &self.http.is_some())
            .field("custom_validators", &self.custom.keys().collect::<Vec<_>>())
            .field("exclude_id", &self.exclude_id)
            .field("locale", &self.locale)
            .finish()
    }
}

/// Builder for constructing a `ValidationContext`.
#[derive(Clone, Default)]
pub struct ValidationContextBuilder {
    database: Option<Arc<dyn DatabaseValidator>>,
    http: Option<Arc<dyn HttpValidator>>,
    custom: HashMap<String, Arc<dyn CustomValidator>>,
    exclude_id: Option<String>,
    locale: Option<String>,
}

impl ValidationContextBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database validator.
    pub fn database(mut self, validator: impl DatabaseValidator + 'static) -> Self {
        self.database = Some(Arc::new(validator));
        self
    }

    /// Set the database validator from an Arc.
    pub fn database_arc(mut self, validator: Arc<dyn DatabaseValidator>) -> Self {
        self.database = Some(validator);
        self
    }

    /// Set the HTTP validator.
    pub fn http(mut self, validator: impl HttpValidator + 'static) -> Self {
        self.http = Some(Arc::new(validator));
        self
    }

    /// Set the HTTP validator from an Arc.
    pub fn http_arc(mut self, validator: Arc<dyn HttpValidator>) -> Self {
        self.http = Some(validator);
        self
    }

    /// Add a custom validator.
    pub fn custom(
        mut self,
        name: impl Into<String>,
        validator: impl CustomValidator + 'static,
    ) -> Self {
        self.custom.insert(name.into(), Arc::new(validator));
        self
    }

    /// Add a custom validator from an Arc.
    pub fn custom_arc(
        mut self,
        name: impl Into<String>,
        validator: Arc<dyn CustomValidator>,
    ) -> Self {
        self.custom.insert(name.into(), validator);
        self
    }

    /// Set the ID to exclude from uniqueness checks (for updates).
    pub fn exclude_id(mut self, id: impl Into<String>) -> Self {
        self.exclude_id = Some(id.into());
        self
    }

    /// Set the locale.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Build the validation context.
    pub fn build(self) -> ValidationContext {
        ValidationContext {
            database: self.database,
            http: self.http,
            custom: self.custom,
            exclude_id: self.exclude_id,
            locale: self.locale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDbValidator;

    #[async_trait]
    impl DatabaseValidator for MockDbValidator {
        async fn exists(&self, _table: &str, _column: &str, _value: &str) -> Result<bool, String> {
            Ok(true)
        }

        async fn is_unique(
            &self,
            _table: &str,
            _column: &str,
            _value: &str,
        ) -> Result<bool, String> {
            Ok(true)
        }

        async fn is_unique_except(
            &self,
            _table: &str,
            _column: &str,
            _value: &str,
            _except_id: &str,
        ) -> Result<bool, String> {
            Ok(true)
        }
    }

    #[test]
    fn context_builder() {
        let ctx = ValidationContextBuilder::new()
            .database(MockDbValidator)
            .exclude_id("123")
            .build();

        assert!(ctx.database().is_some());
        assert!(ctx.http().is_none());
        assert_eq!(ctx.exclude_id(), Some("123"));
    }

    #[test]
    fn empty_context() {
        let ctx = ValidationContext::new();
        assert!(ctx.database().is_none());
        assert!(ctx.http().is_none());
        assert!(ctx.exclude_id().is_none());
    }
}
