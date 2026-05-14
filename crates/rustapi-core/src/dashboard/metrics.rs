//! Dashboard metrics collection.
//!
//! Tracks execution-path counters (Ultra Fast / Fast / Full), per-route hit
//! counts, and latency totals with near-zero overhead using atomics.
//!
//! Disabled at compile-time when the `dashboard` feature is not enabled.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ─── Execution Path ──────────────────────────────────────────────────────────

/// Which server execution branch handled this request.
#[derive(Debug, Clone, Copy)]
pub enum ExecutionPath {
    /// No middleware AND no interceptors — maximum performance path.
    UltraFast,
    /// No middleware, but has interceptors.
    Fast,
    /// Has middleware layers.
    Full,
}

// ─── Request Stages ──────────────────────────────────────────────────────────

/// Coarse request lifecycle stages tracked by the dashboard.
#[derive(Debug, Clone, Copy)]
pub enum RequestStage {
    /// Request has entered the RustAPI request pipeline.
    Received,
    /// Request matched a registered route or framework admin endpoint.
    Routed,
    /// Response has been produced.
    Completed,
    /// Response completed with a client or server error status.
    Failed,
}

// ─── Per-Route Metrics ────────────────────────────────────────────────────────

#[derive(Default)]
struct RouteCounters {
    hits: AtomicU64,
    total_latency_ms: AtomicU64,
    errors: AtomicU64,
}

// ─── Route Inventory Item ─────────────────────────────────────────────────────

/// A registered route exposed in the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInventoryItem {
    /// Path pattern in `{param}` format, e.g. `/users/{id}`.
    pub path: String,
    /// HTTP methods registered on this path.
    pub methods: Vec<String>,
    /// Tags collected from OpenAPI operation metadata.
    pub tags: Vec<String>,
    /// Canonical feature gates associated with this route, when known.
    pub feature_gates: Vec<String>,
    /// Route group derived from the first path segment.
    pub group: String,
    /// Whether this route is one of the built-in health/readiness/liveness endpoints.
    pub health_eligible: bool,
    /// Whether replay capture is safe by default for this route.
    pub replay_eligible: bool,
    /// Internal counter key. Dynamic route parameter names are normalized so
    /// `/users/{id}` and `/users/123` resolve to the same metrics bucket.
    #[serde(skip)]
    pub metrics_key: String,
}

impl RouteInventoryItem {
    /// Create a route inventory entry with conservative defaults.
    pub fn new(path: impl Into<String>, methods: Vec<String>) -> Self {
        let path = path.into();
        let replay_eligible = !path.starts_with("/__rustapi/");
        Self {
            metrics_key: metrics_key_for_path(&path),
            group: route_group(&path),
            path,
            methods,
            tags: Vec::new(),
            feature_gates: Vec::new(),
            health_eligible: false,
            replay_eligible,
        }
    }

    /// Attach OpenAPI tags collected from the route operations.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Attach canonical feature gates associated with the route.
    pub fn with_feature_gates(mut self, feature_gates: Vec<String>) -> Self {
        self.feature_gates = feature_gates;
        self
    }

    /// Mark whether the route is a health endpoint.
    pub fn health_eligible(mut self, health_eligible: bool) -> Self {
        self.health_eligible = health_eligible;
        if health_eligible {
            self.replay_eligible = false;
        }
        self
    }

    /// Mark whether the route is eligible for replay capture by default.
    pub fn replay_eligible(mut self, replay_eligible: bool) -> Self {
        self.replay_eligible = replay_eligible;
        self
    }
}

// ─── Snapshot types ───────────────────────────────────────────────────────────

/// Live metrics snapshot for a single route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMetricsSnapshot {
    pub path: String,
    pub methods: Vec<String>,
    pub tags: Vec<String>,
    pub feature_gates: Vec<String>,
    pub group: String,
    pub health_eligible: bool,
    pub replay_eligible: bool,
    pub hit_count: u64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
}

