//! Response types for RustAPI

use crate::error::{ApiError, ErrorResponse};
use bytes::Bytes;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use http_body_util::Full;
use serde::Serialize;

/// HTTP Response type
pub type Response = http::Response<Full<Bytes>>;

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
            .body(Full::new(Bytes::new()))
            .unwrap()
    }
}

// Implement for &'static str
impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from(self)))
            .unwrap()
    }
}

// Implement for String
impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from(self)))
            .unwrap()
    }
}

// Implement for StatusCode
impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self)
            .body(Full::new(Bytes::new()))
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
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let error_response = ErrorResponse::from(self);
        let body = serde_json::to_vec(&error_response).unwrap_or_else(|_| {
            br#"{"error":{"type":"internal_error","message":"Failed to serialize error"}}"#.to_vec()
        });

        http::Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
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
                .body(Full::new(Bytes::from(body)))
                .unwrap(),
            Err(err) => ApiError::internal(format!("Failed to serialize response: {}", err))
                .into_response(),
        }
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
            .body(Full::new(Bytes::new()))
            .unwrap()
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
            .body(Full::new(Bytes::from(self.0.into())))
            .unwrap()
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
            .body(Full::new(Bytes::new()))
            .unwrap()
    }
}
