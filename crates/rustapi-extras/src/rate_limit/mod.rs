//! Rate limiting middleware.
//!
//! This module provides IP-based rate limiting to protect your API from
//! abuse and ensure fair usage.
//!
//! # Example
//!
//! ```ignore
//! use rustapi_extras::rate_limit::RateLimitLayer;
//! use std::time::Duration;
//!
//! // Allow 100 requests per minute per IP
//! let rate_limit = RateLimitLayer::new(100, Duration::from_secs(60));
//! ```

use bytes::Bytes;
use dashmap::DashMap;
use http::StatusCode;
use http_body_util::Full;
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{Request, Response, ResponseBody};
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Internal entry for tracking rate limit state per client.
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// Internal store for tracking request counts per IP.
#[derive(Debug)]
struct RateLimitStore {
    entries: DashMap<IpAddr, RateLimitEntry>,
}

impl RateLimitStore {
    fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Check and update rate limit for a client IP.
    /// Returns (is_allowed, current_count, remaining, reset_timestamp)
    fn check_and_update(
        &self,
        ip: IpAddr,
        max_requests: u32,
        window: Duration,
    ) -> (bool, u32, u32, u64) {
        let now = Instant::now();
        let mut entry = self.entries.entry(ip).or_insert_with(|| RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Check if window has expired and reset if needed
        if now.duration_since(entry.window_start) >= window {
            entry.count = 0;
            entry.window_start = now;
        }

        // Increment count
        entry.count += 1;
        let current_count = entry.count;

        // Calculate remaining
        let remaining = max_requests.saturating_sub(current_count);
        let is_allowed = current_count <= max_requests;

        // Calculate actual reset timestamp based on window start
        let elapsed = now.duration_since(entry.window_start);
        let time_until_reset = window.saturating_sub(elapsed);
        let actual_reset = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + time_until_reset.as_secs();

        (is_allowed, current_count, remaining, actual_reset)
    }

    /// Get current rate limit info for a client without incrementing.
    #[allow(dead_code)]
    fn get_info(&self, ip: IpAddr, max_requests: u32, window: Duration) -> Option<RateLimitInfo> {
        let now = Instant::now();

        self.entries.get(&ip).map(|entry| {
            // Check if window has expired
            let (count, window_start) = if now.duration_since(entry.window_start) >= window {
                (0, now)
            } else {
                (entry.count, entry.window_start)
            };

            let remaining = max_requests.saturating_sub(count);
            let elapsed = now.duration_since(window_start);
            let time_until_reset = window.saturating_sub(elapsed);
            let reset = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + time_until_reset.as_secs();

            RateLimitInfo {
                limit: max_requests,
                remaining,
                reset,
            }
        })
    }
}

/// Rate limiting middleware layer.
///
/// Tracks request counts per client IP and returns 429 Too Many Requests
/// when the limit is exceeded.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::rate_limit::RateLimitLayer;
/// use std::time::Duration;
///
/// let app = RustApi::new()
///     .layer(RateLimitLayer::new(100, Duration::from_secs(60)))
///     .route("/api", get(handler));
/// ```
#[derive(Clone)]
pub struct RateLimitLayer {
    requests: u32,
    window: Duration,
    store: Arc<RateLimitStore>,
}

impl RateLimitLayer {
    /// Create a new rate limit layer.
    ///
    /// # Arguments
    ///
    /// * `requests` - Maximum number of requests allowed per window
    /// * `window` - Duration of the rate limit window
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rustapi_extras::rate_limit::RateLimitLayer;
    /// use std::time::Duration;
    ///
    /// // Allow 100 requests per minute
    /// let layer = RateLimitLayer::new(100, Duration::from_secs(60));
    /// ```
    pub fn new(requests: u32, window: Duration) -> Self {
        Self {
            requests,
            window,
            store: Arc::new(RateLimitStore::new()),
        }
    }

    /// Get the configured request limit.
    pub fn requests(&self) -> u32 {
        self.requests
    }

    /// Get the configured window duration.
    pub fn window(&self) -> Duration {
        self.window
    }

    /// Get the internal store (for testing purposes).
    #[cfg(test)]
    #[allow(dead_code)]
    #[allow(private_interfaces)]
    pub(crate) fn store(&self) -> &Arc<RateLimitStore> {
        &self.store
    }

    /// Extract client IP from request.
    ///
    /// Checks X-Forwarded-For header first, then falls back to a default IP.
    fn extract_client_ip(req: &Request) -> IpAddr {
        // Try X-Forwarded-For header first
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // Take the first IP in the chain (original client)
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }

