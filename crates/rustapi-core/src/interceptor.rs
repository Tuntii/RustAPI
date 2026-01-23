//! Request/Response Interceptor System for RustAPI
//!
//! This module provides interceptors that can modify requests before handlers
//! and responses after handlers, without the complexity of Tower layers.
//!
//! # Overview
//!
//! Interceptors provide a simpler alternative to middleware for common use cases:
//! - Adding headers to all requests/responses
//! - Logging and metrics
//! - Request/response transformation
//!
//! # Execution Order
//!
//! Request interceptors execute in registration order (1 → 2 → 3 → Handler).
//! Response interceptors execute in reverse order (Handler → 3 → 2 → 1).
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{RustApi, interceptor::{RequestInterceptor, ResponseInterceptor}};
//!
//! struct AddRequestId;
//!
//! impl RequestInterceptor for AddRequestId {
//!     fn intercept(&self, mut req: Request) -> Request {
//!         req.extensions_mut().insert(uuid::Uuid::new_v4());
//!         req
//!     }
//! }
//!
//! struct AddServerHeader;
//!
//! impl ResponseInterceptor for AddServerHeader {
//!     fn intercept(&self, mut res: Response) -> Response {
//!         res.headers_mut().insert("X-Server", "RustAPI".parse().unwrap());
//!         res
//!     }
//! }
//!
//! RustApi::new()
//!     .request_interceptor(AddRequestId)
//!     .response_interceptor(AddServerHeader)
//!     .route("/", get(handler))
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

use crate::request::Request;
use crate::response::Response;

/// Trait for intercepting and modifying requests before they reach handlers.
///
/// Request interceptors are executed in the order they are registered.
/// Each interceptor receives the request, can modify it, and returns the
/// (potentially modified) request for the next interceptor or handler.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::interceptor::RequestInterceptor;
/// use rustapi_core::Request;
///
/// struct LoggingInterceptor;
///
/// impl RequestInterceptor for LoggingInterceptor {
///     fn intercept(&self, req: Request) -> Request {
///         println!("Request: {} {}", req.method(), req.path());
///         req
///     }
/// }
/// ```
pub trait RequestInterceptor: Send + Sync + 'static {
    /// Intercept and optionally modify the request.
    ///
    /// The returned request will be passed to the next interceptor or handler.
    fn intercept(&self, request: Request) -> Request;

    /// Clone this interceptor into a boxed trait object.
    fn clone_box(&self) -> Box<dyn RequestInterceptor>;
}

impl Clone for Box<dyn RequestInterceptor> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Trait for intercepting and modifying responses after handlers complete.
///
/// Response interceptors are executed in reverse registration order.
/// Each interceptor receives the response, can modify it, and returns the
/// (potentially modified) response for the previous interceptor or client.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::interceptor::ResponseInterceptor;
/// use rustapi_core::Response;
///
/// struct AddCorsHeaders;
///
/// impl ResponseInterceptor for AddCorsHeaders {
///     fn intercept(&self, mut res: Response) -> Response {
///         res.headers_mut().insert(
///             "Access-Control-Allow-Origin",
///             "*".parse().unwrap()
///         );
///         res
///     }
/// }
/// ```
pub trait ResponseInterceptor: Send + Sync + 'static {
    /// Intercept and optionally modify the response.
    ///
    /// The returned response will be passed to the previous interceptor or client.
    fn intercept(&self, response: Response) -> Response;

    /// Clone this interceptor into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ResponseInterceptor>;
}

impl Clone for Box<dyn ResponseInterceptor> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Chain of request and response interceptors.
///
/// Manages the execution of multiple interceptors in the correct order:
/// - Request interceptors: executed in registration order (first registered = first executed)
/// - Response interceptors: executed in reverse order (last registered = first executed)
#[derive(Clone, Default)]
pub struct InterceptorChain {
    request_interceptors: Vec<Box<dyn RequestInterceptor>>,
    response_interceptors: Vec<Box<dyn ResponseInterceptor>>,
}

impl InterceptorChain {
    /// Create a new empty interceptor chain.
    pub fn new() -> Self {
        Self {
            request_interceptors: Vec::new(),
            response_interceptors: Vec::new(),
        }
    }

    /// Add a request interceptor to the chain.
    ///
    /// Interceptors are executed in the order they are added.
    pub fn add_request_interceptor<I: RequestInterceptor>(&mut self, interceptor: I) {
        self.request_interceptors.push(Box::new(interceptor));
    }

    /// Add a response interceptor to the chain.
    ///
    /// Interceptors are executed in reverse order (last added = first executed after handler).
    pub fn add_response_interceptor<I: ResponseInterceptor>(&mut self, interceptor: I) {
        self.response_interceptors.push(Box::new(interceptor));
    }

    /// Get the number of request interceptors.
    pub fn request_interceptor_count(&self) -> usize {
        self.request_interceptors.len()
    }

