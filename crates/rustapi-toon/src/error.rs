//! TOON Error types and conversions

use std::fmt;
use rustapi_core::ApiError;

/// Error type for TOON operations
#[derive(Debug)]
pub enum ToonError {
    /// Error during TOON encoding (serialization)
    Encode(String),
    /// Error during TOON decoding (parsing/deserialization)
    Decode(String),
    /// Invalid content type for TOON request
    InvalidContentType,
    /// Empty body provided
    EmptyBody,
}

impl fmt::Display for ToonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(msg) => write!(f, "TOON encoding error: {}", msg),
            Self::Decode(msg) => write!(f, "TOON decoding error: {}", msg),
            Self::InvalidContentType => write!(f, "Invalid content type: expected application/toon or text/toon"),
            Self::EmptyBody => write!(f, "Empty request body"),
        }
    }
}

impl std::error::Error for ToonError {}

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
            ToonError::Encode(msg) => ApiError::internal(format!("Failed to encode TOON: {}", msg)),
            ToonError::Decode(msg) => ApiError::bad_request(format!("Invalid TOON: {}", msg)),
            ToonError::InvalidContentType => ApiError::bad_request(
                "Invalid content type: expected application/toon or text/toon",
            ),
            ToonError::EmptyBody => ApiError::bad_request("Empty request body"),
        }
    }
}
