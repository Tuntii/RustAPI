//! Middleware infrastructure for RustAPI
//!
//! This module provides Tower-compatible middleware support for RustAPI applications.
//! Middleware can be added using the `.layer()` method on `RustApi`.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_core::middleware::RequestIdLayer;
//!
//! RustApi::new()
//!     .layer(RequestIdLayer::new())
//!     .route("/", get(handler))
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

mod body_limit;
#[cfg(feature = "compression")]
mod compression;
mod layer;
#[cfg(feature = "metrics")]
mod metrics;
mod request_id;
mod tracing_layer;

pub use body_limit::{BodyLimitLayer, DEFAULT_BODY_LIMIT};
#[cfg(feature = "compression")]
pub use compression::{CompressionAlgorithm, CompressionConfig, CompressionLayer};
pub use layer::{BoxedNext, LayerStack, MiddlewareLayer};
#[cfg(feature = "metrics")]
pub use metrics::{CustomMetricsBuilder, MetricsLayer, MetricsResponse};
pub use request_id::{RequestId, RequestIdLayer};
pub use tracing_layer::TracingLayer;
