//! Diff utilities for comparing original and replayed HTTP responses.
//!
//! Pure functions for computing structural diffs between [`RecordedResponse`]
//! instances. Supports JSON field-level and raw text diff.

use serde::{Deserialize, Serialize};

use super::entry::RecordedResponse;
use super::truncation::try_parse_json;

/// Result of diffing an original vs. replayed response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// Whether there are any differences.
    pub has_diff: bool,

    /// Status code diff `(original, replayed)` if different.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_diff: Option<(u16, u16)>,

    /// Header differences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header_diffs: Vec<FieldDiff>,

    /// Body difference summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_diff: Option<BodyDiff>,
}

/// Which field differs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffField {
    /// HTTP status code.
    Status,
    /// A specific header.
    Header(String),
    /// A JSON body field path (dot-separated).
    BodyField(String),
    /// The raw body (non-JSON).
    BodyRaw,
}

/// A single field difference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    /// The field that differs.
    pub field: DiffField,
    /// Original value (if present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    /// Replayed value (if present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayed: Option<String>,
}

/// Body difference details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyDiff {
    /// If both bodies are JSON, field-level diffs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_diffs: Vec<FieldDiff>,

    /// If bodies are not JSON-diffable, a raw text diff summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_diff_summary: Option<String>,
}

/// Compare two recorded responses and produce a diff.
///
/// # Arguments
///
/// * `original` - The originally recorded response.
/// * `replayed` - The response from replaying the request.
/// * `ignore_headers` - Header names to exclude from comparison (e.g., `date`, `x-request-id`).
pub fn compute_diff(
    original: &RecordedResponse,
    replayed: &RecordedResponse,
    ignore_headers: &[String],
) -> DiffResult {
    let mut result = DiffResult {
        has_diff: false,
        status_diff: None,
        header_diffs: Vec::new(),
        body_diff: None,
    };

    // Compare status codes
    if original.status != replayed.status {
        result.has_diff = true;
        result.status_diff = Some((original.status, replayed.status));
    }

    // Compare headers
    let ignore_set: std::collections::HashSet<String> = ignore_headers
        .iter()
        .map(|h| h.to_lowercase())
        .collect();

    // Check headers in original
    for (key, orig_val) in &original.headers {
        let key_lower = key.to_lowercase();
        if ignore_set.contains(&key_lower) {
            continue;
        }
        match replayed.headers.get(key) {
            Some(replay_val) if replay_val != orig_val => {
                result.has_diff = true;
                result.header_diffs.push(FieldDiff {
                    field: DiffField::Header(key.clone()),
                    original: Some(orig_val.clone()),
                    replayed: Some(replay_val.clone()),
                });
            }
            None => {
                result.has_diff = true;
                result.header_diffs.push(FieldDiff {
                    field: DiffField::Header(key.clone()),
                    original: Some(orig_val.clone()),
                    replayed: None,
                });
            }
            _ => {} // same value
        }
    }

    // Check headers only in replayed
    for (key, replay_val) in &replayed.headers {
        let key_lower = key.to_lowercase();
        if ignore_set.contains(&key_lower) {
            continue;
        }
        if !original.headers.contains_key(key) {
            result.has_diff = true;
            result.header_diffs.push(FieldDiff {
                field: DiffField::Header(key.clone()),
                original: None,
                replayed: Some(replay_val.clone()),
            });
        }
    }

    // Compare bodies
    match (&original.body, &replayed.body) {
        (Some(orig_body), Some(replay_body)) => {
            if orig_body != replay_body {
                result.has_diff = true;

                // Try JSON diff
                let orig_json = try_parse_json(orig_body);
                let replay_json = try_parse_json(replay_body);

                match (orig_json, replay_json) {
                    (Some(orig_val), Some(replay_val)) => {
                        let field_diffs = diff_json(&orig_val, &replay_val, "");
                        if !field_diffs.is_empty() {
                            result.body_diff = Some(BodyDiff {
                                field_diffs,
                                raw_diff_summary: None,
                            });
                        }
                    }
                    _ => {
                        // Fall back to raw diff summary
                        let summary = format!(
                            "Bodies differ: original {} bytes, replayed {} bytes",
                            orig_body.len(),
                            replay_body.len()
                        );
                        result.body_diff = Some(BodyDiff {
                            field_diffs: Vec::new(),
                            raw_diff_summary: Some(summary),
                        });
                    }
                }
            }
        }
        (Some(orig_body), None) => {
            result.has_diff = true;
            result.body_diff = Some(BodyDiff {
                field_diffs: Vec::new(),
                raw_diff_summary: Some(format!(
                    "Original has body ({} bytes), replayed has no body",
                    orig_body.len()
                )),
            });
        }
        (None, Some(replay_body)) => {
            result.has_diff = true;
            result.body_diff = Some(BodyDiff {
                field_diffs: Vec::new(),
                raw_diff_summary: Some(format!(
                    "Original has no body, replayed has body ({} bytes)",
                    replay_body.len()
                )),
            });
        }
        (None, None) => {} // both empty, no diff
    }

    result
}

