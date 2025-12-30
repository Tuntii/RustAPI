//! RustApi application builder

use crate::error::Result;
use crate::router::{MethodRouter, Router};
use crate::server::Server;
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
}

impl RustApi {
    /// Create a new RustAPI application
    pub fn new() -> Self {
        // Initialize tracing if not already done
        let _ = tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("info,rustapi=debug")
            }))
            .with(tracing_subscriber::fmt::layer())
            .try_init();

        Self {
            router: Router::new(),
            openapi_spec: rustapi_openapi::OpenApiSpec::new("RustAPI Application", "1.0.0"),
        }
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
    pub fn state<S>(self, state: S) -> Self
    where
        S: Clone + Send + Sync + 'static,
    {
        // For now, state is handled by the router/handlers directly capturing it 
        // or through a middleware. The current router (matchit) implementation 
        // doesn't support state injection directly in the same way axum does.
        // This is a placeholder for future state management.
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
    pub fn register_schema<T: for<'a> rustapi_openapi::Schema<'a>>(mut self) -> Self {
        self.openapi_spec = self.openapi_spec.register::<T>();
        self
    }
    
    /// Configure OpenAPI info (title, version, description)
    pub fn openapi_info(mut self, title: &str, version: &str, description: Option<&str>) -> Self {
        self.openapi_spec = rustapi_openapi::OpenApiSpec::new(title, version);
        if let Some(desc) = description {
            self.openapi_spec = self.openapi_spec.description(desc);
        }
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
            self.openapi_spec = self.openapi_spec.path(path, method.as_str(), op.clone());
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

    /// Mount a route created with #[rustapi::get], #[rustapi::post], etc.
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
        self.openapi_spec = self.openapi_spec.path(route.path, route.method, route.operation);
        
        self.route_with_method(route.path, method_enum, route.handler)
    }

    /// Helper to mount a single method handler
    fn route_with_method(mut self, path: &str, method: http::Method, handler: crate::handler::BoxedHandler) -> Self {
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

    /// Enable Swagger UI documentation
    ///
    /// This adds two endpoints:
    /// - `{path}` - Swagger UI interface
    /// - `{path}/openapi.json` - OpenAPI JSON specification
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .route("/users", get(list_users))
    ///     .docs("/docs")  // Swagger UI at /docs, spec at /docs/openapi.json
    ///     .run("127.0.0.1:8080")
    ///     .await
    /// ```
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
        let spec_json = serde_json::to_string_pretty(&self.openapi_spec.to_json()).unwrap_or_default();
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
            async move {
                let html = rustapi_openapi::swagger_ui_html(&url);
                html
            }
        };
        
        self.route(&openapi_path, get(spec_handler))
            .route(path, get(docs_handler))
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
    pub async fn run(self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server = Server::new(self.router);
        server.run(addr).await
    }

    /// Get the inner router (for testing or advanced usage)
    pub fn into_router(self) -> Router {
        self.router
    }
}

impl Default for RustApi {
    fn default() -> Self {
        Self::new()
    }
}
