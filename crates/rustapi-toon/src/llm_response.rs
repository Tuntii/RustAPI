//! # LLM-Optimized Response Wrapper
//!
//! Provides `LlmResponse<T>` for AI/LLM endpoints with automatic
//! token counting and format optimization.
//!
//! ## Features
//!
//! - Automatic content negotiation (JSON vs TOON)
//! - Token counting headers
//! - Token savings calculation
//!
//! ## Response Headers
//!
//! - `X-Token-Count-JSON`: Estimated token count in JSON format
//! - `X-Token-Count-TOON`: Estimated token count in TOON format
//! - `X-Token-Savings`: Percentage of tokens saved with TOON
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_rs::toon::{LlmResponse, AcceptHeader};
//!
//! #[derive(Serialize)]
//! struct ChatResponse {
//!     messages: Vec<Message>,
//! }
//!
//! async fn chat(accept: AcceptHeader) -> LlmResponse<ChatResponse> {
//!     let response = ChatResponse {
//!         messages: vec![...],
//!     };
//!     LlmResponse::new(response, accept.preferred)
//! }
//! ```

use crate::{OutputFormat, JSON_CONTENT_TYPE, TOON_CONTENT_TYPE};
use http::{header, StatusCode};
use rustapi_core::{ApiError, IntoResponse, Response};
use rustapi_openapi::{
    MediaType, Operation, OperationModifier, ResponseModifier, ResponseSpec, SchemaRef,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Header name for JSON token count
pub const X_TOKEN_COUNT_JSON: &str = "x-token-count-json";
/// Header name for TOON token count
pub const X_TOKEN_COUNT_TOON: &str = "x-token-count-toon";
/// Header name for token savings percentage
pub const X_TOKEN_SAVINGS: &str = "x-token-savings";
/// Header name for format used
pub const X_FORMAT_USED: &str = "x-format-used";

/// LLM-optimized response wrapper with token counting.
///
/// This wrapper automatically:
/// 1. Serializes to the requested format (JSON or TOON)
/// 2. Calculates estimated token counts for both formats
/// 3. Adds informative headers about token usage
///
/// ## Token Estimation
///
/// Token counts are estimated using a simple heuristic:
/// - ~4 characters per token (GPT-3/4 average)
///
/// For more accurate counts, use a proper tokenizer.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use rustapi_rs::toon::{LlmResponse, AcceptHeader, OutputFormat};
///
/// #[derive(Serialize)]
/// struct ApiData {
///     items: Vec<Item>,
/// }
///
/// // With content negotiation
/// async fn get_items(accept: AcceptHeader) -> LlmResponse<ApiData> {
///     let data = ApiData { items: vec![...] };
///     LlmResponse::new(data, accept.preferred)
/// }
///
/// // Always TOON format
/// async fn get_items_toon() -> LlmResponse<ApiData> {
///     let data = ApiData { items: vec![...] };
///     LlmResponse::toon(data)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LlmResponse<T> {
    data: T,
    format: OutputFormat,
    include_token_headers: bool,
}

impl<T> LlmResponse<T> {
    /// Create a new LLM response with the specified format.
    pub fn new(data: T, format: OutputFormat) -> Self {
        Self {
            data,
            format,
            include_token_headers: true,
        }
    }

    /// Create a JSON-formatted LLM response.
    pub fn json(data: T) -> Self {
        Self::new(data, OutputFormat::Json)
    }

    /// Create a TOON-formatted LLM response.
    pub fn toon(data: T) -> Self {
        Self::new(data, OutputFormat::Toon)
    }

    /// Disable token counting headers.
    pub fn without_token_headers(mut self) -> Self {
        self.include_token_headers = false;
        self
    }

    /// Enable token counting headers (default).
    pub fn with_token_headers(mut self) -> Self {
        self.include_token_headers = true;
        self
    }
}

/// Estimate token count using simple character-based heuristic.
/// ~4 characters per token (GPT-3/4 average)
fn estimate_tokens(text: &str) -> usize {
    // Simple heuristic: ~4 chars per token
    // Accounts for whitespace and punctuation overhead
    let char_count = text.len();
    char_count.div_ceil(4) // Round up
}

/// Calculate token savings percentage.
fn calculate_savings(json_tokens: usize, toon_tokens: usize) -> f64 {
    if json_tokens == 0 {
        return 0.0;
    }
    let savings = json_tokens.saturating_sub(toon_tokens) as f64 / json_tokens as f64 * 100.0;
    (savings * 100.0).round() / 100.0 // Round to 2 decimal places
}

