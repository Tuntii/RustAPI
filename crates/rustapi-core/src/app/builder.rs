use super::config::RustApiConfig;
use super::dispatcher::RequestDispatcher;
use super::helpers::{add_path_params_to_operation, normalize_prefix_for_openapi};
#[cfg(feature = "swagger-ui")]
use super::helpers::{check_basic_auth, unauthorized_response};
#[cfg(feature = "dashboard")]
use super::helpers::{
    infer_route_feature_gates, is_dashboard_replay_eligible, openapi_tags_for_route,
};
use super::production::ProductionDefaultsConfig;
use super::types::RustApi;
use crate::error::Result;
use crate::events::LifecycleHooks;
use crate::interceptor::{InterceptorChain, RequestInterceptor, ResponseInterceptor};
use crate::middleware::{BodyLimitLayer, LayerStack, MiddlewareLayer, DEFAULT_BODY_LIMIT};
use crate::response::IntoResponse;
use crate::router::{MethodRouter, Router};
use crate::server::Server;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

impl RustApi {
    /// Create a new RustAPI application
    pub fn new() -> Self {
        // Initialize tracing if not already done
        let _ = tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info,rustapi=debug")),
            )
            .with(tracing_subscriber::fmt::layer())
            .try_init();

        Self {
            router: Router::new(),
            openapi_spec: rustapi_openapi::OpenApiSpec::new("RustAPI Application", "1.0.0")
                .register::<rustapi_openapi::ErrorSchema>()
                .register::<rustapi_openapi::ErrorBodySchema>()
                .register::<rustapi_openapi::ValidationErrorSchema>()
                .register::<rustapi_openapi::ValidationErrorBodySchema>()
                .register::<rustapi_openapi::FieldErrorSchema>(),
            layers: LayerStack::new(),
            body_limit: Some(DEFAULT_BODY_LIMIT), // Default 1MB limit
            interceptors: InterceptorChain::new(),
            lifecycle_hooks: LifecycleHooks::new(),
            hot_reload: false,
            #[cfg(feature = "http3")]
            http3_config: None,
            health_check: None,
            health_endpoint_config: None,
            status_config: None,
            #[cfg(feature = "dashboard")]
            dashboard_config: None,
        }
    }

    /// The primary way to build a RustAPI application.
    ///
    /// Collects all routes decorated with `#[rustapi_rs::get]`, `#[rustapi_rs::post]`, etc.
    /// at link time via `linkme` and registers them automatically â€” no manual `.route()`
    /// or `.mount_route()` calls needed. This is baked into the core and requires no
    /// feature flags.
    ///
    /// When the `swagger-ui` feature is enabled (included in the default `core` feature),
    /// Swagger UI is served at `/docs`. Without it, only the auto-discovered routes are
    /// registered.
    ///
    /// Use [`RustApi::new()`] when handlers are plain `async fn` not annotated with
    /// the route macros, or when you need full manual control over route registration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// #[rustapi_rs::get("/users")]
    /// async fn list_users() -> Json<Vec<User>> {
    ///     Json(vec![])
    /// }
    ///
    /// #[rustapi_rs::main]
    /// async fn main() -> Result<()> {
    ///     RustApi::auto().run("0.0.0.0:8080").await
    /// }
    /// ```
    #[cfg(feature = "swagger-ui")]
    pub fn auto() -> Self {
        Self::new().mount_auto_routes_grouped().docs("/docs")
    }

    #[cfg(not(feature = "swagger-ui"))]
    pub fn auto() -> Self {
        Self::new().mount_auto_routes_grouped()
    }

    /// Create a configurable RustAPI application with auto-routes.
    ///
    /// Provides builder methods for customization while still
    /// auto-registering all decorated routes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::config()
    ///     .docs_path("/api-docs")
    ///     .body_limit(5 * 1024 * 1024)  // 5MB
    ///     .openapi_info("My API", "2.0.0", Some("API Description"))
    ///     .run("0.0.0.0:8080")
    ///     .await?;
    /// ```
    pub fn config() -> RustApiConfig {
        RustApiConfig::new()
    }

    /// Set the global body size limit for request bodies
    ///
    /// This protects against denial-of-service attacks via large payloads.
    /// The default limit is 1MB (1024 * 1024 bytes).
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum body size in bytes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::new()
    ///     .body_limit(5 * 1024 * 1024)  // 5MB limit
    ///     .route("/upload", post(upload_handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn body_limit(mut self, limit: usize) -> Self {
        self.body_limit = Some(limit);
        self
    }

    /// Disable the body size limit
    ///
    /// Warning: This removes protection against large payload attacks.
    /// Only use this if you have other mechanisms to limit request sizes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .no_body_limit()  // Disable body size limit
    ///     .route("/upload", post(upload_handler))
    /// ```
    pub fn no_body_limit(mut self) -> Self {
        self.body_limit = None;
        self
    }

    /// Add a middleware layer to the application
    ///
    /// Layers are executed in the order they are added (outermost first).
    /// The first layer added will be the first to process the request and
    /// the last to process the response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    /// use rustapi_core::middleware::{RequestIdLayer, TracingLayer};
    ///
    /// RustApi::new()
    ///     .layer(RequestIdLayer::new())  // First to process request
    ///     .layer(TracingLayer::new())    // Second to process request
    ///     .route("/", get(handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: MiddlewareLayer,
    {
        self.layers.push(Box::new(layer));
        self
    }

    /// Add a request interceptor to the application
    ///
    /// Request interceptors are executed in registration order before the route handler.
    /// Each interceptor can modify the request before passing it to the next interceptor
    /// or handler.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::{RustApi, interceptor::RequestInterceptor, Request};
    ///
    /// #[derive(Clone)]
    /// struct AddRequestId;
    ///
    /// impl RequestInterceptor for AddRequestId {
    ///     fn intercept(&self, mut req: Request) -> Request {
    ///         req.extensions_mut().insert(uuid::Uuid::new_v4());
    ///         req
    ///     }
    ///
    ///     fn clone_box(&self) -> Box<dyn RequestInterceptor> {
    ///         Box::new(self.clone())
    ///     }
    /// }
    ///
    /// RustApi::new()
    ///     .request_interceptor(AddRequestId)
    ///     .route("/", get(handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn request_interceptor<I>(mut self, interceptor: I) -> Self
    where
        I: RequestInterceptor,
    {
        self.interceptors.add_request_interceptor(interceptor);
        self
    }

    /// Add a response interceptor to the application
    ///
    /// Response interceptors are executed in reverse registration order after the route
    /// handler completes. Each interceptor can modify the response before passing it
    /// to the previous interceptor or client.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::{RustApi, interceptor::ResponseInterceptor, Response};
    ///
    /// #[derive(Clone)]
    /// struct AddServerHeader;
    ///
    /// impl ResponseInterceptor for AddServerHeader {
    ///     fn intercept(&self, mut res: Response) -> Response {
    ///         res.headers_mut().insert("X-Server", "RustAPI".parse().unwrap());
    ///         res
    ///     }
    ///
    ///     fn clone_box(&self) -> Box<dyn ResponseInterceptor> {
    ///         Box::new(self.clone())
    ///     }
    /// }
    ///
    /// RustApi::new()
    ///     .response_interceptor(AddServerHeader)
    ///     .route("/", get(handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn response_interceptor<I>(mut self, interceptor: I) -> Self
    where
        I: ResponseInterceptor,
    {
        self.interceptors.add_response_interceptor(interceptor);
        self
    }

    /// Add application state
    ///
    /// State is shared across all handlers and can be extracted using `State<T>`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Clone)]
    /// struct AppState {
    ///     db: DbPool,
    /// }
    ///
    /// RustApi::new()
    ///     .state(AppState::new())
    /// ```
    pub fn state<S>(self, _state: S) -> Self
    where
        S: Clone + Send + Sync + 'static,
    {
        // Store state in the router's shared Extensions so `State<T>` extractor can retrieve it.
        let state = _state;
        let mut app = self;
        let r = std::mem::take(&mut app.router);
        app.router = r.state(state);
        app
    }

    /// Register an `on_start` lifecycle hook
    ///
    /// The callback runs **after** route registration and **before** the server
    /// begins accepting connections. Multiple hooks execute in registration order.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .on_start(|| async {
    ///         println!("ğŸš€ Server starting...");
    ///         // e.g. run DB migrations, warm caches
    ///     })
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn on_start<F, Fut>(mut self, hook: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.lifecycle_hooks
            .on_start
            .push(Box::new(move || Box::pin(hook())));
        self
    }

    /// Register an `on_shutdown` lifecycle hook
    ///
    /// The callback runs **after** the shutdown signal is received and the server
    /// stops accepting new connections. Multiple hooks execute in registration order.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .on_shutdown(|| async {
    ///         println!("ğŸ‘‹ Server shutting down...");
    ///         // e.g. flush logs, close DB connections
    ///     })
    ///     .run_with_shutdown("127.0.0.1:8080", ctrl_c())
    ///     .await
    /// ```
    pub fn on_shutdown<F, Fut>(mut self, hook: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.lifecycle_hooks
            .on_shutdown
            .push(Box::new(move || Box::pin(hook())));
        self
    }

    /// Enable hot-reload mode for development
    ///
    /// When enabled:
    /// - A dev-mode banner is printed at startup
    /// - The `RUSTAPI_HOT_RELOAD` env var is set so that `cargo rustapi watch`
    ///   can detect the server is reload-aware
    /// - If the server is **not** already running under the CLI watcher,
    ///   a helpful hint is printed suggesting `cargo rustapi run --watch`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .hot_reload(true)
    ///     .route("/", get(hello))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }

    /// Register an OpenAPI schema
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Schema)]
    /// struct User { ... }
    ///
    /// RustApi::new()
    ///     .register_schema::<User>()
    /// ```
    pub fn register_schema<T: rustapi_openapi::schema::RustApiSchema>(mut self) -> Self {
        self.openapi_spec = self.openapi_spec.register::<T>();
        self
    }

    /// Configure OpenAPI info (title, version, description)
    pub fn openapi_info(mut self, title: &str, version: &str, description: Option<&str>) -> Self {
        // NOTE: Do not reset the spec here; doing so would drop collected paths/schemas.
        // This is especially important for `RustApi::auto()` and `RustApi::config()`.
        self.openapi_spec.info.title = title.to_string();
        self.openapi_spec.info.version = version.to_string();
        self.openapi_spec.info.description = description.map(|d| d.to_string());
        self
    }

    /// Get the current OpenAPI spec (for advanced usage/testing).
    pub fn openapi_spec(&self) -> &rustapi_openapi::OpenApiSpec {
        &self.openapi_spec
    }

    /// If RUSTAPI_DUMP_OPENAPI=1 (or true), print the generated OpenAPI spec as JSON
    /// to stdout and exit immediately. Used by `cargo rustapi mcp generate` to
    /// extract the spec without needing a running HTTP server.
    fn maybe_dump_openapi(&self) {
        if let Ok(val) = std::env::var("RUSTAPI_DUMP_OPENAPI") {
            if matches!(val.as_str(), "1" | "true" | "yes") {
                let json = self.openapi_spec.to_json();
                // Print clean JSON only
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    println!("{}", pretty);
                } else {
                    println!("{}", json);
                }
                std::process::exit(0);
            }
        }
    }

    pub(super) fn mount_auto_routes_grouped(mut self) -> Self {
        let routes = crate::auto_route::collect_auto_routes();

        if routes.is_empty() {
            // This is a common source of confusion with linkme-based auto registration.
            // We emit a clear warning so users know their annotated handlers were not linked.
            tracing::warn!(
                target: "rustapi::auto",
                count = 0,
                "RustApi::auto() collected 0 routes. \
                 This usually means either:\n\
                 - No handlers were annotated with #[rustapi_rs::get], #[post], etc.\n\
                 - The binary/test was not linked with the annotated modules (common in some test setups).\n\
                 - You are building a library (cdylib/rlib) where linkme distributed slices may not be populated.\n\n\
                 You can still register routes manually with .route() or check with rustapi_rs::auto_route_count()."
            );
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                target: "rustapi::auto",
                count = routes.len(),
                "Auto route collection found handlers"
            );
        }

        // Use BTreeMap for deterministic route registration order
        let mut by_path: BTreeMap<String, MethodRouter> = BTreeMap::new();

        for route in routes {
            let crate::handler::Route {
                path: route_path,
                method,
                handler,
                operation,
                component_registrar,
                ..
            } = route;

            let method_enum = match method {
                "GET" => http::Method::GET,
                "POST" => http::Method::POST,
                "PUT" => http::Method::PUT,
                "DELETE" => http::Method::DELETE,
                "PATCH" => http::Method::PATCH,
                _ => http::Method::GET,
            };

            let path = if route_path.starts_with('/') {
                route_path.to_string()
            } else {
                format!("/{}", route_path)
            };

            let entry = by_path.entry(path).or_default();
            entry.insert_boxed_with_operation(method_enum, handler, operation, component_registrar);
        }

        #[cfg(feature = "tracing")]
        let route_count: usize = by_path.values().map(|mr| mr.allowed_methods().len()).sum();
        #[cfg(feature = "tracing")]
        let path_count = by_path.len();

        for (path, method_router) in by_path {
            self = self.route(&path, method_router);
        }

        crate::trace_info!(
            paths = path_count,
            routes = route_count,
            "Auto-registered routes"
        );

        // Apply any auto-registered schemas.
        crate::auto_schema::apply_auto_schemas(&mut self.openapi_spec);

        self
    }

    /// Add a route
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(index))
    ///     .route("/users", get(list_users).post(create_user))
    ///     .route("/users/{id}", get(get_user).delete(delete_user))
    /// ```
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        for register_components in &method_router.component_registrars {
            register_components(&mut self.openapi_spec);
        }

        // Register operations in OpenAPI spec
        for (method, op) in &method_router.operations {
            let mut op = op.clone();
            add_path_params_to_operation(path, &mut op, &BTreeMap::new());
            self.openapi_spec = self.openapi_spec.path(path, method.as_str(), op);
        }

        self.router = self.router.route(path, method_router);
        self
    }

    /// Add a typed route
    pub fn typed<P: crate::typed_path::TypedPath>(self, method_router: MethodRouter) -> Self {
        self.route(P::PATH, method_router)
    }

    /// Mount a handler (convenience method)
    ///
    /// Alias for `.route(path, method_router)` for a single handler.
    #[deprecated(note = "Use route() directly or mount_route() for macro-based routing")]
    pub fn mount(self, path: &str, method_router: MethodRouter) -> Self {
        self.route(path, method_router)
    }

    /// Mount a route created with `#[rustapi::get]`, `#[rustapi::post]`, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// #[rustapi::get("/users")]
    /// async fn list_users() -> Json<Vec<User>> {
    ///     Json(vec![])
    /// }
    ///
    /// RustApi::new()
    ///     .mount_route(route!(list_users))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn mount_route(mut self, route: crate::handler::Route) -> Self {
        let method_enum = match route.method {
            "GET" => http::Method::GET,
            "POST" => http::Method::POST,
            "PUT" => http::Method::PUT,
            "DELETE" => http::Method::DELETE,
            "PATCH" => http::Method::PATCH,
            _ => http::Method::GET,
        };

        (route.component_registrar)(&mut self.openapi_spec);

        // Register operation in OpenAPI spec
        let mut op = route.operation;
        add_path_params_to_operation(route.path, &mut op, &route.param_schemas);
        self.openapi_spec = self.openapi_spec.path(route.path, route.method, op);

        self.route_with_method(route.path, method_enum, route.handler)
    }

    /// Helper to mount a single method handler
    fn route_with_method(
        self,
        path: &str,
        method: http::Method,
        handler: crate::handler::BoxedHandler,
    ) -> Self {
        use crate::router::MethodRouter;
        // use http::Method; // Removed

        // This is simplified. In a real implementation we'd merge with existing router at this path
        // For now we assume one handler per path or we simply allow overwriting for this MVP step
        // (matchit router doesn't allow easy merging/updating existing entries without rebuilding)
        //
        // TOOD: Enhance Router to support method merging

        let path = if !path.starts_with('/') {
            format!("/{}", path)
        } else {
            path.to_string()
        };

        // Check if we already have this path?
        // For MVP, valid assumption: user calls .route() or .mount() once per path-method-combo
        // But we need to handle multiple methods on same path.
        // Our Router wrapper currently just inserts.

        // Since we can't easily query matchit, we'll just insert.
        // Limitations: strictly sequential mounting for now.

        let mut handlers = std::collections::HashMap::new();
        handlers.insert(method, handler);

        let method_router = MethodRouter::from_boxed(handlers);
        self.route(&path, method_router)
    }

    /// Nest a router under a prefix
    ///
    /// All routes from the nested router will be registered with the prefix
    /// prepended to their paths. OpenAPI operations from the nested router
    /// are also propagated to the parent's OpenAPI spec with prefixed paths.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let api_v1 = Router::new()
    ///     .route("/users", get(list_users));
    ///
    /// RustApi::new()
    ///     .nest("/api/v1", api_v1)
    /// ```
    pub fn nest(mut self, prefix: &str, router: Router) -> Self {
        // Normalize the prefix for OpenAPI paths
        let normalized_prefix = normalize_prefix_for_openapi(prefix);

        // Propagate OpenAPI operations from nested router with prefixed paths
        // We need to do this before calling router.nest() because it consumes the router
        for (matchit_path, method_router) in router.method_routers() {
            for register_components in &method_router.component_registrars {
                register_components(&mut self.openapi_spec);
            }

            // Get the display path from registered_routes (has {param} format)
            let display_path = router
                .registered_routes()
                .get(matchit_path)
                .map(|info| info.path.clone())
                .unwrap_or_else(|| matchit_path.clone());

            // Build the prefixed display path for OpenAPI
            let prefixed_path = if display_path == "/" {
                normalized_prefix.clone()
            } else {
                format!("{}{}", normalized_prefix, display_path)
            };

            // Register each operation in the OpenAPI spec
            for (method, op) in &method_router.operations {
                let mut op = op.clone();
                add_path_params_to_operation(&prefixed_path, &mut op, &BTreeMap::new());
                self.openapi_spec = self.openapi_spec.path(&prefixed_path, method.as_str(), op);
            }
        }

        // Delegate to Router::nest for actual route registration
        self.router = self.router.nest(prefix, router);
        self
    }

    /// Serve static files from a directory
    ///
    /// Maps a URL path prefix to a filesystem directory. Requests to paths under
    /// the prefix will serve files from the corresponding location in the directory.
    ///
    /// # Arguments
    ///
    /// * `prefix` - URL path prefix (e.g., "/static", "/assets")
    /// * `root` - Filesystem directory path
    ///
    /// # Features
    ///
    /// - Automatic MIME type detection
    /// - ETag and Last-Modified headers for caching
    /// - Index file serving for directories
    /// - Path traversal prevention
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::new()
    ///     .serve_static("/assets", "./public")
    ///     .serve_static("/uploads", "./uploads")
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn serve_static(self, prefix: &str, root: impl Into<std::path::PathBuf>) -> Self {
        self.serve_static_with_config(crate::static_files::StaticFileConfig::new(root, prefix))
    }

    /// Serve static files with custom configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::static_files::StaticFileConfig;
    ///
    /// let config = StaticFileConfig::new("./public", "/assets")
    ///     .max_age(86400)  // Cache for 1 day
    ///     .fallback("index.html");  // SPA fallback
    ///
    /// RustApi::new()
    ///     .serve_static_with_config(config)
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub fn serve_static_with_config(self, config: crate::static_files::StaticFileConfig) -> Self {
        use crate::router::MethodRouter;
        use std::collections::HashMap;

        let prefix = config.prefix.clone();
        let catch_all_path = format!("{}/*path", prefix.trim_end_matches('/'));

        // Create the static file handler
        let handler: crate::handler::BoxedHandler =
            std::sync::Arc::new(move |req: crate::Request| {
                let config = config.clone();
                let path = req.uri().path().to_string();

                Box::pin(async move {
                    let relative_path = path.strip_prefix(&config.prefix).unwrap_or(&path);

                    match crate::static_files::StaticFile::serve(relative_path, &config).await {
                        Ok(response) => response,
                        Err(err) => err.into_response(),
                    }
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>>
            });

        let mut handlers = HashMap::new();
        handlers.insert(http::Method::GET, handler);
        let method_router = MethodRouter::from_boxed(handlers);

        self.route(&catch_all_path, method_router)
    }

    /// Enable response compression
    ///
    /// Adds gzip/deflate compression for response bodies. The compression
    /// is based on the client's Accept-Encoding header.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_rs::prelude::*;
    ///
    /// RustApi::new()
    ///     .compression()
    ///     .route("/", get(handler))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    #[cfg(feature = "compression")]
    pub fn compression(self) -> Self {
        self.layer(crate::middleware::CompressionLayer::new())
    }

    /// Enable response compression with custom configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::middleware::CompressionConfig;
    ///
    /// RustApi::new()
    ///     .compression_with_config(
    ///         CompressionConfig::new()
    ///             .min_size(512)
    ///             .level(9)
    ///     )
    ///     .route("/", get(handler))
    /// ```
    #[cfg(feature = "compression")]
    pub fn compression_with_config(self, config: crate::middleware::CompressionConfig) -> Self {
        self.layer(crate::middleware::CompressionLayer::with_config(config))
    }

    /// Enable Swagger UI documentation
    ///
    /// This adds two endpoints:
    /// - `{path}` - Swagger UI interface
    /// - `{path}/openapi.json` - OpenAPI JSON specification
    ///
    /// **Important:** Call `.docs()` AFTER registering all routes. The OpenAPI
    /// specification is captured at the time `.docs()` is called, so routes
    /// added afterwards will not appear in the documentation.
    ///
    /// # Example
    ///
    /// ```text
    /// RustApi::new()
    ///     .route("/users", get(list_users))     // Add routes first
    ///     .route("/posts", get(list_posts))     // Add more routes
    ///     .docs("/docs")  // Then enable docs - captures all routes above
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    ///
    /// For `RustApi::auto()`, routes are collected before `.docs()` is called,
    /// so this is handled automatically.
    #[cfg(feature = "swagger-ui")]
    pub fn docs(self, path: &str) -> Self {
        let title = self.openapi_spec.info.title.clone();
        let version = self.openapi_spec.info.version.clone();
        let description = self.openapi_spec.info.description.clone();

        self.docs_with_info(path, &title, &version, description.as_deref())
    }

    /// Enable Swagger UI documentation with custom API info
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .docs_with_info("/docs", "My API", "2.0.0", Some("API for managing users"))
    /// ```
    #[cfg(feature = "swagger-ui")]
    pub fn docs_with_info(
        mut self,
        path: &str,
        title: &str,
        version: &str,
        description: Option<&str>,
    ) -> Self {
        use crate::router::get;
        // Update spec info
        self.openapi_spec.info.title = title.to_string();
        self.openapi_spec.info.version = version.to_string();
        if let Some(desc) = description {
            self.openapi_spec.info.description = Some(desc.to_string());
        }

        let path = path.trim_end_matches('/');
        let openapi_path = format!("{}/openapi.json", path);

        // Clone values for closures
        let spec_value = self.openapi_spec.to_json();
        let spec_json = serde_json::to_string_pretty(&spec_value).unwrap_or_else(|e| {
            // Safe fallback if JSON serialization fails (though unlikely for Value)
            tracing::error!("Failed to serialize OpenAPI spec: {}", e);
            "{}".to_string()
        });
        let openapi_url = openapi_path.clone();

        // Add OpenAPI JSON endpoint
        let spec_handler = move || {
            let json = spec_json.clone();
            async move {
                http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(crate::response::Body::from(json))
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to build response: {}", e);
                        http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(crate::response::Body::from("Internal Server Error"))
                            .unwrap()
                    })
            }
        };

        // Add Swagger UI endpoint
        let docs_handler = move || {
            let url = openapi_url.clone();
            async move {
                let response = rustapi_openapi::swagger_ui_html(&url);
                response.map(crate::response::Body::Full)
            }
        };

        self.route(&openapi_path, get(spec_handler))
            .route(path, get(docs_handler))
    }

    /// Enable Swagger UI documentation with Basic Auth protection
    ///
    /// When username and password are provided, the docs endpoint will require
    /// Basic Authentication. This is useful for protecting API documentation
    /// in production environments.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/users", get(list_users))
    ///     .docs_with_auth("/docs", "admin", "secret123")
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    #[cfg(feature = "swagger-ui")]
    pub fn docs_with_auth(self, path: &str, username: &str, password: &str) -> Self {
        let title = self.openapi_spec.info.title.clone();
        let version = self.openapi_spec.info.version.clone();
        let description = self.openapi_spec.info.description.clone();

        self.docs_with_auth_and_info(
            path,
            username,
            password,
            &title,
            &version,
            description.as_deref(),
        )
    }

    /// Enable Swagger UI documentation with Basic Auth and custom API info
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .docs_with_auth_and_info(
    ///         "/docs",
    ///         "admin",
    ///         "secret",
    ///         "My API",
    ///         "2.0.0",
    ///         Some("Protected API documentation")
    ///     )
    /// ```
    #[cfg(feature = "swagger-ui")]
    pub fn docs_with_auth_and_info(
        mut self,
        path: &str,
        username: &str,
        password: &str,
        title: &str,
        version: &str,
        description: Option<&str>,
    ) -> Self {
        use crate::router::MethodRouter;
        use std::collections::HashMap;

        #[inline]
        fn base64_encode(input: &[u8]) -> String {
            const ALPHA: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
            for chunk in input.chunks(3) {
                let b0 = chunk[0] as usize;
                let b1 = if chunk.len() > 1 {
                    chunk[1] as usize
                } else {
                    0
                };
                let b2 = if chunk.len() > 2 {
                    chunk[2] as usize
                } else {
                    0
                };
                out.push(ALPHA[b0 >> 2] as char);
                out.push(ALPHA[((b0 & 3) << 4) | (b1 >> 4)] as char);
                out.push(if chunk.len() > 1 {
                    ALPHA[((b1 & 0xf) << 2) | (b2 >> 6)] as char
                } else {
                    '='
                });
                out.push(if chunk.len() > 2 {
                    ALPHA[b2 & 63] as char
                } else {
                    '='
                });
            }
            out
        }

        // Update spec info
        self.openapi_spec.info.title = title.to_string();
        self.openapi_spec.info.version = version.to_string();
        if let Some(desc) = description {
            self.openapi_spec.info.description = Some(desc.to_string());
        }

        let path = path.trim_end_matches('/');
        let openapi_path = format!("{}/openapi.json", path);

        // Create expected auth header value
        let credentials = format!("{}:{}", username, password);
        let encoded = base64_encode(credentials.as_bytes());
        let expected_auth = format!("Basic {}", encoded);

        // Clone values for closures
        let spec_value = self.openapi_spec.to_json();
        let spec_json = serde_json::to_string_pretty(&spec_value).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize OpenAPI spec: {}", e);
            "{}".to_string()
        });
        let openapi_url = openapi_path.clone();
        let expected_auth_spec = expected_auth.clone();
        let expected_auth_docs = expected_auth;

        // Create spec handler with auth check
        let spec_handler: crate::handler::BoxedHandler =
            std::sync::Arc::new(move |req: crate::Request| {
                let json = spec_json.clone();
                let expected = expected_auth_spec.clone();
                Box::pin(async move {
                    if !check_basic_auth(&req, &expected) {
                        return unauthorized_response();
                    }
                    http::Response::builder()
                        .status(http::StatusCode::OK)
                        .header(http::header::CONTENT_TYPE, "application/json")
                        .body(crate::response::Body::from(json))
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to build response: {}", e);
                            http::Response::builder()
                                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                                .body(crate::response::Body::from("Internal Server Error"))
                                .unwrap()
                        })
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>>
            });

        // Create docs handler with auth check
        let docs_handler: crate::handler::BoxedHandler =
            std::sync::Arc::new(move |req: crate::Request| {
                let url = openapi_url.clone();
                let expected = expected_auth_docs.clone();
                Box::pin(async move {
                    if !check_basic_auth(&req, &expected) {
                        return unauthorized_response();
                    }
                    let response = rustapi_openapi::swagger_ui_html(&url);
                    response.map(crate::response::Body::Full)
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = crate::Response> + Send>>
            });

        // Create method routers with boxed handlers
        let mut spec_handlers = HashMap::new();
        spec_handlers.insert(http::Method::GET, spec_handler);
        let spec_router = MethodRouter::from_boxed(spec_handlers);

        let mut docs_handlers = HashMap::new();
        docs_handlers.insert(http::Method::GET, docs_handler);
        let docs_router = MethodRouter::from_boxed(docs_handlers);

        self.route(&openapi_path, spec_router)
            .route(path, docs_router)
    }

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

    /// Print a hot-reload dev banner if `.hot_reload(true)` is set
    fn print_hot_reload_banner(&self, addr: &str) {
        if !self.hot_reload {
            return;
        }

        // Set the env var so the CLI watcher can detect it
        std::env::set_var("RUSTAPI_HOT_RELOAD", "1");

        let is_under_watcher = std::env::var("RUSTAPI_HOT_RELOAD")
            .map(|v| v == "1")
            .unwrap_or(false);

        tracing::info!("ğŸ”„ Hot-reload mode enabled");

        if is_under_watcher {
            tracing::info!("   File watcher active â€” changes will trigger rebuild + restart");
        } else {
            tracing::info!("   Tip: Run with `cargo rustapi run --watch` for automatic hot-reload");
        }

        tracing::info!("   Listening on http://{addr}");
    }

    // Helper to apply status page logic (monitor, layer, route)
    fn apply_health_endpoints(&mut self) {
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

    fn apply_status_page(&mut self) {
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
    fn apply_dashboard(&mut self) {
        use crate::dashboard::{DashboardMetrics, RouteInventoryItem};
        use crate::handler::BoxedHandler;
        use crate::response::Body;
        use crate::router::MethodRouter;
        use std::collections::HashMap;
        use std::sync::Arc;

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

        let metrics = Arc::new(DashboardMetrics::new_with_replay_admin_path(
            inventory,
            config.replay_api_path.clone(),
        ));

        // Insert metrics into router state using the public .state() API
        let router = std::mem::take(&mut self.router);
        self.router = router.state(Arc::clone(&metrics));

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
            let metrics_c = Arc::clone(&metrics);
            let config_c = config.clone();
            let handler: BoxedHandler = Arc::new(move |req| {
                let metrics = Arc::clone(&metrics_c);
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
            let metrics_c = Arc::clone(&metrics);
            let config_c = config.clone();
            let wildcard_path = format!("{}/*path", prefix);
            let handler: BoxedHandler = Arc::new(move |req| {
                let metrics = Arc::clone(&metrics_c);
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

    /// Run the server
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(hello))
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
    pub async fn run(mut self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.maybe_dump_openapi();

        // Hot-reload mode banner
        self.print_hot_reload_banner(addr);

        // Apply health endpoints if configured
        self.apply_health_endpoints();

        // Apply status page if configured
        self.apply_status_page();

        // Apply embedded dashboard if configured
        #[cfg(feature = "dashboard")]
        self.apply_dashboard();

        // Apply body limit layer if configured (should be first in the chain)
        if let Some(limit) = self.body_limit {
            // Prepend body limit layer so it's the first to process requests
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        // Run on_start lifecycle hooks before accepting connections
        for hook in self.lifecycle_hooks.on_start {
            hook().await;
        }

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
        self.maybe_dump_openapi();

        // Hot-reload mode banner
        self.print_hot_reload_banner(addr.as_ref());

        // Apply health endpoints if configured
        self.apply_health_endpoints();

        // Apply status page if configured
        self.apply_status_page();

        // Apply embedded dashboard if configured
        #[cfg(feature = "dashboard")]
        self.apply_dashboard();

        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        // Run on_start lifecycle hooks before accepting connections
        for hook in self.lifecycle_hooks.on_start {
            hook().await;
        }

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

    /// Get the inner router (for testing or advanced usage)
    pub fn into_router(self) -> Router {
        self.router
    }

    /// Get a reference to the inner router (for advanced usage, e.g. in-process MCP dispatch).
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Get the layer stack (for testing)
    pub fn layers(&self) -> &LayerStack {
        &self.layers
    }

    /// Get the interceptor chain (for testing)
    pub fn interceptors(&self) -> &InterceptorChain {
        &self.interceptors
    }

    /// Returns a dispatcher that can execute requests directly through this
    /// app's router + layers + interceptors, with zero network overhead.
    ///
    /// This is intended for in-process protocol integrations (e.g. MCP tool calls
    /// when running side-by-side with the main HTTP server).
    pub fn request_dispatcher(&self) -> RequestDispatcher {
        RequestDispatcher {
            router: Arc::new(self.router.clone()),
            layers: self.layers().clone(),
            interceptors: self.interceptors().clone(),
        }
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

        // Apply health endpoints if configured
        self.apply_health_endpoints();

        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

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

        // Apply health endpoints if configured
        self.apply_health_endpoints();

        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

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

        // Apply health endpoints if configured
        self.apply_health_endpoints();

        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

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

impl Default for RustApi {
    fn default() -> Self {
        Self::new()
    }
}
