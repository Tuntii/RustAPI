//! Header and body redaction utilities.
//!
//! Pure functions for redacting sensitive data from headers and JSON bodies.
//! No IO operations.

use std::collections::{HashMap, HashSet};

/// Configuration for redaction of sensitive data.
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// Header names to redact (lowercase).
    pub header_names: HashSet<String>,
    /// JSON body field paths to redact.
    pub body_field_paths: HashSet<String>,
    /// Replacement string. Default: `"[REDACTED]"`.
    pub replacement: String,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            header_names: HashSet::new(),
            body_field_paths: HashSet::new(),
            replacement: "[REDACTED]".to_string(),
        }
    }
}

/// Redact sensitive values from a header map.
///
/// Returns a new map with sensitive header values replaced by `"[REDACTED]"`.
/// Header name comparison is case-insensitive.
pub fn redact_headers(
    headers: &HashMap<String, String>,
    sensitive: &HashSet<String>,
) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| {
            let key_lower = k.to_lowercase();
            if sensitive.contains(&key_lower) {
                (k.clone(), "[REDACTED]".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

/// Redact JSON body fields by path.
///
/// Parses the body as JSON, replaces matching field values with the replacement
/// string, and returns the modified JSON string. Returns `None` if the input
/// is not valid JSON.
///
/// Field paths are top-level keys only (e.g., `"password"`, `"ssn"`).
pub fn redact_body(body: &str, field_paths: &HashSet<String>, replacement: &str) -> Option<String> {
    if field_paths.is_empty() {
        return Some(body.to_string());
    }

    let mut value: serde_json::Value = serde_json::from_str(body).ok()?;
    redact_value(&mut value, field_paths, replacement);
    serde_json::to_string(&value).ok()
}

/// Recursively redact fields in a JSON value.
fn redact_value(value: &mut serde_json::Value, fields: &HashSet<String>, replacement: &str) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if fields.contains(key) {
                    *val = serde_json::Value::String(replacement.to_string());
                } else {
                    redact_value(val, fields, replacement);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_value(item, fields, replacement);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_headers_basic() {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer secret".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-api-key".to_string(), "key123".to_string());

        let mut sensitive = HashSet::new();
        sensitive.insert("authorization".to_string());
        sensitive.insert("x-api-key".to_string());

        let redacted = redact_headers(&headers, &sensitive);

        assert_eq!(redacted.get("authorization").unwrap(), "[REDACTED]");
        assert_eq!(redacted.get("content-type").unwrap(), "application/json");
        assert_eq!(redacted.get("x-api-key").unwrap(), "[REDACTED]");
    }

    #[test]
    fn test_redact_headers_case_insensitive() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer secret".to_string());

        let mut sensitive = HashSet::new();
        sensitive.insert("authorization".to_string());

        let redacted = redact_headers(&headers, &sensitive);
        assert_eq!(redacted.get("Authorization").unwrap(), "[REDACTED]");
    }

    #[test]
    fn test_redact_headers_empty_sensitive() {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer secret".to_string());

        let redacted = redact_headers(&headers, &HashSet::new());
        assert_eq!(redacted.get("authorization").unwrap(), "Bearer secret");
    }

    #[test]
    fn test_redact_body_basic() {
        let body = r#"{"username":"john","password":"secret123","email":"john@example.com"}"#;
        let mut fields = HashSet::new();
        fields.insert("password".to_string());

        let result = redact_body(body, &fields, "[REDACTED]").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["username"], "john");
        assert_eq!(parsed["password"], "[REDACTED]");
        assert_eq!(parsed["email"], "john@example.com");
    }

    #[test]
    fn test_redact_body_nested() {
        let body = r#"{"user":{"name":"john","ssn":"123-45-6789"}}"#;
        let mut fields = HashSet::new();
        fields.insert("ssn".to_string());

        let result = redact_body(body, &fields, "[REDACTED]").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["user"]["name"], "john");
        assert_eq!(parsed["user"]["ssn"], "[REDACTED]");
    }

    #[test]
    fn test_redact_body_array() {
        let body = r#"[{"password":"a"},{"password":"b"}]"#;
        let mut fields = HashSet::new();
        fields.insert("password".to_string());

        let result = redact_body(body, &fields, "[REDACTED]").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed[0]["password"], "[REDACTED]");
        assert_eq!(parsed[1]["password"], "[REDACTED]");
    }

    #[test]
    fn test_redact_body_not_json() {
        let body = "this is not json";
        let mut fields = HashSet::new();
        fields.insert("password".to_string());

        assert!(redact_body(body, &fields, "[REDACTED]").is_none());
    }

    #[test]
    fn test_redact_body_empty_fields() {
        let body = r#"{"password":"secret"}"#;
        let result = redact_body(body, &HashSet::new(), "[REDACTED]").unwrap();
        assert_eq!(result, body);
    }
}