impl<T: Serialize> IntoResponse for LlmResponse<T> {
    fn into_response(self) -> Response {
        // Always serialize to both formats for token counting
        let json_result = serde_json::to_string(&self.data);
        let toon_result = toon_format::encode_default(&self.data);

        // Calculate token counts if enabled
        let (json_tokens, toon_tokens, savings) = if self.include_token_headers {
            let json_tokens = json_result
                .as_ref()
                .map(|s| estimate_tokens(s))
                .unwrap_or(0);
            let toon_tokens = toon_result
                .as_ref()
                .map(|s| estimate_tokens(s))
                .unwrap_or(0);
            let savings = calculate_savings(json_tokens, toon_tokens);
            (Some(json_tokens), Some(toon_tokens), Some(savings))
        } else {
            (None, None, None)
        };

        // Serialize to the requested format
        let (body, content_type) = match self.format {
            OutputFormat::Json => match json_result {
                Ok(json) => (json, JSON_CONTENT_TYPE),
                Err(e) => {
                    tracing::error!("Failed to serialize to JSON: {}", e);
                    return ApiError::internal(format!("JSON serialization error: {}", e))
                        .into_response();
                }
            },
            OutputFormat::Toon => match toon_result {
                Ok(toon) => (toon, TOON_CONTENT_TYPE),
                Err(e) => {
                    tracing::error!("Failed to serialize to TOON: {}", e);
                    return ApiError::internal(format!("TOON serialization error: {}", e))
                        .into_response();
                }
            },
        };

        // Build response with headers
        let mut builder = http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(
                X_FORMAT_USED,
                match self.format {
                    OutputFormat::Json => "json",
                    OutputFormat::Toon => "toon",
                },
            );

        // Token counting headers
        if let Some(json_tokens) = json_tokens {
            builder = builder.header(X_TOKEN_COUNT_JSON, json_tokens.to_string());
        }
        if let Some(toon_tokens) = toon_tokens {
            builder = builder.header(X_TOKEN_COUNT_TOON, toon_tokens.to_string());
        }
        if let Some(savings) = savings {
            builder = builder.header(X_TOKEN_SAVINGS, format!("{:.2}%", savings));
        }

        builder
            .body(rustapi_core::ResponseBody::from(body))
            .unwrap()
    }
}

// OpenAPI support
impl<T: Send> OperationModifier for LlmResponse<T> {
    fn update_operation(_op: &mut Operation) {
        // LlmResponse is a response type, no request body modification needed
    }
}

impl<T: Serialize> ResponseModifier for LlmResponse<T> {
    fn update_response(op: &mut Operation) {
        let mut content = BTreeMap::new();

        // JSON response
        content.insert(
            JSON_CONTENT_TYPE.to_string(),
            MediaType {
                schema: Some(SchemaRef::Inline(serde_json::json!({
                    "type": "object",
                    "description": "JSON formatted response with token counting headers"
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
                    "description": "TOON (Token-Oriented Object Notation) formatted response with token counting headers"
                }))),
                example: None,
            },
        );

        let response = ResponseSpec {
            description: "LLM-optimized response with token counting headers (X-Token-Count-JSON, X-Token-Count-TOON, X-Token-Savings)".to_string(),
            content,
            headers: BTreeMap::new(),
        };
        op.responses.insert("200".to_string(), response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        id: u64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_estimate_tokens() {
        // ~4 chars per token
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1); // 4 chars = 1 token
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars = ~3 tokens
        assert_eq!(estimate_tokens("a"), 1); // rounds up
    }

    #[test]
    fn test_calculate_savings() {
        assert_eq!(calculate_savings(100, 70), 30.0);
        assert_eq!(calculate_savings(100, 80), 20.0);
        assert_eq!(calculate_savings(100, 100), 0.0);
        assert_eq!(calculate_savings(0, 0), 0.0);
    }

    #[test]
    fn test_llm_response_json_format() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
            active: true,
        };
        let response = LlmResponse::json(data);
        assert!(matches!(response.format, OutputFormat::Json));
    }

    #[test]
    fn test_llm_response_toon_format() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
            active: true,
        };
        let response = LlmResponse::toon(data);
        assert!(matches!(response.format, OutputFormat::Toon));
    }

    #[test]
    fn test_llm_response_without_headers() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
            active: true,
        };
        let response = LlmResponse::json(data).without_token_headers();
        assert!(!response.include_token_headers);
    }

    #[test]
    fn test_llm_response_with_headers() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
            active: true,
        };
        let response = LlmResponse::toon(data)
            .without_token_headers()
            .with_token_headers();
        assert!(response.include_token_headers);
    }
}
