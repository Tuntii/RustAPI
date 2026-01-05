//! Context builder for templates

use serde::Serialize;
use tera::Context;

/// Builder for constructing template context
///
/// This provides a fluent API for building template context without
/// needing to create a struct for simple cases.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_view::ContextBuilder;
///
/// let context = ContextBuilder::new()
///     .insert("name", "Alice")
///     .insert("age", 30)
///     .insert_if("admin", true, |_| user.is_admin())
///     .build();
/// ```
pub struct ContextBuilder {
    context: Context,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new() -> Self {
        Self {
            context: Context::new(),
        }
    }

    /// Insert a value into the context
    pub fn insert<T: Serialize + ?Sized>(mut self, key: impl Into<String>, value: &T) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Insert a value if a condition is met
    pub fn insert_if<T: Serialize + ?Sized, F>(
        self,
        key: impl Into<String>,
        value: &T,
        condition: F,
    ) -> Self
    where
        F: FnOnce(&T) -> bool,
    {
        if condition(value) {
            self.insert(key, value)
        } else {
            self
        }
    }

    /// Insert a value if it's Some
    pub fn insert_some<T: Serialize>(self, key: impl Into<String>, value: Option<&T>) -> Self {
        if let Some(v) = value {
            self.insert(key, v)
        } else {
            self
        }
    }

    /// Extend with values from a serializable struct
    pub fn extend<T: Serialize>(mut self, value: &T) -> Result<Self, tera::Error> {
        let additional = Context::from_serialize(value)?;
        self.context.extend(additional);
        Ok(self)
    }

    /// Build the context
    pub fn build(self) -> Context {
        self.context
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ContextBuilder> for Context {
    fn from(builder: ContextBuilder) -> Self {
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let context = ContextBuilder::new()
            .insert("name", &"Alice")
            .insert("age", &30)
            .build();

        assert!(context.contains_key("name"));
        assert!(context.contains_key("age"));
    }

    #[test]
    fn test_insert_if() {
        let show = true;
        let context = ContextBuilder::new()
            .insert_if("visible", &"yes", |_| show)
            .insert_if("hidden", &"no", |_| !show)
            .build();

        assert!(context.contains_key("visible"));
        assert!(!context.contains_key("hidden"));
    }

    #[test]
    fn test_insert_some() {
        let name: Option<&str> = Some("Alice");
        let missing: Option<&str> = None;

        let context = ContextBuilder::new()
            .insert_some("name", name)
            .insert_some("missing", missing)
            .build();

        assert!(context.contains_key("name"));
        assert!(!context.contains_key("missing"));
    }
}
