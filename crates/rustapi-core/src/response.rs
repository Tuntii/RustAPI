//! Response types for RustAPI
//!
//! This module provides types for building HTTP responses. The core trait is
//! [`IntoResponse`], which allows any type to be converted into an HTTP response.
//!
//! # Built-in Response Types
//!
//! | Type | Status | Content-Type | Description |
//! |------|--------|--------------|-------------|
//! | `String` / `&str` | 200 | text/plain | Plain text response |
//! | `()` | 200 | - | Empty response |
//! | [`Json<T>`] | 200 | application/json | JSON response |
//! | [`Created<T>`] | 201 | application/json | Created resource |
//! | [`NoContent`] | 204 | - | No content response |
//! | [`Html<T>`] | 200 | text/html | HTML response |
//! | [`Redirect`] | 3xx | - | HTTP redirect |
//! | [`WithStatus<T, N>`] | N | varies | Custom status code |
//! | [`ApiError`] | varies | application/json | Error response |
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{Json, Created, NoContent, IntoResponse};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! // Return JSON with 200 OK
//! async fn get_user() -> Json<User> {
//!     Json(User { id: 1, name: "Alice".to_string() })
//! }
//!
//! // Return JSON with 201 Created
//! async fn create_user() -> Created<User> {
//!     Created(User { id: 2, name: "Bob".to_string() })
//! }
//!
//! // Return 204 No Content
//! async fn delete_user() -> NoContent {
//!     NoContent
//! }
//!
//! // Return custom status code
//! async fn accepted() -> WithStatus<String, 202> {
//!     WithStatus("Request accepted".to_string())
//! }
//! ```
//!
//! # Tuple Responses
//!
//! You can also return tuples to customize the response:
//!
//! ```rust,ignore
//! use http::StatusCode;
//!
//! // (StatusCode, body)
//! async fn custom_status() -> (StatusCode, String) {
//!     (StatusCode::ACCEPTED, "Accepted".to_string())
//! }
//!
//! // (StatusCode, headers, body)
//! async fn with_headers() -> (StatusCode, HeaderMap, String) {
//!     let mut headers = HeaderMap::new();
//!     headers.insert("X-Custom", "value".parse().unwrap());
//!     (StatusCode::OK, headers, "Hello".to_string())
//! }
//! ```

use crate::error::{ApiError, ErrorResponse};
use bytes::Bytes;
use futures_util::StreamExt;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use http_body_util::Full;
use rustapi_openapi::schema::{RustApiSchema, SchemaCtx};
use rustapi_openapi::{MediaType, Operation, ResponseModifier, ResponseSpec, SchemaRef};
use serde::Serialize;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Unified response body type
pub enum Body {
    /// Fully buffered body (default)
    Full(Full<Bytes>),
    /// Streaming body
    Streaming(Pin<Box<dyn http_body::Body<Data = Bytes, Error = ApiError> + Send + 'static>>),
}

impl Body {
    /// Create a new full body from bytes
    pub fn new(bytes: Bytes) -> Self {
        Self::Full(Full::new(bytes))
    }

    /// Create an empty body
    pub fn empty() -> Self {
        Self::Full(Full::new(Bytes::new()))
    }

    /// Create a streaming body
    pub fn from_stream<S, E>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<Bytes, E>> + Send + 'static,
        E: Into<ApiError> + 'static,
    {
        let body = http_body_util::StreamBody::new(
            stream.map(|res| res.map_err(|e| e.into()).map(http_body::Frame::data)),
        );
        Self::Streaming(Box::pin(body))
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}

impl http_body::Body for Body {
    type Data = Bytes;
    type Error = ApiError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Body::Full(b) => Pin::new(b)
                .poll_frame(cx)
                .map_err(|_| ApiError::internal("Infallible error")),
            Body::Streaming(b) => b.as_mut().poll_frame(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Body::Full(b) => b.is_end_stream(),
            Body::Streaming(b) => b.is_end_stream(),
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match self {
            Body::Full(b) => b.size_hint(),
            Body::Streaming(b) => b.size_hint(),
        }
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Self::new(bytes)
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Self::new(Bytes::from(s))
    }
}

impl From<&'static str> for Body {
    fn from(s: &'static str) -> Self {
        Self::new(Bytes::from(s))
    }
}

impl From<Vec<u8>> for Body {
    fn from(v: Vec<u8>) -> Self {
        Self::new(Bytes::from(v))
    }
}

/// HTTP Response type
pub type Response = http::Response<Body>;

/// Trait for types that can be converted into an HTTP response
pub trait IntoResponse {
    /// Convert self into a Response
    fn into_response(self) -> Response;
}

// Implement for Response itself
impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

// Implement for () - returns 200 OK with empty body
impl IntoResponse for () {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap()
    }
}

