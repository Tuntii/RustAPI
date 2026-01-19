//! TestClient for integration testing without network binding
//!
//! This module provides a test client that allows sending simulated HTTP requests
//! through the full middleware and handler pipeline without starting a real server.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{RustApi, get};
//! use rustapi_testing::TestClient;
//!
//! async fn hello() -> &'static str {
//!     "Hello, World!"
//! }
//!
//! #[tokio::test]
//! async fn test_hello() {
//!     let app = RustApi::new().route("/", get(hello));
//!     let client = TestClient::new(app);
//!     
//!     let response = client.get("/").await;
//!     response.assert_status(200);
//!     assert_eq!(response.text(), "Hello, World!");
//! }
//! ```

use bytes::Bytes;
use http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use http_body_util::BodyExt;
use rustapi_core::middleware::{BodyLimitLayer, BoxedNext, LayerStack, DEFAULT_BODY_LIMIT};
use rustapi_core::{ApiError, BodyVariant, IntoResponse, Request, Response, RouteMatch, Router};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Test client for integration testing without network binding
///
/// TestClient wraps a RustApi instance and allows sending simulated HTTP requests
/// through the full middleware and handler pipeline.
pub struct TestClient {
    router: Arc<Router>,
    layers: Arc<LayerStack>,
}

impl TestClient {
    /// Create a new test client from a RustApi instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let app = RustApi::new().route("/", get(handler));
    /// let client = TestClient::new(app);
    /// ```
    pub fn new(app: rustapi_core::RustApi) -> Self {
        // Get the router and layers from the app
        let layers = app.layers().clone();
        let router = app.into_router();

        // Apply body limit layer if not already present
        let mut layers = layers;
        layers.prepend(Box::new(BodyLimitLayer::new(DEFAULT_BODY_LIMIT)));

        Self {
            router: Arc::new(router),
            layers: Arc::new(layers),
        }
    }

    /// Create a new test client with custom body limit
    pub fn with_body_limit(app: rustapi_core::RustApi, limit: usize) -> Self {
        let layers = app.layers().clone();
        let router = app.into_router();

        let mut layers = layers;
        layers.prepend(Box::new(BodyLimitLayer::new(limit)));

        Self {
            router: Arc::new(router),
            layers: Arc::new(layers),
        }
    }

    /// Send a GET request
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.get("/users").await;
    /// ```
    pub async fn get(&self, path: &str) -> TestResponse {
        self.request(TestRequest::get(path)).await
    }

    /// Send a POST request with JSON body
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.post_json("/users", &CreateUser { name: "Alice" }).await;
    /// ```
    pub async fn post_json<T: Serialize>(&self, path: &str, body: &T) -> TestResponse {
        self.request(TestRequest::post(path).json(body)).await
    }

