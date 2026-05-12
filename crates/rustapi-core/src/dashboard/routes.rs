//! HTTP route handlers for the embedded dashboard admin surface.
//!
//! Routes:
//!   GET  /__rustapi/dashboard             → HTML UI page
//!   GET  /__rustapi/dashboard/api/snapshot → DashboardSnapshot JSON (auth required)
//!   GET  /__rustapi/dashboard/api/routes   → Route inventory JSON (auth required)
//!   GET  /__rustapi/dashboard/api/metrics  → Live counters JSON  (auth required)

use super::auth::DashboardAuth;
use super::config::DashboardConfig;
use super::metrics::DashboardMetrics;
use crate::response::{Body, Response};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::json;
use std::sync::Arc;

static DASHBOARD_HTML: &str = include_str!("dashboard.html");

// The macro must be defined BEFORE it is used in `dispatch` below.
macro_rules! check_auth {
    ($headers:expr, $config:expr) => {
        if let Some(ref token) = $config.admin_token {
            if let Err(resp) = DashboardAuth::check($headers, token) {
                return Some(resp);
            }
        }
    };
}

// ─── Dispatch ─────────────────────────────────────────────────────────────────

/// Dispatch a request to the appropriate dashboard handler.
///
/// Returns `Some(Response)` when the path matches a dashboard route,
/// `None` to pass through to the regular router.
pub async fn dispatch(
    headers: &http::HeaderMap,
    method: &str,
    path: &str,
    metrics: &Arc<DashboardMetrics>,
    config: &DashboardConfig,
) -> Option<Response> {
    // Only handle paths inside our prefix
    if !path.starts_with(config.path.as_str()) {
        return None;
    }

    // Strip the prefix to get the sub-path
    let suffix = path[config.path.len()..].trim_start_matches('/');

    match (method, suffix) {
        // HTML page — no auth required (browsers can't easily send Bearer headers)
        ("GET", "" | "index.html") => Some(serve_html()),

        // JSON API endpoints — auth required when admin_token is set
        ("GET", "api/snapshot") => {
            check_auth!(headers, config);
            Some(serve_snapshot(metrics))
        }
        ("GET", "api/routes") => {
            check_auth!(headers, config);
            Some(serve_routes(metrics))
        }
        ("GET", "api/metrics") => {
            check_auth!(headers, config);
            Some(serve_live_metrics(metrics))
        }

        _ => None,
    }
}

// ─── Private handlers ────────────────────────────────────────────────────────

fn serve_html() -> Response {
    http::Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(Body::Full(Full::new(Bytes::from_static(
            DASHBOARD_HTML.as_bytes(),
        ))))
        .unwrap()
}

fn serve_snapshot(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(serde_json::to_value(snap).unwrap_or_default())
}

fn serve_routes(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(json!({ "routes": snap.routes }))
}

fn serve_live_metrics(metrics: &Arc<DashboardMetrics>) -> Response {
    use std::sync::atomic::Ordering;
    json_ok(json!({
        "total_reqs":      metrics.total_reqs.load(Ordering::Relaxed),
        "ultra_fast_reqs": metrics.ultra_fast_reqs.load(Ordering::Relaxed),
        "fast_reqs":       metrics.fast_reqs.load(Ordering::Relaxed),
        "full_reqs":       metrics.full_reqs.load(Ordering::Relaxed),
    }))
}

fn json_ok(body: serde_json::Value) -> Response {
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    http::Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(Body::Full(Full::new(Bytes::from(bytes))))
        .unwrap()
}
