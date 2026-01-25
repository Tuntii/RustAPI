//! RustApi application builder

use crate::error::Result;
use crate::interceptor::{InterceptorChain, RequestInterceptor, ResponseInterceptor};
use crate::middleware::{BodyLimitLayer, LayerStack, MiddlewareLayer, DEFAULT_BODY_LIMIT};
use crate::response::IntoResponse;
use crate::router::{MethodRouter, Router};
use crate::server::Server;
use std::collections::HashMap;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Main application builder for RustAPI
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     RustApi::new()
///         .state(AppState::new())
///         .route("/", get(hello))
///         .route("/users/{id}", get(get_user))
///         .run("127.0.0.1:8080")
///         .await
/// }
/// ```
pub struct RustApi {
    router: Router,
    openapi_spec: rustapi_openapi::OpenApiSpec,
    layers: LayerStack,
    body_limit: Option<usize>,
    interceptors: InterceptorChain,
    #[cfg(feature = "http3")]
    http3_config: Option<crate::http3::Http3Config>,
    status_config: Option<crate::status::StatusConfig>,
}

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
            #[cfg(feature = "http3")]
            http3_config: None,
            status_config: None,
        }
    }

    /// Create a zero-config RustAPI application.
    ///
    /// All routes decorated with `#[rustapi::get]`, `#[rustapi::post]`, etc.
    /// are automatically registered. Swagger UI is enabled at `/docs` by default.
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
    /// #[rustapi::main]
    /// async fn main() -> Result<()> {
    ///     // Zero config - routes are auto-registered!
    ///     RustApi::auto()
    ///         .run("0.0.0.0:8080")
    ///         .await
    /// }
    /// ```
    #[cfg(feature = "swagger-ui")]
    pub fn auto() -> Self {
        // Build app with grouped auto-routes and auto-schemas, then enable docs.
        Self::new().mount_auto_routes_grouped().docs("/docs")
    }

    /// Create a zero-config RustAPI application (without swagger-ui feature).
    ///
    /// All routes decorated with `#[rustapi::get]`, `#[rustapi::post]`, etc.
    /// are automatically registered.
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
        app.router = app.router.state(state);
        app
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
    pub fn register_schema<T: for<'a> rustapi_openapi::Schema<'a>>(mut self) -> Self {
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

    fn mount_auto_routes_grouped(mut self) -> Self {
        let routes = crate::auto_route::collect_auto_routes();
        let mut by_path: HashMap<String, MethodRouter> = HashMap::new();

        for route in routes {
            let method_enum = match route.method {
                "GET" => http::Method::GET,
                "POST" => http::Method::POST,
                "PUT" => http::Method::PUT,
                "DELETE" => http::Method::DELETE,
                "PATCH" => http::Method::PATCH,
                _ => http::Method::GET,
            };

            let path = if route.path.starts_with('/') {
                route.path.to_string()
            } else {
                format!("/{}", route.path)
            };

            let entry = by_path.entry(path).or_default();
            entry.insert_boxed_with_operation(method_enum, route.handler, route.operation);
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
        // Register operations in OpenAPI spec
        for (method, op) in &method_router.operations {
            let mut op = op.clone();
            add_path_params_to_operation(path, &mut op, &std::collections::HashMap::new());
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
                add_path_params_to_operation(
                    &prefixed_path,
                    &mut op,
                    &std::collections::HashMap::new(),
                );
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
        let spec_json =
            serde_json::to_string_pretty(&self.openapi_spec.to_json()).unwrap_or_default();
        let openapi_url = openapi_path.clone();

        // Add OpenAPI JSON endpoint
        let spec_handler = move || {
            let json = spec_json.clone();
            async move {
                http::Response::builder()
                    .status(http::StatusCode::OK)
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(crate::response::Body::from(json))
                    .unwrap()
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
        use base64::{engine::general_purpose::STANDARD, Engine};
        use std::collections::HashMap;

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
        let encoded = STANDARD.encode(credentials.as_bytes());
        let expected_auth = format!("Basic {}", encoded);

        // Clone values for closures
        let spec_json =
            serde_json::to_string_pretty(&self.openapi_spec.to_json()).unwrap_or_default();
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
                        .unwrap()
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

    // Helper to apply status page logic (monitor, layer, route)
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
            let router = std::mem::replace(&mut self.router, crate::router::Router::new());
            self.router = router.route(&path, method_router);
        }
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
        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured (should be first in the chain)
        if let Some(limit) = self.body_limit {
            // Prepend body limit layer so it's the first to process requests
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
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
        // Apply status page if configured
        self.apply_status_page();

        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        let server = Server::new(self.router, self.layers, self.interceptors);
        server.run_with_shutdown(addr.as_ref(), signal).await
    }

    /// Get the inner router (for testing or advanced usage)
    pub fn into_router(self) -> Router {
        self.router
    }

    /// Get the layer stack (for testing)
    pub fn layers(&self) -> &LayerStack {
        &self.layers
    }

    /// Get the interceptor chain (for testing)
    pub fn interceptors(&self) -> &InterceptorChain {
        &self.interceptors
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

        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router),
            Arc::new(self.layers),
            Arc::new(self.interceptors),
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

        // Apply status page if configured
        self.apply_status_page();

        // Apply body limit layer if configured
        if let Some(limit) = self.body_limit {
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router),
            Arc::new(self.layers),
            Arc::new(self.interceptors),
        )
        .await?;

        server.run().await
    }

    /// Run both HTTP/1.1 and HTTP/3 servers simultaneously
    ///
    /// This allows clients to use either protocol. The HTTP/1.1 server
    /// will advertise HTTP/3 availability via Alt-Svc header.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/", get(hello))
    ///     .run_dual_stack("0.0.0.0:8080", Http3Config::new("cert.pem", "key.pem"))
    ///     .await
    /// ```
    /// Configure HTTP/3 support
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

    /// Run both HTTP/1.1 and HTTP/3 servers simultaneously
    ///
    /// This allows clients to use either protocol. The HTTP/1.1 server
    /// will advertise HTTP/3 availability via Alt-Svc header.
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
        _http_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Dual-stack requires Router, LayerStack, InterceptorChain to implement Clone.
        // For now, we only run HTTP/3.
        // In the future, we can either:
        // 1. Make Router/LayerStack/InterceptorChain Clone
        // 2. Use Arc<RwLock<...>> pattern
        // 3. Create shared state mechanism

        let config = self
            .http3_config
            .take()
            .ok_or("HTTP/3 config not set. Use .with_http3(...)")?;

        tracing::warn!("run_dual_stack currently only runs HTTP/3. HTTP/1.1 support coming soon.");
        self.run_http3(config).await
    }
}

fn add_path_params_to_operation(
    path: &str,
    op: &mut rustapi_openapi::Operation,
    param_schemas: &std::collections::HashMap<String, String>,
) {
    let mut params: Vec<String> = Vec::new();
    let mut in_brace = false;
    let mut current = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_brace = true;
                current.clear();
            }
            '}' => {
                if in_brace {
                    in_brace = false;
                    if !current.is_empty() {
                        params.push(current.clone());
                    }
                }
            }
            _ => {
                if in_brace {
                    current.push(ch);
                }
            }
        }
    }

    if params.is_empty() {
        return;
    }

    let op_params = op.parameters.get_or_insert_with(Vec::new);

    for name in params {
        let already = op_params
            .iter()
            .any(|p| p.location == "path" && p.name == name);
        if already {
            continue;
        }

        // Use custom schema if provided, otherwise infer from name
        let schema = if let Some(schema_type) = param_schemas.get(&name) {
            schema_type_to_openapi_schema(schema_type)
        } else {
            infer_path_param_schema(&name)
        };

        op_params.push(rustapi_openapi::Parameter {
            name,
            location: "path".to_string(),
            required: true,
            description: None,
            schema,
        });
    }
}

