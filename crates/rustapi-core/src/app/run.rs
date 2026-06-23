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

    pub(super) fn print_hot_reload_banner(&self, addr: &str) -> Option<bool> {
        if !self.hot_reload {
            return None;
        }

        let is_under_watcher = std::env::var("RUSTAPI_HOT_RELOAD")
            .map(|v| v == "1")
            .unwrap_or(false);

        std::env::set_var("RUSTAPI_HOT_RELOAD", "1");

        tracing::info!("Hot-reload mode enabled");

        if is_under_watcher {
            tracing::info!("   File watcher active - changes will trigger rebuild + restart");
        } else {
            tracing::info!("   Tip: Run with `cargo rustapi run --watch` for automatic hot-reload");
        }

        tracing::info!("   Listening on http://{addr}");
        Some(is_under_watcher)
    }

    async fn run_shutdown_hooks(hooks: Vec<crate::events::LifecycleHook>) {
        for hook in hooks {
            hook().await;
        }
    }

    pub(super) fn apply_status_page(&mut self) {
        if let Some(config) = &self.status_config {
            let monitor = std::sync::Arc::new(crate::status::StatusMonitor::new());

            self.layers
                .push(Box::new(crate::status::StatusLayer::new(monitor.clone())));

            use crate::router::MethodRouter;
            use std::collections::HashMap;

            let monitor = monitor.clone();
            let config = config.clone();
            let path = config.path.clone();

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

        let router = std::mem::take(&mut self.router);
        self.router = router.state(std::sync::Arc::clone(&metrics));

        let prefix = config.path.trim_end_matches('/').to_owned();

        fn not_found() -> crate::response::Response {
            http::Response::builder()
                .status(404)
                .body(Body::Full(http_body_util::Full::new(bytes::Bytes::from(
                    "Not Found",
                ))))
                .unwrap()
        }

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

    #[cfg(feature = "dashboard")]
    pub fn dashboard(mut self, config: crate::dashboard::DashboardConfig) -> Self {
        self.dashboard_config = Some(config);
        self
    }

    pub async fn run(mut self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.prepare_for_serve(addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = Server::new(self.router, self.layers, self.interceptors);
        let result = server.run(addr).await;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        result
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

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = Server::new(self.router, self.layers, self.interceptors);
        server.run_with_shutdown(addr.as_ref(), signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    /// Run HTTP/3 with TLS certificates and a graceful shutdown signal.
    #[cfg(feature = "http3")]
    pub async fn run_http3_with_shutdown<F>(
        mut self,
        config: crate::http3::Http3Config,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::Arc;

        let addr = config.socket_addr();
        self.prepare_for_serve(&addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run_with_shutdown(signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    #[cfg(feature = "http3")]
    pub async fn run_http3(
        mut self,
        config: crate::http3::Http3Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        let addr = config.socket_addr();
        self.prepare_for_serve(&addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        let result = server.run().await;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        result
    }

    /// Run HTTP/3 (self-signed) with a graceful shutdown signal.
    #[cfg(feature = "http3-dev")]
    pub async fn run_http3_dev_with_shutdown<F>(
        mut self,
        addr: &str,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::Arc;

        self.prepare_for_serve(addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run_with_shutdown(signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    #[cfg(feature = "http3-dev")]
    pub async fn run_http3_dev(
        mut self,
        addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        self.prepare_for_serve(addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        let result = server.run().await;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        result
    }

    /// Configure HTTP/3 support for `run_http3` and `run_dual_stack`.
    #[cfg(feature = "http3")]
    pub fn with_http3(mut self, cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        self.http3_config = Some(crate::http3::Http3Config::new(cert_path, key_path));
        self
    }

    /// Run HTTP/1.1 and HTTP/3 together with a graceful shutdown signal.
    #[cfg(feature = "http3")]
    pub async fn run_dual_stack_with_shutdown<F>(
        mut self,
        http_addr: &str,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
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

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
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

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_for_signal = notify.clone();
        tokio::spawn(async move {
            signal.await;
            notify_for_signal.notify_waiters();
        });
        let wait_for_shutdown = {
            let notify = notify.clone();
            async move {
                notify.notified().await;
            }
        };
        let wait_for_shutdown_http3 = async move {
            notify.notified().await;
        };

        tokio::try_join!(
            http1_server.run_with_shutdown(&http_addr, wait_for_shutdown),
            http3_server.run_with_shutdown(wait_for_shutdown_http3),
        )?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

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

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
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

        tokio::try_join!(http1_server.run(&http_addr), http3_server.run(),)?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }
}
