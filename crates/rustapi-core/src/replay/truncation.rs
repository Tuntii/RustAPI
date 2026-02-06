//! Body truncation and content sniffing utilities.
//!
//! Pure functions for truncating large bodies and detecting content types.
//! No IO operations.

/// The kind of content detected by [`content_sniff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentKind {
    /// JSON content.
    Json,
    /// XML content.
    Xml,
    /// HTML content.
    Html,
    /// Plain text content.
    PlainText,
    /// Binary content.
    Binary,
    /// Unknown content type.
    Unknown,
}

/// Configuration for body truncation.
#[derive(Debug, Clone)]
pub struct TruncationConfig {
    /// Maximum body size in bytes.
    pub max_size: usize,
    /// Suffix appended to truncated bodies.
    pub suffix: String,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            max_size: 262_144, // 256KB
            suffix: "... [truncated]".to_string(),
        }
    }
}

/// Truncate a body string to the given maximum byte size.
///
/// Returns a tuple of `(body, was_truncated)`. If the body is within the
/// limit, it is returned unchanged with `false`. Otherwise, it is truncated
/// at a valid UTF-8 boundary and `true` is returned.
pub fn truncate_body(body: &str, max_size: usize) -> (String, bool) {
    if body.len() <= max_size {
        return (body.to_string(), false);
    }

    // Find valid UTF-8 boundary
    let mut end = max_size;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = body[..end].to_string();
    truncated.push_str("... [truncated]");
    (truncated, true)
}

/// Sniff content type from raw bytes (no IO).
///
/// Inspects the first few bytes to guess the content kind.
pub fn content_sniff(bytes: &[u8]) -> ContentKind {
    if bytes.is_empty() {
        return ContentKind::Unknown;
    }

    // Skip leading whitespace
    let trimmed = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .map(|i| &bytes[i..])
        .unwrap_or(bytes);

    if trimmed.is_empty() {
        return ContentKind::PlainText;
    }

    match trimmed[0] {
        b'{' | b'[' => ContentKind::Json,
        b'<' => {
            // Check for HTML vs XML
            let prefix = std::str::from_utf8(&trimmed[..trimmed.len().min(100)])
                .unwrap_or("")
                .to_lowercase();
            if prefix.contains("<!doctype html") || prefix.contains("<html") {
                ContentKind::Html
            } else {
                ContentKind::Xml
            }
        }
        _ => {
            // Check if it looks like text
            let is_text = trimmed
                .iter()
                .take(512)
                .all(|&b| b.is_ascii() || b > 0x7F);
            if is_text {
                ContentKind::PlainText
            } else {
                ContentKind::Binary
            }
        }
    }
}

/// Attempt to parse a body string as JSON.
///
/// Returns `Some(Value)` on success, `None` if not valid JSON.
pub fn try_parse_json(body: &str) -> Option<serde_json::Value> {
    serde_json::from_str(body).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_body_within_limit() {
        let body = "hello world";
        let (result, truncated) = truncate_body(body, 100);
        assert_eq!(result, "hello world");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_body_exceeds_limit() {
        let body = "hello world, this is a long string";
        let (result, truncated) = truncate_body(body, 11);
        assert!(result.starts_with("hello world"));
        assert!(result.ends_with("... [truncated]"));
        assert!(truncated);
    }

    #[test]
    fn test_truncate_body_exact_limit() {
        let body = "hello";
        let (result, truncated) = truncate_body(body, 5);
        assert_eq!(result, "hello");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_body_utf8_boundary() {
        let body = "hëllo wörld"; // multi-byte characters
        let (_, truncated) = truncate_body(body, 3);
        assert!(truncated);
        // Should not panic or produce invalid UTF-8
    }

    #[test]
    fn test_content_sniff_json_object() {
        assert_eq!(content_sniff(b"{\"key\":\"value\"}"), ContentKind::Json);
    }

    #[test]
    fn test_content_sniff_json_array() {
        assert_eq!(content_sniff(b"[1, 2, 3]"), ContentKind::Json);
    }

    #[test]
    fn test_content_sniff_json_with_whitespace() {
        assert_eq!(content_sniff(b"  \n  {\"key\":\"value\"}"), ContentKind::Json);
    }

    #[test]
    fn test_content_sniff_xml() {
        assert_eq!(
            content_sniff(b"<?xml version=\"1.0\"?><root/>"),
            ContentKind::Xml
        );
    }

    #[test]
    fn test_content_sniff_html() {
        assert_eq!(
            content_sniff(b"<!DOCTYPE html><html>"),
            ContentKind::Html
        );
    }

    #[test]
    fn test_content_sniff_plain_text() {
        assert_eq!(
            content_sniff(b"Hello, this is plain text"),
            ContentKind::PlainText
        );
    }

    #[test]
    fn test_content_sniff_empty() {
        assert_eq!(content_sniff(b""), ContentKind::Unknown);
    }

    #[test]
    fn test_try_parse_json_valid() {
        let result = try_parse_json(r#"{"key":"value"}"#);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn test_try_parse_json_invalid() {
        assert!(try_parse_json("not json").is_none());
    }

    #[test]
    fn test_try_parse_json_array() {
        let result = try_parse_json("[1, 2, 3]");
        assert!(result.is_some());
    }
}
