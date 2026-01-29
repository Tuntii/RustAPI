//! Extractors for RustAPI
//!
//! Extractors automatically parse and validate data from incoming HTTP requests.
//! They implement the [`FromRequest`] or [`FromRequestParts`] traits and can be
//! used as handler function parameters.
//!
//! # Available Extractors
//!
//! | Extractor | Description | Consumes Body |
//! |-----------|-------------|---------------|
//! | [`Json<T>`] | Parse JSON request body | Yes |
//! | [`ValidatedJson<T>`] | Parse and validate JSON body | Yes |
//! | [`Query<T>`] | Parse query string parameters | No |
//! | [`Path<T>`] | Extract path parameters | No |
//! | [`State<T>`] | Access shared application state | No |
//! | [`Body`] | Raw request body bytes | Yes |
//! | [`Headers`] | Access all request headers | No |
//! | [`HeaderValue`] | Extract a specific header | No |
//! | [`Extension<T>`] | Access middleware-injected data | No |
//! | [`ClientIp`] | Extract client IP address | No |
//! | [`Cookies`] | Parse request cookies (requires `cookies` feature) | No |
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{Json, Query, Path, State};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct CreateUser {
//!     name: String,
//!     email: String,
//! }
//!
//! #[derive(Deserialize)]
//! struct Pagination {
//!     page: Option<u32>,
//!     limit: Option<u32>,
//! }
//!
//! // Multiple extractors can be combined
//! async fn create_user(
//!     State(db): State<DbPool>,
//!     Query(pagination): Query<Pagination>,
//!     Json(body): Json<CreateUser>,
//! ) -> impl IntoResponse {
//!     // Use db, pagination, and body...
//! }
//! ```
//!
//! # Extractor Order
//!
//! When using multiple extractors, body-consuming extractors (like `Json` or `Body`)
//! must come last since they consume the request body. Non-body extractors can be
//! in any order.

use crate::error::{ApiError, Result};
use crate::json;
use crate::request::Request;
use crate::response::IntoResponse;
use crate::stream::{StreamingBody, StreamingConfig};
use crate::validation::Validatable;
use bytes::Bytes;
use http::{header, StatusCode};
use rustapi_validate::v2::{AsyncValidate, ValidationContext};

use rustapi_openapi::schema::{JsonSchema2020, RustApiSchema, SchemaCtx, SchemaRef, TypeArray};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

/// Trait for extracting data from request parts (headers, path, query)
///
/// This is used for extractors that don't need the request body.
///
/// # Example: Implementing a custom extractor that requires a specific header
///
/// ```rust
/// use rustapi_core::FromRequestParts;
/// use rustapi_core::{Request, ApiError, Result};
/// use http::StatusCode;
///
/// struct ApiKey(String);
///
/// impl FromRequestParts for ApiKey {
///     fn from_request_parts(req: &Request) -> Result<Self> {
///         if let Some(key) = req.headers().get("x-api-key") {
///             if let Ok(key_str) = key.to_str() {
///                 return Ok(ApiKey(key_str.to_string()));
///             }
///         }
///         Err(ApiError::unauthorized("Missing or invalid API key"))
///     }
/// }
/// ```
pub trait FromRequestParts: Sized {
    /// Extract from request parts
    fn from_request_parts(req: &Request) -> Result<Self>;
}

/// Trait for extracting data from the full request (including body)
///
/// This is used for extractors that consume the request body.
///
/// # Example: Implementing a custom extractor that consumes the body
///
/// ```rust
/// use rustapi_core::FromRequest;
/// use rustapi_core::{Request, ApiError, Result};
/// use std::future::Future;
///
/// struct PlainText(String);
///
/// impl FromRequest for PlainText {
///     async fn from_request(req: &mut Request) -> Result<Self> {
///         // Ensure body is loaded
///         req.load_body().await?;
///         
///         // Consume the body
///         if let Some(bytes) = req.take_body() {
///             if let Ok(text) = String::from_utf8(bytes.to_vec()) {
///                 return Ok(PlainText(text));
///             }
///         }
///         
///         Err(ApiError::bad_request("Invalid plain text body"))
///     }
/// }
/// ```
pub trait FromRequest: Sized {
    /// Extract from the full request
    fn from_request(req: &mut Request) -> impl Future<Output = Result<Self>> + Send;
}

// Blanket impl: FromRequestParts -> FromRequest
impl<T: FromRequestParts> FromRequest for T {
    async fn from_request(req: &mut Request) -> Result<Self> {
        T::from_request_parts(req)
    }
}

/// JSON body extractor
///
/// Parses the request body as JSON and deserializes into type `T`.
/// Also works as a response type when T: Serialize.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct CreateUser {
///     name: String,
///     email: String,
/// }
///
/// async fn create_user(Json(body): Json<CreateUser>) -> impl IntoResponse {
///     // body is already deserialized
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Json<T>(pub T);

impl<T: DeserializeOwned + Send> FromRequest for Json<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        req.load_body().await?;
        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        // Use simd-json accelerated parsing when available (2-4x faster)
        let value: T = json::from_slice(&body)?;
        Ok(Json(value))
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Json(value)
    }
}

