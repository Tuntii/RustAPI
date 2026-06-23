#[cfg(feature = "cookies")]
use rustapi_core::Cookies;
use rustapi_core::{
    AsyncValidatedJson, BodyVariant, ClientIp, Extension, FromRequest, FromRequestParts,
    HeaderValue, Headers, PathParams, Request,
};

use bytes::Bytes;
use http::{Extensions, Method};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::Arc;

/// Create a test request with the given method, path, and headers
fn create_test_request_with_headers(
    method: Method,
    path: &str,
    headers: Vec<(&str, &str)>,
) -> Request {
    let uri: http::Uri = path.parse().unwrap();
    let mut builder = http::Request::builder().method(method).uri(uri);

    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let req = builder.body(()).unwrap();
    let (parts, _) = req.into_parts();

    Request::new(
        parts,
        BodyVariant::Buffered(Bytes::new()),
        Arc::new(Extensions::new()),
        PathParams::new(),
    )
}

/// Create a test request with extensions
fn create_test_request_with_extensions<T: Clone + Send + Sync + 'static>(
    method: Method,
    path: &str,
    extension: T,
) -> Request {
    let uri: http::Uri = path.parse().unwrap();
    let builder = http::Request::builder().method(method).uri(uri);

    let req = builder.body(()).unwrap();
    let (mut parts, _) = req.into_parts();
    parts.extensions.insert(extension);

    Request::new(
        parts,
        BodyVariant::Buffered(Bytes::new()),
        Arc::new(Extensions::new()),
        PathParams::new(),
    )
}

// **Feature: phase3-batteries-included, Property 14: Headers extractor completeness**
//
// For any request with headers H, the `Headers` extractor SHALL return a map
// containing all key-value pairs in H.
//
// **Validates: Requirements 5.1**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_headers_extractor_completeness(
        // Generate random header names and values
        // Using alphanumeric strings to ensure valid header names/values
        headers in prop::collection::vec(
            (
                "[a-z][a-z0-9-]{0,20}",  // Valid header name pattern
                "[a-zA-Z0-9 ]{1,50}"     // Valid header value pattern
            ),
            0..10
        )
    ) {
        let result: Result<(), TestCaseError> = (|| {
            // Convert to header tuples
            let header_tuples: Vec<(&str, &str)> = headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            // Create request with headers
            let request = create_test_request_with_headers(
                Method::GET,
                "/test",
                header_tuples.clone(),
            );

            // Extract headers
            let extracted = Headers::from_request_parts(&request)
                .map_err(|e| TestCaseError::fail(format!("Failed to extract headers: {}", e)))?;

            // Verify all original headers are present
            // HTTP allows duplicate headers - get_all() returns all values for a header name
            for (name, value) in &headers {
                // Check that the header name exists
                let all_values: Vec<_> = extracted.get_all(name.as_str()).iter().collect();
                prop_assert!(
                    !all_values.is_empty(),
                    "Header '{}' not found",
                    name
                );

                // Check that the value is among the extracted values
                let value_found = all_values.iter().any(|v| {
                    v.to_str().map(|s| s == value.as_str()).unwrap_or(false)
                });

                prop_assert!(
                    value_found,
                    "Header '{}' value '{}' not found in extracted values",
                    name,
                    value
                );
            }

            Ok(())
        })();
        result?;
    }
}

