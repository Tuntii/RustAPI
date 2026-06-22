/// Configuration for RustAPI's built-in production baseline preset.
///
/// This preset bundles together the most common foundation pieces for a
/// production HTTP service:
/// - request IDs on every response
/// - structured tracing spans with service metadata
/// - standard `/health`, `/ready`, and `/live` probes
#[derive(Debug, Clone)]
pub struct ProductionDefaultsConfig {
    pub(super) service_name: String,
    pub(super) version: Option<String>,
    pub(super) tracing_level: tracing::Level,
    pub(super) health_endpoint_config: Option<crate::health::HealthEndpointConfig>,
    pub(super) enable_request_id: bool,
    pub(super) enable_tracing: bool,
    pub(super) enable_health_endpoints: bool,
}

impl ProductionDefaultsConfig {
    /// Create a new production baseline configuration.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            version: None,
            tracing_level: tracing::Level::INFO,
            health_endpoint_config: None,
            enable_request_id: true,
            enable_tracing: true,
            enable_health_endpoints: true,
        }
    }

    /// Annotate tracing spans and default health payloads with an application version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the tracing log level used by the preset tracing layer.
    pub fn tracing_level(mut self, level: tracing::Level) -> Self {
        self.tracing_level = level;
        self
    }

    /// Override the default health endpoint paths.
    pub fn health_endpoint_config(mut self, config: crate::health::HealthEndpointConfig) -> Self {
        self.health_endpoint_config = Some(config);
        self
    }

    /// Enable or disable request ID propagation.
    pub fn request_id(mut self, enabled: bool) -> Self {
        self.enable_request_id = enabled;
        self
    }

    /// Enable or disable structured tracing middleware.
    pub fn tracing(mut self, enabled: bool) -> Self {
        self.enable_tracing = enabled;
        self
    }

    /// Enable or disable built-in health endpoints.
    pub fn health_endpoints(mut self, enabled: bool) -> Self {
        self.enable_health_endpoints = enabled;
        self
    }
}
