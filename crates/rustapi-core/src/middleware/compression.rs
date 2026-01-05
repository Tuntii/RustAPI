//! Response compression middleware
//!
//! This module provides Gzip and Brotli compression for response bodies.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_core::middleware::CompressionLayer;
//!
//! RustApi::new()
//!     .layer(CompressionLayer::new())
//!     .route("/", get(handler))
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

use crate::middleware::{BoxedNext, MiddlewareLayer};
use crate::request::Request;
use crate::response::Response;
use bytes::Bytes;
use flate2::write::{DeflateEncoder, GzEncoder};
use flate2::Compression;
use http::header;
use http_body_util::{BodyExt, Full};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// Gzip compression
    Gzip,
    /// Deflate compression
    Deflate,
    /// Brotli compression (if enabled)
    #[cfg(feature = "compression-brotli")]
    Brotli,
    /// No compression
    Identity,
}

impl CompressionAlgorithm {
    /// Get the Content-Encoding header value
    pub fn content_encoding(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
            #[cfg(feature = "compression-brotli")]
            Self::Brotli => "br",
            Self::Identity => "identity",
        }
    }

    /// Parse from Accept-Encoding header
    pub fn from_accept_encoding(header: &str) -> Self {
        let encodings: Vec<(f32, &str)> = header
            .split(',')
            .map(|part| {
                let part = part.trim();
                let (encoding, quality) = if let Some((enc, q)) = part.split_once(";q=") {
                    (enc.trim(), q.trim().parse().unwrap_or(1.0))
                } else {
                    (part, 1.0)
                };
                (quality, encoding)
            })
            .collect();

        // Sort by quality (highest first)
        let mut sorted = encodings;
        sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (_, encoding) in sorted {
            match encoding.to_lowercase().as_str() {
                #[cfg(feature = "compression-brotli")]
                "br" => return Self::Brotli,
                "gzip" => return Self::Gzip,
                "deflate" => return Self::Deflate,
                "*" => return Self::Gzip, // Default to gzip for wildcard
                _ => continue,
            }
        }

        Self::Identity
    }
}

/// Configuration for compression middleware
#[derive(Clone)]
pub struct CompressionConfig {
    /// Minimum response size to compress (default: 1024 bytes)
    pub min_size: usize,
    /// Compression level (0-9 for gzip/deflate, 0-11 for brotli)
    pub level: u32,
    /// Content types to compress (empty = all compressible types)
    pub content_types: Vec<String>,
    /// Enable gzip compression
    pub gzip: bool,
    /// Enable deflate compression
    pub deflate: bool,
    /// Enable brotli compression
    #[cfg(feature = "compression-brotli")]
    pub brotli: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024,
            level: 6, // Good balance between speed and compression
            content_types: vec![
                "text/".to_string(),
                "application/json".to_string(),
                "application/javascript".to_string(),
                "application/xml".to_string(),
                "image/svg+xml".to_string(),
            ],
            gzip: true,
            deflate: true,
            #[cfg(feature = "compression-brotli")]
            brotli: true,
        }
    }
}

impl CompressionConfig {
    /// Create a new compression config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum size for compression
    pub fn min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Set compression level (0-9)
    pub fn level(mut self, level: u32) -> Self {
        self.level = level.min(9);
        self
    }

    /// Enable or disable gzip
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.gzip = enabled;
        self
    }

    /// Enable or disable deflate
    pub fn deflate(mut self, enabled: bool) -> Self {
        self.deflate = enabled;
        self
    }

    /// Enable or disable brotli
    #[cfg(feature = "compression-brotli")]
    pub fn brotli(mut self, enabled: bool) -> Self {
        self.brotli = enabled;
        self
    }

    /// Add a content type to compress
    pub fn add_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_types.push(content_type.into());
        self
    }

    /// Set content types to compress
    pub fn content_types(mut self, types: Vec<String>) -> Self {
        self.content_types = types;
        self
    }

    /// Check if a content type should be compressed
    fn should_compress_content_type(&self, content_type: &str) -> bool {
        if self.content_types.is_empty() {
            return true;
        }
        self.content_types
            .iter()
            .any(|ct| content_type.starts_with(ct.as_str()))
    }
}

/// Compression middleware layer
#[derive(Clone)]
pub struct CompressionLayer {
    config: CompressionConfig,
}

impl CompressionLayer {
    /// Create a new compression layer with default config
    pub fn new() -> Self {
        Self {
            config: CompressionConfig::default(),
        }
    }

