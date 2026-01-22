//! TOON extractor and response types

use crate::error::ToonError;
use crate::{TOON_CONTENT_TYPE, TOON_CONTENT_TYPE_TEXT};
use bytes::Bytes;
use http::{header, StatusCode};
use http_body_util::Full;
use rustapi_core::{ApiError, FromRequest, IntoResponse, Request, Response, Result};
use rustapi_openapi::{
    MediaType, Operation, OperationModifier, ResponseModifier, ResponseSpec, SchemaRef,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// TOON body extractor and response type
///
/// This extractor parses TOON-formatted request bodies and deserializes
/// them into the specified type. It can also be used as a response type
/// to serialize data into TOON format.
///
/// # Request Extraction
///
/// Accepts request bodies with content types:
/// - `application/toon`
/// - `text/toon`
///
/// # Example - Extractor
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use rustapi_rs::toon::Toon;
///
/// #[derive(Deserialize)]
/// struct CreateUser {
///     name: String,
///     email: String,
/// }
///
/// async fn create_user(Toon(user): Toon<CreateUser>) -> impl IntoResponse {
///     // user is parsed from TOON format
///     Json(user)
/// }
/// ```
///
/// # Example - Response
///
/// ```rust,ignore
/// use rustapi_rs::prelude::*;
/// use rustapi_rs::toon::Toon;
///
/// #[derive(Serialize)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// async fn get_user() -> Toon<User> {
///     Toon(User {
///         id: 1,
///         name: "Alice".to_string(),
///     })
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Toon<T>(pub T);

impl<T: DeserializeOwned + Send> FromRequest for Toon<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        // Check content type (optional - if provided, must be toon)
        if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
            let content_type_str = content_type.to_str().unwrap_or("");
            let is_toon = content_type_str.starts_with(TOON_CONTENT_TYPE)
                || content_type_str.starts_with(TOON_CONTENT_TYPE_TEXT);

            if !is_toon && !content_type_str.is_empty() {
                return Err(ToonError::InvalidContentType.into());
            }
        }

        // Get body bytes
        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        if body.is_empty() {
            return Err(ToonError::EmptyBody.into());
        }

        // Parse TOON
        let body_str =
            std::str::from_utf8(&body).map_err(|e| ApiError::bad_request(e.to_string()))?;

        let value: T =
            toon_format::decode_default(body_str).map_err(|e| ToonError::Decode(e.to_string()))?;

        Ok(Toon(value))
    }
}

impl<T> Deref for Toon<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Toon<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Toon<T> {
    fn from(value: T) -> Self {
        Toon(value)
    }
}

impl<T: Serialize> IntoResponse for Toon<T> {
    fn into_response(self) -> Response {
        match toon_format::encode_default(&self.0) {
            Ok(body) => http::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, TOON_CONTENT_TYPE)
                .body(rustapi_core::ResponseBody::from(body))
                .unwrap(),
            Err(err) => {
                let error: ApiError = ToonError::Encode(err.to_string()).into();
                error.into_response()
            }
        }
    }
}

// OpenAPI support: OperationModifier for Toon extractor
impl<T: Send> OperationModifier for Toon<T> {
    fn update_operation(op: &mut Operation) {
        let mut content = HashMap::new();
        content.insert(
            TOON_CONTENT_TYPE.to_string(),
            MediaType {
                schema: SchemaRef::Inline(serde_json::json!({
                    "type": "string",
                    "description": "TOON (Token-Oriented Object Notation) formatted request body"
                })),
            },
        );

        op.request_body = Some(rustapi_openapi::RequestBody {
            required: true,
            content,
        });
    }
}

// OpenAPI support: ResponseModifier for Toon response
impl<T: Serialize> ResponseModifier for Toon<T> {
    fn update_response(op: &mut Operation) {
        let mut content = HashMap::new();
        content.insert(
            TOON_CONTENT_TYPE.to_string(),
            MediaType {
                schema: SchemaRef::Inline(serde_json::json!({
                    "type": "string",
                    "description": "TOON (Token-Oriented Object Notation) formatted response"
                })),
            },
        );

        let response = ResponseSpec {
            description: "TOON formatted response - token-optimized for LLMs".to_string(),
            content: Some(content),
        };
        op.responses.insert("200".to_string(), response);
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct User {
        name: String,
        age: u32,
    }

    #[test]
    fn test_toon_encode() {
        let user = User {
            name: "Alice".to_string(),
            age: 30,
        };

        let toon_str = toon_format::encode_default(&user).unwrap();
        assert!(toon_str.contains("name:"));
        assert!(toon_str.contains("Alice"));
        assert!(toon_str.contains("age:"));
        assert!(toon_str.contains("30"));
    }

    #[test]
    fn test_toon_decode() {
        let toon_str = "name: Alice\nage: 30";
        let user: User = toon_format::decode_default(toon_str).unwrap();

        assert_eq!(user.name, "Alice");
        assert_eq!(user.age, 30);
    }

    #[test]
    fn test_toon_roundtrip() {
        let original = User {
            name: "Bob".to_string(),
            age: 25,
        };

        let encoded = toon_format::encode_default(&original).unwrap();
        let decoded: User = toon_format::decode_default(&encoded).unwrap();

        assert_eq!(original, decoded);
    }
}