/// Deep-diff two JSON values, producing field-level diffs.
///
/// Recursively compares JSON structures. The `prefix` argument tracks the
/// current path (e.g., `"user.address.city"`).
pub fn diff_json(
    original: &serde_json::Value,
    replayed: &serde_json::Value,
    prefix: &str,
) -> Vec<FieldDiff> {
    let mut diffs = Vec::new();

    if original == replayed {
        return diffs;
    }

    match (original, replayed) {
        (serde_json::Value::Object(orig_map), serde_json::Value::Object(replay_map)) => {
            // Check keys in original
            for (key, orig_val) in orig_map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match replay_map.get(key) {
                    Some(replay_val) => {
                        diffs.extend(diff_json(orig_val, replay_val, &path));
                    }
                    None => {
                        diffs.push(FieldDiff {
                            field: DiffField::BodyField(path),
                            original: Some(value_to_string(orig_val)),
                            replayed: None,
                        });
                    }
                }
            }

            // Check keys only in replayed
            for (key, replay_val) in replay_map {
                if !orig_map.contains_key(key) {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    diffs.push(FieldDiff {
                        field: DiffField::BodyField(path),
                        original: None,
                        replayed: Some(value_to_string(replay_val)),
                    });
                }
            }
        }
        (serde_json::Value::Array(orig_arr), serde_json::Value::Array(replay_arr)) => {
            let max_len = orig_arr.len().max(replay_arr.len());
            for i in 0..max_len {
                let path = if prefix.is_empty() {
                    format!("[{}]", i)
                } else {
                    format!("{}[{}]", prefix, i)
                };

                match (orig_arr.get(i), replay_arr.get(i)) {
                    (Some(orig_val), Some(replay_val)) => {
                        diffs.extend(diff_json(orig_val, replay_val, &path));
                    }
                    (Some(orig_val), None) => {
                        diffs.push(FieldDiff {
                            field: DiffField::BodyField(path),
                            original: Some(value_to_string(orig_val)),
                            replayed: None,
                        });
                    }
                    (None, Some(replay_val)) => {
                        diffs.push(FieldDiff {
                            field: DiffField::BodyField(path),
                            original: None,
                            replayed: Some(value_to_string(replay_val)),
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            // Leaf values differ
            let path = if prefix.is_empty() {
                "(root)".to_string()
            } else {
                prefix.to_string()
            };
            diffs.push(FieldDiff {
                field: DiffField::BodyField(path),
                original: Some(value_to_string(original)),
                replayed: Some(value_to_string(replayed)),
            });
        }
    }

    diffs
}

/// Convert a JSON value to a compact string representation.
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_response(status: u16, body: Option<&str>) -> RecordedResponse {
        RecordedResponse {
            status,
            headers: HashMap::new(),
            body: body.map(|s| s.to_string()),
            body_size: body.map(|s| s.len()).unwrap_or(0),
            body_truncated: false,
        }
    }

    #[test]
    fn test_identical_responses_no_diff() {
        let resp = make_response(200, Some(r#"{"ok":true}"#));
        let diff = compute_diff(&resp, &resp, &[]);
        assert!(!diff.has_diff);
        assert!(diff.status_diff.is_none());
        assert!(diff.header_diffs.is_empty());
        assert!(diff.body_diff.is_none());
    }

    #[test]
    fn test_different_status() {
        let orig = make_response(200, None);
        let replay = make_response(500, None);
        let diff = compute_diff(&orig, &replay, &[]);

        assert!(diff.has_diff);
        assert_eq!(diff.status_diff, Some((200, 500)));
    }

    #[test]
    fn test_different_headers() {
        let mut orig = make_response(200, None);
        orig.headers
            .insert("x-custom".to_string(), "value1".to_string());

        let mut replay = make_response(200, None);
        replay
            .headers
            .insert("x-custom".to_string(), "value2".to_string());

        let diff = compute_diff(&orig, &replay, &[]);
        assert!(diff.has_diff);
        assert_eq!(diff.header_diffs.len(), 1);
    }

    #[test]
    fn test_ignore_headers() {
        let mut orig = make_response(200, None);
        orig.headers
            .insert("date".to_string(), "Mon, 01 Jan".to_string());

        let mut replay = make_response(200, None);
        replay
            .headers
            .insert("date".to_string(), "Tue, 02 Jan".to_string());

        let diff = compute_diff(&orig, &replay, &["date".to_string()]);
        assert!(!diff.has_diff);
    }

    #[test]
    fn test_missing_header_in_replay() {
        let mut orig = make_response(200, None);
        orig.headers
            .insert("x-custom".to_string(), "value".to_string());

        let replay = make_response(200, None);
        let diff = compute_diff(&orig, &replay, &[]);

        assert!(diff.has_diff);
        assert_eq!(diff.header_diffs.len(), 1);
        assert!(diff.header_diffs[0].replayed.is_none());
    }

    #[test]
    fn test_json_body_diff() {
        let orig = make_response(200, Some(r#"{"name":"alice","age":30}"#));
        let replay = make_response(200, Some(r#"{"name":"bob","age":30}"#));

        let diff = compute_diff(&orig, &replay, &[]);
        assert!(diff.has_diff);

        let body_diff = diff.body_diff.unwrap();
        assert_eq!(body_diff.field_diffs.len(), 1);
        assert!(matches!(&body_diff.field_diffs[0].field, DiffField::BodyField(f) if f == "name"));
        assert_eq!(body_diff.field_diffs[0].original.as_deref(), Some("alice"));
        assert_eq!(body_diff.field_diffs[0].replayed.as_deref(), Some("bob"));
    }

    #[test]
    fn test_json_nested_diff() {
        let orig = make_response(200, Some(r#"{"user":{"name":"alice","age":30}}"#));
        let replay = make_response(200, Some(r#"{"user":{"name":"alice","age":31}}"#));

        let diff = compute_diff(&orig, &replay, &[]);
        assert!(diff.has_diff);

        let body_diff = diff.body_diff.unwrap();
        assert_eq!(body_diff.field_diffs.len(), 1);
        assert!(
            matches!(&body_diff.field_diffs[0].field, DiffField::BodyField(f) if f == "user.age")
        );
    }

    #[test]
    fn test_non_json_body_diff() {
        let orig = make_response(200, Some("hello world"));
        let replay = make_response(200, Some("hello changed"));

        let diff = compute_diff(&orig, &replay, &[]);
        assert!(diff.has_diff);

        let body_diff = diff.body_diff.unwrap();
        assert!(body_diff.raw_diff_summary.is_some());
    }

    #[test]
    fn test_body_presence_diff() {
        let orig = make_response(200, Some("body here"));
        let replay = make_response(200, None);

        let diff = compute_diff(&orig, &replay, &[]);
        assert!(diff.has_diff);
        assert!(diff.body_diff.is_some());
    }

    #[test]
    fn test_diff_json_array() {
        let orig: serde_json::Value = serde_json::from_str("[1, 2, 3]").unwrap();
        let replay: serde_json::Value = serde_json::from_str("[1, 2, 4]").unwrap();

        let diffs = diff_json(&orig, &replay, "");
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0].field, DiffField::BodyField(f) if f == "[2]"));
    }

    #[test]
    fn test_diff_json_extra_key() {
        let orig: serde_json::Value =
            serde_json::from_str(r#"{"a":1}"#).unwrap();
        let replay: serde_json::Value =
            serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();

        let diffs = diff_json(&orig, &replay, "");
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0].field, DiffField::BodyField(f) if f == "b"));
        assert!(diffs[0].original.is_none());
    }
}