    /// Send a request with full control
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let response = client.request(
    ///     TestRequest::put("/users/1")
    ///         .header("Authorization", "Bearer token")
    ///         .json(&UpdateUser { name: "Bob" })
    /// ).await;
    /// ```
    pub async fn request(&self, req: TestRequest) -> TestResponse {
        let method = req.method.clone();
        let path = req.path.clone();

        // Match the route to get path params
        let (handler, params) = match self.router.match_route(&path, &method) {
            RouteMatch::Found { handler, params } => (handler.clone(), params),
            RouteMatch::NotFound => {
                let response =
                    ApiError::not_found(format!("No route found for {} {}", method, path))
                        .into_response();
                return TestResponse::from_response(response).await;
            }
            RouteMatch::MethodNotAllowed { allowed } => {
                let allowed_str: Vec<&str> = allowed.iter().map(|m| m.as_str()).collect();
                let mut response = ApiError::new(
                    StatusCode::METHOD_NOT_ALLOWED,
                    "method_not_allowed",
                    format!("Method {} not allowed for {}", method, path),
                )
                .into_response();

                response
                    .headers_mut()
                    .insert(header::ALLOW, allowed_str.join(", ").parse().unwrap());
                return TestResponse::from_response(response).await;
            }
        };

        // Build the internal Request
        let uri: http::Uri = path.parse().unwrap_or_else(|_| "/".parse().unwrap());
        let mut builder = http::Request::builder().method(method).uri(uri);

        // Add headers
        for (key, value) in req.headers.iter() {
            builder = builder.header(key, value);
        }

        let http_req = builder.body(()).unwrap();
        let (parts, _) = http_req.into_parts();

        let body_bytes = req.body.unwrap_or_default();

        let request = Request::new(
            parts,
            BodyVariant::Buffered(body_bytes),
            self.router.state_ref(),
            params,
        );

        // Create the final handler as a BoxedNext
        let final_handler: BoxedNext = Arc::new(move |req: Request| {
            let handler = handler.clone();
            Box::pin(async move { handler(req).await })
                as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        // Execute through middleware stack
        let response = self.layers.execute(request, final_handler).await;

        TestResponse::from_response(response).await
    }
}

/// Test request builder
///
/// Provides a fluent API for building test requests with custom methods,
/// headers, and body content.
#[derive(Debug, Clone)]
pub struct TestRequest {
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Option<Bytes>,
}

impl TestRequest {
    /// Create a new request with the given method and path
    fn new(method: Method, path: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Create a GET request
    pub fn get(path: &str) -> Self {
        Self::new(Method::GET, path)
    }

    /// Create a POST request
    pub fn post(path: &str) -> Self {
        Self::new(Method::POST, path)
    }

    /// Create a PUT request
    pub fn put(path: &str) -> Self {
        Self::new(Method::PUT, path)
    }

    /// Create a PATCH request
    pub fn patch(path: &str) -> Self {
        Self::new(Method::PATCH, path)
    }

    /// Create a DELETE request
    pub fn delete(path: &str) -> Self {
        Self::new(Method::DELETE, path)
    }

    /// Add a header to the request
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let req = TestRequest::get("/")
    ///     .header("Authorization", "Bearer token")
    ///     .header("Accept", "application/json");
    /// ```
    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let (Ok(name), Ok(val)) = (
            key.parse::<http::header::HeaderName>(),
            HeaderValue::from_str(value),
        ) {
            self.headers.insert(name, val);
        }
        self
    }

    /// Set the request body as JSON
    ///
    /// This automatically sets the Content-Type header to `application/json`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let req = TestRequest::post("/users")
    ///     .json(&CreateUser { name: "Alice" });
    /// ```
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        match serde_json::to_vec(body) {
            Ok(bytes) => {
                self.body = Some(Bytes::from(bytes));
                self.headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
            }
            Err(_) => {
                // If serialization fails, leave body empty
            }
        }
        self
    }

    /// Set the request body as raw bytes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let req = TestRequest::post("/upload")
    ///     .body("raw content");
    /// ```
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the Content-Type header
    pub fn content_type(self, content_type: &str) -> Self {
        self.header("content-type", content_type)
    }
}

/// Test response with assertion helpers
///
/// Provides methods to inspect and assert on the response status, headers, and body.
#[derive(Debug)]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl TestResponse {
    /// Create a TestResponse from an HTTP response
    async fn from_response(response: Response) -> Self {
        let (parts, body) = response.into_parts();
        let body_bytes = body
            .collect()
            .await
            .map(|b| b.to_bytes())
            .unwrap_or_default();

        Self {
            status: parts.status,
            headers: parts.headers,
            body: body_bytes,
        }
    }

    /// Get the response status code
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the response headers
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get the response body as bytes
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Get the response body as a string
    ///
    /// Returns an empty string if the body is not valid UTF-8.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// Parse the response body as JSON
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let user: User = response.json().unwrap();
    /// ```
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Assert that the response has the expected status code
    ///
    /// # Panics
    ///
    /// Panics if the status code doesn't match.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// response.assert_status(StatusCode::OK);
    /// response.assert_status(200);
    /// ```
    pub fn assert_status<S: Into<StatusCode>>(&self, expected: S) -> &Self {
        let expected = expected.into();
        assert_eq!(
            self.status,
            expected,
            "Expected status {}, got {}. Body: {}",
            expected,
            self.status,
            self.text()
        );
        self
    }

    /// Assert that the response has the expected header value
    ///
    /// # Panics
    ///
    /// Panics if the header doesn't exist or doesn't match.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// response.assert_header("content-type", "application/json");
    /// ```
    pub fn assert_header(&self, key: &str, expected: &str) -> &Self {
        let actual = self
            .headers
            .get(key)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        assert_eq!(
            actual, expected,
            "Expected header '{}' to be '{}', got '{}'",
            key, expected, actual
        );
        self
    }

    /// Assert that the response body matches the expected JSON value
    ///
    /// # Panics
    ///
    /// Panics if the body can't be parsed as JSON or doesn't match.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// response.assert_json(&User { id: 1, name: "Alice".to_string() });
    /// ```
    pub fn assert_json<T: DeserializeOwned + PartialEq + std::fmt::Debug>(
        &self,
        expected: &T,
    ) -> &Self {
        let actual: T = self.json().expect("Failed to parse response body as JSON");
        assert_eq!(&actual, expected, "JSON body mismatch");
        self
    }

    /// Assert that the response body contains the expected string
    ///
    /// # Panics
    ///
    /// Panics if the body doesn't contain the expected string.
    pub fn assert_body_contains(&self, expected: &str) -> &Self {
        let body = self.text();
        assert!(
            body.contains(expected),
            "Expected body to contain '{}', got '{}'",
            expected,
            body
        );
        self
    }
}