/// Default pre-allocation size for JSON response buffers (256 bytes)
/// This covers most small to medium JSON responses without reallocation.
const JSON_RESPONSE_INITIAL_CAPACITY: usize = 256;

// IntoResponse for Json - allows using Json<T> as a return type
impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> crate::response::Response {
        // Use pre-allocated buffer to reduce allocations
        match json::to_vec_with_capacity(&self.0, JSON_RESPONSE_INITIAL_CAPACITY) {
            Ok(body) => http::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(crate::response::Body::from(body))
                .unwrap(),
            Err(err) => {
                ApiError::internal(format!("Failed to serialize response: {}", err)).into_response()
            }
        }
    }
}

/// Validated JSON body extractor
///
/// Parses the request body as JSON, deserializes into type `T`, and validates
/// using the `Validate` trait. Returns a 422 Unprocessable Entity error with
/// detailed field-level validation errors if validation fails.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(email)]
///     email: String,
///     #[validate(length(min = 8))]
///     password: String,
/// }
///
/// async fn register(ValidatedJson(body): ValidatedJson<CreateUser>) -> impl IntoResponse {
///     // body is already validated!
///     // If email is invalid or password too short, a 422 error is returned automatically
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    /// Create a new ValidatedJson wrapper
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Get the inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: DeserializeOwned + Validatable + Send> FromRequest for ValidatedJson<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        req.load_body().await?;
        // First, deserialize the JSON body using simd-json when available
        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        let value: T = json::from_slice(&body)?;

        // Then, validate it using the unified Validatable trait
        value.do_validate()?;

        Ok(ValidatedJson(value))
    }
}

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for ValidatedJson<T> {
    fn from(value: T) -> Self {
        ValidatedJson(value)
    }
}

impl<T: Serialize> IntoResponse for ValidatedJson<T> {
    fn into_response(self) -> crate::response::Response {
        Json(self.0).into_response()
    }
}

/// Async validated JSON body extractor
///
/// Parses the request body as JSON, deserializes into type `T`, and validates
/// using the `AsyncValidate` trait from `rustapi-validate`.
///
/// This extractor supports async validation rules, such as database uniqueness checks.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use rustapi_validate::v2::prelude::*;
///
/// #[derive(Deserialize, Validate, AsyncValidate)]
/// struct CreateUser {
///     #[validate(email)]
///     email: String,
///     
///     #[validate(async_unique(table = "users", column = "email"))]
///     username: String,
/// }
///
/// async fn register(AsyncValidatedJson(body): AsyncValidatedJson<CreateUser>) -> impl IntoResponse {
///     // body is validated asynchronously (e.g. checked existing email in DB)
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct AsyncValidatedJson<T>(pub T);

impl<T> AsyncValidatedJson<T> {
    /// Create a new AsyncValidatedJson wrapper
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Get the inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for AsyncValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AsyncValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for AsyncValidatedJson<T> {
    fn from(value: T) -> Self {
        AsyncValidatedJson(value)
    }
}

impl<T: Serialize> IntoResponse for AsyncValidatedJson<T> {
    fn into_response(self) -> crate::response::Response {
        Json(self.0).into_response()
    }
}

impl<T: DeserializeOwned + AsyncValidate + Send + Sync> FromRequest for AsyncValidatedJson<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        req.load_body().await?;

        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        let value: T = json::from_slice(&body)?;

        // Create validation context from request
        // TODO: Extract validators from App State
        let ctx = ValidationContext::default();

        // Perform full validation (sync + async)
        if let Err(errors) = value.validate_full(&ctx).await {
            // Convert v2 ValidationErrors to ApiError
            let field_errors: Vec<crate::error::FieldError> = errors
                .fields
                .iter()
                .flat_map(|(field, errs)| {
                    let field_name = field.to_string();
                    errs.iter().map(move |e| crate::error::FieldError {
                        field: field_name.clone(),
                        code: e.code.to_string(),
                        message: e.message.clone(),
                    })
                })
                .collect();

            return Err(ApiError::validation(field_errors));
        }

        Ok(AsyncValidatedJson(value))
    }
}

/// Query string extractor
///
/// Parses the query string into type `T`.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: Option<u32>,
///     limit: Option<u32>,
/// }
///
/// async fn list_users(Query(params): Query<Pagination>) -> impl IntoResponse {
///     // params.page, params.limit
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Query<T>(pub T);