// **Feature: phase3-batteries-included, Property 15: HeaderValue extractor correctness**
//
// For any request with header "X" having value V, `HeaderValue::extract(req, "X")` SHALL return V;
// for requests without header "X", it SHALL return an error.
//
// **Validates: Requirements 5.2**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_header_value_extractor_correctness(
        header_name in "[a-z][a-z0-9-]{0,20}",
        header_value in "[a-zA-Z0-9 ]{1,50}",
        has_header in prop::bool::ANY,
    ) {
        let result: Result<(), TestCaseError> = (|| {
            let headers = if has_header {
                vec![(header_name.as_str(), header_value.as_str())]
            } else {
                vec![]
            };

            let _request = create_test_request_with_headers(Method::GET, "/test", headers);

            // We need to use a static string for the header name in the extractor
            // So we'll test with a known header name
            let test_header = "x-test-header";
            let request_with_known_header = if has_header {
                create_test_request_with_headers(
                    Method::GET,
                    "/test",
                    vec![(test_header, header_value.as_str())],
                )
            } else {
                create_test_request_with_headers(Method::GET, "/test", vec![])
            };

            let result = HeaderValue::extract(&request_with_known_header, test_header);

            if has_header {
                let extracted = result
                    .map_err(|e| TestCaseError::fail(format!("Expected header to be found: {}", e)))?;
                prop_assert_eq!(
                    extracted.value(),
                    header_value.as_str(),
                    "Header value mismatch"
                );
            } else {
                prop_assert!(
                    result.is_err(),
                    "Expected error when header is missing"
                );
            }

            Ok(())
        })();
        result?;
    }
}

// **Feature: phase3-batteries-included, Property 17: ClientIp extractor with forwarding**
//
// For any request with socket IP S and X-Forwarded-For header F, when forwarding is enabled,
// `ClientIp` SHALL return the first IP in F; when disabled, it SHALL return S.
//
// **Validates: Requirements 5.4**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_client_ip_extractor_with_forwarding(
        // Generate valid IPv4 addresses
        forwarded_ip in (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
            .prop_map(|(a, b, c, d)| format!("{}.{}.{}.{}", a, b, c, d)),
        socket_ip in (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
            .prop_map(|(a, b, c, d)| std::net::IpAddr::V4(std::net::Ipv4Addr::new(a, b, c, d))),
        has_forwarded_header in prop::bool::ANY,
        trust_proxy in prop::bool::ANY,
    ) {
        let result: Result<(), TestCaseError> = (|| {
            let headers = if has_forwarded_header {
                vec![("x-forwarded-for", forwarded_ip.as_str())]
            } else {
                vec![]
            };

            // Create request with headers
            let uri: http::Uri = "/test".parse().unwrap();
            let mut builder = http::Request::builder().method(Method::GET).uri(uri);
            for (name, value) in &headers {
                builder = builder.header(*name, *value);
            }
            let req = builder.body(()).unwrap();
            let (mut parts, _) = req.into_parts();

            // Add socket address to extensions
            let socket_addr = std::net::SocketAddr::new(socket_ip, 8080);
            parts.extensions.insert(socket_addr);

            let request = Request::new(
                parts,
                BodyVariant::Buffered(Bytes::new()),
                Arc::new(Extensions::new()),
                PathParams::new(),
            );

            let extracted = ClientIp::extract_with_config(&request, trust_proxy)
                .map_err(|e| TestCaseError::fail(format!("Failed to extract ClientIp: {}", e)))?;

            if trust_proxy && has_forwarded_header {
                // Should use X-Forwarded-For
                let expected_ip: std::net::IpAddr = forwarded_ip.parse()
                    .map_err(|e| TestCaseError::fail(format!("Invalid IP: {}", e)))?;
                prop_assert_eq!(
                    extracted.0,
                    expected_ip,
                    "Should use X-Forwarded-For IP when trust_proxy is enabled"
                );
            } else {
                // Should use socket IP
                prop_assert_eq!(
                    extracted.0,
                    socket_ip,
                    "Should use socket IP when trust_proxy is disabled or no X-Forwarded-For"
                );
            }

            Ok(())
        })();
        result?;
    }
}