    /// Create a compression layer with custom config
    pub fn with_config(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Set minimum size for compression
    pub fn min_size(mut self, size: usize) -> Self {
        self.config.min_size = size;
        self
    }

    /// Set compression level
    pub fn level(mut self, level: u32) -> Self {
        self.config.level = level.min(9);
        self
    }

    /// Compress bytes using the specified algorithm
    fn compress(
        &self,
        data: &[u8],
        algorithm: CompressionAlgorithm,
    ) -> Result<Vec<u8>, std::io::Error> {
        let level = Compression::new(self.config.level);

        match algorithm {
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), level);
                encoder.write_all(data)?;
                encoder.finish()
            }
            CompressionAlgorithm::Deflate => {
                let mut encoder = DeflateEncoder::new(Vec::new(), level);
                encoder.write_all(data)?;
                encoder.finish()
            }
            #[cfg(feature = "compression-brotli")]
            CompressionAlgorithm::Brotli => {
                use brotli::enc::BrotliEncoderParams;
                let mut output = Vec::new();
                let params = BrotliEncoderParams::default();
                brotli::BrotliCompress(&mut &data[..], &mut output, &params)?;
                Ok(output)
            }
            CompressionAlgorithm::Identity => Ok(data.to_vec()),
        }
    }
}

impl Default for CompressionLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for CompressionLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();

        // Get accepted encoding from request
        let accept_encoding = req
            .headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            // Call next handler
            let response = next(req).await;

            // Determine compression algorithm
            let algorithm = accept_encoding
                .as_ref()
                .map(|ae| CompressionAlgorithm::from_accept_encoding(ae))
                .unwrap_or(CompressionAlgorithm::Identity);

            // Check if we should compress
            if algorithm == CompressionAlgorithm::Identity {
                return response;
            }

            // Check if response is already encoded
            if response.headers().contains_key(header::CONTENT_ENCODING) {
                return response;
            }

            // Check content type
            let content_type = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !config.should_compress_content_type(content_type) {
                return response;
            }

            // Get body
            let (parts, body) = response.into_parts();
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => return http::Response::from_parts(parts, Full::new(Bytes::new())),
            };

            // Check minimum size
            if body_bytes.len() < config.min_size {
                let response = http::Response::from_parts(parts, Full::new(body_bytes));
                return response;
            }

            // Compress
            let layer = CompressionLayer { config };
            match layer.compress(&body_bytes, algorithm) {
                Ok(compressed) => {
                    // Only use compressed if it's smaller
                    if compressed.len() < body_bytes.len() {
                        let mut response =
                            http::Response::from_parts(parts, Full::new(Bytes::from(compressed)));
                        response.headers_mut().insert(
                            header::CONTENT_ENCODING,
                            algorithm.content_encoding().parse().unwrap(),
                        );
                        response.headers_mut().remove(header::CONTENT_LENGTH);
                        response
                    } else {
                        http::Response::from_parts(parts, Full::new(body_bytes))
                    }
                }
                Err(_) => http::Response::from_parts(parts, Full::new(body_bytes)),
            }
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_encoding() {
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("gzip"),
            CompressionAlgorithm::Gzip
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("deflate"),
            CompressionAlgorithm::Deflate
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("gzip, deflate"),
            CompressionAlgorithm::Gzip
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("deflate;q=1.0, gzip;q=0.5"),
            CompressionAlgorithm::Deflate
        );
        assert_eq!(
            CompressionAlgorithm::from_accept_encoding("identity"),
            CompressionAlgorithm::Identity
        );
    }

    #[test]
    fn test_compression_config() {
        let config = CompressionConfig::new()
            .min_size(512)
            .level(9)
            .gzip(true)
            .deflate(false)
            .add_content_type("application/custom");

        assert_eq!(config.min_size, 512);
        assert_eq!(config.level, 9);
        assert!(config.gzip);
        assert!(!config.deflate);
        assert!(config
            .content_types
            .contains(&"application/custom".to_string()));
    }

    #[test]
    fn test_content_type_filtering() {
        let config = CompressionConfig::new();

        assert!(config.should_compress_content_type("text/html"));
        assert!(config.should_compress_content_type("application/json"));
        assert!(config.should_compress_content_type("text/plain"));
        assert!(!config.should_compress_content_type("image/png"));
    }

    #[test]
    fn test_gzip_compression() {
        let layer = CompressionLayer::new();
        let data = b"Hello, World! This is test data that should be compressed.";

        let compressed = layer.compress(data, CompressionAlgorithm::Gzip).unwrap();

        // Compressed data should be valid gzip (starts with magic bytes)
        assert!(compressed.len() >= 2);
        assert_eq!(compressed[0], 0x1f);
        assert_eq!(compressed[1], 0x8b);
    }

    #[test]
    fn test_deflate_compression() {
        let layer = CompressionLayer::new();
        let data = b"Hello, World! This is test data that should be compressed.";

        let compressed = layer.compress(data, CompressionAlgorithm::Deflate).unwrap();

        // Deflate produces output
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_identity_no_compression() {
        let layer = CompressionLayer::new();
        let data = b"Hello, World!";

        let result = layer
            .compress(data, CompressionAlgorithm::Identity)
            .unwrap();
        assert_eq!(result, data);
    }
}
