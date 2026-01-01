//! TOON Error types and conversions

use rustapi_core::ApiError;
use thiserror::Error;

/// Error type for TOON operations
#[derive(Error, Debug)]
pub enum ToonError {
    /// Error during TOON encoding (serialization)
    #[error("TOON encoding error: {0}")]
    Encode(String),

    /// Error during TOON decoding (parsing/deserialization)
    #[error("TOON decoding error: {0}")]
    Decode(String),

    /// Invalid content type for TOON request
    #[error("Invalid content type: expected application/toon or text/toon")]
    InvalidContentType,

    /// Empty body provided
    #[error("Empty request body")]
    EmptyBody,
}

impl From<toon_format::ToonError> for ToonError {
    fn from(err: toon_format::ToonError) -> Self {
        match &err {
            toon_format::ToonError::SerializationError(_) => ToonError::Encode(err.to_string()),
            _ => ToonError::Decode(err.to_string()),
        }
    }
}

impl From<ToonError> for ApiError {
    fn from(err: ToonError) -> Self {
        match err {
            ToonError::Encode(msg) => {
                ApiError::internal(format!("Failed to encode TOON: {}", msg))
            }
            ToonError::Decode(msg) => ApiError::bad_request(format!("Invalid TOON: {}", msg)),
            ToonError::InvalidContentType => ApiError::bad_request(
                "Invalid content type: expected application/toon or text/toon",
            ),
            ToonError::EmptyBody => ApiError::bad_request("Empty request body"),
        }
    }
}
