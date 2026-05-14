//! HTTP route handlers for the embedded dashboard admin surface.
//!
//! Routes:
//!   GET  /__rustapi/dashboard             → HTML UI page
//!   GET  /__rustapi/dashboard/api/snapshot → DashboardSnapshot JSON (auth required)
//!   GET  /__rustapi/dashboard/api/routes   → Route inventory JSON (auth required)
//!   GET  /__rustapi/dashboard/api/metrics  → Live counters JSON  (auth required)
//!   GET  /__rustapi/dashboard/api/topology → Route graph JSON    (auth required)
//!   GET  /__rustapi/dashboard/api/events   → Stage counters JSON (auth required)
//!   GET  /__rustapi/dashboard/api/health   → Health summary JSON (auth required)
//!   GET  /__rustapi/dashboard/api/replay   → Replay index JSON   (auth required)

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
    let prefix = config.normalized_path();
    let suffix = dashboard_suffix(path, &prefix)?;

    match (method, suffix) {
        // HTML page — no auth required (browsers can't easily send Bearer headers)
        ("GET", "" | "index.html") => Some(serve_html(config)),

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
        ("GET", "api/topology") => {
            check_auth!(headers, config);
            Some(serve_topology(metrics))
        }
        ("GET", "api/events") => {
            check_auth!(headers, config);
            Some(serve_events(metrics))
        }
        ("GET", "api/health") => {
            check_auth!(headers, config);
            Some(serve_health(metrics))
        }
        ("GET", "api/replay") => {
            check_auth!(headers, config);
            Some(serve_replay(metrics))
        }

        _ => None,
    }
}

// ─── Private handlers ────────────────────────────────────────────────────────

fn serve_html(config: &DashboardConfig) -> Response {
    let title = escape_html(&config.title);
    let html = DASHBOARD_HTML.replace("__RUSTAPI_DASHBOARD_TITLE__", &title);

    http::Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(Body::Full(Full::new(Bytes::from(html))))
        .unwrap()
}

fn dashboard_suffix<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    if prefix == "/" {
        return path.strip_prefix('/');
    }

    if path == prefix {
        return Some("");
    }

    path.strip_prefix(prefix)?.strip_prefix('/')
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
    let snap = metrics.snapshot();
    json_ok(json!({
        "live_counters": snap.live_counters,
        "stages": snap.stages,
    }))
}

fn serve_topology(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(json!({ "route_graph": snap.route_graph }))
}

fn serve_events(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(json!({ "stages": snap.stages }))
}

fn serve_health(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(json!({ "health_summary": snap.health_summary }))
}

fn serve_replay(metrics: &Arc<DashboardMetrics>) -> Response {
    let snap = metrics.snapshot();
    json_ok(json!({ "replay_index": snap.replay_index }))
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