// Implement for &'static str
impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(self))
            .unwrap()
    }
}

// Implement for String
impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(self))
            .unwrap()
    }
}

// Implement for StatusCode
impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self)
            .body(Body::empty())
            .unwrap()
    }
}

// Implement for (StatusCode, impl IntoResponse)
impl<R: IntoResponse> IntoResponse for (StatusCode, R) {
    fn into_response(self) -> Response {
        let mut response = self.1.into_response();
        *response.status_mut() = self.0;
        response
    }
}

// Implement for (StatusCode, HeaderMap, impl IntoResponse)
impl<R: IntoResponse> IntoResponse for (StatusCode, HeaderMap, R) {
    fn into_response(self) -> Response {
        let mut response = self.2.into_response();
        *response.status_mut() = self.0;
        response.headers_mut().extend(self.1);
        response
    }
}

// Implement for Result<T, E> where both implement IntoResponse
impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(v) => v.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

// Implement for ApiError
// Implement for ApiError with environment-aware error masking
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        // ErrorResponse::from now handles environment-aware masking
        let error_response = ErrorResponse::from(self);
        let body = serde_json::to_vec(&error_response).unwrap_or_else(|_| {
            br#"{"error":{"type":"internal_error","message":"Failed to serialize error"}}"#.to_vec()
        });

        http::Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }
}

impl ResponseModifier for ApiError {
    fn update_response(op: &mut Operation) {
        // We define common error responses here
        // 400 Bad Request
        op.responses.insert(
            "400".to_string(),
            ResponseSpec {
                description: "Bad Request".to_string(),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(SchemaRef::Ref {
                                reference: "#/components/schemas/ErrorSchema".to_string(),
                            }),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );

        // 500 Internal Server Error
        op.responses.insert(
            "500".to_string(),
            ResponseSpec {
                description: "Internal Server Error".to_string(),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(SchemaRef::Ref {
                                reference: "#/components/schemas/ErrorSchema".to_string(),
                            }),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );
    }
}

/// 201 Created response wrapper
///
/// Returns HTTP 201 with JSON body.
///
/// # Example
///
/// ```rust,ignore
/// async fn create_user(body: UserIn) -> Result<Created<UserOut>> {
///     let user = db.create(body).await?;
///     Ok(Created(user))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Created<T>(pub T);

impl<T: Serialize> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.0) {
            Ok(body) => http::Response::builder()
                .status(StatusCode::CREATED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
            Err(err) => {
                ApiError::internal(format!("Failed to serialize response: {}", err)).into_response()
            }
        }
    }
}

impl<T: RustApiSchema> ResponseModifier for Created<T> {
    fn update_response(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        let schema_ref = T::schema(&mut ctx);

        op.responses.insert(
            "201".to_string(),
            ResponseSpec {
                description: "Created".to_string(),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(schema_ref),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );
    }
}

/// 204 No Content response
///
/// Returns HTTP 204 with empty body.
///
/// # Example
///
/// ```rust,ignore
/// async fn delete_user(id: i64) -> Result<NoContent> {
///     db.delete(id).await?;
///     Ok(NoContent)
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct NoContent;

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap()
    }
}

impl ResponseModifier for NoContent {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "204".to_string(),
            ResponseSpec {
                description: "No Content".to_string(),
                content: BTreeMap::new(),
                headers: BTreeMap::new(),
            },
        );
    }
}

/// HTML response wrapper
#[derive(Debug, Clone)]
pub struct Html<T>(pub T);

impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(self.0.into()))
            .unwrap()
    }
}

impl<T> ResponseModifier for Html<T> {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "HTML Content".to_string(),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "text/html".to_string(),
                        MediaType {
                            schema: Some(SchemaRef::Inline(
                                serde_json::json!({ "type": "string" }),
                            )),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );
    }
}

/// Redirect response
#[derive(Debug, Clone)]
pub struct Redirect {
    status: StatusCode,
    location: HeaderValue,
}

impl Redirect {
    /// Create a 302 Found redirect
    pub fn to(uri: &str) -> Self {
        Self {
            status: StatusCode::FOUND,
            location: HeaderValue::from_str(uri).expect("Invalid redirect URI"),
        }
    }

    /// Create a 301 Permanent redirect
    pub fn permanent(uri: &str) -> Self {
        Self {
            status: StatusCode::MOVED_PERMANENTLY,
            location: HeaderValue::from_str(uri).expect("Invalid redirect URI"),
        }
    }

    /// Create a 307 Temporary redirect
    pub fn temporary(uri: &str) -> Self {
        Self {
            status: StatusCode::TEMPORARY_REDIRECT,
            location: HeaderValue::from_str(uri).expect("Invalid redirect URI"),
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self.status)
            .header(header::LOCATION, self.location)
            .body(Body::empty())
            .unwrap()
    }
}

impl ResponseModifier for Redirect {
    fn update_response(op: &mut Operation) {
        // Can be 301, 302, 307. We'll verify what we can generically say.
        // Or we document "3xx"
        op.responses.insert(
            "3xx".to_string(),
            ResponseSpec {
                description: "Redirection".to_string(),
                content: BTreeMap::new(),
                headers: BTreeMap::new(),
            },
        );
    }
}

/// Generic wrapper for returning a response with a custom status code.
///
/// The status code is specified as a const generic parameter.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::response::WithStatus;
///
/// async fn accepted_handler() -> WithStatus<String, 202> {
///     WithStatus("Request accepted for processing".to_string())
/// }
///
/// async fn custom_status() -> WithStatus<&'static str, 418> {
///     WithStatus("I'm a teapot")
/// }
/// ```
#[derive(Debug, Clone)]
pub struct WithStatus<T, const CODE: u16>(pub T);

impl<T: IntoResponse, const CODE: u16> IntoResponse for WithStatus<T, CODE> {
    fn into_response(self) -> Response {
        let mut response = self.0.into_response();
        // Convert the const generic to StatusCode
        if let Ok(status) = StatusCode::from_u16(CODE) {
            *response.status_mut() = status;
        }
        response
    }
}

