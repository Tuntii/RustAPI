//! Tower Layer integration for RustAPI middleware
//!
//! This module provides the infrastructure for applying Tower-compatible layers
//! to the RustAPI request/response pipeline.

use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;

/// A boxed middleware function type
#[allow(dead_code)]
pub type BoxedMiddleware = Arc<
    dyn Fn(Request, BoxedNext) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        + Send
        + Sync,
>;

/// A boxed next function for middleware chains
pub type BoxedNext =
    Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> + Send + Sync>;

/// Trait for middleware that can be applied to RustAPI
///
/// This trait allows both Tower layers and custom middleware to be used
/// with the `.layer()` method.
pub trait MiddlewareLayer: Send + Sync + 'static {
    /// Apply this middleware to a request, calling `next` to continue the chain
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>>;

    /// Clone this middleware into a boxed trait object
    fn clone_box(&self) -> Box<dyn MiddlewareLayer>;
}

impl Clone for Box<dyn MiddlewareLayer> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A stack of middleware layers
#[derive(Clone, Default)]
pub struct LayerStack {
    layers: Vec<Box<dyn MiddlewareLayer>>,
}

impl LayerStack {
    /// Create a new empty layer stack
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a middleware layer to the stack
    ///
    /// Layers are executed in the order they are added (outermost first).
    pub fn push(&mut self, layer: Box<dyn MiddlewareLayer>) {
        self.layers.push(layer);
    }

    /// Add a middleware layer to the beginning of the stack
    ///
    /// This layer will be executed first (outermost).
    pub fn prepend(&mut self, layer: Box<dyn MiddlewareLayer>) {
        self.layers.insert(0, layer);
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Get the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Execute the middleware stack with a final handler
    pub fn execute(
        &self,
        req: Request,
        handler: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        if self.layers.is_empty() {
            return handler(req);
        }

        // Build the chain from inside out
        // The last layer added should be the outermost (first to execute)
        let mut next = handler;

        for layer in self.layers.iter().rev() {
            let layer = layer.clone_box();
            let current_next = next;
            next = Arc::new(move |req: Request| {
                let layer = layer.clone_box();
                let next = current_next.clone();
                Box::pin(async move { layer.call(req, next).await })
                    as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });
        }

        next(req)
    }
}

/// Wrapper to adapt a Tower Layer to RustAPI's middleware system
#[allow(dead_code)]
pub struct TowerLayerAdapter<L> {
    layer: L,
}

impl<L> TowerLayerAdapter<L>
where
    L: Clone + Send + Sync + 'static,
{
    /// Create a new adapter from a Tower layer
    #[allow(dead_code)]
    pub fn new(layer: L) -> Self {
        Self { layer }
    }
}

impl<L> Clone for TowerLayerAdapter<L>
where
    L: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
        }
    }
}

/// A simple service wrapper for the next handler in the chain
#[allow(dead_code)]
pub struct NextService {
    next: BoxedNext,
}

impl NextService {
    #[allow(dead_code)]
    pub fn new(next: BoxedNext) -> Self {
        Self { next }
    }
}

impl Clone for NextService {
    fn clone(&self) -> Self {
        Self {
            next: self.next.clone(),
        }
    }
}

