//! Content Negotiation for TOON/JSON responses
//!
//! This module provides `Negotiate<T>` - a response wrapper that automatically
//! chooses between JSON and TOON format based on the client's `Accept` header.

use crate::{TOON_CONTENT_TYPE, TOON_CONTENT_TYPE_TEXT};
use http::{header, StatusCode};
use rustapi_core::{ApiError, FromRequestParts, IntoResponse, Request, Response};
use rustapi_openapi::{
    MediaType, Operation, OperationModifier, ResponseModifier, ResponseSpec, SchemaRef,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// JSON Content-Type
pub const JSON_CONTENT_TYPE: &str = "application/json";

/// Supported output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON format (default)
    #[default]
    Json,
    /// TOON format (token-optimized)
    Toon,
}

impl OutputFormat {
    /// Get the content-type string for this format
    pub fn content_type(&self) -> &'static str {
        match self {
            OutputFormat::Json => JSON_CONTENT_TYPE,
            OutputFormat::Toon => TOON_CONTENT_TYPE,
        }
    }
}

/// Parsed Accept header with quality values
///
/// Parses `Accept` headers like:
/// - `application/json`
/// - `application/toon`
/// - `application/json, application/toon;q=0.9`
/// - `*/*`
#[derive(Debug, Clone)]
pub struct AcceptHeader {
    /// Preferred format based on Accept header parsing
    pub preferred: OutputFormat,
    /// Raw media types with quality values (sorted by quality, descending)
    pub media_types: Vec<MediaTypeEntry>,
}

/// A single media type entry from Accept header
#[derive(Debug, Clone)]
pub struct MediaTypeEntry {
    /// Media type (e.g., "application/json")
    pub media_type: String,
    /// Quality value (0.0 - 1.0), default is 1.0
    pub quality: f32,
}

impl Default for AcceptHeader {
    fn default() -> Self {
        Self {
            preferred: OutputFormat::Json,
            media_types: vec![MediaTypeEntry {
                media_type: JSON_CONTENT_TYPE.to_string(),
                quality: 1.0,
            }],
        }
    }
}

impl AcceptHeader {
    /// Parse an Accept header value
    pub fn parse(header_value: &str) -> Self {
        let mut entries: Vec<MediaTypeEntry> = header_value
            .split(',')
            .filter_map(|part| {
                let part = part.trim();
                if part.is_empty() {
                    return None;
                }

                let (media_type, quality) = if let Some(q_pos) = part.find(";q=") {
                    let (mt, q_part) = part.split_at(q_pos);
                    let q_str = q_part.trim_start_matches(";q=").trim();
                    let quality = q_str.parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0);
                    (mt.trim().to_string(), quality)
                } else if let Some(semi_pos) = part.find(';') {
                    // Handle other parameters, ignore them
                    (part[..semi_pos].trim().to_string(), 1.0)
                } else {
                    (part.to_string(), 1.0)
                };

                Some(MediaTypeEntry {
                    media_type,
                    quality,
                })
            })
            .collect();

        // Sort by quality (descending)
        entries.sort_by(|a, b| {
            b.quality
                .partial_cmp(&a.quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Determine preferred format
        let preferred = Self::determine_format(&entries);

        Self {
            preferred,
            media_types: entries,
        }
    }

    /// Determine the output format based on media type entries
    fn determine_format(entries: &[MediaTypeEntry]) -> OutputFormat {
        for entry in entries {
            let mt = entry.media_type.to_lowercase();

            // Check for TOON
            if mt == TOON_CONTENT_TYPE || mt == TOON_CONTENT_TYPE_TEXT {
                return OutputFormat::Toon;
            }

            // Check for JSON
            if mt == JSON_CONTENT_TYPE || mt == "application/json" || mt == "text/json" {
                return OutputFormat::Json;
            }

            // Wildcard accepts anything, default to JSON
            if mt == "*/*" || mt == "application/*" || mt == "text/*" {
                return OutputFormat::Json;
            }
        }

        // Default to JSON
        OutputFormat::Json
    }

    /// Check if TOON format is acceptable
    pub fn accepts_toon(&self) -> bool {
        self.media_types.iter().any(|e| {
            let mt = e.media_type.to_lowercase();
            mt == TOON_CONTENT_TYPE
                || mt == TOON_CONTENT_TYPE_TEXT
                || mt == "*/*"
                || mt == "application/*"
        })
    }

    /// Check if JSON format is acceptable
    pub fn accepts_json(&self) -> bool {
        self.media_types.iter().any(|e| {
            let mt = e.media_type.to_lowercase();
            mt == JSON_CONTENT_TYPE || mt == "text/json" || mt == "*/*" || mt == "application/*"
        })
    }
}

impl FromRequestParts for AcceptHeader {
    fn from_request_parts(req: &Request) -> rustapi_core::Result<Self> {
        let accept = req
            .headers()
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(AcceptHeader::parse)
            .unwrap_or_default();

        Ok(accept)
    }
}

/// Content-negotiated response wrapper
///
/// Automatically serializes to JSON or TOON based on the client's `Accept` header.
/// If the client prefers TOON (`Accept: application/toon`), returns TOON format.
/// Otherwise, defaults to JSON.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use rustapi_rs::toon::{Negotiate, AcceptHeader};
///
/// #[derive(Serialize)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// // Automatic negotiation via extractor
/// async fn get_user(accept: AcceptHeader) -> Negotiate<User> {
///     Negotiate::new(
///         User { id: 1, name: "Alice".to_string() },
///         accept.preferred,
///     )
/// }
///
/// // Or explicitly choose format
/// async fn get_user_toon() -> Negotiate<User> {
///     Negotiate::toon(User { id: 1, name: "Alice".to_string() })
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Negotiate<T> {
    /// The data to serialize
    pub data: T,
    /// The output format to use
    pub format: OutputFormat,
}

impl<T> Negotiate<T> {
    /// Create a new negotiated response with the specified format
    pub fn new(data: T, format: OutputFormat) -> Self {
        Self { data, format }
    }

    /// Create a JSON response
    pub fn json(data: T) -> Self {
        Self {
            data,
            format: OutputFormat::Json,
        }
    }

    /// Create a TOON response
    pub fn toon(data: T) -> Self {
        Self {
            data,
            format: OutputFormat::Toon,
        }
    }

    /// Get the output format
    pub fn format(&self) -> OutputFormat {
        self.format
    }
}

impl<T: Serialize> IntoResponse for Negotiate<T> {
    fn into_response(self) -> Response {
        match self.format {
            OutputFormat::Json => match serde_json::to_vec(&self.data) {
                Ok(body) => http::Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, JSON_CONTENT_TYPE)
                    .body(rustapi_core::ResponseBody::from(body))
                    .unwrap(),
                Err(err) => {
                    let error = ApiError::internal(format!("JSON serialization error: {}", err));
                    error.into_response()
                }
            },
            OutputFormat::Toon => match toon_format::encode_default(&self.data) {
                Ok(body) => http::Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, TOON_CONTENT_TYPE)
                    .body(rustapi_core::ResponseBody::from(body))
                    .unwrap(),
                Err(err) => {
                    let error = ApiError::internal(format!("TOON serialization error: {}", err));
                    error.into_response()
                }
            },
        }
    }
}