// **Feature: phase3-batteries-included, Property 18: Extension extractor retrieval**
//
// For any type T and value V inserted into request extensions by middleware,
// `Extension<T>` SHALL return V.
//
// **Validates: Requirements 5.5**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_extension_extractor_retrieval(
        value in any::<i64>(),
        has_extension in prop::bool::ANY,
    ) {
        let result: Result<(), TestCaseError> = (|| {
            // Create a simple wrapper type for testing
            #[derive(Clone, Debug, PartialEq)]
            struct TestExtension(i64);

            let uri: http::Uri = "/test".parse().unwrap();
            let builder = http::Request::builder().method(Method::GET).uri(uri);
            let req = builder.body(()).unwrap();
            let (mut parts, _) = req.into_parts();

            if has_extension {
                parts.extensions.insert(TestExtension(value));
            }

            let request = Request::new(
                parts,
                BodyVariant::Buffered(Bytes::new()),
                Arc::new(Extensions::new()),
                PathParams::new(),
            );

            let result = Extension::<TestExtension>::from_request_parts(&request);

            if has_extension {
                let extracted = result
                    .map_err(|e| TestCaseError::fail(format!("Expected extension to be found: {}", e)))?;
                prop_assert_eq!(
                    extracted.0,
                    TestExtension(value),
                    "Extension value mismatch"
                );
            } else {
                prop_assert!(
                    result.is_err(),
                    "Expected error when extension is missing"
                );
            }

            Ok(())
        })();
        result?;
    }
}

// Unit tests for basic functionality

#[test]
fn test_headers_extractor_basic() {
    let request = create_test_request_with_headers(
        Method::GET,
        "/test",
        vec![
            ("content-type", "application/json"),
            ("accept", "text/html"),
        ],
    );

    let headers = Headers::from_request_parts(&request).unwrap();

    assert!(headers.contains("content-type"));
    assert!(headers.contains("accept"));
    assert!(!headers.contains("x-custom"));
    assert_eq!(headers.len(), 2);
}

#[test]
fn test_header_value_extractor_present() {
    let request = create_test_request_with_headers(
        Method::GET,
        "/test",
        vec![("authorization", "Bearer token123")],
    );

    let result = HeaderValue::extract(&request, "authorization");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().value(), "Bearer token123");
}

#[test]
fn test_header_value_extractor_missing() {
    let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

    let result = HeaderValue::extract(&request, "authorization");
    assert!(result.is_err());
}

#[test]
fn test_client_ip_from_forwarded_header() {
    let request = create_test_request_with_headers(
        Method::GET,
        "/test",
        vec![("x-forwarded-for", "192.168.1.100, 10.0.0.1")],
    );

    let ip = ClientIp::extract_with_config(&request, true).unwrap();
    assert_eq!(ip.0, "192.168.1.100".parse::<std::net::IpAddr>().unwrap());
}

#[test]
fn test_client_ip_ignores_forwarded_when_not_trusted() {
    let uri: http::Uri = "/test".parse().unwrap();
    let builder = http::Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header("x-forwarded-for", "192.168.1.100");
    let req = builder.body(()).unwrap();
    let (mut parts, _) = req.into_parts();

    let socket_addr = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        8080,
    );
    parts.extensions.insert(socket_addr);

    let request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::new()),
        Arc::new(Extensions::new()),
        PathParams::new(),
    );

    let ip = ClientIp::extract_with_config(&request, false).unwrap();
    assert_eq!(ip.0, "10.0.0.1".parse::<std::net::IpAddr>().unwrap());
}

#[test]
fn test_extension_extractor_present() {
    #[derive(Clone, Debug, PartialEq)]
    struct MyData(String);

    let request =
        create_test_request_with_extensions(Method::GET, "/test", MyData("hello".to_string()));

    let result = Extension::<MyData>::from_request_parts(&request);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, MyData("hello".to_string()));
}

#[test]
fn test_extension_extractor_missing() {
    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    struct MyData(String);

    let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

    let result = Extension::<MyData>::from_request_parts(&request);
    assert!(result.is_err());
}

// Cookies tests (feature-gated)
#[cfg(feature = "cookies")]
mod cookies_tests {
    use super::*;

