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
//!
//! ## Note
//!
//! This crate is typically not used directly. Use `rustapi-rs` instead for the
//! full framework experience with all features and re-exports.

mod app;
mod error;
mod extract;
mod handler;
pub mod middleware;
pub mod path_validation;
mod request;
mod response;
mod router;
mod server;
pub mod sse;
pub mod stream;
#[cfg(any(test, feature = "test-utils"))]
mod test_client;

// Public API
pub use app::RustApi;
pub use error::{ApiError, Environment, Result, get_environment};
pub use extract::{Body, ClientIp, Extension, FromRequest, FromRequestParts, HeaderValue, Headers, Json, Path, Query, State, ValidatedJson};
#[cfg(feature = "cookies")]
pub use extract::Cookies;
pub use handler::{
    Handler, HandlerService, Route, RouteHandler,
    get_route, post_route, put_route, patch_route, delete_route,
};
pub use middleware::{BodyLimitLayer, RequestId, RequestIdLayer, TracingLayer, DEFAULT_BODY_LIMIT};
#[cfg(feature = "metrics")]
pub use middleware::{MetricsLayer, MetricsResponse};
pub use request::Request;
pub use response::{Created, Html, IntoResponse, NoContent, Redirect, Response, WithStatus};
pub use router::{delete, get, patch, post, put, MethodRouter, Router};
pub use sse::{Sse, SseEvent};
pub use stream::StreamBody;
#[cfg(any(test, feature = "test-utils"))]
pub use test_client::{TestClient, TestRequest, TestResponse};