// OpenAPI support
impl<T: Send> OperationModifier for Negotiate<T> {
    fn update_operation(_op: &mut Operation) {
        // Negotiate is a response type, no request body modification needed
    }
}

impl<T: Serialize> ResponseModifier for Negotiate<T> {
    fn update_response(op: &mut Operation) {
        let mut content = BTreeMap::new();

        // JSON response
        content.insert(
            JSON_CONTENT_TYPE.to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(serde_json::json!({
                    "type": "object",
                    "description": "JSON formatted response"
                }))),
                example: None,
            },
        );

        // TOON response
        content.insert(
            TOON_CONTENT_TYPE.to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(serde_json::json!({
                    "type": "string",
                    "description": "TOON (Token-Oriented Object Notation) formatted response"
                }))),
                example: None,
            },
        );

        let response = ResponseSpec {
            description: "Content-negotiated response (JSON or TOON based on Accept header)"
                .to_string(),
            content,
            headers: BTreeMap::new(),
        };
        op.responses.insert("200".to_string(), response);
    }
}

// Also implement for AcceptHeader extractor
impl OperationModifier for AcceptHeader {
    fn update_operation(_op: &mut Operation) {
        // Accept header parsing doesn't modify operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accept_header_parse_json() {
        let accept = AcceptHeader::parse("application/json");
        assert_eq!(accept.preferred, OutputFormat::Json);
        assert!(accept.accepts_json());
    }

    #[test]
    fn test_accept_header_parse_toon() {
        let accept = AcceptHeader::parse("application/toon");
        assert_eq!(accept.preferred, OutputFormat::Toon);
        assert!(accept.accepts_toon());
    }

    #[test]
    fn test_accept_header_parse_with_quality() {
        let accept = AcceptHeader::parse("application/json;q=0.5, application/toon;q=0.9");
        assert_eq!(accept.preferred, OutputFormat::Toon);
        assert_eq!(accept.media_types.len(), 2);
        // First should be toon (higher quality)
        assert_eq!(accept.media_types[0].media_type, "application/toon");
        assert_eq!(accept.media_types[0].quality, 0.9);
    }

    #[test]
    fn test_accept_header_parse_wildcard() {
        let accept = AcceptHeader::parse("*/*");
        assert_eq!(accept.preferred, OutputFormat::Json);
        assert!(accept.accepts_json());
        assert!(accept.accepts_toon());
    }

    #[test]
    fn test_accept_header_parse_multiple() {
        let accept = AcceptHeader::parse("text/html, application/json, application/toon;q=0.8");
        // JSON comes before TOON (both have default q=1.0, but JSON is checked first)
        assert_eq!(accept.preferred, OutputFormat::Json);
    }

    #[test]
    fn test_accept_header_default() {
        let accept = AcceptHeader::default();
        assert_eq!(accept.preferred, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_content_type() {
        assert_eq!(OutputFormat::Json.content_type(), "application/json");
        assert_eq!(OutputFormat::Toon.content_type(), "application/toon");
    }
}