impl<T: RustApiSchema, const CODE: u16> ResponseModifier for WithStatus<T, CODE> {
    fn update_response(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        let schema_ref = T::schema(&mut ctx);

        op.responses.insert(
            CODE.to_string(),
            ResponseSpec {
                description: format!("Response with status {}", CODE),
                content: {
                    let mut map = BTreeMap::new();
                    map.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(schema_ref),
                            example: None,
                        },
                    );
                    map
                },
                headers: BTreeMap::new(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Helper to extract body bytes from a Full<Bytes> body
    async fn body_to_bytes(body: Body) -> Bytes {
        use http_body_util::BodyExt;
        body.collect().await.unwrap().to_bytes()
    }

    // **Feature: phase3-batteries-included, Property 19: WithStatus response correctness**
    //
    // For any status code N and body B, `WithStatus<B, N>` SHALL produce a response
    // with status N and body equal to B serialized.
    //
    // **Validates: Requirements 6.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_with_status_response_correctness(
            body in "[a-zA-Z0-9 ]{0,100}",
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // We need to test with specific const generics, so we'll test a few representative cases
                // and verify the pattern holds. Since const generics must be known at compile time,
                // we test the behavior by checking that the status code is correctly applied.

                // Test with 200 OK
                let response_200: Response = WithStatus::<_, 200>(body.clone()).into_response();
                prop_assert_eq!(response_200.status().as_u16(), 200);

                // Test with 201 Created
                let response_201: Response = WithStatus::<_, 201>(body.clone()).into_response();
                prop_assert_eq!(response_201.status().as_u16(), 201);

                // Test with 202 Accepted
                let response_202: Response = WithStatus::<_, 202>(body.clone()).into_response();
                prop_assert_eq!(response_202.status().as_u16(), 202);

                // Test with 204 No Content
                let response_204: Response = WithStatus::<_, 204>(body.clone()).into_response();
                prop_assert_eq!(response_204.status().as_u16(), 204);

                // Test with 400 Bad Request
                let response_400: Response = WithStatus::<_, 400>(body.clone()).into_response();
                prop_assert_eq!(response_400.status().as_u16(), 400);

                // Test with 404 Not Found
                let response_404: Response = WithStatus::<_, 404>(body.clone()).into_response();
                prop_assert_eq!(response_404.status().as_u16(), 404);

                // Test with 418 I'm a teapot
                let response_418: Response = WithStatus::<_, 418>(body.clone()).into_response();
                prop_assert_eq!(response_418.status().as_u16(), 418);

                // Test with 500 Internal Server Error
                let response_500: Response = WithStatus::<_, 500>(body.clone()).into_response();
                prop_assert_eq!(response_500.status().as_u16(), 500);

                // Test with 503 Service Unavailable
                let response_503: Response = WithStatus::<_, 503>(body.clone()).into_response();
                prop_assert_eq!(response_503.status().as_u16(), 503);

                // Verify body is preserved (using a fresh 200 response)
                let response_for_body: Response = WithStatus::<_, 200>(body.clone()).into_response();
                let body_bytes = body_to_bytes(response_for_body.into_body()).await;
                let body_str = String::from_utf8_lossy(&body_bytes);
                prop_assert_eq!(body_str.as_ref(), body.as_str());

                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn test_with_status_preserves_content_type() {
        // Test that WithStatus preserves the content type from the inner response
        let response: Response = WithStatus::<_, 202>("hello world").into_response();

        assert_eq!(response.status().as_u16(), 202);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_with_status_with_empty_body() {
        let response: Response = WithStatus::<_, 204>(()).into_response();

        assert_eq!(response.status().as_u16(), 204);
        // Empty body should have zero size
        let body_bytes = body_to_bytes(response.into_body()).await;
        assert!(body_bytes.is_empty());
    }

    #[test]
    fn test_with_status_common_codes() {
        // Test common HTTP status codes
        assert_eq!(
            WithStatus::<_, 100>("").into_response().status().as_u16(),
            100
        ); // Continue
        assert_eq!(
            WithStatus::<_, 200>("").into_response().status().as_u16(),
            200
        ); // OK
        assert_eq!(
            WithStatus::<_, 201>("").into_response().status().as_u16(),
            201
        ); // Created
        assert_eq!(
            WithStatus::<_, 202>("").into_response().status().as_u16(),
            202
        ); // Accepted
        assert_eq!(
            WithStatus::<_, 204>("").into_response().status().as_u16(),
            204
        ); // No Content
        assert_eq!(
            WithStatus::<_, 301>("").into_response().status().as_u16(),
            301
        ); // Moved Permanently
        assert_eq!(
            WithStatus::<_, 302>("").into_response().status().as_u16(),
            302
        ); // Found
        assert_eq!(
            WithStatus::<_, 400>("").into_response().status().as_u16(),
            400
        ); // Bad Request
        assert_eq!(
            WithStatus::<_, 401>("").into_response().status().as_u16(),
            401
        ); // Unauthorized
        assert_eq!(
            WithStatus::<_, 403>("").into_response().status().as_u16(),
            403
        ); // Forbidden
        assert_eq!(
            WithStatus::<_, 404>("").into_response().status().as_u16(),
            404
        ); // Not Found
        assert_eq!(
            WithStatus::<_, 500>("").into_response().status().as_u16(),
            500
        ); // Internal Server Error
        assert_eq!(
            WithStatus::<_, 502>("").into_response().status().as_u16(),
            502
        ); // Bad Gateway
        assert_eq!(
            WithStatus::<_, 503>("").into_response().status().as_u16(),
            503
        ); // Service Unavailable
    }
}