impl<T: DeserializeOwned> FromRequestParts for Query<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        let query = req.query_string().unwrap_or("");
        let value: T = serde_urlencoded::from_str(query)
            .map_err(|e| ApiError::bad_request(format!("Invalid query string: {}", e)))?;
        Ok(Query(value))
    }
}

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Path parameter extractor
///
/// Extracts path parameters defined in the route pattern.
///
/// # Example
///
/// For route `/users/{id}`:
///
/// ```rust,ignore
/// async fn get_user(Path(id): Path<i64>) -> impl IntoResponse {
///     // id is extracted from path
/// }
/// ```
///
/// For multiple params `/users/{user_id}/posts/{post_id}`:
///
/// ```rust,ignore
/// async fn get_post(Path((user_id, post_id)): Path<(i64, i64)>) -> impl IntoResponse {
///     // Both params extracted
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Path<T>(pub T);

impl<T: FromStr> FromRequestParts for Path<T>
where
    T::Err: std::fmt::Display,
{
    fn from_request_parts(req: &Request) -> Result<Self> {
        let params = req.path_params();

        // For single param, get the first one
        if let Some((_, value)) = params.iter().next() {
            let parsed = value
                .parse::<T>()
                .map_err(|e| ApiError::bad_request(format!("Invalid path parameter: {}", e)))?;
            return Ok(Path(parsed));
        }

        Err(ApiError::internal("Missing path parameter"))
    }
}

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Typed path extractor
///
/// Extracts path parameters and deserializes them into a struct implementing `Deserialize`.
/// This is similar to `Path<T>`, but supports complex structs that can be deserialized
/// from a map of parameter names to values (e.g. via `serde_json`).
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct UserParams {
///     id: u64,
///     category: String,
/// }
///
/// async fn get_user(Typed(params): Typed<UserParams>) -> impl IntoResponse {
///     // params.id, params.category
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Typed<T>(pub T);

impl<T: DeserializeOwned + Send> FromRequestParts for Typed<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        let params = req.path_params();
        let mut map = serde_json::Map::new();
        for (k, v) in params.iter() {
            map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
        }
        let value = serde_json::Value::Object(map);
        let parsed: T = serde_json::from_value(value)
            .map_err(|e| ApiError::bad_request(format!("Invalid path parameters: {}", e)))?;
        Ok(Typed(parsed))
    }
}

impl<T> Deref for Typed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// State extractor
///
/// Extracts shared application state.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Clone)]
/// struct AppState {
///     db: DbPool,
/// }
///
/// async fn handler(State(state): State<AppState>) -> impl IntoResponse {
///     // Use state.db
/// }
/// ```
#[derive(Debug, Clone)]
pub struct State<T>(pub T);

impl<T: Clone + Send + Sync + 'static> FromRequestParts for State<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.state().get::<T>().cloned().map(State).ok_or_else(|| {
            ApiError::internal(format!(
                "State of type `{}` not found. Did you forget to call .state()?",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl<T> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Raw body bytes extractor
#[derive(Debug, Clone)]
pub struct Body(pub Bytes);

impl FromRequest for Body {
    async fn from_request(req: &mut Request) -> Result<Self> {
        req.load_body().await?;
        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;
        Ok(Body(body))
    }
}

impl Deref for Body {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Streaming body extractor
pub struct BodyStream(pub StreamingBody);

impl FromRequest for BodyStream {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let config = StreamingConfig::default();

        if let Some(stream) = req.take_stream() {
            Ok(BodyStream(StreamingBody::new(stream, config.max_body_size)))
        } else if let Some(bytes) = req.take_body() {
            // Handle buffered body as stream
            let stream = futures_util::stream::once(async move { Ok(bytes) });
            Ok(BodyStream(StreamingBody::from_stream(
                stream,
                config.max_body_size,
            )))
        } else {
            Err(ApiError::internal("Body already consumed"))
        }
    }
}

impl Deref for BodyStream {
    type Target = StreamingBody;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BodyStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Forward stream implementation
impl futures_util::Stream for BodyStream {
    type Item = Result<Bytes, ApiError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.0).poll_next(cx)
    }
}

/// Optional extractor wrapper
///
/// Makes any extractor optional - returns None instead of error on failure.
impl<T: FromRequestParts> FromRequestParts for Option<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        Ok(T::from_request_parts(req).ok())
    }
}

/// Headers extractor
///
/// Provides access to all request headers as a typed map.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::extract::Headers;
///
/// async fn handler(headers: Headers) -> impl IntoResponse {
///     if let Some(content_type) = headers.get("content-type") {
///         format!("Content-Type: {:?}", content_type)
///     } else {
///         "No Content-Type header".to_string()
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Headers(pub http::HeaderMap);

impl Headers {
    /// Get a header value by name
    pub fn get(&self, name: &str) -> Option<&http::HeaderValue> {
        self.0.get(name)
    }

    /// Check if a header exists
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    /// Get the number of headers
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if headers are empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over all headers
    pub fn iter(&self) -> http::header::Iter<'_, http::HeaderValue> {
        self.0.iter()
    }
}

impl FromRequestParts for Headers {
    fn from_request_parts(req: &Request) -> Result<Self> {
        Ok(Headers(req.headers().clone()))
    }
}

