//! Static file serving for RustAPI
//!
//! This module provides types for serving static files from a directory.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! RustApi::new()
//!     .serve_static("/assets", "./static")
//!     .serve_static("/uploads", "./uploads")
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

use crate::error::ApiError;
use crate::response::{IntoResponse, Response};
use bytes::Bytes;
use http::{header, StatusCode};
use http_body_util::Full;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

/// MIME type detection based on file extension
fn mime_type_for_extension(extension: &str) -> &'static str {
    match extension.to_lowercase().as_str() {
        // Text
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "csv" => "text/csv",

        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "avif" => "image/avif",

        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",

        // Audio/Video
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",

        // Documents
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",

        // WebAssembly
        "wasm" => "application/wasm",

        // Default
        _ => "application/octet-stream",
    }
}

/// Calculate ETag from file metadata
fn calculate_etag(modified: SystemTime, size: u64) -> String {
    let timestamp = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("\"{:x}-{:x}\"", timestamp, size)
}

/// Format system time as HTTP date (RFC 7231)
fn format_http_date(time: SystemTime) -> String {
    use std::time::Duration;

    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let secs = duration.as_secs();

    // Simple HTTP date formatting
    // In production, you'd use a proper date formatting library
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate day of week and date (simplified)
    let days_since_epoch = days;
    let day_of_week = (days_since_epoch + 4) % 7; // Jan 1, 1970 was Thursday
    let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // Calculate year, month, day (simplified leap year handling)
    let mut year = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let mut month = 0;
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (i, &days_in_month) in days_in_months.iter().enumerate() {
        if remaining_days < days_in_month as i64 {
            month = i;
            break;
        }
        remaining_days -= days_in_month as i64;
    }

    let day = remaining_days + 1;

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        day_names[day_of_week as usize], day, month_names[month], year, hours, minutes, seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Static file serving configuration
#[derive(Clone)]
pub struct StaticFileConfig {
    /// Root directory for static files
    pub root: PathBuf,
    /// URL path prefix
    pub prefix: String,
    /// Whether to serve index.html for directories
    pub serve_index: bool,
    /// Index file name (default: "index.html")
    pub index_file: String,
    /// Enable ETag headers
    pub etag: bool,
    /// Enable Last-Modified headers
    pub last_modified: bool,
    /// Cache-Control max-age in seconds (0 = no caching)
    pub max_age: u64,
    /// Fallback file for SPA routing (e.g., "index.html")
    pub fallback: Option<String>,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("./static"),
            prefix: "/".to_string(),
            serve_index: true,
            index_file: "index.html".to_string(),
            etag: true,
            last_modified: true,
            max_age: 3600, // 1 hour
            fallback: None,
        }
    }
}

impl StaticFileConfig {
    /// Create a new static file configuration
    pub fn new(root: impl Into<PathBuf>, prefix: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            prefix: prefix.into(),
            ..Default::default()
        }
    }

    /// Set whether to serve index.html for directories
    pub fn serve_index(mut self, enabled: bool) -> Self {
        self.serve_index = enabled;
        self
    }

    /// Set the index file name
    pub fn index_file(mut self, name: impl Into<String>) -> Self {
        self.index_file = name.into();
        self
    }

    /// Enable or disable ETag headers
    pub fn etag(mut self, enabled: bool) -> Self {
        self.etag = enabled;
        self
    }

    /// Enable or disable Last-Modified headers
    pub fn last_modified(mut self, enabled: bool) -> Self {
        self.last_modified = enabled;
        self
    }

    /// Set Cache-Control max-age in seconds
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = seconds;
        self
    }

    /// Set a fallback file for SPA routing
    pub fn fallback(mut self, file: impl Into<String>) -> Self {
        self.fallback = Some(file.into());
        self
    }
}

/// Static file response
pub struct StaticFile {
    #[allow(dead_code)]
    path: PathBuf,
    #[allow(dead_code)]
    config: StaticFileConfig,
}

impl StaticFile {
    /// Create a new static file response
    pub fn new(path: impl Into<PathBuf>, config: StaticFileConfig) -> Self {
        Self {
            path: path.into(),
            config,
        }
    }

    /// Serve a file from a path relative to the root
    pub async fn serve(
        relative_path: &str,
        config: &StaticFileConfig,
    ) -> Result<Response, ApiError> {
        // Sanitize path to prevent directory traversal
        let clean_path = sanitize_path(relative_path);
        let file_path = config.root.join(&clean_path);

        // Check if it's a directory
        if file_path.is_dir() {
            if config.serve_index {
                let index_path = file_path.join(&config.index_file);
                if index_path.exists() {
                    return Self::serve_file(&index_path, config).await;
                }
            }
            return Err(ApiError::not_found("Directory listing not allowed"));
        }

        // Try to serve the file
        match Self::serve_file(&file_path, config).await {
            Ok(response) => Ok(response),
            Err(_) if config.fallback.is_some() => {
                // Try fallback
                let fallback_path = config.root.join(config.fallback.as_ref().unwrap());
                Self::serve_file(&fallback_path, config).await
            }
            Err(e) => Err(e),
        }
    }

