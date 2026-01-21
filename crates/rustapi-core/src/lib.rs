//! # RustAPI Core
//!
//! Core library providing the foundational types and traits for RustAPI.
//!
//! This crate provides the essential building blocks for the RustAPI web framework:
//!
//! - **Application Builder**: [`RustApi`] - The main entry point for building web applications
//! - **Routing**: [`Router`], [`get`], [`post`], [`put`], [`patch`], [`delete`] - HTTP routing primitives
//! - **Extractors**: [`Json`], [`Query`], [`Path`], [`State`], [`Body`], [`Headers`] - Request data extraction
//! - **Responses**: [`IntoResponse`], [`Created`], [`NoContent`], [`Html`], [`Redirect`] - Response types
//! - **Middleware**: [`BodyLimitLayer`], [`RequestIdLayer`], [`TracingLayer`] - Request processing layers
//! - **Error Handling**: [`ApiError`], [`Result`] - Structured error responses
//! - **Testing**: `TestClient` - Integration testing without network binding (requires `test-utils` feature)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustapi_core::{RustApi, get, Json};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Message {
//!     text: String,
//! }
//!
//! async fn hello() -> Json<Message> {
//!     Json(Message { text: "Hello, World!".to_string() })
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     RustApi::new()
//!         .route("/", get(hello))
//!         .run("127.0.0.1:8080")
//!         .await
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `metrics` - Enable Prometheus metrics middleware
//! - `cookies` - Enable cookie parsing extractor
//! - `test-utils` - Enable testing utilities like `TestClient`
//! - `swagger-ui` - Enable Swagger UI documentation endpoint
//! - `http3` - Enable HTTP/3 (QUIC) support
//! - `http3-dev` - Enable HTTP/3 with self-signed certificate generation
//!
//! ## Note
//!
//! This crate is typically not used directly. Use `rustapi-rs` instead for the
//! full framework experience with all features and re-exports.

mod app;
pub mod auto_route;
pub use auto_route::collect_auto_routes;
pub mod auto_schema;
pub use auto_schema::apply_auto_schemas;
mod error;
mod extract;
mod handler;
pub mod hateoas;
pub mod health;
#[cfg(feature = "http3")]
pub mod http3;
pub mod interceptor;
pub mod json;
pub mod middleware;
pub mod multipart;
pub mod path_params;
pub mod path_validation;
mod request;
mod response;
mod router;
mod server;
pub mod sse;
pub mod static_files;
pub mod stream;
pub mod typed_path;
#[macro_use]
mod tracing_macros;

/// Private module for macro internals - DO NOT USE DIRECTLY
///
/// This module is used by procedural macros to register routes.
/// It is not part of the public API and may change at any time.
#[doc(hidden)]
pub mod __private {
    pub use crate::auto_route::AUTO_ROUTES;
    pub use crate::auto_schema::AUTO_SCHEMAS;
    pub use linkme;
    pub use rustapi_openapi;
}

// Public API
pub use app::{RustApi, RustApiConfig};
pub use error::{get_environment, ApiError, Environment, FieldError, Result};
#[cfg(feature = "cookies")]
pub use extract::Cookies;
pub use extract::{
    Body, BodyStream, ClientIp, Extension, FromRequest, FromRequestParts, HeaderValue, Headers,
    Json, Path, Query, State, Typed, ValidatedJson,
};
pub use handler::{
    delete_route, get_route, patch_route, post_route, put_route, Handler, HandlerService, Route,
    RouteHandler,
};
pub use hateoas::{Link, LinkOrArray, Linkable, PageInfo, Resource, ResourceCollection};
pub use health::{HealthCheck, HealthCheckBuilder, HealthCheckResult, HealthStatus};
pub use http::StatusCode;
#[cfg(feature = "http3")]
pub use http3::{Http3Config, Http3Server};
pub use interceptor::{InterceptorChain, RequestInterceptor, ResponseInterceptor};
#[cfg(feature = "compression")]
pub use middleware::CompressionLayer;
pub use middleware::{BodyLimitLayer, RequestId, RequestIdLayer, TracingLayer, DEFAULT_BODY_LIMIT};
#[cfg(feature = "metrics")]
pub use middleware::{MetricsLayer, MetricsResponse};
pub use multipart::{Multipart, MultipartConfig, MultipartField, UploadedFile};
pub use request::{BodyVariant, Request};
pub use response::{Created, Html, IntoResponse, NoContent, Redirect, Response, WithStatus};
pub use router::{delete, get, patch, post, put, MethodRouter, RouteMatch, Router};
pub use sse::{sse_response, KeepAlive, Sse, SseEvent};
pub use static_files::{serve_dir, StaticFile, StaticFileConfig};
pub use stream::{StreamBody, StreamingBody, StreamingConfig};
pub use typed_path::TypedPath;
