//! RustApi application builder

use crate::error::Result;
use crate::router::{get, MethodRouter, Router};
use crate::server::Server;
use crate::response::{Html, Response};
use crate::extract::Json;
use rustapi_openapi::{OpenApiDoc, swagger_ui_html};
use std::sync::Arc;
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
    openapi: Option<OpenApiDoc>,
    docs_path: Option<String>,
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
            openapi: None,
            docs_path: None,
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

    /// Configure OpenAPI documentation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_openapi::OpenApiDoc;
    ///
    /// RustApi::new()
    ///     .openapi(OpenApiDoc::new("My API", "1.0.0")
    ///         .description("A sample API")
    ///         .server("http://localhost:8080"))
    /// ```
    pub fn openapi(mut self, doc: OpenApiDoc) -> Self {
        self.openapi = Some(doc);
        self
    }

    /// Enable Swagger UI at the specified path
    ///
    /// Also enables `/openapi.json` endpoint automatically.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// RustApi::new()
    ///     .openapi(OpenApiDoc::new("My API", "1.0.0"))
    ///     .docs("/docs")  // Swagger UI at /docs, spec at /openapi.json
    /// ```
    pub fn docs(mut self, path: &str) -> Self {
        self.docs_path = Some(path.to_string());
        // Create default OpenAPI doc if not set
        if self.openapi.is_none() {
            self.openapi = Some(OpenApiDoc::new("RustAPI", "1.0.0"));
        }
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
        // Add OpenAPI endpoints if configured
        if let Some(doc) = self.openapi.take() {
            let doc = Arc::new(doc);
            let docs_path = self.docs_path.take();
            
            // Add /openapi.json endpoint
            let doc_for_json = doc.clone();
            self.router = self.router.route(
                "/openapi.json",
                get(move || {
                    let doc = doc_for_json.clone();
                    async move {
                        OpenApiJsonResponse(doc.to_json())
                    }
                }),
            );

            // Add Swagger UI endpoint if docs path is set
            if let Some(path) = docs_path {
                let title = doc.title().to_string();
                self.router = self.router.route(
                    &path,
                    get(move || {
                        let title = title.clone();
                        async move {
                            Html(swagger_ui_html("/openapi.json", &title))
                        }
                    }),
                );
                tracing::info!("📚 Swagger UI available at http://{}{}", addr, path);
            }
            
            tracing::info!("📄 OpenAPI spec at http://{}/openapi.json", addr);
        }

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

/// Response type for OpenAPI JSON
struct OpenApiJsonResponse(String);

impl crate::response::IntoResponse for OpenApiJsonResponse {
    fn into_response(self) -> Response {
        use bytes::Bytes;
        use http::{header, StatusCode};
        use http_body_util::Full;

        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(self.0)))
            .unwrap()
    }
}