    // **Feature: phase3-batteries-included, Property 16: Cookies extractor parsing**
    //
    // For any request with Cookie header containing cookies C, the `Cookies` extractor
    // SHALL return a CookieJar containing exactly the cookies in C.
    // Note: Duplicate cookie names result in only the last value being kept.
    //
    // **Validates: Requirements 5.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_cookies_extractor_parsing(
            // Generate random cookie names and values
            // Using alphanumeric strings to ensure valid cookie names/values
            cookies in prop::collection::vec(
                (
                    "[a-zA-Z][a-zA-Z0-9_]{0,15}",  // Valid cookie name pattern
                    "[a-zA-Z0-9]{1,30}"            // Valid cookie value pattern (no special chars)
                ),
                0..5
            )
        ) {
            let result: Result<(), TestCaseError> = (|| {
                // Build cookie header string
                let cookie_header = cookies
                    .iter()
                    .map(|(name, value)| format!("{}={}", name, value))
                    .collect::<Vec<_>>()
                    .join("; ");

                let headers = if !cookies.is_empty() {
                    vec![("cookie", cookie_header.as_str())]
                } else {
                    vec![]
                };

                let request = create_test_request_with_headers(Method::GET, "/test", headers);

                // Extract cookies
                let extracted = Cookies::from_request_parts(&request)
                    .map_err(|e| TestCaseError::fail(format!("Failed to extract cookies: {}", e)))?;

                // Build expected cookies map - last value wins for duplicate names
                let mut expected_cookies: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
                for (name, value) in &cookies {
                    expected_cookies.insert(name.as_str(), value.as_str());
                }

                // Verify all expected cookies are present with correct values
                for (name, expected_value) in &expected_cookies {
                    let cookie = extracted.get(name)
                        .ok_or_else(|| TestCaseError::fail(format!("Cookie '{}' not found", name)))?;

                    prop_assert_eq!(
                        cookie.value(),
                        *expected_value,
                        "Cookie '{}' value mismatch",
                        name
                    );
                }

                // Count cookies in jar should match unique cookie names
                let extracted_count = extracted.iter().count();
                prop_assert_eq!(
                    extracted_count,
                    expected_cookies.len(),
                    "Expected {} unique cookies, got {}",
                    expected_cookies.len(),
                    extracted_count
                );

                Ok(())
            })();
            result?;
        }
    }

    #[test]
    fn test_cookies_extractor_basic() {
        let request = create_test_request_with_headers(
            Method::GET,
            "/test",
            vec![("cookie", "session=abc123; user=john")],
        );

        let cookies = Cookies::from_request_parts(&request).unwrap();

        assert!(cookies.contains("session"));
        assert!(cookies.contains("user"));
        assert!(!cookies.contains("other"));

        assert_eq!(cookies.get("session").unwrap().value(), "abc123");
        assert_eq!(cookies.get("user").unwrap().value(), "john");
    }

    #[test]
    fn test_cookies_extractor_empty() {
        let request = create_test_request_with_headers(Method::GET, "/test", vec![]);

        let cookies = Cookies::from_request_parts(&request).unwrap();
        assert_eq!(cookies.iter().count(), 0);
    }

    #[test]
    fn test_cookies_extractor_single() {
        let request = create_test_request_with_headers(
            Method::GET,
            "/test",
            vec![("cookie", "token=xyz789")],
        );

        let cookies = Cookies::from_request_parts(&request).unwrap();
        assert_eq!(cookies.iter().count(), 1);
        assert_eq!(cookies.get("token").unwrap().value(), "xyz789");
    }
}

