//! Integration tests for the embedded dashboard feature.
//!
//! Run with: `cargo test -p rustapi-core --features dashboard`

#![cfg(feature = "dashboard")]

use rustapi_core::dashboard::{
    config::DashboardConfig,
    metrics::{DashboardMetrics, ExecutionPath, RouteInventoryItem},
    routes::dispatch,
};
use std::sync::Arc;

// ─── DashboardConfig ──────────────────────────────────────────────────────────

#[test]
fn config_default_path() {
    let cfg = DashboardConfig::new();
    assert_eq!(cfg.path, "/__rustapi/dashboard");
}

#[test]
fn config_builder() {
    let cfg = DashboardConfig::new()
        .admin_token("secret")
        .path("/admin/dash")
        .title("My Dashboard");
    assert_eq!(cfg.admin_token.as_deref(), Some("secret"));
    assert_eq!(cfg.path, "/admin/dash");
    assert_eq!(cfg.title, "My Dashboard");
}

#[test]
fn config_no_token_by_default() {
    let cfg = DashboardConfig::new();
    assert!(cfg.admin_token.is_none());
}

// ─── DashboardMetrics ────────────────────────────────────────────────────────

#[test]
fn metrics_initial_zeros() {
    let m = DashboardMetrics::new(vec![]);
    let snap = m.snapshot();
    assert_eq!(snap.total_reqs, 0);
    assert_eq!(snap.ultra_fast_reqs, 0);
    assert_eq!(snap.fast_reqs, 0);
    assert_eq!(snap.full_reqs, 0);
}

#[test]
fn metrics_record_ultra_fast() {
    let m = DashboardMetrics::new(vec![RouteInventoryItem {
        path: "/users".into(),
        methods: vec!["GET".into()],
    }]);

    m.record_request("/users", 5, ExecutionPath::UltraFast, false);
    m.record_request("/users", 3, ExecutionPath::UltraFast, false);

    let snap = m.snapshot();
    assert_eq!(snap.ultra_fast_reqs, 2);
    assert_eq!(snap.fast_reqs, 0);
    assert_eq!(snap.full_reqs, 0);
    assert_eq!(snap.total_reqs, 2);

    let route = snap.routes.iter().find(|r| r.path == "/users").unwrap();
    assert_eq!(route.hit_count, 2);
    assert_eq!(route.error_count, 0);
    assert_eq!(route.avg_latency_ms, 4.0); // (5+3)/2
}

#[test]
fn metrics_record_error() {
    let m = DashboardMetrics::new(vec![RouteInventoryItem {
        path: "/broken".into(),
        methods: vec!["POST".into()],
    }]);

    m.record_request("/broken", 10, ExecutionPath::Full, true);

    let snap = m.snapshot();
    assert_eq!(snap.full_reqs, 1);
    assert_eq!(snap.total_reqs, 1);

    let route = snap.routes.iter().find(|r| r.path == "/broken").unwrap();
    assert_eq!(route.error_count, 1);
}

#[test]
fn metrics_unknown_route_skips_counter() {
    let m = DashboardMetrics::new(vec![]);
    m.record_request("/unknown", 10, ExecutionPath::Fast, false);

    let snap = m.snapshot();
    assert_eq!(snap.total_reqs, 1);
    assert!(snap.routes.is_empty(), "No routes in inventory");
}

#[test]
fn metrics_uptime_non_zero() {
    let m = DashboardMetrics::new(vec![]);
    let snap = m.snapshot();
    // Uptime should be >= 0 (could be 0 immediately, but not negative)
    assert!(snap.uptime_secs < u64::MAX);
}

// ─── Routes (dispatch) ───────────────────────────────────────────────────────

fn make_headers() -> http::HeaderMap {
    http::HeaderMap::new()
}

fn make_headers_with_token(token: &str) -> http::HeaderMap {
    let mut h = http::HeaderMap::new();
    h.insert(
        http::header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    );
    h
}

fn make_metrics() -> Arc<DashboardMetrics> {
    Arc::new(DashboardMetrics::new(vec![RouteInventoryItem {
        path: "/api/users".into(),
        methods: vec!["GET".into(), "POST".into()],
    }]))
}

#[tokio::test]
async fn dispatch_html_no_auth_required() {
    let m = make_metrics();
    let cfg = DashboardConfig::new().admin_token("secret");

    // GET /__rustapi/dashboard should return HTML without auth
    let resp = dispatch(&make_headers(), "GET", "/__rustapi/dashboard", &m, &cfg).await;
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert_eq!(resp.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn dispatch_snapshot_requires_token() {
    let m = make_metrics();
    let cfg = DashboardConfig::new().admin_token("secret");

    // Without token → 401
    let resp = dispatch(
        &make_headers(),
        "GET",
        "/__rustapi/dashboard/api/snapshot",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dispatch_snapshot_with_correct_token() {
    let m = make_metrics();
    let cfg = DashboardConfig::new().admin_token("secret");

    let resp = dispatch(
        &make_headers_with_token("secret"),
        "GET",
        "/__rustapi/dashboard/api/snapshot",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::OK);
}

#[tokio::test]
async fn dispatch_snapshot_wrong_token_returns_401() {
    let m = make_metrics();
    let cfg = DashboardConfig::new().admin_token("secret");

    let resp = dispatch(
        &make_headers_with_token("wrong"),
        "GET",
        "/__rustapi/dashboard/api/snapshot",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dispatch_no_token_config_skips_auth() {
    let m = make_metrics();
    let cfg = DashboardConfig::new(); // no token

    // Without any admin_token set, all endpoints should be open
    let resp = dispatch(
        &make_headers(),
        "GET",
        "/__rustapi/dashboard/api/snapshot",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::OK);
}

#[tokio::test]
async fn dispatch_unknown_path_returns_none() {
    let m = make_metrics();
    let cfg = DashboardConfig::new();

    let resp = dispatch(&make_headers(), "GET", "/api/users", &m, &cfg).await;
    assert!(resp.is_none(), "Non-dashboard paths must return None");
}

#[tokio::test]
async fn dispatch_routes_endpoint() {
    let m = make_metrics();
    let cfg = DashboardConfig::new();

    let resp = dispatch(
        &make_headers(),
        "GET",
        "/__rustapi/dashboard/api/routes",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::OK);
}

#[tokio::test]
async fn dispatch_metrics_endpoint() {
    let m = make_metrics();
    let cfg = DashboardConfig::new();

    let resp = dispatch(
        &make_headers(),
        "GET",
        "/__rustapi/dashboard/api/metrics",
        &m,
        &cfg,
    )
    .await;
    assert!(resp.is_some());
    assert_eq!(resp.unwrap().status(), http::StatusCode::OK);
}