/// Convert a schema type string to an OpenAPI schema reference
fn schema_type_to_openapi_schema(schema_type: &str) -> rustapi_openapi::SchemaRef {
    match schema_type.to_lowercase().as_str() {
        "uuid" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string",
            "format": "uuid"
        })),
        "integer" | "int" | "int64" | "i64" => {
            rustapi_openapi::SchemaRef::Inline(serde_json::json!({
                "type": "integer",
                "format": "int64"
            }))
        }
        "int32" | "i32" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "integer",
            "format": "int32"
        })),
        "number" | "float" | "f64" | "f32" => {
            rustapi_openapi::SchemaRef::Inline(serde_json::json!({
                "type": "number"
            }))
        }
        "boolean" | "bool" => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "boolean"
        })),
        _ => rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string"
        })),
    }
}

/// Infer the OpenAPI schema type for a path parameter based on naming conventions.
///
/// Common patterns:
/// - `*_id`, `*Id`, `id` → integer (but NOT *uuid)
/// - `*_count`, `*_num`, `page`, `limit`, `offset` → integer  
/// - `*_uuid`, `uuid` → string with uuid format
/// - `year`, `month`, `day` → integer
/// - Everything else → string
fn infer_path_param_schema(name: &str) -> rustapi_openapi::SchemaRef {
    let lower = name.to_lowercase();

    // UUID patterns (check first to avoid false positive from "id" suffix)
    let is_uuid = lower == "uuid" || lower.ends_with("_uuid") || lower.ends_with("uuid");

    if is_uuid {
        return rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "string",
            "format": "uuid"
        }));
    }

    // Integer patterns
    // Integer patterns
    let is_integer = lower == "page"
        || lower == "limit"
        || lower == "offset"
        || lower == "count"
        || lower.ends_with("_count")
        || lower.ends_with("_num")
        || lower == "year"
        || lower == "month"
        || lower == "day"
        || lower == "index"
        || lower == "position";

    if is_integer {
        rustapi_openapi::SchemaRef::Inline(serde_json::json!({
            "type": "integer",
            "format": "int64"
        }))
    } else {
        rustapi_openapi::SchemaRef::Inline(serde_json::json!({ "type": "string" }))
    }
}

