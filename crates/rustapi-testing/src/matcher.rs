use http::{HeaderMap, Method};
use serde_json::Value;

/// Matcher for HTTP requests
#[derive(Debug, Clone, Default)]
pub struct RequestMatcher {
    pub(crate) method: Option<Method>,
    pub(crate) path: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body_json: Option<Value>,
    pub(crate) body_string: Option<String>,
}

impl RequestMatcher {
    /// Create a new matcher
    pub fn new() -> Self {
        Self::default()
    }

    /// Match a specific HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Match a specific path
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Match a specific header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Match exact JSON body
    pub fn body_json(mut self, body: impl serde::Serialize) -> Self {
        self.body_json =
            Some(serde_json::to_value(body).expect("Failed to serialize body matcher"));
        self
    }

    /// Match exact string body
    pub fn body_string(mut self, body: impl Into<String>) -> Self {
        self.body_string = Some(body.into());
        self
    }

    /// Check if the matcher matches a request
    pub fn matches(&self, method: &Method, path: &str, headers: &HeaderMap, body: &[u8]) -> bool {
        if let Some(m) = &self.method {
            if m != method {
                return false;
            }
        }

        if let Some(p) = &self.path {
            if p != path {
                return false;
            }
        }

        for (k, v) in &self.headers {
            match headers.get(k) {
                Some(val) => {
                    if val != v.as_str() {
                        return false;
                    }
                }
                None => return false,
            }
        }

        if let Some(expected_json) = &self.body_json {
            if let Ok(actual_json) = serde_json::from_slice::<Value>(body) {
                if &actual_json != expected_json {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(expected_str) = &self.body_string {
            if let Ok(actual_str) = std::str::from_utf8(body) {
                if actual_str != expected_str {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    /// **Feature: v1-features-roadmap, Property 20: Mock server request matching**
    /// **Validates: Requirements 9.1**
    ///
    /// For any HTTP request matcher:
    /// - Matcher SHALL correctly identify matching requests
    /// - Matcher SHALL correctly reject non-matching requests
    /// - Empty matcher SHALL match all requests
    /// - Multiple criteria SHALL be combined with AND logic
    /// - Header matching SHALL be case-sensitive for values

    /// Strategy for generating HTTP methods
    fn method_strategy() -> impl Strategy<Value = Method> {
        prop_oneof![
            Just(Method::GET),
            Just(Method::POST),
            Just(Method::PUT),
            Just(Method::DELETE),
            Just(Method::PATCH),
            Just(Method::HEAD),
            Just(Method::OPTIONS),
        ]
    }

    /// Strategy for generating paths
    fn path_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("/api/[a-z]{3,8}(/[0-9]{1,5})?").unwrap()
    }

    /// Strategy for generating header names
    fn header_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Content-Type".to_string()),
            Just("Authorization".to_string()),
            Just("X-Request-Id".to_string()),
            Just("Accept".to_string()),
        ]
    }

    /// Strategy for generating header values
    fn header_value_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("application/json".to_string()),
            Just("Bearer token123".to_string()),
            Just("text/plain".to_string()),
            prop::string::string_regex("[a-z0-9-]{5,15}").unwrap(),
        ]
    }

    /// Strategy for generating JSON bodies
    fn json_body_strategy() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(json!({"name": "test"})),
            Just(json!({"id": 123, "status": "active"})),
            Just(json!({"data": [1, 2, 3]})),
            Just(json!({"message": "hello"})),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 20: Empty matcher matches all requests
        #[test]
        fn prop_empty_matcher_matches_all(
            method in method_strategy(),
            path in path_strategy(),
            body in "[a-zA-Z0-9]{0,50}",
        ) {
            let matcher = RequestMatcher::new();
            let headers = HeaderMap::new();
            
            // Empty matcher MUST match any request
            prop_assert!(matcher.matches(&method, &path, &headers, body.as_bytes()));
        }

        /// Property 20: Method matcher correctly identifies method
        #[test]
        fn prop_method_matcher_correctness(
            target_method in method_strategy(),
            other_method in method_strategy(),
            path in path_strategy(),
        ) {
            let matcher = RequestMatcher::new().method(target_method.clone());
            let headers = HeaderMap::new();
            let body = b"";

            // MUST match requests with same method
            prop_assert!(matcher.matches(&target_method, &path, &headers, body));

            // MUST reject requests with different method
            if target_method != other_method {
                prop_assert!(!matcher.matches(&other_method, &path, &headers, body));
            }
        }

        /// Property 20: Path matcher is exact match
        #[test]
        fn prop_path_matcher_exact(
            method in method_strategy(),
            target_path in path_strategy(),
            other_path in path_strategy(),
        ) {
            let matcher = RequestMatcher::new().path(target_path.clone());
            let headers = HeaderMap::new();
            let body = b"";

            // MUST match exact path
            prop_assert!(matcher.matches(&method, &target_path, &headers, body));

            // MUST reject different path
            if target_path != other_path {
                prop_assert!(!matcher.matches(&method, &other_path, &headers, body));
            }
        }

        /// Property 20: Header matcher requires exact value
        #[test]
        fn prop_header_matcher_exact(
            method in method_strategy(),
            path in path_strategy(),
            header_name in header_name_strategy(),
            header_value in header_value_strategy(),
            other_value in header_value_strategy(),
        ) {
            let matcher = RequestMatcher::new()
                .header(header_name.clone(), header_value.clone());
            
            let mut headers_match = HeaderMap::new();
            headers_match.insert(
                http::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
                http::header::HeaderValue::from_str(&header_value).unwrap(),
            );

            let body = b"";

            // MUST match when header value is exact
            prop_assert!(matcher.matches(&method, &path, &headers_match, body));

            // MUST reject when header value differs
            if header_value != other_value {
                let mut headers_differ = HeaderMap::new();
                headers_differ.insert(
                    http::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
                    http::header::HeaderValue::from_str(&other_value).unwrap(),
                );
                prop_assert!(!matcher.matches(&method, &path, &headers_differ, body));
            }

            // MUST reject when header is missing
            let headers_empty = HeaderMap::new();
            prop_assert!(!matcher.matches(&method, &path, &headers_empty, body));
        }

        /// Property 20: JSON body matcher requires exact match
        #[test]
        fn prop_json_body_matcher_exact(
            method in method_strategy(),
            path in path_strategy(),
            json_body in json_body_strategy(),
        ) {
            let matcher = RequestMatcher::new().body_json(json_body.clone());
            let headers = HeaderMap::new();

            let matching_body = serde_json::to_vec(&json_body).unwrap();
            
            // MUST match exact JSON body
            prop_assert!(matcher.matches(&method, &path, &headers, &matching_body));

            // MUST reject different JSON body
            let different_json = json!({"different": "value"});
            let different_body = serde_json::to_vec(&different_json).unwrap();
            prop_assert!(!matcher.matches(&method, &path, &headers, &different_body));

            // MUST reject invalid JSON
            let invalid_json = b"not json at all";
            prop_assert!(!matcher.matches(&method, &path, &headers, invalid_json));
        }

        /// Property 20: String body matcher requires exact match
        #[test]
        fn prop_string_body_matcher_exact(
            method in method_strategy(),
            path in path_strategy(),
            body_string in "[a-zA-Z0-9 ]{5,30}",
            other_string in "[a-zA-Z0-9 ]{5,30}",
        ) {
            let matcher = RequestMatcher::new().body_string(body_string.clone());
            let headers = HeaderMap::new();

            // MUST match exact string
            prop_assert!(matcher.matches(&method, &path, &headers, body_string.as_bytes()));

            // MUST reject different string
            if body_string != other_string {
                prop_assert!(!matcher.matches(&method, &path, &headers, other_string.as_bytes()));
            }
        }

        /// Property 20: Multiple criteria combined with AND logic
        #[test]
        fn prop_multiple_criteria_and_logic(
            target_method in method_strategy(),
            other_method in method_strategy(),
            target_path in path_strategy(),
            other_path in path_strategy(),
            header_name in header_name_strategy(),
            header_value in header_value_strategy(),
        ) {
            let matcher = RequestMatcher::new()
                .method(target_method.clone())
                .path(target_path.clone())
                .header(header_name.clone(), header_value.clone());

            let mut headers_correct = HeaderMap::new();
            headers_correct.insert(
                http::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
                http::header::HeaderValue::from_str(&header_value).unwrap(),
            );

            let body = b"";

            // MUST match when ALL criteria match
            prop_assert!(matcher.matches(&target_method, &target_path, &headers_correct, body));

            // MUST reject when ANY criterion fails
            if target_method != other_method {
                // Wrong method
                prop_assert!(!matcher.matches(&other_method, &target_path, &headers_correct, body));
            }

            if target_path != other_path {
                // Wrong path
                prop_assert!(!matcher.matches(&target_method, &other_path, &headers_correct, body));
            }

            // Wrong/missing header
            let headers_empty = HeaderMap::new();
            prop_assert!(!matcher.matches(&target_method, &target_path, &headers_empty, body));
        }

        /// Property 20: Matcher is case-sensitive for paths
        #[test]
        fn prop_path_case_sensitive(
            method in method_strategy(),
            path in "[a-z]{5,10}",
        ) {
            let lowercase_path = format!("/api/{}", path.to_lowercase());
            let uppercase_path = format!("/api/{}", path.to_uppercase());

            let matcher = RequestMatcher::new().path(lowercase_path.clone());
            let headers = HeaderMap::new();
            let body = b"";

            // MUST match exact case
            prop_assert!(matcher.matches(&method, &lowercase_path, &headers, body));

            // MUST reject different case (if different)
            if lowercase_path != uppercase_path {
                prop_assert!(!matcher.matches(&method, &uppercase_path, &headers, body));
            }
        }

        /// Property 20: Multiple headers must all match
        #[test]
        fn prop_multiple_headers_all_match(
            method in method_strategy(),
            path in path_strategy(),
        ) {
            let matcher = RequestMatcher::new()
                .header("Content-Type", "application/json")
                .header("X-Request-Id", "req-123")
                .header("Authorization", "Bearer token");

            let mut headers_all = HeaderMap::new();
            headers_all.insert("content-type", "application/json".parse().unwrap());
            headers_all.insert("x-request-id", "req-123".parse().unwrap());
            headers_all.insert("authorization", "Bearer token".parse().unwrap());

            let mut headers_missing_one = HeaderMap::new();
            headers_missing_one.insert("content-type", "application/json".parse().unwrap());
            headers_missing_one.insert("x-request-id", "req-123".parse().unwrap());
            // Missing Authorization

            let body = b"";

            // MUST match when all headers present
            prop_assert!(matcher.matches(&method, &path, &headers_all, body));

            // MUST reject when any header missing
            prop_assert!(!matcher.matches(&method, &path, &headers_missing_one, body));
        }

        /// Property 20: JSON body whitespace doesn't affect matching
        #[test]
        fn prop_json_whitespace_normalized(
            method in method_strategy(),
            path in path_strategy(),
        ) {
            let json_value = json!({"name": "test", "id": 123});
            let matcher = RequestMatcher::new().body_json(json_value.clone());
            let headers = HeaderMap::new();

            // Compact JSON
            let compact = serde_json::to_vec(&json_value).unwrap();
            prop_assert!(matcher.matches(&method, &path, &headers, &compact));

            // Pretty-printed JSON (different whitespace)
            let pretty = serde_json::to_vec_pretty(&json_value).unwrap();
            prop_assert!(matcher.matches(&method, &path, &headers, &pretty));
        }

        /// Property 20: JSON field order doesn't affect matching
        #[test]
        fn prop_json_field_order_normalized(
            method in method_strategy(),
            path in path_strategy(),
        ) {
            let json_ordered = json!({"a": 1, "b": 2, "c": 3});
            let json_reordered = json!({"c": 3, "a": 1, "b": 2});
            
            let matcher = RequestMatcher::new().body_json(json_ordered);
            let headers = HeaderMap::new();

            let body = serde_json::to_vec(&json_reordered).unwrap();
            
            // MUST match regardless of field order (JSON semantics)
            prop_assert!(matcher.matches(&method, &path, &headers, &body));
        }

        /// Property 20: Matcher with no criteria matches everything
        #[test]
        fn prop_default_matcher_permissive(
            method in method_strategy(),
            path in path_strategy(),
            body in prop::collection::vec(0u8..255u8, 0..100),
        ) {
            let matcher = RequestMatcher::default();
            let headers = HeaderMap::new();

            // Default matcher MUST be permissive
            prop_assert!(matcher.matches(&method, &path, &headers, &body));
        }
    }
}