impl Deref for Headers {
    type Target = http::HeaderMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Single header value extractor
///
/// Extracts a specific header value by name. Returns an error if the header is missing.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::extract::HeaderValue;
///
/// async fn handler(
///     auth: HeaderValue<{ "authorization" }>,
/// ) -> impl IntoResponse {
///     format!("Auth header: {}", auth.0)
/// }
/// ```
///
/// Note: Due to Rust's const generics limitations, you may need to use the
/// `HeaderValueOf` type alias or extract headers manually using the `Headers` extractor.
#[derive(Debug, Clone)]
pub struct HeaderValue(pub String, pub &'static str);

impl HeaderValue {
    /// Create a new HeaderValue extractor for a specific header name
    pub fn new(name: &'static str, value: String) -> Self {
        Self(value, name)
    }

    /// Get the header value
    pub fn value(&self) -> &str {
        &self.0
    }

    /// Get the header name
    pub fn name(&self) -> &'static str {
        self.1
    }

    /// Extract a specific header from a request
    pub fn extract(req: &Request, name: &'static str) -> Result<Self> {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| HeaderValue(s.to_string(), name))
            .ok_or_else(|| ApiError::bad_request(format!("Missing required header: {}", name)))
    }
}

impl Deref for HeaderValue {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extension extractor
///
/// Retrieves typed data from request extensions that was inserted by middleware.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::extract::Extension;
///
/// // Middleware inserts user data
/// #[derive(Clone)]
/// struct CurrentUser { id: i64 }
///
/// async fn handler(Extension(user): Extension<CurrentUser>) -> impl IntoResponse {
///     format!("User ID: {}", user.id)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Extension<T>(pub T);

impl<T: Clone + Send + Sync + 'static> FromRequestParts for Extension<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.extensions()
            .get::<T>()
            .cloned()
            .map(Extension)
            .ok_or_else(|| {
                ApiError::internal(format!(
                    "Extension of type `{}` not found. Did middleware insert it?",
                    std::any::type_name::<T>()
                ))
            })
    }
}

impl<T> Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Extension<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Client IP address extractor
///
/// Extracts the client IP address from the request. When `trust_proxy` is enabled,
/// it will use the `X-Forwarded-For` header if present.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::extract::ClientIp;
///
/// async fn handler(ClientIp(ip): ClientIp) -> impl IntoResponse {
///     format!("Your IP: {}", ip)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ClientIp(pub std::net::IpAddr);

impl ClientIp {
    /// Extract client IP, optionally trusting X-Forwarded-For header
    pub fn extract_with_config(req: &Request, trust_proxy: bool) -> Result<Self> {
        if trust_proxy {
            // Try X-Forwarded-For header first
            if let Some(forwarded) = req.headers().get("x-forwarded-for") {
                if let Ok(forwarded_str) = forwarded.to_str() {
                    // X-Forwarded-For can contain multiple IPs, take the first one
                    if let Some(first_ip) = forwarded_str.split(',').next() {
                        if let Ok(ip) = first_ip.trim().parse() {
                            return Ok(ClientIp(ip));
                        }
                    }
                }
            }
        }

        // Fall back to socket address from extensions (if set by server)
        if let Some(addr) = req.extensions().get::<std::net::SocketAddr>() {
            return Ok(ClientIp(addr.ip()));
        }

        // Default to localhost if no IP information available
        Ok(ClientIp(std::net::IpAddr::V4(std::net::Ipv4Addr::new(
            127, 0, 0, 1,
        ))))
    }
}

impl FromRequestParts for ClientIp {
    fn from_request_parts(req: &Request) -> Result<Self> {
        // By default, trust proxy headers
        Self::extract_with_config(req, true)
    }
}

/// Cookies extractor
///
/// Parses and provides access to request cookies from the Cookie header.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::extract::Cookies;
///
/// async fn handler(cookies: Cookies) -> impl IntoResponse {
///     if let Some(session) = cookies.get("session_id") {
///         format!("Session: {}", session.value())
///     } else {
///         "No session cookie".to_string()
///     }
/// }
/// ```
#[cfg(feature = "cookies")]
#[derive(Debug, Clone)]
pub struct Cookies(pub cookie::CookieJar);

#[cfg(feature = "cookies")]
impl Cookies {
    /// Get a cookie by name
    pub fn get(&self, name: &str) -> Option<&cookie::Cookie<'static>> {
        self.0.get(name)
    }

    /// Iterate over all cookies
    pub fn iter(&self) -> impl Iterator<Item = &cookie::Cookie<'static>> {
        self.0.iter()
    }

    /// Check if a cookie exists
    pub fn contains(&self, name: &str) -> bool {
        self.0.get(name).is_some()
    }
}

#[cfg(feature = "cookies")]
impl FromRequestParts for Cookies {
    fn from_request_parts(req: &Request) -> Result<Self> {
        let mut jar = cookie::CookieJar::new();

        if let Some(cookie_header) = req.headers().get(header::COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                // Parse each cookie from the header
                for cookie_part in cookie_str.split(';') {
                    let trimmed = cookie_part.trim();
                    if !trimmed.is_empty() {
                        if let Ok(cookie) = cookie::Cookie::parse(trimmed.to_string()) {
                            jar.add_original(cookie.into_owned());
                        }
                    }
                }
            }
        }

