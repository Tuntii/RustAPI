//! Integration tests for RustAPI framework
//!
//! These tests cover cross-cutting concerns that involve multiple crates working together.

#![allow(unused_imports)]
use rustapi_rs::prelude::*;

// ============================================================================
// Router Integration Tests
// ============================================================================

mod router_tests {
    use rustapi_rs::get;

    #[get("/integ-method-test")]
    async fn method_test() -> &'static str {
        "get"
    }

    #[test]
    fn test_router_method_routing() {
        let routes = rustapi_rs::collect_auto_routes();
        let found = routes
            .iter()
            .any(|r| r.path() == "/integ-method-test" && r.method() == "GET");

        assert!(found, "GET /integ-method-test should be registered");
    }

    #[get("/integ-users/{user_id}")]
    async fn user_handler(rustapi_rs::Path(user_id): rustapi_rs::Path<i64>) -> String {
        format!("user={}", user_id)
    }

    #[test]
    fn test_router_path_params() {
        let routes = rustapi_rs::collect_auto_routes();
        let found = routes.iter().any(|r| r.path() == "/integ-users/{user_id}");

        assert!(found, "Path param route should be registered");
    }
}

// ============================================================================
// State Management Tests
// ============================================================================

mod state_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone)]
    struct Counter(Arc<AtomicUsize>);

    impl Counter {
        fn new() -> Self {
            Self(Arc::new(AtomicUsize::new(0)))
        }

        fn increment(&self) -> usize {
            self.0.fetch_add(1, Ordering::SeqCst)
        }

        fn get(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn test_state_sharing() {
        let counter = Counter::new();

        // Simulate multiple handlers accessing state
        let c1 = counter.clone();
        let c2 = counter.clone();
        let c3 = counter.clone();

        c1.increment();
        c2.increment();
        c3.increment();

        assert_eq!(counter.get(), 3, "All handlers should share same state");
    }

    #[test]
    fn test_state_thread_safety() {
        use std::thread;

        let counter = Counter::new();
        let mut handles = vec![];

        for _ in 0..10 {
            let c = counter.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    c.increment();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(counter.get(), 1000, "All increments should be counted");
    }
}

// ============================================================================
// JSON Serialization Tests
// ============================================================================

mod json_tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i64,
        name: String,
        tags: Vec<String>,
        active: bool,
    }

    #[test]
    fn test_json_roundtrip() {
        let data = TestData {
            id: 42,
            name: "Test Item".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            active: true,
        };

        let json = serde_json::to_string(&data).unwrap();
        let parsed: TestData = serde_json::from_str(&json).unwrap();

        assert_eq!(data, parsed, "Data should survive JSON roundtrip");
    }

    #[test]
    fn test_json_error_format() {
        // Test that invalid JSON produces expected error
        let bad_json = r#"{"id": "not_a_number"}"#;
        let result: Result<TestData, _> = serde_json::from_str(bad_json);

        assert!(result.is_err(), "Should fail to parse invalid JSON");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid type"),
            "Error should mention type mismatch"
        );
    }

    #[test]
    fn test_json_with_special_chars() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TextData {
            content: String,
        }

        let data = TextData {
            content: "Hello \"World\" with\nnewlines\tand\ttabs".to_string(),
        };

        let json = serde_json::to_string(&data).unwrap();
        let parsed: TextData = serde_json::from_str(&json).unwrap();

        assert_eq!(data, parsed, "Special characters should be preserved");
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_tests {
    use rustapi_rs::prelude::*;

    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::not_found("User not found");
        assert_eq!(error.error_type, "not_found");
        assert_eq!(error.message, "User not found");
    }

    #[test]
    fn test_api_error_bad_request() {
        let error = ApiError::bad_request("Invalid input");
        assert_eq!(error.error_type, "bad_request");
        assert_eq!(error.message, "Invalid input");
    }

    #[test]
    fn test_api_error_validation() {
        let error = ApiError::validation(vec![rustapi_rs::FieldError {
            field: "email".to_string(),
            code: "email".to_string(),
            message: "Invalid email format".to_string(),
        }]);

        assert!(error.fields.is_some(), "Should have field errors");
        assert_eq!(error.fields.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_result_type_ok() {
        fn handler() -> Result<String, ApiError> {
            Ok("success".to_string())
        }

        let result = handler();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_result_type_err() {
        fn handler(fail: bool) -> Result<String, ApiError> {
            if fail {
                Err(ApiError::bad_request("Failed"))
            } else {
                Ok("success".to_string())
            }
        }

        assert!(handler(true).is_err());
        assert!(handler(false).is_ok());
    }
}

// ============================================================================
// OpenAPI Schema Tests
// ============================================================================

mod openapi_tests {
    use rustapi_rs::prelude::*;
    use utoipa::ToSchema;

    #[derive(Debug, Clone, Serialize, Schema)]
    struct IntegApiResponse {
        success: bool,
        data: Option<String>,
        count: i32,
    }

    #[test]
    fn test_schema_generation() {
        let (name, _schema) = <IntegApiResponse as ToSchema>::schema();

        assert_eq!(
            name, "IntegApiResponse",
            "Schema name should match struct name"
        );
    }

    #[test]
    fn test_auto_collects_schemas() {
        let app = RustApi::auto();
        let spec = app.openapi_spec();

        // Should have schemas section
        assert!(!spec.schemas.is_empty(), "OpenAPI spec should have schemas");
    }
}

// ============================================================================
// Extractor Tests
// ============================================================================

mod extractor_tests {
    #[test]
    fn test_path_parsing() {
        // Simulate path parameter parsing
        let path = "/users/123/posts/456";
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[1].parse::<i64>().unwrap(), 123);
        assert_eq!(segments[3].parse::<i64>().unwrap(), 456);
    }

    #[test]
    fn test_path_parsing_uuid() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.chars().filter(|c| *c == '-').count(), 4);
    }
}

