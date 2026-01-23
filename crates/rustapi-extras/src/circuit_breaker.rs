//! Circuit breaker middleware for resilient service calls
//!
//! This module implements the circuit breaker pattern to prevent cascading failures
//! and give failing services time to recover.
//!
//! # States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Too many failures, requests fail fast
//! - **HalfOpen**: Testing if service recovered
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::CircuitBreakerLayer;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(
//!             CircuitBreakerLayer::new()
//!                 .failure_threshold(5)
//!                 .timeout(Duration::from_secs(30))
//!         )
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response, ResponseBody,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally
    Closed,
    /// Circuit is open, requests fail fast
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Duration to wait before transitioning from Open to HalfOpen
    pub timeout: Duration,
    /// Number of successful requests in HalfOpen state before closing
    pub success_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker state tracker
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: usize,
    success_count: usize,
    last_failure_time: Option<Instant>,
    total_requests: u64,
    total_failures: u64,
    total_successes: u64,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            total_requests: 0,
            total_failures: 0,
            total_successes: 0,
        }
    }
}

/// Circuit break middleware layer
#[derive(Clone)]
pub struct CircuitBreakerLayer {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

impl CircuitBreakerLayer {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self {
            config: CircuitBreakerConfig::default(),
            state: Arc::new(RwLock::new(CircuitBreakerState::default())),
        }
    }

    /// Set the failure threshold
    pub fn failure_threshold(mut self, threshold: usize) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    /// Set the timeout before transitioning to half-open
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set the success threshold in half-open state
    pub fn success_threshold(mut self, threshold: usize) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    /// Get the current circuit state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.state
    }

    /// Get circuit breaker statistics
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        CircuitBreakerStats {
            state: state.state,
            total_requests: state.total_requests,
            total_failures: state.total_failures,
            total_successes: state.total_successes,
            failure_count: state.failure_count,
            success_count: state.success_count,
        }
    }

    /// Reset the circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::default();
    }
}

impl Default for CircuitBreakerLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitState,
    /// Total requests processed
    pub total_requests: u64,
    /// Total failures
    pub total_failures: u64,
    /// Total successes
    pub total_successes: u64,
    /// Current failure count
    pub failure_count: usize,
    /// Current success count (in half-open state)
    pub success_count: usize,
}

impl MiddlewareLayer for CircuitBreakerLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let state = self.state.clone();

        Box::pin(async move {
            // Check current state
            let mut state_guard = state.write().await;
            state_guard.total_requests += 1;

            match state_guard.state {
                CircuitState::Open => {
                    // Check if timeout has elapsed
                    if let Some(last_failure) = state_guard.last_failure_time {
                        if last_failure.elapsed() >= config.timeout {
                            // Transition to half-open
                            tracing::info!("Circuit breaker transitioning to HalfOpen");
                            state_guard.state = CircuitState::HalfOpen;
                            state_guard.success_count = 0;
                        } else {
                            // Still open, fail fast
                            drop(state_guard);
                            return http::Response::builder()
                                .status(503)
                                .header("Content-Type", "application/json")
                                .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from(
                                    serde_json::json!({
                                        "error": {
                                            "type": "service_unavailable",
                                            "message": "Circuit breaker is OPEN"
                                        }
                                    })
                                    .to_string(),
                                )))
                                .unwrap();
                        }
                    }
                }
                CircuitState::HalfOpen => {
                    // Allow request but monitor closely
                }
                CircuitState::Closed => {
                    // Normal operation
                }
            }

            drop(state_guard);

            // Execute request
            let response = next(req).await;

            // Update state based on result
            let mut state_guard = state.write().await;

            // Check if response indicates success (2xx status)
            if response.status().is_success() {
                state_guard.total_successes += 1;

                match state_guard.state {
                    CircuitState::HalfOpen => {
                        state_guard.success_count += 1;
                        if state_guard.success_count >= config.success_threshold {
                            // Transition to closed
                            tracing::info!("Circuit breaker transitioning to Closed");
                            state_guard.state = CircuitState::Closed;
                            state_guard.failure_count = 0;
                            state_guard.success_count = 0;
                        }
                    }
                    CircuitState::Closed => {
                        // Reset failure count on success
                        state_guard.failure_count = 0;
                    }
                    _ => {}
                }
            } else {
                // Non-2xx status is treated as failure
                record_failure(&mut state_guard, &config);
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

fn record_failure(state: &mut CircuitBreakerState, config: &CircuitBreakerConfig) {
    state.total_failures += 1;
    state.failure_count += 1;
    state.last_failure_time = Some(Instant::now());

    match state.state {
        CircuitState::Closed => {
            if state.failure_count >= config.failure_threshold {
                // Open the circuit
                tracing::warn!(
                    "Circuit breaker OPENING after {} failures",
                    state.failure_count
                );
                state.state = CircuitState::Open;
            }
        }
        CircuitState::HalfOpen => {
            // Failed in half-open, go back to open
            tracing::warn!("Circuit breaker returning to OPEN state");
            state.state = CircuitState::Open;
            state.success_count = 0;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::sync::Arc;

    #[tokio::test]
    async fn circuit_breaker_opens_after_threshold() {
        let breaker = CircuitBreakerLayer::new()
            .failure_threshold(3)
            .timeout(Duration::from_secs(1));

        // Create a handler that always fails
        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(500)
                    .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from("Error"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        // Make requests that fail
        for _ in 0..3 {
            let req = http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap();
            let req = Request::from_http_request(req, Bytes::new());

            let _ = breaker.call(req, next.clone()).await;
        }

        // Circuit should be open now
        let state = breaker.get_state().await;
        assert_eq!(state, CircuitState::Open);

        // Next request should fail fast
        let req = http::Request::builder()
            .method("GET")
            .uri("/")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = breaker.call(req, next.clone()).await;
        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn circuit_breaker_recovers() {
        let breaker = CircuitBreakerLayer::new()
            .failure_threshold(2)
            .timeout(Duration::from_millis(100))
            .success_threshold(2);

        // Fail requests to open circuit
        let fail_next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(500)
                    .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from("Error"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        for _ in 0..2 {
            let req = http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap();
            let req = Request::from_http_request(req, Bytes::new());
            let _ = breaker.call(req, fail_next.clone()).await;
        }

        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Make successful requests
        let success_next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from("OK"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        for _ in 0..2 {
            let req = http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap();
            let req = Request::from_http_request(req, Bytes::new());
            let result = breaker.call(req, success_next.clone()).await;
            assert!(result.status().is_success());
        }

        // Circuit should be closed now
        let state = breaker.get_state().await;
        assert_eq!(state, CircuitState::Closed);
    }
}