        Ok(Cookies(jar))
    }
}

#[cfg(feature = "cookies")]
impl Deref for Cookies {
    type Target = cookie::CookieJar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement FromRequestParts for common primitive types (path params)
macro_rules! impl_from_request_parts_for_primitives {
    ($($ty:ty),*) => {
        $(
            impl FromRequestParts for $ty {
                fn from_request_parts(req: &Request) -> Result<Self> {
                    let Path(value) = Path::<$ty>::from_request_parts(req)?;
                    Ok(value)
                }
            }
        )*
    };
}

impl_from_request_parts_for_primitives!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, String
);

// OperationModifier implementations for extractors

use rustapi_openapi::{MediaType, Operation, OperationModifier, Parameter, RequestBody, ResponseModifier, ResponseSpec};

// ValidatedJson - Adds request body
impl<T: RustApiSchema> OperationModifier for ValidatedJson<T> {
    fn update_operation(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        let schema_ref = T::schema(&mut ctx);

        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType { schema: Some(schema_ref), example: None },
        );

        op.request_body = Some(RequestBody {
            description: None,
            required: Some(true),
            content,
        });

        // Add 422 Validation Error response
        let mut responses_content = BTreeMap::new();
        responses_content.insert(
             "application/json".to_string(),
             MediaType {
                 schema: Some(SchemaRef::Ref {
                     reference: "#/components/schemas/ValidationErrorSchema".to_string(),
                 }),
                 example: None,
             },
        );

        op.responses.insert(
            "422".to_string(),
            ResponseSpec {
                description: "Validation Error".to_string(),
                content: responses_content,
                headers: BTreeMap::new(),
            },
        );
    }
}

// Json - Adds request body (Same as ValidatedJson)
impl<T: RustApiSchema> OperationModifier for Json<T> {
    fn update_operation(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        let schema_ref = T::schema(&mut ctx);

        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType { schema: Some(schema_ref), example: None },
        );

        op.request_body = Some(RequestBody {
            description: None,
            required: Some(true),
            content,
        });
    }
}

// Path - No op (handled by app routing)
impl<T> OperationModifier for Path<T> {
    fn update_operation(_op: &mut Operation) {}
}

// Typed - No op
impl<T> OperationModifier for Typed<T> {
    fn update_operation(_op: &mut Operation) {}
}

// Query - Extracts query params using field_schemas
impl<T: RustApiSchema> OperationModifier for Query<T> {
    fn update_operation(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        if let Some(fields) = T::field_schemas(&mut ctx) {
            let new_params: Vec<Parameter> = fields.into_iter().map(|(name, schema)| {
                Parameter {
                    name,
                    location: "query".to_string(),
                    required: false, // Assume optional
                    deprecated: None,
                    description: None,
                    schema: Some(schema),
                }
            }).collect();

            op.parameters.extend(new_params);
        }
    }
}

// State - No op
impl<T> OperationModifier for State<T> {
    fn update_operation(_op: &mut Operation) {}
}

// Body - Generic binary body
impl OperationModifier for Body {
    fn update_operation(op: &mut Operation) {
        let mut content = BTreeMap::new();
        content.insert(
            "application/octet-stream".to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(
                    serde_json::json!({ "type": "string", "format": "binary" }),
                )),
                example: None,
            },
        );

        op.request_body = Some(RequestBody {
            description: None,
            required: Some(true),
            content,
        });
    }
}

// BodyStream - Generic binary stream
impl OperationModifier for BodyStream {
    fn update_operation(op: &mut Operation) {
        let mut content = BTreeMap::new();
        content.insert(
            "application/octet-stream".to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(
                    serde_json::json!({ "type": "string", "format": "binary" }),
                )),
                example: None,
            },
        );

        op.request_body = Some(RequestBody {
            description: None,
            required: Some(true),
            content,
        });
    }
}

// ResponseModifier implementations for extractors

// Json<T> - 200 OK with schema T
impl<T: RustApiSchema> ResponseModifier for Json<T> {
    fn update_response(op: &mut Operation) {
        let mut ctx = SchemaCtx::new();
        let schema_ref = T::schema(&mut ctx);

        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType { schema: Some(schema_ref), example: None },
        );

        op.responses.insert(
            "200".to_string(),
            ResponseSpec {
                description: "Successful response".to_string(),
                content,
                headers: BTreeMap::new(),
            },
        );
    }
}

// RustApiSchema implementations

impl<T: RustApiSchema> RustApiSchema for Json<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        T::schema(ctx)
    }
}

impl<T: RustApiSchema> RustApiSchema for ValidatedJson<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        T::schema(ctx)
    }
}

impl<T: RustApiSchema> RustApiSchema for AsyncValidatedJson<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        T::schema(ctx)
    }
}

