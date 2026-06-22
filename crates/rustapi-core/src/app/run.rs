#[cfg(feature = "dashboard")]
use super::helpers::{
    infer_route_feature_gates, is_dashboard_replay_eligible, openapi_tags_for_route,
};
use super::types::RustApi;
use crate::error::Result;
use crate::middleware::BodyLimitLayer;
use crate::response::IntoResponse;
use crate::server::Server;
impl RustApi {
    async fn prepare_for_serve(&mut self, addr: &str) {
        self.maybe_dump_openapi();
        self.print_hot_reload_banner(addr);
        self.apply_health_endpoints();
        self.apply_status_page();
        #[cfg(feature = "dashboard")]
        self.apply_dashboard();
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }
        for hook in std::mem::take(&mut self.lifecycle_hooks.on_start) {
            hook().await;
        }
    }

    pub(super) fn print_hot_reload_banner(&self, addr: &str) {
        if !self.hot_reload {
            return;
        }

        let is_under_watcher = std::env::var("RUSTAPI_HOT_RELOAD")
            .map(|v| v == "1")
            .unwrap_or(false);

        // Set the env var so the CLI watcher can detect it
        std::env::set_var("RUSTAPI_HOT_RELOAD", "1");

        tracing::info!("Hot-reload mode enabled");

        if is_under_watcher {
            tracing::info!("   File watcher active - changes will trigger rebuild + restart");
        } else {
            tracing::info!("   Tip: Run with `cargo rustapi run --watch` for automatic hot-reload");
        }

        tracing::info!("   Listening on http://{addr}");
    }
    pub(super) fn apply_status_page(&mut self) {
        if let Some(config) = &self.status_config {
            let monitor = std::sync::Arc::new(crate::status::StatusMonitor::new());

            // 1. Add middleware layer
            self.layers
                .push(Box::new(crate::status::StatusLayer::new(monitor.clone())));

            // 2. Add status route
            use crate::router::MethodRouter;
            use std::collections::HashMap;

            let monitor = monitor.clone();
            let config = config.clone();
            let path = config.path.clone(); // Clone path before moving config

            let handler: crate::handler::BoxedHandler = std::sync::Arc::new(move |_| {
                let monitor = monitor.clone();
                let config = config.clone();
                Box::pin(async move {
                    crate::status::status_handler(monitor, config)
                        .await
                        .into_response()
                })
            });

            let mut handlers = HashMap::new();
            handlers.insert(http::Method::GET, handler);
            let method_router = MethodRouter::from_boxed(handlers);

            // We need to take the router out to call route() which consumes it
            let router = std::mem::take(&mut self.router);
            self.router = router.route(&path, method_router);
        }
    }

    #[cfg(feature = "dashboard")]
    pub(super) fn apply_dashboard(&mut self) {
        use crate::dashboard::{DashboardMetrics, RouteInventoryItem};
        use crate::handler::BoxedHandler;
        use crate::response::Body;
        use crate::router::MethodRouter;
        use std::collections::HashMap;

        let mut config = match self.dashboard_config.take() {
            Some(c) => c,
            None => return,
        };
        config.normalize_paths();

        // Build route inventory from currently registered routes. This snapshot
        // intentionally happens before dashboard routes are mounted so the UI
        // represents application endpoints rather than the dashboard itself.
        let mut inventory: Vec<RouteInventoryItem> = self
            .router
            .registered_routes()
            .values()
            .map(|info| {
                let methods: Vec<String> = info.methods.iter().map(|m| m.to_string()).collect();
                let health_eligible = self
                    .health_endpoint_config
                    .as_ref()
                    .map(|health| {
                        info.path == health.health_path
                            || info.path == health.readiness_path
                            || info.path == health.liveness_path
                    })
                    .unwrap_or(false);

                RouteInventoryItem::new(info.path.clone(), methods)
                    .with_tags(openapi_tags_for_route(
                        &self.openapi_spec,
                        &info.path,
                        &info.methods,
                    ))
                    .with_feature_gates(infer_route_feature_gates(&info.path))
                    .health_eligible(health_eligible)
                    .replay_eligible(is_dashboard_replay_eligible(&info.path, health_eligible))
            })
            .collect();
        inventory.sort_by(|a, b| a.path.cmp(&b.path));

        let metrics = std::sync::Arc::new(DashboardMetrics::new_with_replay_admin_path(
            inventory,
            config.replay_api_path.clone(),
        ));

        // Insert metrics into router state using the public .state() API
        let router = std::mem::take(&mut self.router);
        self.router = router.state(std::sync::Arc::clone(&metrics));

        // Register dashboard routes
        let prefix = config.path.trim_end_matches('/').to_owned();

        fn not_found() -> crate::response::Response {
            http::Response::builder()
                .status(404)
                .body(Body::Full(http_body_util::Full::new(bytes::Bytes::from(
                    "Not Found",
                ))))
                .unwrap()
        }

        // Route 1: GET /__rustapi/dashboard  (the SPA page)
        {
            let metrics_c = std::sync::Arc::clone(&metrics);
            let config_c = config.clone();
            let handler: BoxedHandler = std::sync::Arc::new(move |req| {
                let metrics = std::sync::Arc::clone(&metrics_c);
                let cfg = config_c.clone();
                Box::pin(async move {
                    let headers = req.headers().clone();
                    let method = req.method().to_string();
                    let path = req.uri().path().to_owned();
                    crate::dashboard::routes::dispatch(&headers, &method, &path, &metrics, &cfg)
                        .await
                        .unwrap_or_else(not_found)
                })
            });
            let mut h = HashMap::new();
            h.insert(http::Method::GET, handler);
            let router = std::mem::take(&mut self.router);
            self.router = router.route(&prefix, MethodRouter::from_boxed(h));
        }

        // Route 2: GET /__rustapi/dashboard/*path  (API sub-paths)
        {
            let metrics_c = std::sync::Arc::clone(&metrics);
            let config_c = config.clone();
            let wildcard_path = format!("{}/*path", prefix);
            let handler: BoxedHandler = std::sync::Arc::new(move |req| {
                let metrics = std::sync::Arc::clone(&metrics_c);
                let cfg = config_c.clone();
                Box::pin(async move {
                    let headers = req.headers().clone();
                    let method = req.method().to_string();
                    let path = req.uri().path().to_owned();
                    crate::dashboard::routes::dispatch(&headers, &method, &path, &metrics, &cfg)
                        .await
                        .unwrap_or_else(not_found)
                })
            });
            let mut h = HashMap::new();
            h.insert(http::Method::GET, handler);
            let router = std::mem::take(&mut self.router);
            self.router = router.route(&wildcard_path, MethodRouter::from_boxed(h));
        }
    }

    /// Enable the embedded isometric system dashboard.
    ///
    /// Registers a self-contained admin surface at the configured path
    /// (default: `/__rustapi/dashboard`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::dashboard::DashboardConfig;
    ///
    /// RustApi::new()
    ///     .route("/api/users", get(list_users))
    ///     .dashboard(
    ///         DashboardConfig::new()
    ///             .admin_token("my-secret")
    ///     )
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    #[cfg(feature = "dashboard")]
    pub fn dashboard(mut self, config: crate::dashboard::DashboardConfig) -> Self {
        self.dashboard_config = Some(config);
        self
    }
    pub async fn run(mut self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.prepare_for_serve(addr).await;

        let server = Server::new(self.router, self.layers, self.interceptors);
        server.run(addr).await
    }

    /// Run the server with graceful shutdown signal
    pub async fn run_with_shutdown<F>(
        mut self,
        addr: impl AsRef<str>,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.prepare_for_serve(addr.as_ref()).await;

        // Wrap the shutdown signal to run on_shutdown hooks after signal fires
        let shutdown_hooks = self.lifecycle_hooks.on_shutdown;
        let wrapped_signal = async move {
            signal.await;
            // Run on_shutdown hooks after the shutdown signal fires
            for hook in shutdown_hooks {
                hook().await;
            }
        };

        let server = Server::new(self.router, self.layers, self.interceptors);
        server
            .run_with_shutdown(addr.as_ref(), wrapped_signal)
            .await
    }

    /// Enable HTTP/3 support with TLS certificates
    ///
    /// HTTP/3 requires TLS certificates. For development, you can use
    /// self-signed certificates with `run_http3_dev`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(hello))
    ///     .run_http3("0.0.0.0:443", "cert.pem", "key.pem")
    ///     .await
    /// ```
    #[cfg(feature = "http3")]
    pub async fn run_http3(
        mut self,
        config: crate::http3::Http3Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        let addr = config.socket_addr();
        self.prepare_for_serve(&addr).await;

        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run().await
    }

    /// Run HTTP/3 server with self-signed certificate (development only)
    ///
    /// This is useful for local development and testing.
    /// **Do not use in production!**
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(hello))
    ///     .run_http3_dev("0.0.0.0:8443")
    ///     .await
    /// ```
    #[cfg(feature = "http3-dev")]
    pub async fn run_http3_dev(
        mut self,
        addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        self.prepare_for_serve(addr).await;

        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run().await
    }

    /// Configure HTTP/3 support for `run_http3` and `run_dual_stack`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .with_http3("cert.pem", "key.pem")
    ///     .run_dual_stack("127.0.0.1:8080")
    ///     .await
    /// ```
    #[cfg(feature = "http3")]
    pub fn with_http3(mut self, cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        self.http3_config = Some(crate::http3::Http3Config::new(cert_path, key_path));
        self
    }

    /// Run both HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP) simultaneously.
    ///
    /// The HTTP/3 listener is bound to the same host and port as `http_addr`
    /// so clients can upgrade to either protocol on one endpoint.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(hello))
    ///     .with_http3("cert.pem", "key.pem")
    ///     .run_dual_stack("0.0.0.0:8080")
    ///     .await
    /// ```
    #[cfg(feature = "http3")]
    pub async fn run_dual_stack(
        mut self,
        http_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        let mut config = self
            .http3_config
            .take()
            .ok_or("HTTP/3 config not set. Use .with_http3(...)")?;

        let http_socket: std::net::SocketAddr = http_addr.parse()?;
        config.bind_addr = if http_socket.ip().is_ipv6() {
            format!("[{}]", http_socket.ip())
        } else {
            http_socket.ip().to_string()
        };
        config.port = http_socket.port();
        let http_addr = http_socket.to_string();

        self.prepare_for_serve(&http_addr).await;

        let router = Arc::new(self.router);
        let layers = Arc::new(self.layers);
        let interceptors = Arc::new(self.interceptors);

        let http1_server =
            Server::from_shared(router.clone(), layers.clone(), interceptors.clone());
        let http3_server =
            crate::http3::Http3Server::new(&config, router, layers, interceptors).await?;

        tracing::info!(
            http1_addr = %http_addr,
            http3_addr = %config.socket_addr(),
            "Starting dual-stack HTTP/1.1 + HTTP/3 servers"
        );

        tokio::try_join!(
            http1_server.run_with_shutdown(&http_addr, std::future::pending::<()>()),
            http3_server.run_with_shutdown(std::future::pending::<()>()),
        )?;

        Ok(())
    }
}
