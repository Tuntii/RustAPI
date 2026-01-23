//! Retry middleware with exponential backoff
//!
//! This module provides automatic retry logic for failed requests with configurable
//! backoff strategies and max attempts.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::RetryLayer;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(
//!             RetryLayer::new()
//!                 .max_attempts(3)
//!                 .initial_backoff(Duration::from_millis(100))
//!         )
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Retry strategy for failed requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Exponential back off (delay doubles each time)
    Exponential,
    /// Linear backoff (delay increases linearly)
    Linear,
}

/// Configuration for retry behavior
#[derive(Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (excluding the initial attempt)
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration (cap for exponential/linear growth)
    pub max_backoff: Duration,
    /// Retry strategy to use
    pub strategy: RetryStrategy,
    /// Which HTTP status codes to retry
    pub retryable_statuses: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            strategy: RetryStrategy::Exponential,
            // Retry on 5xx errors and 429 (Too Many Requests)
            retryable_statuses: vec![429, 500, 502, 503, 504],
        }
    }
}

/// Retry middleware layer
#[derive(Clone)]
pub struct RetryLayer {
    config: RetryConfig,
}

impl RetryLayer {
    /// Create a new retry layer with default configuration
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }

    /// Set the maximum number of retry attempts
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.config.max_attempts = attempts;
        self
    }

    /// Set the initial backoff duration
    pub fn initial_backoff(mut self, duration: Duration) -> Self {
        self.config.initial_backoff = duration;
        self
    }

    /// Set the maximum backoff duration
    pub fn max_backoff(mut self, duration: Duration) -> Self {
        self.config.max_backoff = duration;
        self
    }

    /// Set the retry strategy
    pub fn strategy(mut self, strategy: RetryStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    /// Set which HTTP status codes should trigger a retry
    pub fn retryable_statuses(mut self, statuses: Vec<u16>) -> Self {
        self.config.retryable_statuses = statuses;
        self
    }

    /// Calculate backoff duration for a given attempt number
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base = self.config.initial_backoff;

        let calculated = match self.config.strategy {
            RetryStrategy::Fixed => base,
            RetryStrategy::Exponential => {
                // 2^attempt * base
                base * 2_u32.saturating_pow(attempt)
            }
            RetryStrategy::Linear => {
                // (attempt + 1) * base
                base * (attempt + 1)
            }
        };

        // Cap at max_backoff
        calculated.min(self.config.max_backoff)
    }
}

impl Default for RetryLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for RetryLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let self_clone = self.clone(); // Clone self to access its methods

        Box::pin(async move {
            let mut current_req = req;

            for attempt in 0..=config.max_attempts {
                // Determine if we need to clone for a potential future retry
                let (req_to_send, next_req_opt) = if attempt < config.max_attempts {
                    if let Some(cloned) = current_req.try_clone() {
                        (current_req, Some(cloned))
                    } else {
                        // Cloning failed, we can't retry after this
                        (current_req, None)
                    }
                } else {
                    (current_req, None)
                };

                let response = next(req_to_send).await;
                let status = response.status().as_u16();

                // Check if we should retry
                if attempt < config.max_attempts && config.retryable_statuses.contains(&status) {
                    if let Some(req) = next_req_opt {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = config.max_attempts,
                            status = status,
                            "Request failed, retrying..."
                        );

                        // Restore request for next attempt
                        current_req = req;

                        // Calculate and sleep for backoff duration
                        let backoff = self_clone.calculate_backoff(attempt);
                        tracing::debug!(backoff_ms = backoff.as_millis(), "Waiting before retry");
                        tokio::time::sleep(backoff).await;

                        continue;
                    }
                }

                // Success or no more retries
                if attempt > 0 {
                    tracing::info!(
                        attempt = attempt + 1,
                        status = status,
                        "Request succeeded after retry"
                    );
                }
                return response;
            }

            // Should be unreachable if logic is correct, but safe fallback
            unreachable!("Retry loop finished without returning response")
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rustapi_core::ResponseBody;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retry_on_503_error() {
        let retry_layer = RetryLayer::new().max_attempts(2);

        let attempt_counter = Arc::new(AtomicU32::new(0));
        let counter_clone = attempt_counter.clone();

        let next: BoxedNext = Arc::new(move |_req: Request| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);

                // Fail first 2 times, succeed on 3rd
                let status = if attempt < 2 { 503 } else { 200 };

                http::Response::builder()
                    .status(status)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = Request::from_http_request(
            http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap(),
            Bytes::new(),
        );

        let response = retry_layer.call(req, next).await;

        // Should succeed after retries
        assert_eq!(response.status(), 200);
        // Should have made 3 attempts total (1 initial + 2 retries)
        assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn exponential_backoff_calculation() {
        let layer = RetryLayer::new()
            .strategy(RetryStrategy::Exponential)
            .initial_backoff(Duration::from_millis(100));

        assert_eq!(layer.calculate_backoff(0), Duration::from_millis(100)); // 2^0 * 100
        assert_eq!(layer.calculate_backoff(1), Duration::from_millis(200)); // 2^1 * 100
        assert_eq!(layer.calculate_backoff(2), Duration::from_millis(400)); // 2^2 * 100
        assert_eq!(layer.calculate_backoff(3), Duration::from_millis(800)); // 2^3 * 100
    }

    #[test]
    fn linear_backoff_calculation() {
        let layer = RetryLayer::new()
            .strategy(RetryStrategy::Linear)
            .initial_backoff(Duration::from_millis(100));

        assert_eq!(layer.calculate_backoff(0), Duration::from_millis(100)); // 1 * 100
        assert_eq!(layer.calculate_backoff(1), Duration::from_millis(200)); // 2 * 100
        assert_eq!(layer.calculate_backoff(2), Duration::from_millis(300)); // 3 * 100
    }

    #[test]
    fn backoff_respects_max() {
        let layer = RetryLayer::new()
            .strategy(RetryStrategy::Exponential)
            .initial_backoff(Duration::from_secs(1))
            .max_backoff(Duration::from_secs(5));

        // 2^10 = 1024 seconds, but should be capped at 5
        assert_eq!(layer.calculate_backoff(10), Duration::from_secs(5));
    }
}
