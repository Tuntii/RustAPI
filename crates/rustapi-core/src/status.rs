use crate::response::IntoResponse;
use crate::{Request, Response};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Configuration for the Status Page
#[derive(Clone, Debug)]
pub struct StatusConfig {
    /// Path to serve the status page (default: "/status")
    pub path: String,
    /// Title of the status page
    pub title: String,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            path: "/status".to_string(),
            title: "System Status".to_string(),
        }
    }
}

impl StatusConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

/// Metrics for a specific endpoint
#[derive(Debug, Clone, Default)]
pub struct EndpointMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: u128,
    pub last_access: Option<String>,
}

impl EndpointMetrics {
    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.total_requests as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
}

/// Shared state for monitoring
#[derive(Debug, Default)]
pub struct StatusMonitor {
    /// Map of route path -> metrics
    metrics: RwLock<HashMap<String, EndpointMetrics>>,
    /// System start time
    start_time: Option<Instant>,
}

impl StatusMonitor {
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(HashMap::new()),
            start_time: Some(Instant::now()),
        }
    }

    pub fn record_request(&self, path: &str, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().unwrap();
        let entry = metrics.entry(path.to_string()).or_default();

        entry.total_requests += 1;
        if success {
            entry.successful_requests += 1;
        } else {
            entry.failed_requests += 1;
        }
        entry.total_latency_ms += duration.as_millis();

        entry.last_access = Some(format_unix_timestamp());
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(0))
    }

    pub fn get_snapshot(&self) -> HashMap<String, EndpointMetrics> {
        self.metrics.read().unwrap().clone()
    }
}

use crate::middleware::{BoxedNext, MiddlewareLayer};

/// Middleware layer for status monitoring
#[derive(Clone)]
pub struct StatusLayer {
    monitor: Arc<StatusMonitor>,
}

impl StatusLayer {
    pub fn new(monitor: Arc<StatusMonitor>) -> Self {
        Self { monitor }
    }
}

impl MiddlewareLayer for StatusLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let monitor = self.monitor.clone();
        let path = req.uri().path().to_string();

        Box::pin(async move {
            let start = Instant::now();
            let response = next(req).await;
            let duration = start.elapsed();

            let status = response.status();
            let success = status.is_success() || status.is_redirection();

            monitor.record_request(&path, duration, success);

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// HTML Status Page Handler
pub async fn status_handler(
    monitor: Arc<StatusMonitor>,
    config: StatusConfig,
) -> impl IntoResponse {
    let metrics = monitor.get_snapshot();
    let uptime = monitor.get_uptime();

    // Simple HTML template
    let mut html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; margin: 0; padding: 20px; background: #f0f2f5; color: #333; }}
        .header {{ background: #fff; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 20px; }}
        .header h1 {{ margin: 0; color: #2c3e50; }}
        .stats-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 20px; }}
        .stat-card {{ background: #fff; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .stat-value {{ font-size: 24px; font-weight: bold; color: #3498db; }}
        .stat-label {{ color: #7f8c8d; font-size: 14px; }}
        table {{ width: 100%; border-collapse: collapse; background: #fff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        th, td {{ padding: 12px 15px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #f8f9fa; font-weight: 600; color: #2c3e50; }}
        tr:hover {{ background-color: #f5f5f5; }}
        .status-ok {{ color: #27ae60; font-weight: bold; }}
        .status-err {{ color: #e74c3c; font-weight: bold; }}
    </style>
    <meta http-equiv="refresh" content="5">
</head>
<body>
    <div class="header">
        <h1>{title}</h1>
        <p>System Uptime: {uptime}</p>
    </div>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value">{total_reqs}</div>
            <div class="stat-label">Total Requests</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{total_eps}</div>
            <div class="stat-label">Active Endpoints</div>
        </div>
    </div>

    <table>
        <thead>
            <tr>
                <th>Endpoint</th>
                <th>Requests</th>
                <th>Success Rate</th>
                <th>Avg Latency</th>
                <th>Last Access</th>
            </tr>
        </thead>
        <tbody>
"#,
        title = config.title,
        uptime = format_duration(uptime),
        total_reqs = metrics.values().map(|m| m.total_requests).sum::<u64>(),
        total_eps = metrics.len()
    );

    // Sort metrics by path
    let mut sorted_metrics: Vec<_> = metrics.iter().collect();
    sorted_metrics.sort_by_key(|(k, _)| *k);

    for (path, m) in sorted_metrics {
        let success_class = if m.success_rate() > 95.0 {
            "status-ok"
        } else {
            "status-err"
        };

        html.push_str(&format!(
            r#"            <tr>
                <td><code>{}</code></td>
                <td>{}</td>
                <td class="{}">{:.1}%</td>
                <td>{:.2} ms</td>
                <td>{}</td>
            </tr>
"#,
            path,
            m.total_requests,
            success_class,
            m.success_rate(),
            m.avg_latency_ms(),
            m.last_access.as_deref().unwrap_or("-")
        ));
    }

    html.push_str(
        r#"        </tbody>
    </table>
</body>
</html>"#,
    );

    crate::response::Html(html)
}

fn format_duration(d: Duration) -> String {
    let seconds = d.as_secs();
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else {
        format!("{}m {}s", minutes, secs)
    }
}

fn format_unix_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("unix:{}", now.as_secs())
}
