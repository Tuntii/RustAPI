use super::types::RustApi;
use crate::middleware::{LayerStack, MiddlewareLayer};

/// Configuration builder for RustAPI with auto-routes
pub struct RustApiConfig {
    docs_path: Option<String>,
    docs_enabled: bool,
    api_title: String,
    api_version: String,
    api_description: Option<String>,
    body_limit: Option<usize>,
    layers: LayerStack,
}

impl Default for RustApiConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RustApiConfig {
    pub fn new() -> Self {
        Self {
            docs_path: Some("/docs".to_string()),
            docs_enabled: true,
            api_title: "RustAPI".to_string(),
            api_version: "1.0.0".to_string(),
            api_description: None,
            body_limit: None,
            layers: LayerStack::new(),
        }
    }

    /// Set the docs path (default: "/docs")
    pub fn docs_path(mut self, path: impl Into<String>) -> Self {
        self.docs_path = Some(path.into());
        self
    }

    /// Enable or disable docs (default: true)
    pub fn docs_enabled(mut self, enabled: bool) -> Self {
        self.docs_enabled = enabled;
        self
    }

    /// Set OpenAPI info
    pub fn openapi_info(
        mut self,
        title: impl Into<String>,
        version: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Self {
        self.api_title = title.into();
        self.api_version = version.into();
        self.api_description = description.map(|d| d.into());
        self
    }

    /// Set body size limit
    pub fn body_limit(mut self, limit: usize) -> Self {
        self.body_limit = Some(limit);
        self
    }

    /// Add a middleware layer
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: MiddlewareLayer,
    {
        self.layers.push(Box::new(layer));
        self
    }

    /// Build the RustApi instance
    pub fn build(self) -> RustApi {
        let mut app = RustApi::new().mount_auto_routes_grouped();

        // Apply configuration
        if let Some(limit) = self.body_limit {
            app = app.body_limit(limit);
        }

        app = app.openapi_info(
            &self.api_title,
            &self.api_version,
            self.api_description.as_deref(),
        );

        #[cfg(feature = "swagger-ui")]
        if self.docs_enabled {
            if let Some(path) = self.docs_path {
                app = app.docs(&path);
            }
        }

        // Apply layers
        // Note: layers are applied in reverse order in RustApi::layer logic (pushing to vec)
        app.layers.extend(self.layers);

        app
    }

    /// Build and run the server
    pub async fn run(
        self,
        addr: impl AsRef<str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.build().run(addr.as_ref()).await
    }
}