/// Top-level live request counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLiveCountersSnapshot {
    pub total_reqs: u64,
    pub ultra_fast_reqs: u64,
    pub fast_reqs: u64,
    pub full_reqs: u64,
}

/// Request-stage counters for coarse lifecycle visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStageSnapshot {
    pub received_reqs: u64,
    pub routed_reqs: u64,
    pub completed_reqs: u64,
    pub failed_reqs: u64,
}

/// Aggregated route topology node for a group of endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGroupSnapshot {
    pub group: String,
    pub route_count: usize,
    pub method_count: usize,
    pub tags: Vec<String>,
    pub hit_count: u64,
    pub error_count: u64,
}

/// Route graph summary used by the topology view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGraphSnapshot {
    pub total_routes: usize,
    pub total_methods: usize,
    pub groups: Vec<RouteGroupSnapshot>,
}

/// Health endpoint summary for the dashboard state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardHealthEndpointSnapshot {
    pub path: String,
    pub kind: String,
    pub status: String,
}

/// Health snapshot derived from registered built-in health endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardHealthSummary {
    pub configured: bool,
    pub endpoints: Vec<DashboardHealthEndpointSnapshot>,
}

/// Replay index summary for UI discovery. The actual replay list/detail API is
/// still served by the replay admin surface when `ReplayLayer` is installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardReplayIndexSnapshot {
    pub available: bool,
    pub admin_path: String,
    pub total_entries: Option<u64>,
    pub note: String,
}

/// Full point-in-time dashboard snapshot (serialised to JSON for the UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub uptime_secs: u64,
    pub start_unix: u64,
    pub total_reqs: u64,
    pub ultra_fast_reqs: u64,
    pub fast_reqs: u64,
    pub full_reqs: u64,
    pub live_counters: DashboardLiveCountersSnapshot,
    pub stages: DashboardStageSnapshot,
    pub route_graph: RouteGraphSnapshot,
    pub health_summary: DashboardHealthSummary,
    pub replay_index: DashboardReplayIndexSnapshot,
    pub routes: Vec<RouteMetricsSnapshot>,
}

// ─── DashboardMetrics ─────────────────────────────────────────────────────────

/// Shared metrics store, inserted into router state as `Arc<DashboardMetrics>`.
///
/// All writes use `Ordering::Relaxed` — counters are eventually consistent,
/// which is sufficient for a live dashboard display.
pub struct DashboardMetrics {
    pub ultra_fast_reqs: AtomicU64,
    pub fast_reqs: AtomicU64,
    pub full_reqs: AtomicU64,
    pub total_reqs: AtomicU64,
    received_reqs: AtomicU64,
    routed_reqs: AtomicU64,
    completed_reqs: AtomicU64,
    failed_reqs: AtomicU64,
    start_time: Instant,
    start_unix: u64,
    route_counters: Arc<DashMap<String, RouteCounters>>,
    route_inventory: Vec<RouteInventoryItem>,
    replay_admin_path: String,
}

impl DashboardMetrics {
    pub fn new(route_inventory: Vec<RouteInventoryItem>) -> Self {
        Self::new_with_replay_admin_path(route_inventory, "/__rustapi/replays")
    }

