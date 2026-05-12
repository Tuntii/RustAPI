//! Dashboard metrics collection.
//!
//! Tracks execution-path counters (Ultra Fast / Fast / Full), per-route hit
//! counts, and latency totals with near-zero overhead using atomics.
//!
//! Disabled at compile-time when the `dashboard` feature is not enabled.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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
}

// ─── Snapshot types ───────────────────────────────────────────────────────────

/// Live metrics snapshot for a single route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMetricsSnapshot {
    pub path: String,
    pub methods: Vec<String>,
    pub hit_count: u64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
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
    start_time: Instant,
    start_unix: u64,
    route_counters: Arc<DashMap<String, RouteCounters>>,
    route_inventory: Vec<RouteInventoryItem>,
}

impl DashboardMetrics {
    pub fn new(route_inventory: Vec<RouteInventoryItem>) -> Self {
        let start_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            ultra_fast_reqs: AtomicU64::new(0),
            fast_reqs: AtomicU64::new(0),
            full_reqs: AtomicU64::new(0),
            total_reqs: AtomicU64::new(0),
            start_time: Instant::now(),
            start_unix,
            route_counters: Arc::new(DashMap::new()),
            route_inventory,
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
                    if let Some(c) = self.route_counters.get(&item.path) {
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
                    hit_count,
                    avg_latency_ms,
                    error_count,
                }
            })
            .collect();

        DashboardSnapshot {
            uptime_secs,
            start_unix: self.start_unix,
            total_reqs: self.total_reqs.load(Ordering::Relaxed),
            ultra_fast_reqs: self.ultra_fast_reqs.load(Ordering::Relaxed),
            fast_reqs: self.fast_reqs.load(Ordering::Relaxed),
            full_reqs: self.full_reqs.load(Ordering::Relaxed),
            routes,
        }
    }
}
