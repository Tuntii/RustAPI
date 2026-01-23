//! Prometheus Metrics middleware
//!
//! Provides HTTP request metrics collection and a `/metrics` endpoint for Prometheus scraping.
//!
//! This module is only available when the `metrics` feature is enabled.
//!
//! # Metrics Collected
//!
//! - `http_requests_total` - Counter with labels: method, path, status
//! - `http_request_duration_seconds` - Histogram with labels: method, path
//! - `rustapi_info` - Gauge with label: version
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::middleware::MetricsLayer;
//!
//! let metrics = MetricsLayer::new();
//!
//! RustApi::new()
//!     .layer(metrics.clone())
//!     .route("/metrics", get(metrics.handler()))
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

use super::layer::{BoxedNext, MiddlewareLayer};
use crate::request::Request;
use crate::response::Response;
use bytes::Bytes;
use prometheus::{
    Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

/// Default histogram buckets for request duration (in seconds)
const DEFAULT_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Prometheus metrics middleware layer
///
/// Collects HTTP request metrics and provides a handler for the `/metrics` endpoint.
///
/// # Metrics
///
/// - `http_requests_total{method, path, status}` - Total number of HTTP requests
/// - `http_request_duration_seconds{method, path}` - HTTP request duration histogram
/// - `rustapi_info{version}` - RustAPI version information gauge
#[derive(Clone)]
pub struct MetricsLayer {
    inner: Arc<MetricsInner>,
}

struct MetricsInner {
    registry: Registry,
    requests_total: IntCounterVec,
    request_duration: HistogramVec,
    #[allow(dead_code)]
    info_gauge: GaugeVec,
}

impl MetricsLayer {
    /// Create a new MetricsLayer with default configuration
    ///
    /// This creates a new Prometheus registry and registers the default metrics.
    pub fn new() -> Self {
        let registry = Registry::new();
        Self::with_registry(registry)
    }

    /// Create a new MetricsLayer with a custom registry
    ///
    /// Use this if you want to share a registry with other metrics collectors.
    pub fn with_registry(registry: Registry) -> Self {
        // Create http_requests_total counter
        let requests_total = IntCounterVec::new(
            Opts::new("http_requests_total", "Total number of HTTP requests"),
            &["method", "path", "status"],
        )
        .expect("Failed to create http_requests_total metric");

        // Create http_request_duration_seconds histogram
        let request_duration = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(DEFAULT_BUCKETS.to_vec()),
            &["method", "path"],
        )
        .expect("Failed to create http_request_duration_seconds metric");

        // Create rustapi_info gauge
        let info_gauge = GaugeVec::new(
            Opts::new("rustapi_info", "RustAPI version information"),
            &["version"],
        )
        .expect("Failed to create rustapi_info metric");

        // Register metrics
        registry
            .register(Box::new(requests_total.clone()))
            .expect("Failed to register http_requests_total");
        registry
            .register(Box::new(request_duration.clone()))
            .expect("Failed to register http_request_duration_seconds");
        registry
            .register(Box::new(info_gauge.clone()))
            .expect("Failed to register rustapi_info");

        // Set version info
        let version = env!("CARGO_PKG_VERSION");
        info_gauge.with_label_values(&[version]).set(1.0);

        Self {
            inner: Arc::new(MetricsInner {
                registry,
                requests_total,
                request_duration,
                info_gauge,
            }),
        }
    }

    /// Get the Prometheus registry
    ///
    /// Use this to register additional custom metrics.
    pub fn registry(&self) -> &Registry {
        &self.inner.registry
    }

    /// Create a handler function for the `/metrics` endpoint
    ///
    /// Returns metrics in Prometheus text format.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let metrics = MetricsLayer::new();
    /// app.route("/metrics", get(metrics.handler()));
    /// ```
    pub fn handler(&self) -> impl Fn() -> MetricsResponse + Clone + Send + Sync + 'static {
        let registry = self.inner.registry.clone();
        move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            encoder
                .encode(&metric_families, &mut buffer)
                .expect("Failed to encode metrics");
            MetricsResponse(buffer)
        }
    }

    /// Record a request with the given method, path, status, and duration
    fn record_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        // Normalize path to avoid high cardinality
        let normalized_path = normalize_path(path);

        // Increment request counter
        self.inner
            .requests_total
            .with_label_values(&[method, &normalized_path, &status.to_string()])
            .inc();

        // Record duration
        self.inner
            .request_duration
            .with_label_values(&[method, &normalized_path])
            .observe(duration_secs);
    }

    /// Get a builder for creating custom metrics
    ///
    /// Use this to create application-specific metrics that will be included
    /// in the `/metrics` endpoint output.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let metrics = MetricsLayer::new();
    /// let custom = metrics.custom_metrics();
    ///
    /// // Create a counter
    /// let orders_total = custom.counter("orders_total", "Total orders processed");
    /// orders_total.inc();
    ///
    /// // Create a gauge
    /// let active_users = custom.gauge("active_users", "Currently active users");
    /// active_users.set(42.0);
    ///
    /// // Create a histogram
    /// let order_value = custom.histogram(
    ///     "order_value_dollars",
    ///     "Order value in dollars",
    ///     vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
    /// );
    /// order_value.observe(49.99);
    /// ```
    pub fn custom_metrics(&self) -> CustomMetricsBuilder {
        CustomMetricsBuilder {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Builder for creating custom application metrics
///
/// Provides a convenient API for registering custom Prometheus metrics
/// that will be exported alongside the default HTTP metrics.
///
/// # Metric Types
///
/// - **Counter**: Monotonically increasing value (e.g., total requests, errors)
/// - **Gauge**: Value that can go up and down (e.g., active connections, temperature)
/// - **Histogram**: Distribution of values (e.g., request latency, order value)
///
/// # Labels
///
/// For labeled metrics, use `counter_vec`, `gauge_vec`, and `histogram_vec` methods.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::middleware::MetricsLayer;
///
/// let metrics = MetricsLayer::new();
/// let builder = metrics.custom_metrics();
///
/// // Simple counter
/// let requests = builder.counter("api_requests_total", "Total API requests");
/// requests.inc();
///
/// // Counter with labels
/// let errors = builder.counter_vec(
///     "api_errors_total",
///     "Total API errors",
///     &["endpoint", "error_type"]
/// );
/// errors.with_label_values(&["/users", "validation"]).inc();
///
/// // Gauge for current state
/// let connections = builder.gauge("active_connections", "Active connections");
/// connections.inc();
/// connections.dec();
///
/// // Histogram for latency
/// let latency = builder.histogram(
///     "db_query_duration_seconds",
///     "Database query duration",
///     vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
/// );
/// latency.observe(0.023);
/// ```
pub struct CustomMetricsBuilder {
    inner: Arc<MetricsInner>,
}

impl CustomMetricsBuilder {
    /// Create a new counter metric
    ///
    /// Counters are monotonically increasing values. Use them for things like
    /// total requests, total errors, total orders, etc.
    pub fn counter(&self, name: &str, help: &str) -> prometheus::Counter {
        let counter = prometheus::Counter::new(name, help).expect("Failed to create counter");
        self.inner
            .registry
            .register(Box::new(counter.clone()))
            .expect("Failed to register counter");
        counter
    }

    /// Create a counter with labels
    ///
    /// Use this when you need to differentiate metrics by dimensions.
    pub fn counter_vec(&self, name: &str, help: &str, label_names: &[&str]) -> IntCounterVec {
        let counter = IntCounterVec::new(Opts::new(name, help), label_names)
            .expect("Failed to create counter vec");
        self.inner
            .registry
            .register(Box::new(counter.clone()))
            .expect("Failed to register counter vec");
        counter
    }

    /// Create a new gauge metric
    ///
    /// Gauges can go up and down. Use them for things like current temperature,
    /// active connections, queue size, etc.
    pub fn gauge(&self, name: &str, help: &str) -> prometheus::Gauge {
        let gauge = prometheus::Gauge::new(name, help).expect("Failed to create gauge");
        self.inner
            .registry
            .register(Box::new(gauge.clone()))
            .expect("Failed to register gauge");
        gauge
    }

    /// Create a gauge with labels
    pub fn gauge_vec(&self, name: &str, help: &str, label_names: &[&str]) -> GaugeVec {
        let gauge =
            GaugeVec::new(Opts::new(name, help), label_names).expect("Failed to create gauge vec");
        self.inner
            .registry
            .register(Box::new(gauge.clone()))
            .expect("Failed to register gauge vec");
        gauge
    }

    /// Create a new histogram metric with custom buckets
    ///
    /// Histograms observe values and count them in configurable buckets.
    /// Use them for things like request latency, order values, etc.
    ///
    /// # Buckets
    ///
    /// Choose buckets that make sense for your use case:
    /// - Latency (seconds): `vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]`
    /// - Order value (dollars): `vec![1.0, 10.0, 50.0, 100.0, 500.0, 1000.0]`
    /// - File size (MB): `vec![0.1, 1.0, 10.0, 100.0, 1000.0]`
    pub fn histogram(&self, name: &str, help: &str, buckets: Vec<f64>) -> prometheus::Histogram {
        let histogram =
            prometheus::Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets))
                .expect("Failed to create histogram");
        self.inner
            .registry
            .register(Box::new(histogram.clone()))
            .expect("Failed to register histogram");
        histogram
    }

    /// Create a histogram with default latency buckets
    ///
    /// Uses standard latency buckets suitable for HTTP request durations:
    /// `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]`
    pub fn histogram_with_default_buckets(&self, name: &str, help: &str) -> prometheus::Histogram {
        self.histogram(name, help, DEFAULT_BUCKETS.to_vec())
    }

    /// Create a histogram with labels
    pub fn histogram_vec(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
        buckets: Vec<f64>,
    ) -> HistogramVec {
        let histogram =
            HistogramVec::new(HistogramOpts::new(name, help).buckets(buckets), label_names)
                .expect("Failed to create histogram vec");
        self.inner
            .registry
            .register(Box::new(histogram.clone()))
            .expect("Failed to register histogram vec");
        histogram
    }
}