    /// Serve a specific file
    async fn serve_file(path: &Path, config: &StaticFileConfig) -> Result<Response, ApiError> {
        // Check if file exists
        let metadata = fs::metadata(path)
            .await
            .map_err(|_| ApiError::not_found(format!("File not found: {}", path.display())))?;

        if !metadata.is_file() {
            return Err(ApiError::not_found("Not a file"));
        }

        // Read file
        let content = fs::read(path)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to read file: {}", e)))?;

        // Determine content type
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content_type = mime_type_for_extension(extension);

        // Build response
        let mut builder = http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, content.len());

        // Add ETag
        if config.etag {
            if let Ok(modified) = metadata.modified() {
                let etag = calculate_etag(modified, metadata.len());
                builder = builder.header(header::ETAG, etag);
            }
        }

        // Add Last-Modified
        if config.last_modified {
            if let Ok(modified) = metadata.modified() {
                let http_date = format_http_date(modified);
                builder = builder.header(header::LAST_MODIFIED, http_date);
            }
        }

        // Add Cache-Control
        if config.max_age > 0 {
            builder = builder.header(
                header::CACHE_CONTROL,
                format!("public, max-age={}", config.max_age),
            );
        }

        builder
            .body(crate::response::Body::from(content))
            .map_err(|e| ApiError::internal(format!("Failed to build response: {}", e)))
    }
}

/// Sanitize a file path to prevent directory traversal
fn sanitize_path(path: &str) -> String {
    // Remove leading slashes
    let path = path.trim_start_matches('/');

    // Split and filter out dangerous components
    let parts: Vec<&str> = path
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != ".." && !part.contains('\\'))
        .collect();

    parts.join("/")
}

/// Create a handler for serving static files
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::static_files::{static_handler, StaticFileConfig};
///
/// let config = StaticFileConfig::new("./public", "/assets");
/// let handler = static_handler(config);
/// ```
pub fn static_handler(
    config: StaticFileConfig,
) -> impl Fn(crate::Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send
       + Sync
       + 'static {
    move |req: crate::Request| {
        let config = config.clone();
        let path = req.uri().path().to_string();

        Box::pin(async move {
            // Strip prefix from path
            let relative_path = path.strip_prefix(&config.prefix).unwrap_or(&path);

            match StaticFile::serve(relative_path, &config).await {
                Ok(response) => response,
                Err(err) => err.into_response(),
            }
        })
    }
}

/// Create a static file serving route
///
/// This is the main function for adding static file serving to RustAPI.
///
/// # Arguments
///
/// * `prefix` - URL path prefix (e.g., "/static")
/// * `root` - File system root directory
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::static_files::serve_dir;
///
/// // The handler can be used with a catch-all route
/// let config = serve_dir("/static", "./public");
/// ```
pub fn serve_dir(prefix: impl Into<String>, root: impl Into<PathBuf>) -> StaticFileConfig {
    StaticFileConfig::new(root.into(), prefix.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(mime_type_for_extension("html"), "text/html; charset=utf-8");
        assert_eq!(mime_type_for_extension("css"), "text/css; charset=utf-8");
        assert_eq!(
            mime_type_for_extension("js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(mime_type_for_extension("png"), "image/png");
        assert_eq!(mime_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(mime_type_for_extension("json"), "application/json");
        assert_eq!(
            mime_type_for_extension("unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("file.txt"), "file.txt");
        assert_eq!(sanitize_path("/file.txt"), "file.txt");
        assert_eq!(sanitize_path("../../../etc/passwd"), "etc/passwd");
        assert_eq!(sanitize_path("foo/../bar"), "foo/bar");
        assert_eq!(sanitize_path("./file.txt"), "file.txt");
        assert_eq!(sanitize_path("foo/./bar"), "foo/bar");
    }

    #[test]
    fn test_etag_calculation() {
        let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000000);
        let etag = calculate_etag(time, 12345);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(etag.contains('-'));
    }

    #[test]
    fn test_static_file_config() {
        let config = StaticFileConfig::new("./public", "/assets")
            .serve_index(true)
            .index_file("index.html")
            .etag(true)
            .last_modified(true)
            .max_age(7200)
            .fallback("index.html");

        assert_eq!(config.root, PathBuf::from("./public"));
        assert_eq!(config.prefix, "/assets");
        assert!(config.serve_index);
        assert_eq!(config.index_file, "index.html");
        assert!(config.etag);
        assert!(config.last_modified);
        assert_eq!(config.max_age, 7200);
        assert_eq!(config.fallback, Some("index.html".to_string()));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // Divisible by 400
        assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
        assert!(is_leap_year(2024)); // Divisible by 4 but not 100
        assert!(!is_leap_year(2023)); // Not divisible by 4
    }
}
