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
    pub fn state<S: Clone + Send + Sync + 'static>(mut self, state: S) -> Self {
        self.router = self.router.state(state);
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
        self.router = self.router.route(path, method_router);
        self
    }

    /// Mount a handler (convenience method)
    ///
    /// Alias for `.route(path, method_router)` for a single handler.
    pub fn mount(self, path: &str, method_router: MethodRouter) -> Self {
        self.route(path, method_router)
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

    /// Enable Swagger UI at the specified path
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .docs("/docs")  // Swagger UI at /docs
    /// ```
    pub fn docs(self, _path: &str) -> Self {
        // TODO: Implement OpenAPI + Swagger UI
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