    pub fn new_with_replay_admin_path(
        route_inventory: Vec<RouteInventoryItem>,
        replay_admin_path: impl Into<String>,
    ) -> Self {
        let start_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            ultra_fast_reqs: AtomicU64::new(0),
            fast_reqs: AtomicU64::new(0),
            full_reqs: AtomicU64::new(0),
            total_reqs: AtomicU64::new(0),
            received_reqs: AtomicU64::new(0),
            routed_reqs: AtomicU64::new(0),
            completed_reqs: AtomicU64::new(0),
            failed_reqs: AtomicU64::new(0),
            start_time: Instant::now(),
            start_unix,
            route_counters: Arc::new(DashMap::new()),
            route_inventory,
            replay_admin_path: replay_admin_path.into(),
        }
    }

    /// Record a request lifecycle stage. Called from `server.rs` when the
    /// dashboard feature is compiled in and enabled on an app.
    #[inline]
    pub fn record_stage(&self, stage: RequestStage) {
        match stage {
            RequestStage::Received => {
                self.received_reqs.fetch_add(1, Ordering::Relaxed);
            }
            RequestStage::Routed => {
                self.routed_reqs.fetch_add(1, Ordering::Relaxed);
            }
            RequestStage::Completed => {
                self.completed_reqs.fetch_add(1, Ordering::Relaxed);
            }
            RequestStage::Failed => {
                self.failed_reqs.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Record one request.  Called from `server.rs` hot-path.
    #[inline]
    pub fn record_request(
        &self,
        path: &str,
        duration_ms: u64,
        exec_path: ExecutionPath,
        is_error: bool,
    ) {
        self.total_reqs.fetch_add(1, Ordering::Relaxed);
        match exec_path {
            ExecutionPath::UltraFast => {
                self.ultra_fast_reqs.fetch_add(1, Ordering::Relaxed);
            }
            ExecutionPath::Fast => {
                self.fast_reqs.fetch_add(1, Ordering::Relaxed);
            }
            ExecutionPath::Full => {
                self.full_reqs.fetch_add(1, Ordering::Relaxed);
            }
        }

        let entry = self.route_counters.entry(path.to_string()).or_default();
        entry.hits.fetch_add(1, Ordering::Relaxed);
        entry
            .total_latency_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        if is_error {
            entry.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Build a serialisable point-in-time snapshot.
    pub fn snapshot(&self) -> DashboardSnapshot {
        let uptime_secs = self.start_time.elapsed().as_secs();

        let routes: Vec<RouteMetricsSnapshot> = self
            .route_inventory
            .iter()
            .map(|item| {
                let (hit_count, avg_latency_ms, error_count) =
                    if let Some(c) = self.route_counters.get(&item.metrics_key) {
                        let hits = c.hits.load(Ordering::Relaxed);
                        let total = c.total_latency_ms.load(Ordering::Relaxed);
                        let errors = c.errors.load(Ordering::Relaxed);
                        let avg = if hits > 0 {
                            total as f64 / hits as f64
                        } else {
                            0.0
                        };
                        (hits, avg, errors)
                    } else {
                        (0, 0.0, 0)
                    };

                RouteMetricsSnapshot {
                    path: item.path.clone(),
                    methods: item.methods.clone(),
                    tags: item.tags.clone(),
                    feature_gates: item.feature_gates.clone(),
                    group: item.group.clone(),
                    health_eligible: item.health_eligible,
                    replay_eligible: item.replay_eligible,
                    hit_count,
                    avg_latency_ms,
                    error_count,
                }
            })
            .collect();

        let live_counters = DashboardLiveCountersSnapshot {
            total_reqs: self.total_reqs.load(Ordering::Relaxed),
            ultra_fast_reqs: self.ultra_fast_reqs.load(Ordering::Relaxed),
            fast_reqs: self.fast_reqs.load(Ordering::Relaxed),
            full_reqs: self.full_reqs.load(Ordering::Relaxed),
        };

        let stages = DashboardStageSnapshot {
            received_reqs: self.received_reqs.load(Ordering::Relaxed),
            routed_reqs: self.routed_reqs.load(Ordering::Relaxed),
            completed_reqs: self.completed_reqs.load(Ordering::Relaxed),
            failed_reqs: self.failed_reqs.load(Ordering::Relaxed),
        };

        let route_graph = build_route_graph(&routes);
        let health_summary = build_health_summary(&routes);
        let replay_index = DashboardReplayIndexSnapshot {
            available: routes.iter().any(|r| r.replay_eligible),
            admin_path: self.replay_admin_path.clone(),
            total_entries: None,
            note: "Use the replay admin API for paginated list/detail/diff data when ReplayLayer is installed.".to_string(),
        };

        DashboardSnapshot {
            uptime_secs,
            start_unix: self.start_unix,
            total_reqs: live_counters.total_reqs,
            ultra_fast_reqs: live_counters.ultra_fast_reqs,
            fast_reqs: live_counters.fast_reqs,
            full_reqs: live_counters.full_reqs,
            live_counters,
            stages,
            route_graph,
            health_summary,
            replay_index,
            routes,
        }
    }
}

/// Normalize route templates and concrete request paths into a shared metrics key.
pub fn metrics_key_for_path(path: &str) -> String {
    const MAX_SEGMENTS: usize = 16;
    const MAX_SEGMENT_LEN: usize = 64;

    if path.is_empty() || path == "/" {
        return "/".to_string();
    }

    let mut normalized = String::with_capacity(path.len().min(256));
    normalized.push('/');

    let mut first = true;
    for (idx, segment) in path.split('/').filter(|s| !s.is_empty()).enumerate() {
        if idx >= MAX_SEGMENTS {
            if !first {
                normalized.push('/');
            }
            normalized.push_str("{truncated}");
            break;
        }

        if !first {
            normalized.push('/');
        }
        first = false;

        if is_route_param_segment(segment)
            || is_dynamic_metrics_segment(segment)
            || segment.len() > MAX_SEGMENT_LEN
        {
            normalized.push_str("{param}");
        } else {
            normalized.push_str(segment);
        }
    }

    if normalized.len() > 256 {
        normalized.truncate(256);
    }

    normalized
}

fn build_route_graph(routes: &[RouteMetricsSnapshot]) -> RouteGraphSnapshot {
    #[derive(Default)]
    struct GroupAcc {
        route_count: usize,
        method_count: usize,
        tags: BTreeSet<String>,
        hit_count: u64,
        error_count: u64,
    }

    let mut groups: BTreeMap<String, GroupAcc> = BTreeMap::new();
    for route in routes {
        let acc = groups.entry(route.group.clone()).or_default();
        acc.route_count += 1;
        acc.method_count += route.methods.len();
        acc.hit_count += route.hit_count;
        acc.error_count += route.error_count;
        acc.tags.extend(route.tags.iter().cloned());
    }

    let total_routes = routes.len();
    let total_methods = routes.iter().map(|r| r.methods.len()).sum();
    let groups = groups
        .into_iter()
        .map(|(group, acc)| RouteGroupSnapshot {
            group,
            route_count: acc.route_count,
            method_count: acc.method_count,
            tags: acc.tags.into_iter().collect(),
            hit_count: acc.hit_count,
            error_count: acc.error_count,
        })
        .collect();

    RouteGraphSnapshot {
        total_routes,
        total_methods,
        groups,
    }
}

fn build_health_summary(routes: &[RouteMetricsSnapshot]) -> DashboardHealthSummary {
    let endpoints: Vec<DashboardHealthEndpointSnapshot> = routes
        .iter()
        .filter(|route| route.health_eligible)
        .map(|route| DashboardHealthEndpointSnapshot {
            kind: health_kind(&route.path),
            path: route.path.clone(),
            status: "configured".to_string(),
        })
        .collect();

    DashboardHealthSummary {
        configured: !endpoints.is_empty(),
        endpoints,
    }
}

fn route_group(path: &str) -> String {
    path.split('/')
        .find(|segment| !segment.is_empty())
        .map(|segment| {
            if is_route_param_segment(segment) {
                "dynamic".to_string()
            } else {
                segment.to_string()
            }
        })
        .unwrap_or_else(|| "root".to_string())
}

fn health_kind(path: &str) -> String {
    if path.contains("ready") {
        "readiness".to_string()
    } else if path.contains("live") {
        "liveness".to_string()
    } else {
        "health".to_string()
    }
}

fn is_route_param_segment(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}

fn is_dynamic_metrics_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }

    if segment.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }

    let hex_or_dash = segment.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-');
    hex_or_dash && segment.len() >= 16
}
