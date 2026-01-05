//! Traffic Insight - Opt-in traffic data collection middleware.
//!
//! This module provides comprehensive request/response monitoring for
//! analytics, debugging, and observability.
//!
//! # Features
//!
//! - **Request/Response Capture**: Collect method, path, status, duration, body sizes
//! - **Header Collection**: Configurable whitelist with sensitive data redaction
//! - **Body Capture**: Opt-in request/response body logging
//! - **Sampling**: Configurable sampling rate to reduce overhead
//! - **In-Memory Storage**: Ring buffer with configurable capacity
//! - **Dashboard Endpoints**: Built-in `/insights` and `/insights/stats` endpoints
//! - **Export**: File (JSON lines), webhook, and custom export sinks
//!
//! # Quick Start
//!
//! ```ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_extras::insight::{InsightLayer, InsightConfig};
//!
//! #[rustapi::main]
//! async fn main() {
//!     let insight = InsightLayer::with_config(
//!         InsightConfig::new()
//!             .sample_rate(1.0)           // Capture all requests
//!             .skip_path("/health")       // Skip health checks
//!             .header_whitelist(vec!["content-type", "user-agent"])
//!     );
//!
//!     RustApi::new()
//!         .layer(insight)
//!         .mount(hello)
//!         .run("127.0.0.1:3000")
//!         .await
//!         .unwrap();
//! }
//!
//! #[rustapi::get("/hello")]
//! async fn hello() -> &'static str {
//!     "Hello, World!"
//! }
//! ```
//!
//! # Configuration
//!
//! Use [`InsightConfig`] to customize behavior:
//!
//! ```ignore
//! use rustapi_extras::insight::InsightConfig;
//!
//! let config = InsightConfig::new()
//!     // Sampling
//!     .sample_rate(0.1)                    // 10% of requests
//!     
//!     // Paths to exclude
//!     .skip_path("/health")
//!     .skip_path("/metrics")
//!     .skip_path_prefix("/internal/")
//!     
//!     // Header capture
//!     .header_whitelist(vec!["content-type", "user-agent", "accept"])
//!     .response_header_whitelist(vec!["content-type", "x-request-id"])
//!     
//!     // Body capture (opt-in)
//!     .capture_request_body(true)
//!     .capture_response_body(true)
//!     .max_body_size(8192)                 // 8KB max
//!     
//!     // Storage
//!     .store_capacity(5000)                // Keep 5000 entries
//!     
//!     // Endpoints
//!     .dashboard_path(Some("/admin/insights"))
//!     .stats_path(Some("/admin/insights/stats"))
//!     
//!     // Callback for custom processing
//!     .on_insight(|insight| {
//!         if insight.duration_ms > 1000 {
//!             tracing::warn!("Slow request: {} {}ms", insight.path, insight.duration_ms);
//!         }
//!     });
//! ```
//!
//! # Dashboard Endpoints
//!
//! The middleware automatically exposes two endpoints:
//!
//! - `GET /insights` - Returns recent insights as JSON
//!   - Query param: `?limit=100` to control number of results
//! - `GET /insights/stats` - Returns aggregated statistics
//!
//! These paths are configurable via [`InsightConfig`].
//!
//! # Export
//!
//! Export insights to external systems:
//!
//! ```ignore
//! use rustapi_extras::insight::export::{FileExporter, WebhookConfig, WebhookExporter, CompositeExporter};
//!
//! // File export (JSON lines format)
//! let file_exporter = FileExporter::new("./insights.jsonl")?;
//!
//! // Webhook export
//! let webhook = WebhookExporter::new(
//!     WebhookConfig::new("https://logs.example.com/ingest")
//!         .auth("Bearer my-token")
//!         .batch_size(100)
//! );
//!
//! // Multiple destinations
//! let composite = CompositeExporter::new()
//!     .add(file_exporter)
//!     .add(webhook);
//! ```
//!
//! # Data Structure
//!
//! Each [`InsightData`] entry contains:
//!
//! - `request_id` - Unique request identifier
//! - `method` - HTTP method
//! - `path` - Request path
//! - `query_params` - Query string parameters
//! - `status` - Response status code
//! - `duration_ms` - Processing time in milliseconds
//! - `request_size` / `response_size` - Body sizes in bytes
//! - `timestamp` - Unix timestamp
//! - `client_ip` - Client IP address
//! - `request_headers` / `response_headers` - Captured headers
//! - `request_body` / `response_body` - Captured bodies (if enabled)
//!
//! # Statistics
//!
//! [`InsightStats`] provides aggregated metrics:
//!
//! - Request counts (total, successful, client errors, server errors)
//! - Duration statistics (avg, min, max, p95, p99)
//! - Bytes transferred (request/response)
//! - Breakdowns by route, method, and status code
//! - Requests per second

mod config;
mod data;
mod layer;
mod store;
pub mod export;

pub use config::InsightConfig;
pub use data::{InsightData, InsightStats};
pub use layer::InsightLayer;
pub use store::{InMemoryInsightStore, InsightStore, NullInsightStore};
