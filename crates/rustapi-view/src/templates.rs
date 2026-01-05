//! Template engine wrapper

use crate::ViewError;
use std::sync::Arc;
use tera::Tera;
use tokio::sync::RwLock;

/// Configuration for the template engine
#[derive(Debug, Clone)]
pub struct TemplatesConfig {
    /// Glob pattern for template files
    pub glob: String,
    /// Whether to auto-reload templates on change (development mode)
    pub auto_reload: bool,
    /// Whether to fail on undefined variables
    pub strict_mode: bool,
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            glob: "templates/**/*.html".to_string(),
            auto_reload: cfg!(debug_assertions),
            strict_mode: false,
        }
    }
}

impl TemplatesConfig {
    /// Create a new config with the given glob pattern
    pub fn new(glob: impl Into<String>) -> Self {
        Self {
            glob: glob.into(),
            ..Default::default()
        }
    }

    /// Set auto-reload behavior
    pub fn auto_reload(mut self, enabled: bool) -> Self {
        self.auto_reload = enabled;
        self
    }

    /// Set strict mode (fail on undefined variables)
    pub fn strict_mode(mut self, enabled: bool) -> Self {
        self.strict_mode = enabled;
        self
    }
}

/// Template engine wrapper providing thread-safe template rendering
///
/// This type wraps the Tera template engine and can be shared across
/// handlers via `State<Templates>`.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_view::Templates;
///
/// let templates = Templates::new("templates/**/*.html")?;
/// ```
#[derive(Clone)]
pub struct Templates {
    inner: Arc<RwLock<Tera>>,
    config: TemplatesConfig,
}

impl Templates {
    /// Create a new template engine from a glob pattern
    ///
    /// The glob pattern specifies which files to load as templates.
    /// Common patterns:
    /// - `templates/**/*.html` - All HTML files in templates directory
    /// - `views/*.tera` - All .tera files in views directory
    ///
    /// # Errors
    ///
    /// Returns an error if the glob pattern is invalid or templates fail to parse.
    pub fn new(glob: impl Into<String>) -> Result<Self, ViewError> {
        let config = TemplatesConfig::new(glob);
        Self::with_config(config)
    }

    /// Create a new template engine with configuration
    pub fn with_config(config: TemplatesConfig) -> Result<Self, ViewError> {
        let mut tera = Tera::new(&config.glob)?;

        // Register custom filters/functions
        register_builtin_filters(&mut tera);

        Ok(Self {
            inner: Arc::new(RwLock::new(tera)),
            config,
        })
    }

    /// Create an empty template engine (for adding templates programmatically)
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Tera::default())),
            config: TemplatesConfig::default(),
        }
    }

    /// Add a template from a string
    pub async fn add_template(
        &self,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<(), ViewError> {
        let mut tera = self.inner.write().await;
        tera.add_raw_template(&name.into(), &content.into())?;
        Ok(())
    }

    /// Render a template with the given context
    pub async fn render(
        &self,
        template: &str,
        context: &tera::Context,
    ) -> Result<String, ViewError> {
        // If auto-reload is enabled and in debug mode, try to reload
        #[cfg(debug_assertions)]
        if self.config.auto_reload {
            let mut tera = self.inner.write().await;
            if let Err(e) = tera.full_reload() {
                tracing::warn!("Template reload failed: {}", e);
            }
        }

        let tera = self.inner.read().await;
        tera.render(template, context).map_err(ViewError::from)
    }

    /// Render a template with a serializable context
    pub async fn render_with<T: serde::Serialize>(
        &self,
        template: &str,
        data: &T,
    ) -> Result<String, ViewError> {
        let context = tera::Context::from_serialize(data)
            .map_err(|e| ViewError::serialization_error(e.to_string()))?;
        self.render(template, &context).await
    }

    /// Check if a template exists
    pub async fn has_template(&self, name: &str) -> bool {
        let tera = self.inner.read().await;
        let result = tera.get_template_names().any(|n| n == name);
        result
    }

    /// Get all template names
    pub async fn template_names(&self) -> Vec<String> {
        let tera = self.inner.read().await;
        tera.get_template_names().map(String::from).collect()
    }

    /// Reload all templates from disk
    pub async fn reload(&self) -> Result<(), ViewError> {
        let mut tera = self.inner.write().await;
        tera.full_reload()?;
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &TemplatesConfig {
        &self.config
    }
}

/// Register built-in template filters
fn register_builtin_filters(tera: &mut Tera) {
    // JSON filter for debugging
    tera.register_filter("json_pretty", |value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>| {
        serde_json::to_string_pretty(value)
            .map(tera::Value::String)
            .map_err(|e| tera::Error::msg(e.to_string()))
    });

    // Truncate string
    tera.register_filter("truncate_words", |value: &tera::Value, args: &std::collections::HashMap<String, tera::Value>| {
        let s = tera::try_get_value!("truncate_words", "value", String, value);
        let length = match args.get("length") {
            Some(val) => tera::try_get_value!("truncate_words", "length", usize, val),
            None => 50,
        };
        let end = match args.get("end") {
            Some(val) => tera::try_get_value!("truncate_words", "end", String, val),
            None => "...".to_string(),
        };

        let words: Vec<&str> = s.split_whitespace().collect();
        if words.len() <= length {
            Ok(tera::Value::String(s))
        } else {
            let truncated: String = words[..length].join(" ");
            Ok(tera::Value::String(format!("{}{}", truncated, end)))
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_templates() {
        let templates = Templates::empty();
        templates.add_template("test", "Hello, {{ name }}!").await.unwrap();
        
        let mut ctx = tera::Context::new();
        ctx.insert("name", "World");
        
        let result = templates.render("test", &ctx).await.unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_render_with_struct() {
        #[derive(serde::Serialize)]
        struct Data {
            name: String,
        }

        let templates = Templates::empty();
        templates.add_template("test", "Hello, {{ name }}!").await.unwrap();
        
        let data = Data { name: "Alice".to_string() };
        let result = templates.render_with("test", &data).await.unwrap();
        assert_eq!(result, "Hello, Alice!");
    }
}