// ============================================================================
// Compression Tests (basic - does not require feature flag)
// ============================================================================

mod compression_tests {
    #[test]
    fn test_accept_encoding_parsing() {
        let accept_encoding = "gzip, deflate, br";
        let encodings: Vec<&str> = accept_encoding.split(',').map(|s| s.trim()).collect();

        assert!(encodings.contains(&"gzip"));
        assert!(encodings.contains(&"deflate"));
        assert!(encodings.contains(&"br"));
    }

    #[test]
    fn test_content_type_check() {
        let compressible = [
            "text/html",
            "application/json",
            "text/css",
            "text/javascript",
        ];
        let not_compressible = ["image/png", "video/mp4", "application/zip"];

        for ct in &compressible {
            assert!(
                ct.starts_with("text/") || ct.contains("json") || ct.contains("xml"),
                "{} should be compressible",
                ct
            );
        }

        for ct in &not_compressible {
            assert!(
                !ct.starts_with("text/") && !ct.contains("json"),
                "{} should not be compressible",
                ct
            );
        }
    }
}

// ============================================================================
// Rate Limiting Tests (basic concepts)
// ============================================================================

mod rate_limit_tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct SimpleRateLimiter {
        counts: Arc<Mutex<HashMap<String, usize>>>,
        limit: usize,
    }

    impl SimpleRateLimiter {
        fn new(limit: usize) -> Self {
            Self {
                counts: Arc::new(Mutex::new(HashMap::new())),
                limit,
            }
        }

        fn check(&self, key: &str) -> bool {
            let mut counts = self.counts.lock().unwrap();
            let count = counts.entry(key.to_string()).or_insert(0);
            if *count < self.limit {
                *count += 1;
                true
            } else {
                false
            }
        }
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = SimpleRateLimiter::new(5);
        let ip = "192.168.1.1";

        for i in 0..5 {
            assert!(limiter.check(ip), "Request {} should be allowed", i + 1);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = SimpleRateLimiter::new(3);
        let ip = "192.168.1.2";

        // Use up the limit
        for _ in 0..3 {
            limiter.check(ip);
        }

        // Next request should be blocked
        assert!(!limiter.check(ip), "Request over limit should be blocked");
    }

    #[test]
    fn test_rate_limiter_multiple_ips() {
        let limiter = SimpleRateLimiter::new(2);

        let ip1 = "192.168.1.1";
        let ip2 = "192.168.1.2";

        // Each IP should have independent limit
        assert!(limiter.check(ip1));
        assert!(limiter.check(ip1));
        assert!(!limiter.check(ip1)); // Over limit

        assert!(limiter.check(ip2)); // Different IP, should work
        assert!(limiter.check(ip2));
        assert!(!limiter.check(ip2)); // Now over limit
    }
}