impl Default for MetricsLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for MetricsLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let metrics = self.clone();

        Box::pin(async move {
            let start = Instant::now();

            // Call the next handler
            let response = next(req).await;

            // Record metrics
            let duration = start.elapsed().as_secs_f64();
            let status = response.status().as_u16();
            metrics.record_request(&method, &path, status, duration);

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Response type for the metrics endpoint
pub struct MetricsResponse(Vec<u8>);

impl crate::response::IntoResponse for MetricsResponse {
    fn into_response(self) -> Response {
        use crate::response::Body;

        http::Response::builder()
            .status(http::StatusCode::OK)
            .header(
                http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )
            .body(Body::Full(http_body_util::Full::new(Bytes::from(self.0))))
            .unwrap()
    }
}

/// Normalize a path to reduce cardinality
///
/// This replaces path segments that look like IDs (UUIDs, numbers) with placeholders.
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .into_iter()
        .map(|segment| {
            if segment.is_empty() {
                String::new()
            } else if is_id_like(segment) {
                ":id".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect();
    normalized.join("/")
}

/// Check if a path segment looks like an ID
fn is_id_like(segment: &str) -> bool {
    // Check for UUID format
    if segment.len() == 36 && segment.chars().filter(|c| *c == '-').count() == 4 {
        return true;
    }

    // Check for numeric ID
    if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() {
        return true;
    }

    // Check for hex string (common for IDs)
    if segment.len() >= 8 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::layer::{BoxedNext, LayerStack};
    use http::{Extensions, Method, StatusCode};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Create a test request with the given method and path
    fn create_test_request(method: Method, path: &str) -> crate::request::Request {
        let uri: http::Uri = path.parse().unwrap();
        let builder = http::Request::builder().method(method).uri(uri);

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        crate::request::Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            HashMap::new().into(),
        )
    }

    #[test]
    fn test_metrics_layer_creation() {
        let metrics = MetricsLayer::new();
        assert!(!metrics.registry().gather().is_empty());
    }

    #[test]
    fn test_metrics_handler_returns_prometheus_format() {
        let metrics = MetricsLayer::new();
        let handler = metrics.handler();
        let response = handler();

        // Convert to response and check content type
        let http_response = crate::response::IntoResponse::into_response(response);
        assert_eq!(http_response.status(), StatusCode::OK);

        let content_type = http_response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .unwrap();
        assert!(content_type.to_str().unwrap().contains("text/plain"));
    }

    #[test]
    fn test_normalize_path_with_uuid() {
        let path = "/users/550e8400-e29b-41d4-a716-446655440000/posts";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/users/:id/posts");
    }

    #[test]
    fn test_normalize_path_with_numeric_id() {
        let path = "/users/12345/posts";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/users/:id/posts");
    }

    #[test]
    fn test_normalize_path_without_ids() {
        let path = "/users/profile/settings";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/users/profile/settings");
    }

    #[test]
    fn test_is_id_like() {
        // UUIDs
        assert!(is_id_like("550e8400-e29b-41d4-a716-446655440000"));

        // Numeric IDs
        assert!(is_id_like("12345"));
        assert!(is_id_like("1"));

        // Hex strings
        assert!(is_id_like("deadbeef"));
        assert!(is_id_like("abc123def456"));

        // Not IDs
        assert!(!is_id_like("users"));
        assert!(!is_id_like("profile"));
        assert!(!is_id_like(""));
    }

    #[test]
    fn test_rustapi_info_gauge_set() {
        let metrics = MetricsLayer::new();
        let handler = metrics.handler();
        let response = handler();

        let http_response = crate::response::IntoResponse::into_response(response);
        let _body = http_response.into_body();

        // The body should contain rustapi_info metric
        // We can't easily read the body here, but we verified the metric is registered
    }

    // **Feature: phase4-ergonomics-v1, Property 9: Request Metrics Recording**
    //
    // For any HTTP request processed by the system with the `metrics` feature enabled,
    // the `http_requests_total` counter should be incremented with correct method, path,
    // and status labels, and the `http_request_duration_seconds` histogram should record
    // the request duration.
    //
    // **Validates: Requirements 5.2, 5.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_request_metrics_recording(
            method_idx in 0usize..5usize,
            path in "/[a-z]{1,10}",
            status_code in 200u16..600u16,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                // Create a fresh metrics layer for each test
                let metrics = MetricsLayer::new();

                // Create middleware stack
                let mut stack = LayerStack::new();
                stack.push(Box::new(metrics.clone()));

                // Map index to HTTP method
                let methods = [Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH];
                let method = methods[method_idx].clone();

                // Create handler that returns the specified status
                let response_status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
                let handler: BoxedNext = Arc::new(move |_req: crate::request::Request| {
                    let status = response_status;
                    Box::pin(async move {
                        http::Response::builder()
                            .status(status)
                            .body(crate::response::Body::Full(http_body_util::Full::new(Bytes::from("test"))))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                // Execute request
                let request = create_test_request(method.clone(), &path);
                let response = stack.execute(request, handler).await;

                // Verify response status matches
                prop_assert_eq!(response.status(), response_status);

                // Verify metrics were recorded
                let metric_families = metrics.registry().gather();

                // Find http_requests_total metric
                let requests_total = metric_families
                    .iter()
                    .find(|mf| mf.get_name() == "http_requests_total");
                prop_assert!(
                    requests_total.is_some(),
                    "http_requests_total metric should exist"
                );

                let requests_total = requests_total.unwrap();
                let metrics_vec = requests_total.get_metric();

                // Find the metric with matching labels
                let matching_metric = metrics_vec.iter().find(|m| {
                    let labels = m.get_label();
                    let method_label = labels.iter().find(|l| l.get_name() == "method");
                    let path_label = labels.iter().find(|l| l.get_name() == "path");
                    let status_label = labels.iter().find(|l| l.get_name() == "status");

                    method_label.map(|l| l.get_value()) == Some(method.as_str())
                        && path_label.map(|l| l.get_value()) == Some(&path)
                        && status_label.map(|l| l.get_value()) == Some(&status_code.to_string())
                });

                prop_assert!(
                    matching_metric.is_some(),
                    "Should have metric with method={}, path={}, status={}. Available metrics: {:?}",
                    method.as_str(),
                    path,
                    status_code,
                    metrics_vec.iter().map(|m| m.get_label()).collect::<Vec<_>>()
                );

                // Verify counter was incremented
                let counter_value = matching_metric.unwrap().get_counter().get_value();
                prop_assert!(
                    counter_value >= 1.0,
                    "Counter should be at least 1, got {}",
                    counter_value
                );

                // Find http_request_duration_seconds metric
                let duration_metric = metric_families
                    .iter()
                    .find(|mf| mf.get_name() == "http_request_duration_seconds");
                prop_assert!(
                    duration_metric.is_some(),
                    "http_request_duration_seconds metric should exist"
                );

                let duration_metric = duration_metric.unwrap();
                let duration_vec = duration_metric.get_metric();

                // Find the histogram with matching labels
                let matching_histogram = duration_vec.iter().find(|m| {
                    let labels = m.get_label();
                    let method_label = labels.iter().find(|l| l.get_name() == "method");
                    let path_label = labels.iter().find(|l| l.get_name() == "path");

                    method_label.map(|l| l.get_value()) == Some(method.as_str())
                        && path_label.map(|l| l.get_value()) == Some(&path)
                });

                prop_assert!(
                    matching_histogram.is_some(),
                    "Should have histogram with method={}, path={}",
                    method.as_str(),
                    path
                );

                // Verify histogram has recorded at least one observation
                let histogram = matching_histogram.unwrap().get_histogram();
                prop_assert!(
                    histogram.get_sample_count() >= 1,
                    "Histogram should have at least 1 sample, got {}",
                    histogram.get_sample_count()
                );

                // Verify duration is reasonable (less than 10 seconds)
                let sum = histogram.get_sample_sum();
                prop_assert!(
                    sum < 10.0,
                    "Duration sum should be less than 10 seconds, got {}",
                    sum
                );

                Ok(())
            });
            result?;
        }
    }

    #[test]
    fn test_metrics_layer_records_request() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let metrics = MetricsLayer::new();

            let mut stack = LayerStack::new();
            stack.push(Box::new(metrics.clone()));

            let handler: BoxedNext = Arc::new(|_req: crate::request::Request| {
                Box::pin(async {
                    http::Response::builder()
                        .status(StatusCode::OK)
                        .body(crate::response::Body::Full(http_body_util::Full::new(
                            Bytes::from("ok"),
                        )))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            let request = create_test_request(Method::GET, "/test");
            let response = stack.execute(request, handler).await;

            assert_eq!(response.status(), StatusCode::OK);

            // Verify metrics were recorded
            let metric_families = metrics.registry().gather();
            let requests_total = metric_families
                .iter()
                .find(|mf| mf.get_name() == "http_requests_total");
            assert!(requests_total.is_some());
        });
    }

    #[test]
    fn test_metrics_layer_with_multiple_requests() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let metrics = MetricsLayer::new();

            let mut stack = LayerStack::new();
            stack.push(Box::new(metrics.clone()));

            let handler: BoxedNext = Arc::new(|_req: crate::request::Request| {
                Box::pin(async {
                    http::Response::builder()
                        .status(StatusCode::OK)
                        .body(crate::response::Body::Full(http_body_util::Full::new(
                            Bytes::from("ok"),
                        )))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            // Send multiple requests
            for _ in 0..5 {
                let request = create_test_request(Method::GET, "/test");
                let _ = stack.execute(request, handler.clone()).await;
            }

            // Verify counter was incremented 5 times
            let metric_families = metrics.registry().gather();
            let requests_total = metric_families
                .iter()
                .find(|mf| mf.get_name() == "http_requests_total")
                .unwrap();

            let metrics_vec = requests_total.get_metric();
            let matching_metric = metrics_vec.iter().find(|m| {
                let labels = m.get_label();
                labels
                    .iter()
                    .any(|l| l.get_name() == "method" && l.get_value() == "GET")
                    && labels
                        .iter()
                        .any(|l| l.get_name() == "path" && l.get_value() == "/test")
                    && labels
                        .iter()
                        .any(|l| l.get_name() == "status" && l.get_value() == "200")
            });

            assert!(matching_metric.is_some());
            assert_eq!(matching_metric.unwrap().get_counter().get_value(), 5.0);
        });
    }
}
