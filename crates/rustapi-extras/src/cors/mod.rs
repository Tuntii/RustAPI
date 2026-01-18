//! CORS (Cross-Origin Resource Sharing) middleware.
//!
//! This module provides configurable CORS middleware with a builder pattern
//! for controlling cross-origin access to your API.
//!
//! # Example
//!
//! ```ignore
//! use rustapi_extras::cors::CorsLayer;
//!
//! let cors = CorsLayer::new()
//!     .allow_origins(["https://example.com"])
//!     .allow_methods([Method::GET, Method::POST])
//!     .allow_credentials(true);
//! ```

use bytes::Bytes;
use http::{header, Method, StatusCode};
use http_body_util::Full;
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Specifies which origins are allowed for CORS requests.
#[derive(Debug, Clone)]
pub enum AllowedOrigins {
    /// Allow any origin (`Access-Control-Allow-Origin: *`).
    Any,
    /// Allow only specific origins.
    List(Vec<String>),
}

impl Default for AllowedOrigins {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

/// CORS middleware layer with builder pattern configuration.
///
/// Handles preflight OPTIONS requests and adds appropriate CORS headers
/// to responses.
#[derive(Debug, Clone)]
pub struct CorsLayer {
    origins: AllowedOrigins,
    methods: Vec<Method>,
    headers: Vec<String>,
    credentials: bool,
    max_age: Option<Duration>,
}

impl Default for CorsLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsLayer {
    /// Create a new CORS layer with restrictive defaults.
    pub fn new() -> Self {
        Self {
            origins: AllowedOrigins::default(),
            methods: vec![Method::GET, Method::HEAD, Method::OPTIONS],
            headers: Vec::new(),
            credentials: false,
            max_age: None,
        }
    }

    /// Create a permissive CORS layer that allows everything.
    ///
    /// This is useful for development but should be used with caution
    /// in production.
    pub fn permissive() -> Self {
        Self {
            origins: AllowedOrigins::Any,
            methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::HEAD,
                Method::OPTIONS,
            ],
            headers: vec!["*".to_string()],
            credentials: false,
            max_age: Some(Duration::from_secs(86400)),
        }
    }

    /// Create a restrictive CORS layer with minimal permissions.
    pub fn restrictive() -> Self {
        Self::new()
    }

    /// Allow any origin.
    pub fn allow_any_origin(mut self) -> Self {
        self.origins = AllowedOrigins::Any;
        self
    }

    /// Allow specific origins.
    pub fn allow_origins<I, S>(mut self, origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.origins = AllowedOrigins::List(origins.into_iter().map(Into::into).collect());
        self
    }

    /// Allow specific HTTP methods.
    pub fn allow_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator<Item = Method>,
    {
        self.methods = methods.into_iter().collect();
        self
    }

    /// Allow specific headers.
    pub fn allow_headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Allow credentials (cookies, authorization headers).
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.credentials = allow;
        self
    }

    /// Set the max age for preflight cache.
    pub fn max_age(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    /// Get the configured origins.
    pub fn origins(&self) -> &AllowedOrigins {
        &self.origins
    }

    /// Get the configured methods.
    pub fn methods(&self) -> &[Method] {
        &self.methods
    }

    /// Get the configured headers.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    /// Check if credentials are allowed.
    pub fn credentials(&self) -> bool {
        self.credentials
    }

    /// Get the max age configuration.
    pub fn max_age_duration(&self) -> Option<Duration> {
        self.max_age
    }

    /// Build the Access-Control-Allow-Methods header value.
    fn methods_header_value(&self) -> String {
        self.methods
            .iter()
            .map(|m| m.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Build the Access-Control-Allow-Headers header value.
    fn headers_header_value(&self) -> String {
        if self.headers.is_empty() {
            "Content-Type, Authorization".to_string()
        } else {
            self.headers.join(", ")
        }
    }
}

impl MiddlewareLayer for CorsLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let origins = self.origins.clone();
        let methods = self.methods_header_value();
        let allow_headers = if self.headers.len() == 1 && self.headers.first().map(|value| value == "*").unwrap_or(false) {
            req.headers()
                .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| "*".to_string())
        } else {
            self.headers_header_value()
        };
        let credentials = self.credentials;
        let max_age = self.max_age;
        let is_any_origin = matches!(origins, AllowedOrigins::Any);

        // Extract origin from request
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // Check if this is a preflight request
        let is_preflight = req.method() == Method::OPTIONS
            && req
                .headers()
                .contains_key(header::ACCESS_CONTROL_REQUEST_METHOD);

        // Clone self for origin check
        let is_origin_allowed = origin
            .as_ref()
            .map(|o| match &origins {
                AllowedOrigins::Any => true,
                AllowedOrigins::List(list) => list.iter().any(|allowed| allowed == o),
            })
            .unwrap_or(false);

        Box::pin(async move {
            // Handle preflight request
            if is_preflight {
                let mut response = http::Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Full::new(Bytes::new()))
                    .unwrap();

                let headers_mut = response.headers_mut();

                // Set Allow-Origin
                if let Some(ref origin) = origin {
                    if is_origin_allowed {
                        if is_any_origin && !credentials {
                            headers_mut
                                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                        } else {
                            headers_mut.insert(
                                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                                origin.parse().unwrap(),
                            );
                        }
                    }
                }

                // Set Allow-Methods
                headers_mut.insert(
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    methods.parse().unwrap(),
                );

                // Set Allow-Headers
                headers_mut.insert(
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    allow_headers.parse().unwrap(),
                );

                // Set Allow-Credentials
                if credentials {
                    headers_mut.insert(
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        "true".parse().unwrap(),
                    );
                }

                // Set Max-Age
                if let Some(max_age) = max_age {
                    headers_mut.insert(
                        header::ACCESS_CONTROL_MAX_AGE,
                        max_age.as_secs().to_string().parse().unwrap(),
                    );
                }

                return response;
            }

            // Process the actual request
            let mut response = next(req).await;

            // Add CORS headers to the response
            if let Some(ref origin) = origin {
                if is_origin_allowed {
                    let headers_mut = response.headers_mut();

                    if is_any_origin && !credentials {
                        headers_mut
                            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
                    } else {
                        headers_mut
                            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
                    }

                    if credentials {
                        headers_mut.insert(
                            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                            "true".parse().unwrap(),
                        );
                    }

                    // Expose headers that the browser can access
                    headers_mut.insert(
                        header::ACCESS_CONTROL_EXPOSE_HEADERS,
                        "Content-Length, Content-Type".parse().unwrap(),
                    );
                }
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}