#[tokio::test]
async fn test_async_validated_json_with_state_context() {
    use async_trait::async_trait;
    use rustapi_validate::prelude::*;
    use rustapi_validate::v2::{AsyncValidationRule, DatabaseValidator, ValidationContextBuilder};
    use serde::{Deserialize, Serialize};

    struct MockDbValidator {
        unique_values: Vec<String>,
    }

    #[async_trait]
    impl DatabaseValidator for MockDbValidator {
        async fn exists(&self, _table: &str, _column: &str, _value: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_unique(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
        ) -> Result<bool, String> {
            Ok(!self.unique_values.contains(&value.to_string()))
        }
        async fn is_unique_except(
            &self,
            _table: &str,
            _column: &str,
            value: &str,
            _except_id: &str,
        ) -> Result<bool, String> {
            Ok(!self.unique_values.contains(&value.to_string()))
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct TestUser {
        email: String,
    }

    impl Validate for TestUser {
        fn validate_with_group(
            &self,
            _group: rustapi_validate::v2::ValidationGroup,
        ) -> Result<(), rustapi_validate::v2::ValidationErrors> {
            Ok(())
        }
    }

    #[async_trait]
    impl AsyncValidate for TestUser {
        async fn validate_async_with_group(
            &self,
            ctx: &ValidationContext,
            _group: rustapi_validate::v2::ValidationGroup,
        ) -> Result<(), rustapi_validate::v2::ValidationErrors> {
            let mut errors = rustapi_validate::v2::ValidationErrors::new();

            let rule = AsyncUniqueRule::new("users", "email");
            if let Err(e) = rule.validate_async(&self.email, ctx).await {
                errors.add("email", e);
            }

            errors.into_result()
        }
    }

    // Test 1: Without context in state (should fail due to missing validator)
    let uri: http::Uri = "/test".parse().unwrap();
    let user = TestUser {
        email: "new@example.com".to_string(),
    };
    let body_bytes = serde_json::to_vec(&user).unwrap();

    let builder = http::Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .header("content-type", "application/json");
    let req = builder.body(()).unwrap();
    let (parts, _) = req.into_parts();

    // Construct Request with BodyVariant::Buffered
    let mut request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::from(body_bytes.clone())),
        Arc::new(Extensions::new()),
        PathParams::new(),
    );

    let result = AsyncValidatedJson::<TestUser>::from_request(&mut request).await;

    assert!(result.is_err(), "Expected error when validator is missing");
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("Database validator not configured") || err_str.contains("async_unique"),
        "Error should mention missing configuration or rule: {:?}",
        err_str
    );

    // Test 2: With context in state (should succeed)
    let db_validator = MockDbValidator {
        unique_values: vec!["taken@example.com".to_string()],
    };
    let ctx = ValidationContextBuilder::new()
        .database(db_validator)
        .build();

    let mut extensions = Extensions::new();
    extensions.insert(ctx);

    let builder = http::Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .header("content-type", "application/json");
    let req = builder.body(()).unwrap();
    let (parts, _) = req.into_parts();

    let mut request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::from(body_bytes.clone())),
        Arc::new(extensions),
        PathParams::new(),
    );

    let result = AsyncValidatedJson::<TestUser>::from_request(&mut request).await;
    assert!(
        result.is_ok(),
        "Expected success when validator is present and value is unique. Error: {:?}",
        result.err()
    );

    // Test 3: With context in state (should fail validation logic)
    let user_taken = TestUser {
        email: "taken@example.com".to_string(),
    };
    let body_taken = serde_json::to_vec(&user_taken).unwrap();

    let db_validator = MockDbValidator {
        unique_values: vec!["taken@example.com".to_string()],
    };
    let ctx = ValidationContextBuilder::new()
        .database(db_validator)
        .build();

    let mut extensions = Extensions::new();
    extensions.insert(ctx);

    let builder = http::Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json");
    let req = builder.body(()).unwrap();
    let (parts, _) = req.into_parts();

    let mut request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::from(body_taken)),
        Arc::new(extensions),
        PathParams::new(),
    );

    let result = AsyncValidatedJson::<TestUser>::from_request(&mut request).await;
    assert!(result.is_err(), "Expected validation error for taken email");
}
