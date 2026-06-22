use super::production::ProductionDefaultsConfig;
use super::types::RustApi;

impl RustApi {
    /// Enable automatic status page with default configuration
    pub fn status_page(self) -> Self {
        self.status_page_with_config(crate::status::StatusConfig::default())
    }

    /// Enable automatic status page with custom configuration
    pub fn status_page_with_config(mut self, config: crate::status::StatusConfig) -> Self {
        self.status_config = Some(config);
        self
    }

    /// Enable built-in `/health`, `/ready`, and `/live` endpoints with default paths.
    ///
    /// The default health check includes a lightweight `self` probe so the
    /// endpoints are immediately useful even before dependency checks are added.
    pub fn health_endpoints(mut self) -> Self {
        self.health_endpoint_config = Some(crate::health::HealthEndpointConfig::default());
        if self.health_check.is_none() {
            self.health_check = Some(crate::health::HealthCheckBuilder::default().build());
        }
        self
    }

    /// Enable built-in health endpoints with custom paths.
    pub fn health_endpoints_with_config(
        mut self,
        config: crate::health::HealthEndpointConfig,
    ) -> Self {
        self.health_endpoint_config = Some(config);
        if self.health_check.is_none() {
            self.health_check = Some(crate::health::HealthCheckBuilder::default().build());
        }
        self
    }

    /// Register a custom health check and enable built-in health endpoints.
    ///
    /// The configured check is used by `/health` and `/ready`, while `/live`
    /// remains a lightweight process-level probe.
    pub fn with_health_check(mut self, health_check: crate::health::HealthCheck) -> Self {
        self.health_check = Some(health_check);
        if self.health_endpoint_config.is_none() {
            self.health_endpoint_config = Some(crate::health::HealthEndpointConfig::default());
        }
        self
    }

    /// Apply a one-call production baseline preset.
    ///
    /// This enables:
    /// - `RequestIdLayer`
    /// - `TracingLayer` with `service` and `environment` fields
    /// - built-in `/health`, `/ready`, and `/live` probes
    pub fn production_defaults(self, service_name: impl Into<String>) -> Self {
        self.production_defaults_with_config(ProductionDefaultsConfig::new(service_name))
    }

    /// Apply the production baseline preset with custom configuration.
    pub fn production_defaults_with_config(mut self, config: ProductionDefaultsConfig) -> Self {
        if config.enable_request_id {
            self = self.layer(crate::middleware::RequestIdLayer::new());
        }

        if config.enable_tracing {
            let mut tracing_layer =
                crate::middleware::TracingLayer::with_level(config.tracing_level)
                    .with_field("service", config.service_name.clone())
                    .with_field("environment", crate::error::get_environment().to_string());

            if let Some(version) = &config.version {
                tracing_layer = tracing_layer.with_field("version", version.clone());
            }

            self = self.layer(tracing_layer);
        }

        if config.enable_health_endpoints {
            if self.health_check.is_none() {
                let mut builder = crate::health::HealthCheckBuilder::default();
                if let Some(version) = &config.version {
                    builder = builder.version(version.clone());
                }
                self.health_check = Some(builder.build());
            }

            if self.health_endpoint_config.is_none() {
                self.health_endpoint_config =
                    Some(config.health_endpoint_config.unwrap_or_default());
            }
        }

        self
    }
    pub(super) fn apply_health_endpoints(&mut self) {
        if let Some(config) = &self.health_endpoint_config {
            use crate::router::get;

            let health_check = self
                .health_check
                .clone()
                .unwrap_or_else(|| crate::health::HealthCheckBuilder::default().build());

            let health_path = config.health_path.clone();
            let readiness_path = config.readiness_path.clone();
            let liveness_path = config.liveness_path.clone();

            let health_handler = {
                let health_check = health_check.clone();
                move || {
                    let health_check = health_check.clone();
                    async move { crate::health::health_response(health_check).await }
                }
            };

            let readiness_handler = {
                let health_check = health_check.clone();
                move || {
                    let health_check = health_check.clone();
                    async move { crate::health::readiness_response(health_check).await }
                }
            };

            let liveness_handler = || async { crate::health::liveness_response().await };

            let router = std::mem::take(&mut self.router);
            self.router = router
                .route(&health_path, get(health_handler))
                .route(&readiness_path, get(readiness_handler))
                .route(&liveness_path, get(liveness_handler));
        }
    }
}