    /// Get the number of response interceptors.
    pub fn response_interceptor_count(&self) -> usize {
        self.response_interceptors.len()
    }

    /// Check if the chain has any interceptors.
    pub fn is_empty(&self) -> bool {
        self.request_interceptors.is_empty() && self.response_interceptors.is_empty()
    }

    /// Execute all request interceptors on the given request.
    ///
    /// Interceptors are executed in registration order.
    /// Each interceptor receives the output of the previous one.
    pub fn intercept_request(&self, mut request: Request) -> Request {
        for interceptor in &self.request_interceptors {
            request = interceptor.intercept(request);
        }
        request
    }

    /// Execute all response interceptors on the given response.
    ///
    /// Interceptors are executed in reverse registration order.
    /// Each interceptor receives the output of the previous one.
    pub fn intercept_response(&self, mut response: Response) -> Response {
        // Execute in reverse order (last registered = first to process response)
        for interceptor in self.response_interceptors.iter().rev() {
            response = interceptor.intercept(response);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_params::PathParams;
    use bytes::Bytes;
    use http::{Extensions, Method, StatusCode};

    use proptest::prelude::*;
    use std::sync::Arc;

    /// Create a test request with the given method and path
    fn create_test_request(method: Method, path: &str) -> Request {
        let uri: http::Uri = path.parse().unwrap();
        let builder = http::Request::builder().method(method).uri(uri);

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        )
    }

    /// Create a test response with the given status
    fn create_test_response(status: StatusCode) -> Response {
        http::Response::builder()
            .status(status)
            .body(crate::response::Body::from("test"))
            .unwrap()
    }

    /// A request interceptor that adds a header tracking its ID
    #[derive(Clone)]
    struct TrackingRequestInterceptor {
        id: usize,
        order: Arc<std::sync::Mutex<Vec<usize>>>,
    }

    impl TrackingRequestInterceptor {
        fn new(id: usize, order: Arc<std::sync::Mutex<Vec<usize>>>) -> Self {
            Self { id, order }
        }
    }

    impl RequestInterceptor for TrackingRequestInterceptor {
        fn intercept(&self, request: Request) -> Request {
            self.order.lock().unwrap().push(self.id);
            request
        }

        fn clone_box(&self) -> Box<dyn RequestInterceptor> {
            Box::new(self.clone())
        }
    }

    /// A response interceptor that adds a header tracking its ID
    #[derive(Clone)]
    struct TrackingResponseInterceptor {
        id: usize,
        order: Arc<std::sync::Mutex<Vec<usize>>>,
    }

    impl TrackingResponseInterceptor {
        fn new(id: usize, order: Arc<std::sync::Mutex<Vec<usize>>>) -> Self {
            Self { id, order }
        }
    }

    impl ResponseInterceptor for TrackingResponseInterceptor {
        fn intercept(&self, response: Response) -> Response {
            self.order.lock().unwrap().push(self.id);
            response
        }

        fn clone_box(&self) -> Box<dyn ResponseInterceptor> {
            Box::new(self.clone())
        }
    }

    // **Feature: v1-features-roadmap, Property 6: Interceptor execution order**
    //
    // For any set of N registered interceptors, request interceptors SHALL execute
    // in registration order (1→N) and response interceptors SHALL execute in
    // reverse order (N→1).
    //
    // **Validates: Requirements 2.1, 2.2, 2.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_interceptor_execution_order(num_interceptors in 1usize..10usize) {
            let request_order = Arc::new(std::sync::Mutex::new(Vec::new()));
            let response_order = Arc::new(std::sync::Mutex::new(Vec::new()));

            let mut chain = InterceptorChain::new();

            // Add interceptors in order 0, 1, 2, ..., n-1
            for i in 0..num_interceptors {
                chain.add_request_interceptor(
                    TrackingRequestInterceptor::new(i, request_order.clone())
                );
                chain.add_response_interceptor(
                    TrackingResponseInterceptor::new(i, response_order.clone())
                );
            }

            // Execute request interceptors
            let request = create_test_request(Method::GET, "/test");
            let _ = chain.intercept_request(request);

            // Execute response interceptors
            let response = create_test_response(StatusCode::OK);
            let _ = chain.intercept_response(response);

            // Verify request interceptor order: should be 0, 1, 2, ..., n-1
            let req_order = request_order.lock().unwrap();
            prop_assert_eq!(req_order.len(), num_interceptors);
            for (idx, &id) in req_order.iter().enumerate() {
                prop_assert_eq!(id, idx, "Request interceptor order mismatch at index {}", idx);
            }

            // Verify response interceptor order: should be n-1, n-2, ..., 1, 0 (reverse)
            let res_order = response_order.lock().unwrap();
            prop_assert_eq!(res_order.len(), num_interceptors);
            for (idx, &id) in res_order.iter().enumerate() {
                let expected = num_interceptors - 1 - idx;
                prop_assert_eq!(id, expected, "Response interceptor order mismatch at index {}", idx);
            }
        }
    }

    /// A response interceptor that modifies a header
    #[derive(Clone)]
    struct HeaderModifyingResponseInterceptor {
        header_name: &'static str,
        header_value: String,
    }

    impl HeaderModifyingResponseInterceptor {
        fn new(header_name: &'static str, header_value: impl Into<String>) -> Self {
            Self {
                header_name,
                header_value: header_value.into(),
            }
        }
    }

    impl ResponseInterceptor for HeaderModifyingResponseInterceptor {
        fn intercept(&self, mut response: Response) -> Response {
            if let Ok(value) = self.header_value.parse() {
                response.headers_mut().insert(self.header_name, value);
            }
            response
        }

        fn clone_box(&self) -> Box<dyn ResponseInterceptor> {
            Box::new(self.clone())
        }
    }

    // **Feature: v1-features-roadmap, Property 7: Interceptor modification propagation**
    //
    // For any modification made by an interceptor, subsequent interceptors and handlers
    // SHALL receive the modified request/response.
    //
    // **Validates: Requirements 2.4, 2.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_interceptor_modification_propagation(
            num_interceptors in 1usize..5usize,
            header_values in prop::collection::vec("[a-zA-Z0-9]{1,10}", 1..5usize),
        ) {
            let mut chain = InterceptorChain::new();

            // Add response interceptors that each add a unique header
            for (i, value) in header_values.iter().enumerate().take(num_interceptors) {
                let header_name = Box::leak(format!("x-test-{}", i).into_boxed_str());
                chain.add_response_interceptor(
                    HeaderModifyingResponseInterceptor::new(header_name, value.clone())
                );
            }

            // Execute response interceptors
            let response = create_test_response(StatusCode::OK);
            let modified_response = chain.intercept_response(response);

            // Verify all headers were added (modifications propagated)
            for (i, value) in header_values.iter().enumerate().take(num_interceptors) {
                let header_name = format!("x-test-{}", i);
                let header_value = modified_response.headers().get(&header_name);
                prop_assert!(header_value.is_some(), "Header {} should be present", header_name);
                prop_assert_eq!(
                    header_value.unwrap().to_str().unwrap(),
                    value,
                    "Header {} should have value {}", header_name, value
                );
            }
        }
    }

    #[test]
    fn test_empty_chain() {
        let chain = InterceptorChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.request_interceptor_count(), 0);
        assert_eq!(chain.response_interceptor_count(), 0);

        // Should pass through unchanged
        let request = create_test_request(Method::GET, "/test");
        let _ = chain.intercept_request(request);

        let response = create_test_response(StatusCode::OK);
        let result = chain.intercept_response(response);
        assert_eq!(result.status(), StatusCode::OK);
    }

    #[test]
    fn test_single_request_interceptor() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut chain = InterceptorChain::new();
        chain.add_request_interceptor(TrackingRequestInterceptor::new(42, order.clone()));

        assert!(!chain.is_empty());
        assert_eq!(chain.request_interceptor_count(), 1);

        let request = create_test_request(Method::GET, "/test");
        let _ = chain.intercept_request(request);

        let recorded = order.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], 42);
    }

    #[test]
    fn test_single_response_interceptor() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut chain = InterceptorChain::new();
        chain.add_response_interceptor(TrackingResponseInterceptor::new(42, order.clone()));

        assert!(!chain.is_empty());
        assert_eq!(chain.response_interceptor_count(), 1);

        let response = create_test_response(StatusCode::OK);
        let _ = chain.intercept_response(response);

        let recorded = order.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], 42);
    }

    #[test]
    fn test_response_header_modification() {
        let mut chain = InterceptorChain::new();
        chain.add_response_interceptor(HeaderModifyingResponseInterceptor::new(
            "x-custom", "value1",
        ));
        chain.add_response_interceptor(HeaderModifyingResponseInterceptor::new(
            "x-another",
            "value2",
        ));

        let response = create_test_response(StatusCode::OK);
        let modified = chain.intercept_response(response);

        // Both headers should be present
        assert_eq!(
            modified
                .headers()
                .get("x-custom")
                .unwrap()
                .to_str()
                .unwrap(),
            "value1"
        );
        assert_eq!(
            modified
                .headers()
                .get("x-another")
                .unwrap()
                .to_str()
                .unwrap(),
            "value2"
        );
    }

    #[test]
    fn test_chain_clone() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut chain = InterceptorChain::new();
        chain.add_request_interceptor(TrackingRequestInterceptor::new(1, order.clone()));
        chain.add_response_interceptor(TrackingResponseInterceptor::new(2, order.clone()));

        // Clone the chain
        let cloned = chain.clone();

        assert_eq!(cloned.request_interceptor_count(), 1);
        assert_eq!(cloned.response_interceptor_count(), 1);
    }
}