impl Service<Request> for NextService {
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let next = self.next.clone();
        Box::pin(async move { Ok(next(req).await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Request;
    use crate::response::Response;
    use bytes::Bytes;
    use http::{Extensions, Method, StatusCode};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use std::collections::HashMap;

    /// Create a test request with the given method and path
    fn create_test_request(method: Method, path: &str) -> Request {
        let uri: http::Uri = path.parse().unwrap();
        let builder = http::Request::builder().method(method).uri(uri);

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        Request::new(
            parts,
            Bytes::new(),
            Arc::new(Extensions::new()),
            HashMap::new(),
        )
    }

    /// A simple test middleware that tracks execution order
    #[derive(Clone)]
    struct OrderTrackingMiddleware {
        id: usize,
        order: Arc<std::sync::Mutex<Vec<(usize, &'static str)>>>,
    }

    impl OrderTrackingMiddleware {
        fn new(id: usize, order: Arc<std::sync::Mutex<Vec<(usize, &'static str)>>>) -> Self {
            Self { id, order }
        }
    }

    impl MiddlewareLayer for OrderTrackingMiddleware {
        fn call(
            &self,
            req: Request,
            next: BoxedNext,
        ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
            let id = self.id;
            let order = self.order.clone();

            Box::pin(async move {
                // Record pre-handler execution
                order.lock().unwrap().push((id, "pre"));

                // Call next
                let response = next(req).await;

                // Record post-handler execution
                order.lock().unwrap().push((id, "post"));

                response
            })
        }

        fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
            Box::new(self.clone())
        }
    }

    /// A middleware that modifies the response status
    #[derive(Clone)]
    #[allow(dead_code)]
    struct StatusModifyingMiddleware {
        status: StatusCode,
    }

    #[allow(dead_code)]
    impl StatusModifyingMiddleware {
        fn new(status: StatusCode) -> Self {
            Self { status }
        }
    }

    impl MiddlewareLayer for StatusModifyingMiddleware {
        fn call(
            &self,
            req: Request,
            next: BoxedNext,
        ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
            let status = self.status;

            Box::pin(async move {
                let mut response = next(req).await;
                *response.status_mut() = status;
                response
            })
        }

        fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
            Box::new(self.clone())
        }
    }

    // **Feature: phase3-batteries-included, Property 1: Layer application preserves handler behavior**
    //
    // For any Tower-compatible layer L and handler H, applying L via `.layer(L)` SHALL result
    // in requests being processed by L before reaching H, and responses being processed by L
    // after leaving H.
    //
    // **Validates: Requirements 1.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_layer_application_preserves_handler_behavior(
            handler_status in 200u16..600u16,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let order = Arc::new(std::sync::Mutex::new(Vec::new()));

                // Create a layer stack with one middleware
                let mut stack = LayerStack::new();
                stack.push(Box::new(OrderTrackingMiddleware::new(1, order.clone())));

                // Create a handler that returns the specified status
                let handler_status = StatusCode::from_u16(handler_status).unwrap_or(StatusCode::OK);
                let handler: BoxedNext = Arc::new(move |_req: Request| {
                    let status = handler_status;
                    Box::pin(async move {
                        http::Response::builder()
                            .status(status)
                            .body(http_body_util::Full::new(Bytes::from("test")))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                // Execute through the stack
                let request = create_test_request(Method::GET, "/test");
                let response = stack.execute(request, handler).await;

                // Verify the handler was called (response has expected status)
                prop_assert_eq!(response.status(), handler_status);

                // Verify middleware executed in correct order (pre before post)
                let execution_order = order.lock().unwrap();
                prop_assert_eq!(execution_order.len(), 2);
                prop_assert_eq!(execution_order[0], (1, "pre"));
                prop_assert_eq!(execution_order[1], (1, "post"));

                Ok(())
            });
            result?;
        }
    }

    #[test]
    fn test_empty_layer_stack_calls_handler_directly() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let stack = LayerStack::new();

            let handler: BoxedNext = Arc::new(|_req: Request| {
                Box::pin(async {
                    http::Response::builder()
                        .status(StatusCode::OK)
                        .body(http_body_util::Full::new(Bytes::from("direct")))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            let request = create_test_request(Method::GET, "/test");
            let response = stack.execute(request, handler).await;

            assert_eq!(response.status(), StatusCode::OK);
        });
    }

    // **Feature: phase3-batteries-included, Property 2: Middleware execution order**
    //
    // For any sequence of layers [L1, L2, ..., Ln] added via `.layer()`, requests SHALL pass
    // through layers in the order L1 → L2 → ... → Ln → Handler → Ln → ... → L2 → L1
    // (outermost first on request, innermost first on response).
    //
    // **Validates: Requirements 1.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_middleware_execution_order(
            num_layers in 1usize..10usize,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let order = Arc::new(std::sync::Mutex::new(Vec::new()));

                // Create a layer stack with multiple middleware
                let mut stack = LayerStack::new();
                for i in 0..num_layers {
                    stack.push(Box::new(OrderTrackingMiddleware::new(i, order.clone())));
                }

                // Create a simple handler
                let handler: BoxedNext = Arc::new(|_req: Request| {
                    Box::pin(async {
                        http::Response::builder()
                            .status(StatusCode::OK)
                            .body(http_body_util::Full::new(Bytes::from("test")))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                // Execute through the stack
                let request = create_test_request(Method::GET, "/test");
                let _response = stack.execute(request, handler).await;

                // Verify execution order
                let execution_order = order.lock().unwrap();

                // Should have 2 * num_layers entries (pre and post for each)
                prop_assert_eq!(execution_order.len(), num_layers * 2);

                // First half should be "pre" in order 0, 1, 2, ... (outermost first)
                for i in 0..num_layers {
                    prop_assert_eq!(execution_order[i], (i, "pre"),
                        "Pre-handler order mismatch at index {}", i);
                }

                // Second half should be "post" in reverse order n-1, n-2, ..., 0 (innermost first)
                for i in 0..num_layers {
                    let expected_id = num_layers - 1 - i;
                    prop_assert_eq!(execution_order[num_layers + i], (expected_id, "post"),
                        "Post-handler order mismatch at index {}", i);
                }

                Ok(())
            });
            result?;
        }
    }

    /// A middleware that short-circuits with an error response without calling next
    #[derive(Clone)]
    struct ShortCircuitMiddleware {
        error_status: StatusCode,
        should_short_circuit: bool,
    }

    impl ShortCircuitMiddleware {
        fn new(error_status: StatusCode, should_short_circuit: bool) -> Self {
            Self {
                error_status,
                should_short_circuit,
            }
        }
    }

    impl MiddlewareLayer for ShortCircuitMiddleware {
        fn call(
            &self,
            req: Request,
            next: BoxedNext,
        ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
            let error_status = self.error_status;
            let should_short_circuit = self.should_short_circuit;

            Box::pin(async move {
                if should_short_circuit {
                    // Return error response without calling next (short-circuit)
                    http::Response::builder()
                        .status(error_status)
                        .body(http_body_util::Full::new(Bytes::from("error")))
                        .unwrap()
                } else {
                    // Continue to next middleware/handler
                    next(req).await
                }
            })
        }

        fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
            Box::new(self.clone())
        }
    }

    // **Feature: phase3-batteries-included, Property 4: Middleware short-circuit on error**
    //
    // For any middleware that returns an error response, the handler SHALL NOT be invoked,
    // and the error response SHALL be returned directly to the client.
    //
    // **Validates: Requirements 1.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_middleware_short_circuit_on_error(
            error_status in 400u16..600u16,
            num_middleware_before in 0usize..5usize,
            num_middleware_after in 0usize..5usize,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let order = Arc::new(std::sync::Mutex::new(Vec::new()));
                let handler_called = Arc::new(std::sync::atomic::AtomicBool::new(false));

                // Create a layer stack with middleware before the short-circuit
                let mut stack = LayerStack::new();

                // Add middleware before the short-circuit middleware
                for i in 0..num_middleware_before {
                    stack.push(Box::new(OrderTrackingMiddleware::new(i, order.clone())));
                }

                // Add the short-circuit middleware
                let error_status = StatusCode::from_u16(error_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                stack.push(Box::new(ShortCircuitMiddleware::new(error_status, true)));

                // Add middleware after the short-circuit middleware (these should NOT execute pre)
                for i in 0..num_middleware_after {
                    stack.push(Box::new(OrderTrackingMiddleware::new(100 + i, order.clone())));
                }

                // Create a handler that tracks if it was called
                let handler_called_clone = handler_called.clone();
                let handler: BoxedNext = Arc::new(move |_req: Request| {
                    let handler_called = handler_called_clone.clone();
                    Box::pin(async move {
                        handler_called.store(true, std::sync::atomic::Ordering::SeqCst);
                        http::Response::builder()
                            .status(StatusCode::OK)
                            .body(http_body_util::Full::new(Bytes::from("handler")))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                // Execute through the stack
                let request = create_test_request(Method::GET, "/test");
                let response = stack.execute(request, handler).await;

                // Verify the error response was returned
                prop_assert_eq!(response.status(), error_status,
                    "Response should have the error status from short-circuit middleware");

                // Verify the handler was NOT called
                prop_assert!(!handler_called.load(std::sync::atomic::Ordering::SeqCst),
                    "Handler should NOT be called when middleware short-circuits");

                // Verify execution order:
                // - Middleware before short-circuit should have "pre" recorded
                // - Middleware after short-circuit should NOT have "pre" recorded (never reached)
                // - All middleware before short-circuit should have "post" recorded (unwinding)
                let execution_order = order.lock().unwrap();

                // Count pre and post for middleware before short-circuit
                let pre_count = execution_order.iter().filter(|(id, phase)| *id < 100 && *phase == "pre").count();
                let post_count = execution_order.iter().filter(|(id, phase)| *id < 100 && *phase == "post").count();

                prop_assert_eq!(pre_count, num_middleware_before,
                    "All middleware before short-circuit should have pre recorded");
                prop_assert_eq!(post_count, num_middleware_before,
                    "All middleware before short-circuit should have post recorded (unwinding)");

                // Middleware after short-circuit should NOT have any entries
                let after_entries = execution_order.iter().filter(|(id, _)| *id >= 100).count();
                prop_assert_eq!(after_entries, 0,
                    "Middleware after short-circuit should NOT be executed");

                Ok(())
            });
            result?;
        }
    }

    #[test]
    fn test_short_circuit_returns_error_response() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut stack = LayerStack::new();
            stack.push(Box::new(ShortCircuitMiddleware::new(
                StatusCode::UNAUTHORIZED,
                true,
            )));

            let handler_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handler_called_clone = handler_called.clone();

            let handler: BoxedNext = Arc::new(move |_req: Request| {
                let handler_called = handler_called_clone.clone();
                Box::pin(async move {
                    handler_called.store(true, std::sync::atomic::Ordering::SeqCst);
                    http::Response::builder()
                        .status(StatusCode::OK)
                        .body(http_body_util::Full::new(Bytes::from("handler")))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            let request = create_test_request(Method::GET, "/test");
            let response = stack.execute(request, handler).await;

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!handler_called.load(std::sync::atomic::Ordering::SeqCst));
        });
    }
}
