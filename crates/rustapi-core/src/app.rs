//! RustApi application builder

use crate::error::Result;
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

        let route_count = by_path
            .values()
            .map(|mr| mr.allowed_methods().len())
            .sum::<usize>();
        let path_count = by_path.len();

        for (path, method_router) in by_path {
            self = self.route(&path, method_router);
        }

        tracing::info!(
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
            add_path_params_to_operation(path, &mut op);
            self.openapi_spec = self.openapi_spec.path(path, method.as_str(), op);
        }

        self.router = self.router.route(path, method_router);
        self
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
        add_path_params_to_operation(route.path, &mut op);
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
                    let relative_path = path
                        .strip_prefix(&config.prefix)
                        .unwrap_or(&path);

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
    /// # Example
    ///
    /// ```text
    /// RustApi::new()
    ///     .route("/users", get(list_users))
    ///     .docs("/docs")  // Swagger UI at /docs, spec at /docs/openapi.json
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
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
                    .body(http_body_util::Full::new(bytes::Bytes::from(json)))
                    .unwrap()
            }
        };

        // Add Swagger UI endpoint
        let docs_handler = move || {
            let url = openapi_url.clone();
            async move { rustapi_openapi::swagger_ui_html(&url) }
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
                        .body(http_body_util::Full::new(bytes::Bytes::from(json)))
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
                    rustapi_openapi::swagger_ui_html(&url)
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
        // Apply body limit layer if configured (should be first in the chain)
        if let Some(limit) = self.body_limit {
            // Prepend body limit layer so it's the first to process requests
            self.layers.prepend(Box::new(BodyLimitLayer::new(limit)));
        }

        let server = Server::new(self.router, self.layers);
        server.run(addr).await
    }

    /// Get the inner router (for testing or advanced usage)
    pub fn into_router(self) -> Router {
        self.router
    }

    /// Get the layer stack (for testing)
    pub fn layers(&self) -> &LayerStack {
        &self.layers
    }
}

fn add_path_params_to_operation(path: &str, op: &mut rustapi_openapi::Operation) {
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

        op_params.push(rustapi_openapi::Parameter {
            name,
            location: "path".to_string(),
            required: true,
            description: None,
            schema: rustapi_openapi::SchemaRef::Inline(serde_json::json!({ "type": "string" })),
        });
    }
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
    use crate::request::Request;
    use bytes::Bytes;
    use http::Method;
    use std::collections::HashMap;

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

        let request = Request::new(parts, Bytes::new(), router.state_ref(), HashMap::new());
        let State(value) = State::<u32>::from_request_parts(&request).unwrap();
        assert_eq!(value, 123u32);
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
        .body(http_body_util::Full::new(bytes::Bytes::from(
            "Unauthorized",
        )))
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
