use super::config::RustApiConfig;
use super::dispatcher::RequestDispatcher;
use super::types::RustApi;
use crate::events::LifecycleHooks;
use crate::interceptor::{InterceptorChain, RequestInterceptor, ResponseInterceptor};
use crate::middleware::{LayerStack, MiddlewareLayer, DEFAULT_BODY_LIMIT};
use crate::router::Router;
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
    /// at link time via `linkme` and registers them automatically ├óÔé¼ÔÇØ no manual `.route()`
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
    ///         println!("─ş┼©┼íÔé¼ Server starting...");
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
    ///         println!("─ş┼©ÔÇİÔÇ╣ Server shutting down...");
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
}

impl Default for RustApi {
    fn default() -> Self {
        Self::new()
    }
}
