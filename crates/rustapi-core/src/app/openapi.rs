#[cfg(feature = "swagger-ui")]
use super::helpers::{check_basic_auth, unauthorized_response};
use super::types::RustApi;

impl RustApi {
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
    pub(super) fn maybe_dump_openapi(&self) {
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
}
