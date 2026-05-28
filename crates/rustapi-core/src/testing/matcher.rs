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
