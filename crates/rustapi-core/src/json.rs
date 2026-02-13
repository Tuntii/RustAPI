//! JSON utilities with optional SIMD acceleration
//!
//! This module provides JSON parsing and serialization utilities that can use
//! SIMD-accelerated parsing when the `simd-json` feature is enabled.
//!
//! # Performance
//!
//! When the `simd-json` feature is enabled, JSON parsing can be 2-4x faster
//! for large payloads. This is particularly beneficial for API servers that
//! handle large JSON request bodies.
//!
//! # Usage
//!
//! The module provides drop-in replacements for `serde_json` functions:
//!
//! ```rust,ignore
//! use rustapi_core::json;
//!
//! // Deserialize from bytes (uses simd-json if available)
//! let value: MyStruct = json::from_slice(&bytes)?;
//!
//! // Serialize to bytes
//! let bytes = json::to_vec(&value)?;
//! ```

use serde::{de::DeserializeOwned, Serialize};

/// Deserialize JSON from a byte slice.
///
/// When the `simd-json` feature is enabled, this uses SIMD-accelerated parsing.
/// Otherwise, it falls back to standard `serde_json`.
#[cfg(feature = "simd-json")]
pub fn from_slice<T: DeserializeOwned>(slice: &[u8]) -> Result<T, JsonError> {
    // simd-json requires mutable access for in-place parsing
    let mut slice_copy = slice.to_vec();
    simd_json::from_slice(&mut slice_copy).map_err(JsonError::SimdJson)
}

/// Deserialize JSON from a byte slice.
///
/// Standard `serde_json` implementation when `simd-json` feature is disabled.
#[cfg(not(feature = "simd-json"))]
pub fn from_slice<T: DeserializeOwned>(slice: &[u8]) -> Result<T, JsonError> {
    serde_json::from_slice(slice).map_err(JsonError::SerdeJson)
}

/// Deserialize JSON from a mutable byte slice (zero-copy with simd-json).
///
/// This variant allows simd-json to parse in-place without copying,
/// providing maximum performance.
#[cfg(feature = "simd-json")]
#[allow(dead_code)]
pub fn from_slice_mut<T: DeserializeOwned>(slice: &mut [u8]) -> Result<T, JsonError> {
    simd_json::from_slice(slice).map_err(JsonError::SimdJson)
}

/// Deserialize JSON from a mutable byte slice.
///
/// Falls back to standard implementation when simd-json is disabled.
#[cfg(not(feature = "simd-json"))]
#[allow(dead_code)]
pub fn from_slice_mut<T: DeserializeOwned>(slice: &mut [u8]) -> Result<T, JsonError> {
    serde_json::from_slice(slice).map_err(JsonError::SerdeJson)
}

/// Serialize a value to a JSON byte vector.
///
/// Uses pre-allocated buffer with estimated capacity for better performance.
#[cfg(feature = "simd-json")]
#[allow(dead_code)]
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    simd_json::to_vec(value).map_err(JsonError::SimdJson)
}

/// Serialize a value to a JSON byte vector.
///
/// Uses pre-allocated buffer with estimated capacity for better performance.
#[cfg(not(feature = "simd-json"))]
#[allow(dead_code)]
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    serde_json::to_vec(value).map_err(JsonError::SerdeJson)
}

/// Serialize a value to a JSON byte vector with pre-allocated capacity.
///
/// Use this when you have a good estimate of the output size to avoid
/// reallocations.
#[cfg(feature = "simd-json")]
pub fn to_vec_with_capacity<T: Serialize>(
    value: &T,
    capacity: usize,
) -> Result<Vec<u8>, JsonError> {
    let mut buf = Vec::with_capacity(capacity);
    simd_json::to_writer(&mut buf, value).map_err(JsonError::SimdJson)?;
    Ok(buf)
}

/// Serialize a value to a JSON byte vector with pre-allocated capacity.
///
/// Use this when you have a good estimate of the output size to avoid
/// reallocations.
#[cfg(not(feature = "simd-json"))]
pub fn to_vec_with_capacity<T: Serialize>(
    value: &T,
    capacity: usize,
) -> Result<Vec<u8>, JsonError> {
    let mut buf = Vec::with_capacity(capacity);
    serde_json::to_writer(&mut buf, value).map_err(JsonError::SerdeJson)?;
    Ok(buf)
}

/// Serialize a value to a pretty-printed JSON byte vector.
#[allow(dead_code)]
pub fn to_vec_pretty<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    serde_json::to_vec_pretty(value).map_err(JsonError::SerdeJson)
}

/// JSON error type that wraps both serde_json and simd-json errors.
#[derive(Debug)]
pub enum JsonError {
    SerdeJson(serde_json::Error),
    #[cfg(feature = "simd-json")]
    SimdJson(simd_json::Error),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::SerdeJson(e) => write!(f, "{}", e),
            #[cfg(feature = "simd-json")]
            JsonError::SimdJson(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for JsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JsonError::SerdeJson(e) => Some(e),
            #[cfg(feature = "simd-json")]
            JsonError::SimdJson(e) => Some(e),
        }
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        JsonError::SerdeJson(e)
    }
}

#[cfg(feature = "simd-json")]
impl From<simd_json::Error> for JsonError {
    fn from(e: simd_json::Error) -> Self {
        JsonError::SimdJson(e)
    }
}
