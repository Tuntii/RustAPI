//! Embedded Isometric System Dashboard.
//!
//! Provides a self-contained visual admin surface at `/__rustapi/dashboard`
//! showing real-time execution-path counters, route topology, and live metrics.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! RustApi::new()
//!     .route("/api/users", get(list_users))
//!     .dashboard(
//!         DashboardConfig::new()
//!             .admin_token("my-secret-token")
//!     )
//!     .run("127.0.0.1:8080")
//!     .await
//! ```
//!
//! Then open `http://localhost:8080/__rustapi/dashboard` in your browser.
//! Enter the dashboard token in the UI field or pass `Authorization: Bearer <token>`
//! when calling JSON API endpoints directly.

pub mod auth;
pub mod config;
pub mod metrics;
pub mod routes;

pub use config::DashboardConfig;
pub use metrics::{
    DashboardHealthEndpointSnapshot, DashboardHealthSummary, DashboardLiveCountersSnapshot,
    DashboardMetrics, DashboardReplayIndexSnapshot, DashboardSnapshot, DashboardStageSnapshot,
    ExecutionPath, RequestStage, RouteGraphSnapshot, RouteGroupSnapshot, RouteInventoryItem,
    RouteMetricsSnapshot,
};
