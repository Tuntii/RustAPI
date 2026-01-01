//! OpenAPI/Swagger Extensions for TOON Format
//!
//! This module provides OpenAPI schema definitions and documentation helpers
//! for TOON format responses.

use crate::TOON_CONTENT_TYPE;

/// TOON format description for OpenAPI
pub const TOON_FORMAT_DESCRIPTION: &str = r#"
**TOON (Token-Oriented Object Notation)**

A compact, human-readable format designed for passing structured data to Large Language Models (LLMs) with significantly reduced token usage (typically 40-60% savings).

### Format Example

**JSON (561 bytes, ~141 tokens):**
```json
[
  {"id": 1, "name": "Alice", "role": "admin", "active": true},
  {"id": 2, "name": "Bob", "role": "user", "active": false}
]
```

**TOON (259 bytes, ~65 tokens) - 54% savings:**
```
[2]{id,name,role,active}:
  1,Alice,admin,true
  2,Bob,user,false
```

### Usage

Set `Accept: application/toon` header to receive TOON formatted responses.

### When to Use TOON

- Sending data to LLM APIs (reduces token costs)
- Bandwidth-constrained environments
- Caching large datasets
- Any scenario where token efficiency matters
"#;

/// Generate OpenAPI schema for TOON format
pub fn toon_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "format": "toon",
        "description": "TOON (Token-Oriented Object Notation) formatted data",
        "externalDocs": {
            "description": "TOON Format Specification",
            "url": "https://toonformat.dev/"
        }
    })
}

/// Generate OpenAPI vendor extension for TOON support
pub fn toon_extension() -> serde_json::Value {
    serde_json::json!({
        "x-toon-support": {
            "enabled": true,
            "contentTypes": [TOON_CONTENT_TYPE, "text/toon"],
            "tokenSavings": "40-60%",
            "documentation": "https://toonformat.dev/"
        }
    })
}

/// Schema for token comparison headers
pub fn token_headers_schema() -> serde_json::Value {
    serde_json::json!({
        "X-Token-Count-JSON": {
            "description": "Estimated token count for JSON format (~4 chars/token)",
            "schema": {
                "type": "integer",
                "example": 141
            }
        },
        "X-Token-Count-TOON": {
            "description": "Estimated token count for TOON format (~4 chars/token)",
            "schema": {
                "type": "integer",
                "example": 65
            }
        },
        "X-Token-Savings": {
            "description": "Percentage of tokens saved by using TOON format",
            "schema": {
                "type": "string",
                "example": "53.90%"
            }
        },
        "X-Format-Used": {
            "description": "The format used in the response (json or toon)",
            "schema": {
                "type": "string",
                "enum": ["json", "toon"]
            }
        }
    })
}

/// Generate example responses showing JSON vs TOON
pub fn format_comparison_example<T: serde::Serialize>(data: &T) -> serde_json::Value {
    let json_str = serde_json::to_string_pretty(data).unwrap_or_default();
    let toon_str = toon_format::encode_default(data).unwrap_or_default();
    
    let json_bytes = json_str.len();
    let toon_bytes = toon_str.len();
    let json_tokens = json_bytes / 4;
    let toon_tokens = toon_bytes / 4;
    let savings = if json_tokens > 0 {
        ((json_tokens - toon_tokens) as f64 / json_tokens as f64) * 100.0
    } else {
        0.0
    };
    
    serde_json::json!({
        "json": {
            "content": json_str,
            "bytes": json_bytes,
            "estimatedTokens": json_tokens
        },
        "toon": {
            "content": toon_str,
            "bytes": toon_bytes,
            "estimatedTokens": toon_tokens
        },
        "savings": {
            "bytes": format!("{:.1}%", ((json_bytes - toon_bytes) as f64 / json_bytes as f64) * 100.0),
            "tokens": format!("{:.1}%", savings)
        }
    })
}

/// OpenAPI info description with TOON support notice
pub fn api_description_with_toon(base_description: &str) -> String {
    format!(
        "{}\n\n---\n\n### 🚀 TOON Format Support\n\nThis API supports **TOON (Token-Oriented Object Notation)** \
        for reduced token usage when sending data to LLMs.\n\n\
        Set `Accept: application/toon` header to receive TOON formatted responses.\n\n\
        **Benefits:**\n\
        - 40-60% token savings\n\
        - Human-readable format\n\
        - Reduced API costs\n\n\
        [Learn more about TOON](https://toonformat.dev/)",
        base_description
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    
    #[derive(Serialize)]
    struct TestUser {
        id: u64,
        name: String,
    }
    
    #[test]
    fn test_toon_schema() {
        let schema = toon_schema();
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "toon");
    }
    
    #[test]
    fn test_toon_extension() {
        let ext = toon_extension();
        assert!(ext["x-toon-support"]["enabled"].as_bool().unwrap());
    }
    
    #[test]
    fn test_token_headers_schema() {
        let headers = token_headers_schema();
        assert!(headers["X-Token-Count-JSON"].is_object());
        assert!(headers["X-Token-Count-TOON"].is_object());
        assert!(headers["X-Token-Savings"].is_object());
        assert!(headers["X-Format-Used"].is_object());
    }
    
    #[test]
    fn test_format_comparison_example() {
        let users = vec![
            TestUser { id: 1, name: "Alice".to_string() },
            TestUser { id: 2, name: "Bob".to_string() },
        ];
        let comparison = format_comparison_example(&users);
        
        assert!(comparison["json"]["bytes"].as_u64().unwrap() > 0);
        assert!(comparison["toon"]["bytes"].as_u64().unwrap() > 0);
        // TOON should be smaller
        assert!(comparison["toon"]["bytes"].as_u64().unwrap() < comparison["json"]["bytes"].as_u64().unwrap());
    }
    
    #[test]
    fn test_api_description_with_toon() {
        let desc = api_description_with_toon("My API");
        assert!(desc.contains("TOON Format Support"));
        assert!(desc.contains("40-60% token savings"));
    }
}