        // Try X-Real-IP header
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }

        // Default to localhost if no IP can be determined
        // In a real server, this would come from the socket address
        "127.0.0.1".parse().unwrap()
    }
}

impl MiddlewareLayer for RateLimitLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let store = self.store.clone();
        let max_requests = self.requests;
        let window = self.window;

        Box::pin(async move {
            let client_ip = RateLimitLayer::extract_client_ip(&req);

            let (is_allowed, _count, remaining, reset) =
                store.check_and_update(client_ip, max_requests, window);

            if !is_allowed {
                // Calculate Retry-After in seconds
                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let retry_after = reset.saturating_sub(now_secs);

                // Return 429 Too Many Requests
                let error_body = serde_json::json!({
                    "error": {
                        "type": "rate_limit_exceeded",
                        "message": "Too many requests",
                        "retry_after": retry_after
                    }
                });

                let body = serde_json::to_vec(&error_body).unwrap_or_default();

                return http::Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .header("X-RateLimit-Limit", max_requests.to_string())
                    .header("X-RateLimit-Remaining", "0")
                    .header("X-RateLimit-Reset", reset.to_string())
                    .header("Retry-After", retry_after.to_string())
                    .body(ResponseBody::Full(Full::new(Bytes::from(body))))
                    .unwrap();
            }

            // Continue to handler and add rate limit headers to response
            let mut response = next(req).await;

            // Add rate limit headers to successful responses
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                max_requests.to_string().parse().unwrap(),
            );
            headers.insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            headers.insert("X-RateLimit-Reset", reset.to_string().parse().unwrap());

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Information about rate limit status for a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitInfo {
    /// Maximum requests allowed per window.
    pub limit: u32,
    /// Remaining requests in current window.
    pub remaining: u32,
    /// Unix timestamp when the window resets.
    pub reset: u64,
}

/// Create a 429 Too Many Requests response (for testing).
#[cfg(test)]
#[allow(dead_code)]
fn create_rate_limit_response(limit: u32, reset: u64, retry_after: u64) -> Response {
    let error_body = serde_json::json!({
        "error": {
            "type": "rate_limit_exceeded",
            "message": "Too many requests",
            "retry_after": retry_after
        }
    });

    let body = serde_json::to_vec(&error_body).unwrap_or_default();

    http::Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header("X-RateLimit-Limit", limit.to_string())
        .header("X-RateLimit-Remaining", "0")
        .header("X-RateLimit-Reset", reset.to_string())
        .header("Retry-After", retry_after.to_string())
        .body(ResponseBody::Full(Full::new(Bytes::from(body))))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Method, StatusCode};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use rustapi_core::middleware::LayerStack;
    use std::sync::Arc;

    /// Create a test request with optional X-Forwarded-For header
    fn create_test_request(ip: Option<&str>) -> Request {
        let uri: http::Uri = "/test".parse().unwrap();
        let mut builder = http::Request::builder().method(Method::GET).uri(uri);

        if let Some(ip_str) = ip {
            builder = builder.header("X-Forwarded-For", ip_str);
        }

        let req = builder.body(()).unwrap();
        Request::from_http_request(req, Bytes::new())
    }

    /// Create a simple success handler
    fn create_success_handler() -> BoxedNext {
        Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(StatusCode::OK)
                    .body(ResponseBody::Full(Full::new(Bytes::from("success"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        })
    }

    /// Strategy for generating valid IPv4 addresses
    fn ipv4_strategy() -> impl Strategy<Value = String> {
        (1u8..255, 0u8..255, 0u8..255, 1u8..255)
            .prop_map(|(a, b, c, d)| format!("{}.{}.{}.{}", a, b, c, d))
    }

    /// Strategy for generating rate limit configurations
    fn rate_limit_config_strategy() -> impl Strategy<Value = (u32, u64)> {
        // requests: 1-100, window_secs: 1-60
        (1u32..100, 1u64..60)
    }

    // **Feature: phase3-batteries-included, Property 11: Rate limit state tracking**
    //
    // For any rate limit configuration (N requests per window W) and client IP, after K requests
    // where K ≤ N, the response headers SHALL show `X-RateLimit-Remaining: N-K` and
    // `X-RateLimit-Limit: N`.
    //
    // **Validates: Requirements 4.1, 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_rate_limit_state_tracking(
            (max_requests, window_secs) in rate_limit_config_strategy(),
            num_requests in 1u32..50,
            ip in ipv4_strategy(),
        ) {
            // Ensure we don't exceed the limit for this test
            let num_requests = num_requests.min(max_requests);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                let layer = RateLimitLayer::new(max_requests, Duration::from_secs(window_secs));
                let mut stack = LayerStack::new();
                stack.push(Box::new(layer));

                // Make num_requests requests
                for k in 1..=num_requests {
                    let handler = create_success_handler();
                    let request = create_test_request(Some(&ip));
                    let response = stack.execute(request, handler).await;

                    // Should be allowed (within limit)
                    prop_assert_eq!(
                        response.status(),
                        StatusCode::OK,
                        "Request {} of {} should be allowed",
                        k,
                        num_requests
                    );

                    // Check X-RateLimit-Limit header
                    let limit_header = response.headers().get("X-RateLimit-Limit");
                    prop_assert!(limit_header.is_some(), "X-RateLimit-Limit header should be present");
                    let limit_value: u32 = limit_header.unwrap().to_str().unwrap().parse().unwrap();
                    prop_assert_eq!(
                        limit_value,
                        max_requests,
                        "X-RateLimit-Limit should equal configured limit"
                    );

                    // Check X-RateLimit-Remaining header
                    let remaining_header = response.headers().get("X-RateLimit-Remaining");
                    prop_assert!(remaining_header.is_some(), "X-RateLimit-Remaining header should be present");
                    let remaining_value: u32 = remaining_header.unwrap().to_str().unwrap().parse().unwrap();
                    let expected_remaining = max_requests.saturating_sub(k);
                    prop_assert_eq!(
                        remaining_value,
                        expected_remaining,
                        "X-RateLimit-Remaining should be {} after {} requests (limit: {})",
                        expected_remaining,
                        k,
                        max_requests
                    );

                    // Check X-RateLimit-Reset header exists
                    let reset_header = response.headers().get("X-RateLimit-Reset");
                    prop_assert!(reset_header.is_some(), "X-RateLimit-Reset header should be present");
                }

                Ok(())
            });
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 12: Rate limit enforcement**
    //
    // For any rate limit configuration (N requests per window), the (N+1)th request from the
    // same IP within the window SHALL return 429 with `Retry-After` header.
    //
    // **Validates: Requirements 4.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_rate_limit_enforcement(
            max_requests in 1u32..20,
            window_secs in 10u64..120,
            ip in ipv4_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                let layer = RateLimitLayer::new(max_requests, Duration::from_secs(window_secs));
                let mut stack = LayerStack::new();
                stack.push(Box::new(layer));

                // Make max_requests requests (all should succeed)
                for k in 1..=max_requests {
                    let handler = create_success_handler();
                    let request = create_test_request(Some(&ip));
                    let response = stack.execute(request, handler).await;

                    prop_assert_eq!(
                        response.status(),
                        StatusCode::OK,
                        "Request {} of {} should be allowed",
                        k,
                        max_requests
                    );
                }

                // The (N+1)th request should be rejected with 429
                let handler = create_success_handler();
                let request = create_test_request(Some(&ip));
                let response = stack.execute(request, handler).await;

                prop_assert_eq!(
                    response.status(),
                    StatusCode::TOO_MANY_REQUESTS,
                    "Request {} should be rejected with 429",
                    max_requests + 1
                );

                // Check Retry-After header is present
                let retry_after = response.headers().get("Retry-After");
                prop_assert!(retry_after.is_some(), "Retry-After header should be present on 429 response");

                // Check X-RateLimit-Remaining is 0
                let remaining = response.headers().get("X-RateLimit-Remaining");
                prop_assert!(remaining.is_some(), "X-RateLimit-Remaining should be present");
                let remaining_value: u32 = remaining.unwrap().to_str().unwrap().parse().unwrap();
                prop_assert_eq!(remaining_value, 0, "X-RateLimit-Remaining should be 0 when limit exceeded");

                // Verify response body contains error type
                let body_bytes = {
                    use http_body_util::BodyExt;
                    let body = response.into_body();
                    body.collect().await.unwrap().to_bytes()
                };
                let body_str = String::from_utf8_lossy(&body_bytes);

                prop_assert!(
                    body_str.contains("\"type\":\"rate_limit_exceeded\"") ||
                    body_str.contains("\"type\": \"rate_limit_exceeded\""),
                    "Response body should contain error type 'rate_limit_exceeded', got: {}",
                    body_str
                );

                Ok(())
            });
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 13: Rate limit window reset**
    //
    // For any rate limit configuration with window W, after W time has elapsed since the first
    // request, the request count for that client SHALL reset to 0.
    //
    // **Validates: Requirements 4.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_rate_limit_window_reset(
            max_requests in 1u32..10,
            ip in ipv4_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                // Use a very short window for testing (10ms)
                let window = Duration::from_millis(10);
                let layer = RateLimitLayer::new(max_requests, window);
                let mut stack = LayerStack::new();
                stack.push(Box::new(layer));

                // Exhaust the rate limit
                for _ in 0..max_requests {
                    let handler = create_success_handler();
                    let request = create_test_request(Some(&ip));
                    let response = stack.execute(request, handler).await;
                    prop_assert_eq!(response.status(), StatusCode::OK);
                }

                // Verify limit is exhausted
                let handler = create_success_handler();
                let request = create_test_request(Some(&ip));
                let response = stack.execute(request, handler).await;
                prop_assert_eq!(
                    response.status(),
                    StatusCode::TOO_MANY_REQUESTS,
                    "Should be rate limited after exhausting limit"
                );

                // Wait for window to expire
                tokio::time::sleep(window + Duration::from_millis(5)).await;

                // After window expires, requests should be allowed again
                let handler = create_success_handler();
                let request = create_test_request(Some(&ip));
                let response = stack.execute(request, handler).await;

                prop_assert_eq!(
                    response.status(),
                    StatusCode::OK,
                    "Request should be allowed after window reset"
                );

                // Check that remaining is reset to max_requests - 1 (since we just made one request)
                let remaining = response.headers().get("X-RateLimit-Remaining");
                prop_assert!(remaining.is_some());
                let remaining_value: u32 = remaining.unwrap().to_str().unwrap().parse().unwrap();
                prop_assert_eq!(
                    remaining_value,
                    max_requests - 1,
                    "Remaining should be {} after window reset and one request",
                    max_requests - 1
                );

                Ok(())
            });
            result?;
        }
    }

    // Unit tests for edge cases

    #[test]
    fn test_rate_limit_layer_creation() {
        let layer = RateLimitLayer::new(100, Duration::from_secs(60));
        assert_eq!(layer.requests(), 100);
        assert_eq!(layer.window(), Duration::from_secs(60));
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let request = create_test_request(Some("192.168.1.1, 10.0.0.1"));
        let ip = RateLimitLayer::extract_client_ip(&request);
        assert_eq!(ip, "192.168.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_client_ip_single_ip() {
        let request = create_test_request(Some("192.168.1.100"));
        let ip = RateLimitLayer::extract_client_ip(&request);
        assert_eq!(ip, "192.168.1.100".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_extract_client_ip_default() {
        let request = create_test_request(None);
        let ip = RateLimitLayer::extract_client_ip(&request);
        assert_eq!(ip, "127.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_different_ips_have_separate_limits() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let layer = RateLimitLayer::new(2, Duration::from_secs(60));
            let mut stack = LayerStack::new();
            stack.push(Box::new(layer));

            // Exhaust limit for IP 1
            for _ in 0..2 {
                let handler = create_success_handler();
                let request = create_test_request(Some("192.168.1.1"));
                let response = stack.execute(request, handler).await;
                assert_eq!(response.status(), StatusCode::OK);
            }

            // IP 1 should be rate limited
            let handler = create_success_handler();
            let request = create_test_request(Some("192.168.1.1"));
            let response = stack.execute(request, handler).await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

            // IP 2 should still be allowed
            let handler = create_success_handler();
            let request = create_test_request(Some("192.168.1.2"));
            let response = stack.execute(request, handler).await;
            assert_eq!(response.status(), StatusCode::OK);
        });
    }

    #[test]
    fn test_rate_limit_response_body_format() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let layer = RateLimitLayer::new(1, Duration::from_secs(60));
            let mut stack = LayerStack::new();
            stack.push(Box::new(layer));

            // First request succeeds
            let handler = create_success_handler();
            let request = create_test_request(Some("10.0.0.1"));
            let response = stack.execute(request, handler).await;
            assert_eq!(response.status(), StatusCode::OK);

            // Second request should be rate limited
            let handler = create_success_handler();
            let request = create_test_request(Some("10.0.0.1"));
            let response = stack.execute(request, handler).await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

            // Check response body
            use http_body_util::BodyExt;
            let body = response.into_body();
            let body_bytes = body.collect().await.unwrap().to_bytes();
            let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

            assert_eq!(body_json["error"]["type"], "rate_limit_exceeded");
            assert_eq!(body_json["error"]["message"], "Too many requests");
            assert!(body_json["error"]["retry_after"].is_number());
        });
    }
}
