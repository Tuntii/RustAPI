//! OpenAPI configuration

/// Configuration for OpenAPI documentation
#[derive(Debug, Clone)]
pub struct OpenApiConfig {
    /// API title
    pub title: String,
    /// API version
    pub version: String,
    /// API description
    pub description: Option<String>,
    /// Path to serve OpenAPI JSON
    pub json_path: String,
    /// Path to serve Swagger UI
    pub docs_path: String,
}

impl Default for OpenApiConfig {
    fn default() -> Self {
        Self {
            title: "RustAPI Application".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            json_path: "/openapi.json".to_string(),
            docs_path: "/docs".to_string(),
        }
    }
}

impl OpenApiConfig {
    /// Create a new OpenAPI configuration
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            ..Default::default()
        }
    }

    /// Set API description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set path for OpenAPI JSON endpoint
    pub fn json_path(mut self, path: impl Into<String>) -> Self {
        self.json_path = path.into();
        self
    }

    /// Set path for Swagger UI docs
    pub fn docs_path(mut self, path: impl Into<String>) -> Self {
        self.docs_path = path.into();
        self
    }
}