impl<T: RustApiSchema> RustApiSchema for Query<T> {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        T::schema(ctx)
    }
    fn field_schemas(ctx: &mut SchemaCtx) -> Option<BTreeMap<String, SchemaRef>> {
        T::field_schemas(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_params::PathParams;
    use bytes::Bytes;
    use http::{Extensions, Method};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use std::sync::Arc;

    /// Create a test request with the given method, path, and headers
    fn create_test_request_with_headers(
        method: Method,
        path: &str,
        headers: Vec<(&str, &str)>,
    ) -> Request {
        let uri: http::Uri = path.parse().unwrap();
        let mut builder = http::Request::builder().method(method).uri(uri);

        for (name, value) in headers {
            builder = builder.header(name, value);
        }

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        )
    }

    /// Create a test request with extensions
    fn create_test_request_with_extensions<T: Clone + Send + Sync + 'static>(
        method: Method,
        path: &str,
        extension: T,
    ) -> Request {
        let uri: http::Uri = path.parse().unwrap();
        let builder = http::Request::builder().method(method).uri(uri);

        let req = builder.body(()).unwrap();
        let (mut parts, _) = req.into_parts();
        parts.extensions.insert(extension);

        Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        )
    }

    // **Feature: phase3-batteries-included, Property 14: Headers extractor completeness**
    //
    // For any request with headers H, the `Headers` extractor SHALL return a map
    // containing all key-value pairs in H.
    //
    // **Validates: Requirements 5.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_headers_extractor_completeness(
            // Generate random header names and values
            // Using alphanumeric strings to ensure valid header names/values
            headers in prop::collection::vec(
                (
                    "[a-z][a-z0-9-]{0,20}",  // Valid header name pattern
                    "[a-zA-Z0-9 ]{1,50}"     // Valid header value pattern
                ),
                0..10
            )
        ) {
            let result: Result<(), TestCaseError> = (|| {
                // Convert to header tuples
                let header_tuples: Vec<(&str, &str)> = headers
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();

                // Create request with headers
                let request = create_test_request_with_headers(
                    Method::GET,
                    "/test",
                    header_tuples.clone(),
                );

                // Extract headers
                let extracted = Headers::from_request_parts(&request)
                    .map_err(|e| TestCaseError::fail(format!("Failed to extract headers: {}", e)))?;

                // Verify all original headers are present
                // HTTP allows duplicate headers - get_all() returns all values for a header name
                for (name, value) in &headers {
                    // Check that the header name exists
                    let all_values: Vec<_> = extracted.get_all(name.as_str()).iter().collect();
                    prop_assert!(
                        !all_values.is_empty(),
                        "Header '{}' not found",
                        name
                    );

                    // Check that the value is among the extracted values
                    let value_found = all_values.iter().any(|v| {
                        v.to_str().map(|s| s == value.as_str()).unwrap_or(false)
                    });

                    prop_assert!(
                        value_found,
                        "Header '{}' value '{}' not found in extracted values",
                        name,
                        value
                    );
                }

                Ok(())
            })();
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 15: HeaderValue extractor correctness**
    //
    // For any request with header "X" having value V, `HeaderValue::extract(req, "X")` SHALL return V;
    // for requests without header "X", it SHALL return an error.
    //
    // **Validates: Requirements 5.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_header_value_extractor_correctness(
            header_name in "[a-z][a-z0-9-]{0,20}",
            header_value in "[a-zA-Z0-9 ]{1,50}",
            has_header in prop::bool::ANY,
        ) {
            let result: Result<(), TestCaseError> = (|| {
                let headers = if has_header {
                    vec![(header_name.as_str(), header_value.as_str())]
                } else {
                    vec![]
                };

                let _request = create_test_request_with_headers(Method::GET, "/test", headers);

                // We need to use a static string for the header name in the extractor
                // So we'll test with a known header name
                let test_header = "x-test-header";
                let request_with_known_header = if has_header {
                    create_test_request_with_headers(
                        Method::GET,
                        "/test",
                        vec![(test_header, header_value.as_str())],
                    )
                } else {
                    create_test_request_with_headers(Method::GET, "/test", vec![])
                };

                let result = HeaderValue::extract(&request_with_known_header, test_header);

                if has_header {
                    let extracted = result
                        .map_err(|e| TestCaseError::fail(format!("Expected header to be found: {}", e)))?;
                    prop_assert_eq!(
                        extracted.value(),
                        header_value.as_str(),
                        "Header value mismatch"
                    );
                } else {
                    prop_assert!(
                        result.is_err(),
                        "Expected error when header is missing"
                    );
                }

                Ok(())
            })();
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 17: ClientIp extractor with forwarding**
    //
    // For any request with socket IP S and X-Forwarded-For header F, when forwarding is enabled,
    // `ClientIp` SHALL return the first IP in F; when disabled, it SHALL return S.
    //
    // **Validates: Requirements 5.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_client_ip_extractor_with_forwarding(
            // Generate valid IPv4 addresses
            forwarded_ip in (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
                .prop_map(|(a, b, c, d)| format!("{}.{}.{}.{}", a, b, c, d)),
            socket_ip in (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
                .prop_map(|(a, b, c, d)| std::net::IpAddr::V4(std::net::Ipv4Addr::new(a, b, c, d))),
            has_forwarded_header in prop::bool::ANY,
            trust_proxy in prop::bool::ANY,
        ) {
            let result: Result<(), TestCaseError> = (|| {
                let headers = if has_forwarded_header {
                    vec![("x-forwarded-for", forwarded_ip.as_str())]
                } else {
                    vec![]
                };

                // Create request with headers
                let uri: http::Uri = "/test".parse().unwrap();
                let mut builder = http::Request::builder().method(Method::GET).uri(uri);
                for (name, value) in &headers {
                    builder = builder.header(*name, *value);
                }
                let req = builder.body(()).unwrap();
                let (mut parts, _) = req.into_parts();

                // Add socket address to extensions
                let socket_addr = std::net::SocketAddr::new(socket_ip, 8080);
                parts.extensions.insert(socket_addr);

                let request = Request::new(
                    parts,
                    crate::request::BodyVariant::Buffered(Bytes::new()),
                    Arc::new(Extensions::new()),
                    PathParams::new(),
                );

                let extracted = ClientIp::extract_with_config(&request, trust_proxy)
                    .map_err(|e| TestCaseError::fail(format!("Failed to extract ClientIp: {}", e)))?;

                if trust_proxy && has_forwarded_header {
                    // Should use X-Forwarded-For
                    let expected_ip: std::net::IpAddr = forwarded_ip.parse()
                        .map_err(|e| TestCaseError::fail(format!("Invalid IP: {}", e)))?;
                    prop_assert_eq!(
                        extracted.0,
                        expected_ip,
                        "Should use X-Forwarded-For IP when trust_proxy is enabled"
                    );
                } else {
                    // Should use socket IP
                    prop_assert_eq!(
                        extracted.0,
                        socket_ip,
                        "Should use socket IP when trust_proxy is disabled or no X-Forwarded-For"
                    );
                }

                Ok(())
            })();
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 18: Extension extractor retrieval**
    //
    // For any type T and value V inserted into request extensions by middleware,
    // `Extension<T>` SHALL return V.
    //
    // **Validates: Requirements 5.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_extension_extractor_retrieval(
            value in any::<i64>(),
            has_extension in prop::bool::ANY,
        ) {
            let result: Result<(), TestCaseError> = (|| {
                // Create a simple wrapper type for testing
                #[derive(Clone, Debug, PartialEq)]
                struct TestExtension(i64);

                let uri: http::Uri = "/test".parse().unwrap();
                let builder = http::Request::builder().method(Method::GET).uri(uri);
                let req = builder.body(()).unwrap();
                let (mut parts, _) = req.into_parts();

                if has_extension {
                    parts.extensions.insert(TestExtension(value));
                }

                let request = Request::new(
                    parts,
                    crate::request::BodyVariant::Buffered(Bytes::new()),
                    Arc::new(Extensions::new()),
                    PathParams::new(),
                );

                let result = Extension::<TestExtension>::from_request_parts(&request);

                if has_extension {
                    let extracted = result
                        .map_err(|e| TestCaseError::fail(format!("Expected extension to be found: {}", e)))?;
                    prop_assert_eq!(
                        extracted.0,
                        TestExtension(value),
                        "Extension value mismatch"
                    );
                } else {
                    prop_assert!(
                        result.is_err(),
                        "Expected error when extension is missing"
                    );
                }

                Ok(())
            })();
            result?;
        }
    }

    // Unit tests for basic functionality

    #[test]
    fn test_headers_extractor_basic() {
        let request = create_test_request_with_headers(
            Method::GET,
            "/test",
            vec![
                ("content-type", "application/json"),
                ("accept", "text/html"),
            ],
        );

        let headers = Headers::from_request_parts(&request).unwrap();

        assert!(headers.contains("content-type"));
        assert!(headers.contains("accept"));
        assert!(!headers.contains("x-custom"));
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_header_value_extractor_present() {
        let request = create_test_request_with_headers(
            Method::GET,
            "/test",
            vec![("authorization", "Bearer token123")],
        );

        let result = HeaderValue::extract(&request, "authorization");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "Bearer token123");
    }

    #[test]
    fn test_header_value_extractor_missing() {
        let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

        let result = HeaderValue::extract(&request, "authorization");
        assert!(result.is_err());
    }

    #[test]
    fn test_client_ip_from_forwarded_header() {
        let request = create_test_request_with_headers(
            Method::GET,
            "/test",
            vec![("x-forwarded-for", "192.168.1.100, 10.0.0.1")],
        );

        let ip = ClientIp::extract_with_config(&request, true).unwrap();
        assert_eq!(ip.0, "192.168.1.100".parse::<std::net::IpAddr>().unwrap());
    }

    #[test]
    fn test_client_ip_ignores_forwarded_when_not_trusted() {
        let uri: http::Uri = "/test".parse().unwrap();
        let builder = http::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("x-forwarded-for", "192.168.1.100");
        let req = builder.body(()).unwrap();
        let (mut parts, _) = req.into_parts();

        let socket_addr = std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
            8080,
        );
        parts.extensions.insert(socket_addr);

        let request = Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        );

        let ip = ClientIp::extract_with_config(&request, false).unwrap();
        assert_eq!(ip.0, "10.0.0.1".parse::<std::net::IpAddr>().unwrap());
    }

    #[test]
    fn test_extension_extractor_present() {
        #[derive(Clone, Debug, PartialEq)]
        struct MyData(String);

        let request =
            create_test_request_with_extensions(Method::GET, "/test", MyData("hello".to_string()));

        let result = Extension::<MyData>::from_request_parts(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, MyData("hello".to_string()));
    }

    #[test]
    fn test_extension_extractor_missing() {
        #[derive(Clone, Debug)]
        #[allow(dead_code)]
        struct MyData(String);

        let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

        let result = Extension::<MyData>::from_request_parts(&request);
        assert!(result.is_err());
    }

    // Cookies tests (feature-gated)
    #[cfg(feature = "cookies")]
    mod cookies_tests {
        use super::*;

        // **Feature: phase3-batteries-included, Property 16: Cookies extractor parsing**
        //
        // For any request with Cookie header containing cookies C, the `Cookies` extractor
        // SHALL return a CookieJar containing exactly the cookies in C.
        // Note: Duplicate cookie names result in only the last value being kept.
        //
        // **Validates: Requirements 5.3**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_cookies_extractor_parsing(
                // Generate random cookie names and values
                // Using alphanumeric strings to ensure valid cookie names/values
                cookies in prop::collection::vec(
                    (
                        "[a-zA-Z][a-zA-Z0-9_]{0,15}",  // Valid cookie name pattern
                        "[a-zA-Z0-9]{1,30}"            // Valid cookie value pattern (no special chars)
                    ),
                    0..5
                )
            ) {
                let result: Result<(), TestCaseError> = (|| {
                    // Build cookie header string
                    let cookie_header = cookies
                        .iter()
                        .map(|(name, value)| format!("{}={}", name, value))
                        .collect::<Vec<_>>()
                        .join("; ");

                    let headers = if !cookies.is_empty() {
                        vec![("cookie", cookie_header.as_str())]
                    } else {
                        vec![]
                    };

                    let request = create_test_request_with_headers(Method::GET, "/test", headers);

                    // Extract cookies
                    let extracted = Cookies::from_request_parts(&request)
                        .map_err(|e| TestCaseError::fail(format!("Failed to extract cookies: {}", e)))?;

                    // Build expected cookies map - last value wins for duplicate names
                    let mut expected_cookies: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
                    for (name, value) in &cookies {
                        expected_cookies.insert(name.as_str(), value.as_str());
                    }

                    // Verify all expected cookies are present with correct values
                    for (name, expected_value) in &expected_cookies {
                        let cookie = extracted.get(name)
                            .ok_or_else(|| TestCaseError::fail(format!("Cookie '{}' not found", name)))?;

                        prop_assert_eq!(
                            cookie.value(),
                            *expected_value,
                            "Cookie '{}' value mismatch",
                            name
                        );
                    }

                    // Count cookies in jar should match unique cookie names
                    let extracted_count = extracted.iter().count();
                    prop_assert_eq!(
                        extracted_count,
                        expected_cookies.len(),
                        "Expected {} unique cookies, got {}",
                        expected_cookies.len(),
                        extracted_count
                    );

                    Ok(())
                })();
                result?;
            }
        }

        #[test]
        fn test_cookies_extractor_basic() {
            let request = create_test_request_with_headers(
                Method::GET,
                "/test",
                vec![("cookie", "session=abc123; user=john")],
            );

            let cookies = Cookies::from_request_parts(&request).unwrap();

            assert!(cookies.contains("session"));
            assert!(cookies.contains("user"));
            assert!(!cookies.contains("other"));

            assert_eq!(cookies.get("session").unwrap().value(), "abc123");
            assert_eq!(cookies.get("user").unwrap().value(), "john");
        }

        #[test]
        fn test_cookies_extractor_empty() {
            let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

            let cookies = Cookies::from_request_parts(&request).unwrap();
            assert_eq!(cookies.iter().count(), 0);
        }

        #[test]
        fn test_cookies_extractor_single() {
            let request = create_test_request_with_headers(
                Method::GET,
                "/test",
                vec![("cookie", "token=xyz789")],
            );

            let cookies = Cookies::from_request_parts(&request).unwrap();
            assert_eq!(cookies.iter().count(), 1);
            assert_eq!(cookies.get("token").unwrap().value(), "xyz789");
        }
    }
}