/// Normalize a prefix for OpenAPI paths.
///
/// Ensures the prefix:
/// - Starts with exactly one leading slash
/// - Has no trailing slash (unless it's just "/")
/// - Has no double slashes
fn normalize_prefix_for_openapi(prefix: &str) -> String {
    // Handle empty string
    if prefix.is_empty() {
        return "/".to_string();
    }

    // Split by slashes and filter out empty segments (handles multiple slashes)
    let segments: Vec<&str> = prefix.split('/').filter(|s| !s.is_empty()).collect();

    // If no segments after filtering, return root
    if segments.is_empty() {
        return "/".to_string();
    }

    // Build the normalized prefix with leading slash
    let mut result = String::with_capacity(prefix.len() + 1);
    for segment in segments {
        result.push('/');
        result.push_str(segment);
    }

    result
}

impl Default for RustApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::RustApi;
    use crate::extract::{FromRequestParts, State};
    use crate::path_params::PathParams;
    use crate::request::Request;
    use crate::router::{get, post, Router};
    use bytes::Bytes;
    use http::Method;
    use proptest::prelude::*;

    #[test]
    fn state_is_available_via_extractor() {
        let app = RustApi::new().state(123u32);
        let router = app.into_router();

        let req = http::Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(())
            .unwrap();
        let (parts, _) = req.into_parts();

        let request = Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            router.state_ref(),
            PathParams::new(),
        );
        let State(value) = State::<u32>::from_request_parts(&request).unwrap();
        assert_eq!(value, 123u32);
    }

    #[test]
    fn test_path_param_type_inference_integer() {
        use super::infer_path_param_schema;

        // Test common integer patterns
        let int_params = [
            "page",
            "limit",
            "offset",
            "count",
            "item_count",
            "year",
            "month",
            "day",
            "index",
            "position",
        ];

        for name in int_params {
            let schema = infer_path_param_schema(name);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(
                        v.get("type").and_then(|v| v.as_str()),
                        Some("integer"),
                        "Expected '{}' to be inferred as integer",
                        name
                    );
                }
                _ => panic!("Expected inline schema for '{}'", name),
            }
        }
    }

    #[test]
    fn test_path_param_type_inference_uuid() {
        use super::infer_path_param_schema;

        // Test UUID patterns
        let uuid_params = ["uuid", "user_uuid", "sessionUuid"];

        for name in uuid_params {
            let schema = infer_path_param_schema(name);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(
                        v.get("type").and_then(|v| v.as_str()),
                        Some("string"),
                        "Expected '{}' to be inferred as string",
                        name
                    );
                    assert_eq!(
                        v.get("format").and_then(|v| v.as_str()),
                        Some("uuid"),
                        "Expected '{}' to have uuid format",
                        name
                    );
                }
                _ => panic!("Expected inline schema for '{}'", name),
            }
        }
    }

    #[test]
    fn test_path_param_type_inference_string() {
        use super::infer_path_param_schema;

        // Test string (default) patterns
        let string_params = [
            "name", "slug", "code", "token", "username", "id", "user_id", "userId", "postId",
        ];

        for name in string_params {
            let schema = infer_path_param_schema(name);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(
                        v.get("type").and_then(|v| v.as_str()),
                        Some("string"),
                        "Expected '{}' to be inferred as string",
                        name
                    );
                    assert!(
                        v.get("format").is_none()
                            || v.get("format").and_then(|v| v.as_str()) != Some("uuid"),
                        "Expected '{}' to NOT have uuid format",
                        name
                    );
                }
                _ => panic!("Expected inline schema for '{}'", name),
            }
        }
    }

    #[test]
    fn test_schema_type_to_openapi_schema() {
        use super::schema_type_to_openapi_schema;

        // Test UUID schema
        let uuid_schema = schema_type_to_openapi_schema("uuid");
        match uuid_schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("string"));
                assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("uuid"));
            }
            _ => panic!("Expected inline schema for uuid"),
        }

        // Test integer schemas
        for schema_type in ["integer", "int", "int64", "i64"] {
            let schema = schema_type_to_openapi_schema(schema_type);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("integer"));
                    assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("int64"));
                }
                _ => panic!("Expected inline schema for {}", schema_type),
            }
        }

        // Test int32 schema
        let int32_schema = schema_type_to_openapi_schema("int32");
        match int32_schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("integer"));
                assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("int32"));
            }
            _ => panic!("Expected inline schema for int32"),
        }

        // Test number/float schema
        for schema_type in ["number", "float"] {
            let schema = schema_type_to_openapi_schema(schema_type);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("number"));
                }
                _ => panic!("Expected inline schema for {}", schema_type),
            }
        }

        // Test boolean schema
        for schema_type in ["boolean", "bool"] {
            let schema = schema_type_to_openapi_schema(schema_type);
            match schema {
                rustapi_openapi::SchemaRef::Inline(v) => {
                    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("boolean"));
                }
                _ => panic!("Expected inline schema for {}", schema_type),
            }
        }

        // Test string schema (default)
        let string_schema = schema_type_to_openapi_schema("string");
        match string_schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("string"));
            }
            _ => panic!("Expected inline schema for string"),
        }
    }

    // **Feature: router-nesting, Property 11: OpenAPI Integration**
    //
    // For any nested routes with OpenAPI operations, the operations should appear
    // in the parent's OpenAPI spec with prefixed paths and preserved metadata.
    //
    // **Validates: Requirements 4.1, 4.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Nested routes appear in OpenAPI spec with prefixed paths
        ///
        /// For any router with routes nested under a prefix, all routes should
        /// appear in the OpenAPI spec with the prefix prepended to their paths.
        #[test]
        fn prop_nested_routes_in_openapi_spec(
            // Generate prefix segments
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            // Generate route path segments
            route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            has_param in any::<bool>(),
        ) {
            async fn handler() -> &'static str { "handler" }

            // Build the prefix
            let prefix = format!("/{}", prefix_segments.join("/"));

            // Build the route path
            let mut route_path = format!("/{}", route_segments.join("/"));
            if has_param {
                route_path.push_str("/{id}");
            }

            // Create nested router and nest it through RustApi
            let nested_router = Router::new().route(&route_path, get(handler));
            let app = RustApi::new().nest(&prefix, nested_router);

            // Build expected prefixed path for OpenAPI (uses {param} format)
            let expected_openapi_path = format!("{}{}", prefix, route_path);

            // Get the OpenAPI spec
            let spec = app.openapi_spec();

            // Property: The prefixed route should exist in OpenAPI paths
            prop_assert!(
                spec.paths.contains_key(&expected_openapi_path),
                "Expected OpenAPI path '{}' not found. Available paths: {:?}",
                expected_openapi_path,
                spec.paths.keys().collect::<Vec<_>>()
            );

            // Property: The path item should have a GET operation
            let path_item = spec.paths.get(&expected_openapi_path).unwrap();
            prop_assert!(
                path_item.get.is_some(),
                "GET operation should exist for path '{}'",
                expected_openapi_path
            );
        }

        /// Property: Multiple HTTP methods are preserved in OpenAPI spec after nesting
        ///
        /// For any router with routes having multiple HTTP methods, nesting should
        /// preserve all method operations in the OpenAPI spec.
        #[test]
        fn prop_multiple_methods_preserved_in_openapi(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        ) {
            async fn get_handler() -> &'static str { "get" }
            async fn post_handler() -> &'static str { "post" }

            // Build the prefix and route path
            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{}", route_segments.join("/"));

            // Create nested router with both GET and POST using separate routes
            // Since MethodRouter doesn't have chaining methods, we create two routes
            let get_route_path = format!("{}/get", route_path);
            let post_route_path = format!("{}/post", route_path);
            let nested_router = Router::new()
                .route(&get_route_path, get(get_handler))
                .route(&post_route_path, post(post_handler));
            let app = RustApi::new().nest(&prefix, nested_router);

            // Build expected prefixed paths for OpenAPI
            let expected_get_path = format!("{}{}", prefix, get_route_path);
            let expected_post_path = format!("{}{}", prefix, post_route_path);

            // Get the OpenAPI spec
            let spec = app.openapi_spec();

            // Property: Both paths should exist
            prop_assert!(
                spec.paths.contains_key(&expected_get_path),
                "Expected OpenAPI path '{}' not found",
                expected_get_path
            );
            prop_assert!(
                spec.paths.contains_key(&expected_post_path),
                "Expected OpenAPI path '{}' not found",
                expected_post_path
            );

            // Property: GET operation should exist on get path
            let get_path_item = spec.paths.get(&expected_get_path).unwrap();
            prop_assert!(
                get_path_item.get.is_some(),
                "GET operation should exist for path '{}'",
                expected_get_path
            );

            // Property: POST operation should exist on post path
            let post_path_item = spec.paths.get(&expected_post_path).unwrap();
            prop_assert!(
                post_path_item.post.is_some(),
                "POST operation should exist for path '{}'",
                expected_post_path
            );
        }

        /// Property: Path parameters are added to OpenAPI operations after nesting
        ///
        /// For any nested route with path parameters, the OpenAPI operation should
        /// include the path parameters.
        #[test]
        fn prop_path_params_in_openapi_after_nesting(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            param_name in "[a-z][a-z0-9]{0,5}",
        ) {
            async fn handler() -> &'static str { "handler" }

            // Build the prefix and route path with parameter
            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{{{}}}", param_name);

            // Create nested router
            let nested_router = Router::new().route(&route_path, get(handler));
            let app = RustApi::new().nest(&prefix, nested_router);

            // Build expected prefixed path for OpenAPI
            let expected_openapi_path = format!("{}{}", prefix, route_path);

            // Get the OpenAPI spec
            let spec = app.openapi_spec();

            // Property: The path should exist
            prop_assert!(
                spec.paths.contains_key(&expected_openapi_path),
                "Expected OpenAPI path '{}' not found",
                expected_openapi_path
            );

            // Property: The GET operation should have the path parameter
            let path_item = spec.paths.get(&expected_openapi_path).unwrap();
            let get_op = path_item.get.as_ref().unwrap();

            prop_assert!(
                get_op.parameters.is_some(),
                "Operation should have parameters for path '{}'",
                expected_openapi_path
            );

            let params = get_op.parameters.as_ref().unwrap();
            let has_param = params.iter().any(|p| p.name == param_name && p.location == "path");
            prop_assert!(
                has_param,
                "Path parameter '{}' should exist in operation parameters. Found: {:?}",
                param_name,
                params.iter().map(|p| &p.name).collect::<Vec<_>>()
            );
        }
    }

    // **Feature: router-nesting, Property 13: RustApi Integration**
    //
    // For any router nested through `RustApi::new().nest()`, the behavior should be
    // identical to nesting through `Router::new().nest()`, and routes should appear
    // in the OpenAPI spec.
    //
    // **Validates: Requirements 6.1, 6.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: RustApi::nest delegates to Router::nest and produces identical route registration
        ///
        /// For any router with routes nested under a prefix, nesting through RustApi
        /// should produce the same route registration as nesting through Router directly.
        #[test]
        fn prop_rustapi_nest_delegates_to_router_nest(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            has_param in any::<bool>(),
        ) {
            async fn handler() -> &'static str { "handler" }

            // Build the prefix
            let prefix = format!("/{}", prefix_segments.join("/"));

            // Build the route path
            let mut route_path = format!("/{}", route_segments.join("/"));
            if has_param {
                route_path.push_str("/{id}");
            }

            // Create nested router
            let nested_router_for_rustapi = Router::new().route(&route_path, get(handler));
            let nested_router_for_router = Router::new().route(&route_path, get(handler));

            // Nest through RustApi
            let rustapi_app = RustApi::new().nest(&prefix, nested_router_for_rustapi);
            let rustapi_router = rustapi_app.into_router();

            // Nest through Router directly
            let router_app = Router::new().nest(&prefix, nested_router_for_router);

            // Property: Both should have the same registered routes
            let rustapi_routes = rustapi_router.registered_routes();
            let router_routes = router_app.registered_routes();

            prop_assert_eq!(
                rustapi_routes.len(),
                router_routes.len(),
                "RustApi and Router should have same number of routes"
            );

            // Property: All routes from Router should exist in RustApi
            for (path, info) in router_routes {
                prop_assert!(
                    rustapi_routes.contains_key(path),
                    "Route '{}' from Router should exist in RustApi routes",
                    path
                );

                let rustapi_info = rustapi_routes.get(path).unwrap();
                prop_assert_eq!(
                    &info.path, &rustapi_info.path,
                    "Display paths should match for route '{}'",
                    path
                );
                prop_assert_eq!(
                    info.methods.len(), rustapi_info.methods.len(),
                    "Method count should match for route '{}'",
                    path
                );
            }
        }

        /// Property: RustApi::nest includes nested routes in OpenAPI spec
        ///
        /// For any router with routes nested through RustApi, all routes should
        /// appear in the OpenAPI specification with prefixed paths.
        #[test]
        fn prop_rustapi_nest_includes_routes_in_openapi(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            has_param in any::<bool>(),
        ) {
            async fn handler() -> &'static str { "handler" }

            // Build the prefix
            let prefix = format!("/{}", prefix_segments.join("/"));

            // Build the route path
            let mut route_path = format!("/{}", route_segments.join("/"));
            if has_param {
                route_path.push_str("/{id}");
            }

            // Create nested router and nest through RustApi
            let nested_router = Router::new().route(&route_path, get(handler));
            let app = RustApi::new().nest(&prefix, nested_router);

            // Build expected prefixed path for OpenAPI
            let expected_openapi_path = format!("{}{}", prefix, route_path);

            // Get the OpenAPI spec
            let spec = app.openapi_spec();

            // Property: The prefixed route should exist in OpenAPI paths
            prop_assert!(
                spec.paths.contains_key(&expected_openapi_path),
                "Expected OpenAPI path '{}' not found. Available paths: {:?}",
                expected_openapi_path,
                spec.paths.keys().collect::<Vec<_>>()
            );

            // Property: The path item should have a GET operation
            let path_item = spec.paths.get(&expected_openapi_path).unwrap();
            prop_assert!(
                path_item.get.is_some(),
                "GET operation should exist for path '{}'",
                expected_openapi_path
            );
        }

        /// Property: RustApi::nest route matching is identical to Router::nest
        ///
        /// For any nested route, matching through RustApi should produce the same
        /// result as matching through Router directly.
        #[test]
        fn prop_rustapi_nest_route_matching_identical(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
            param_value in "[a-z0-9]{1,10}",
        ) {
            use crate::router::RouteMatch;

            async fn handler() -> &'static str { "handler" }

            // Build the prefix and route path with parameter
            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{}/{{id}}", route_segments.join("/"));

            // Create nested routers
            let nested_router_for_rustapi = Router::new().route(&route_path, get(handler));
            let nested_router_for_router = Router::new().route(&route_path, get(handler));

            // Nest through both RustApi and Router
            let rustapi_app = RustApi::new().nest(&prefix, nested_router_for_rustapi);
            let rustapi_router = rustapi_app.into_router();
            let router_app = Router::new().nest(&prefix, nested_router_for_router);

            // Build the full path to match
            let full_path = format!("{}/{}/{}", prefix, route_segments.join("/"), param_value);

            // Match through both
            let rustapi_match = rustapi_router.match_route(&full_path, &Method::GET);
            let router_match = router_app.match_route(&full_path, &Method::GET);

            // Property: Both should return Found with same parameters
            match (rustapi_match, router_match) {
                (RouteMatch::Found { params: rustapi_params, .. }, RouteMatch::Found { params: router_params, .. }) => {
                    prop_assert_eq!(
                        rustapi_params.len(),
                        router_params.len(),
                        "Parameter count should match"
                    );
                    for (key, value) in &router_params {
                        prop_assert!(
                            rustapi_params.contains_key(key),
                            "RustApi should have parameter '{}'",
                            key
                        );
                        prop_assert_eq!(
                            rustapi_params.get(key).unwrap(),
                            value,
                            "Parameter '{}' value should match",
                            key
                        );
                    }
                }
                (rustapi_result, router_result) => {
                    prop_assert!(
                        false,
                        "Both should return Found, but RustApi returned {:?} and Router returned {:?}",
                        match rustapi_result {
                            RouteMatch::Found { .. } => "Found",
                            RouteMatch::NotFound => "NotFound",
                            RouteMatch::MethodNotAllowed { .. } => "MethodNotAllowed",
                        },
                        match router_result {
                            RouteMatch::Found { .. } => "Found",
                            RouteMatch::NotFound => "NotFound",
                            RouteMatch::MethodNotAllowed { .. } => "MethodNotAllowed",
                        }
                    );
                }
            }
        }
    }

    /// Unit test: Verify OpenAPI operations are propagated during nesting
    #[test]
    fn test_openapi_operations_propagated_during_nesting() {
        async fn list_users() -> &'static str {
            "list users"
        }
        async fn get_user() -> &'static str {
            "get user"
        }
        async fn create_user() -> &'static str {
            "create user"
        }

        // Create nested router with multiple routes
        // Note: We use separate routes since MethodRouter doesn't support chaining
        let users_router = Router::new()
            .route("/", get(list_users))
            .route("/create", post(create_user))
            .route("/{id}", get(get_user));

        // Nest under /api/v1/users
        let app = RustApi::new().nest("/api/v1/users", users_router);

        let spec = app.openapi_spec();

        // Verify /api/v1/users path exists with GET
        assert!(
            spec.paths.contains_key("/api/v1/users"),
            "Should have /api/v1/users path"
        );
        let users_path = spec.paths.get("/api/v1/users").unwrap();
        assert!(users_path.get.is_some(), "Should have GET operation");

        // Verify /api/v1/users/create path exists with POST
        assert!(
            spec.paths.contains_key("/api/v1/users/create"),
            "Should have /api/v1/users/create path"
        );
        let create_path = spec.paths.get("/api/v1/users/create").unwrap();
        assert!(create_path.post.is_some(), "Should have POST operation");

        // Verify /api/v1/users/{id} path exists with GET
        assert!(
            spec.paths.contains_key("/api/v1/users/{id}"),
            "Should have /api/v1/users/{{id}} path"
        );
        let user_path = spec.paths.get("/api/v1/users/{id}").unwrap();
        assert!(
            user_path.get.is_some(),
            "Should have GET operation for user by id"
        );

        // Verify path parameter is added
        let get_user_op = user_path.get.as_ref().unwrap();
        assert!(get_user_op.parameters.is_some(), "Should have parameters");
        let params = get_user_op.parameters.as_ref().unwrap();
        assert!(
            params
                .iter()
                .any(|p| p.name == "id" && p.location == "path"),
            "Should have 'id' path parameter"
        );
    }

    /// Unit test: Verify nested routes don't appear without nesting
    #[test]
    fn test_openapi_spec_empty_without_routes() {
        let app = RustApi::new();
        let spec = app.openapi_spec();

        // Should have no paths (except potentially default ones)
        assert!(
            spec.paths.is_empty(),
            "OpenAPI spec should have no paths without routes"
        );
    }

    /// Unit test: Verify RustApi::nest delegates correctly to Router::nest
    ///
    /// **Feature: router-nesting, Property 13: RustApi Integration**
    /// **Validates: Requirements 6.1, 6.2**
    #[test]
    fn test_rustapi_nest_delegates_to_router_nest() {
        use crate::router::RouteMatch;

        async fn list_users() -> &'static str {
            "list users"
        }
        async fn get_user() -> &'static str {
            "get user"
        }
        async fn create_user() -> &'static str {
            "create user"
        }

        // Create nested router with multiple routes
        let users_router = Router::new()
            .route("/", get(list_users))
            .route("/create", post(create_user))
            .route("/{id}", get(get_user));

        // Nest through RustApi
        let app = RustApi::new().nest("/api/v1/users", users_router);
        let router = app.into_router();

        // Verify routes are registered correctly
        let routes = router.registered_routes();
        assert_eq!(routes.len(), 3, "Should have 3 routes registered");

        // Verify route paths
        assert!(
            routes.contains_key("/api/v1/users"),
            "Should have /api/v1/users route"
        );
        assert!(
            routes.contains_key("/api/v1/users/create"),
            "Should have /api/v1/users/create route"
        );
        assert!(
            routes.contains_key("/api/v1/users/:id"),
            "Should have /api/v1/users/:id route"
        );

        // Verify route matching works
        match router.match_route("/api/v1/users", &Method::GET) {
            RouteMatch::Found { params, .. } => {
                assert!(params.is_empty(), "Root route should have no params");
            }
            _ => panic!("GET /api/v1/users should be found"),
        }

        match router.match_route("/api/v1/users/create", &Method::POST) {
            RouteMatch::Found { params, .. } => {
                assert!(params.is_empty(), "Create route should have no params");
            }
            _ => panic!("POST /api/v1/users/create should be found"),
        }

        match router.match_route("/api/v1/users/123", &Method::GET) {
            RouteMatch::Found { params, .. } => {
                assert_eq!(
                    params.get("id"),
                    Some(&"123".to_string()),
                    "Should extract id param"
                );
            }
            _ => panic!("GET /api/v1/users/123 should be found"),
        }

        // Verify method not allowed
        match router.match_route("/api/v1/users", &Method::DELETE) {
            RouteMatch::MethodNotAllowed { allowed } => {
                assert!(allowed.contains(&Method::GET), "Should allow GET");
            }
            _ => panic!("DELETE /api/v1/users should return MethodNotAllowed"),
        }
    }

    /// Unit test: Verify RustApi::nest includes routes in OpenAPI spec
    ///
    /// **Feature: router-nesting, Property 13: RustApi Integration**
    /// **Validates: Requirements 6.1, 6.2**
    #[test]
    fn test_rustapi_nest_includes_routes_in_openapi_spec() {
        async fn list_items() -> &'static str {
            "list items"
        }
        async fn get_item() -> &'static str {
            "get item"
        }

        // Create nested router
        let items_router = Router::new()
            .route("/", get(list_items))
            .route("/{item_id}", get(get_item));

        // Nest through RustApi
        let app = RustApi::new().nest("/api/items", items_router);

        // Verify OpenAPI spec
        let spec = app.openapi_spec();

        // Verify paths exist
        assert!(
            spec.paths.contains_key("/api/items"),
            "Should have /api/items in OpenAPI"
        );
        assert!(
            spec.paths.contains_key("/api/items/{item_id}"),
            "Should have /api/items/{{item_id}} in OpenAPI"
        );

        // Verify operations
        let list_path = spec.paths.get("/api/items").unwrap();
        assert!(
            list_path.get.is_some(),
            "Should have GET operation for /api/items"
        );

        let get_path = spec.paths.get("/api/items/{item_id}").unwrap();
        assert!(
            get_path.get.is_some(),
            "Should have GET operation for /api/items/{{item_id}}"
        );

        // Verify path parameter is added
        let get_op = get_path.get.as_ref().unwrap();
        assert!(get_op.parameters.is_some(), "Should have parameters");
        let params = get_op.parameters.as_ref().unwrap();
        assert!(
            params
                .iter()
                .any(|p| p.name == "item_id" && p.location == "path"),
            "Should have 'item_id' path parameter"
        );
    }
}

/// Check Basic Auth header against expected credentials
#[cfg(feature = "swagger-ui")]
fn check_basic_auth(req: &crate::Request, expected: &str) -> bool {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|auth| auth == expected)
        .unwrap_or(false)
}

/// Create 401 Unauthorized response with WWW-Authenticate header
#[cfg(feature = "swagger-ui")]
fn unauthorized_response() -> crate::Response {
    http::Response::builder()
        .status(http::StatusCode::UNAUTHORIZED)
        .header(
            http::header::WWW_AUTHENTICATE,
            "Basic realm=\"API Documentation\"",
        )
        .header(http::header::CONTENT_TYPE, "text/plain")
        .body(crate::response::Body::from("Unauthorized"))
        .unwrap()
}

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
