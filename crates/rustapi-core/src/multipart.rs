//! Multipart form data extractor for file uploads
//!
//! This module provides types for handling `multipart/form-data` requests,
//! commonly used for file uploads.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::multipart::{Multipart, FieldData};
//!
//! async fn upload(mut multipart: Multipart) -> Result<String, ApiError> {
//!     while let Some(field) = multipart.next_field().await? {
//!         let name = field.name().unwrap_or("unknown");
//!         let filename = field.file_name().map(|s| s.to_string());
//!         let data = field.bytes().await?;
//!         
//!         println!("Field: {}, File: {:?}, Size: {} bytes", name, filename, data.len());
//!     }
//!     Ok("Upload successful".to_string())
//! }
//! ```

use crate::error::{ApiError, Result};
use crate::extract::FromRequest;
use crate::request::Request;
use bytes::Bytes;
use std::path::Path;

/// Maximum file size (default: 10MB)
pub const DEFAULT_MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of fields in multipart form (default: 100)
pub const DEFAULT_MAX_FIELDS: usize = 100;

/// Multipart form data extractor
///
/// Parses `multipart/form-data` requests, commonly used for file uploads.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::multipart::Multipart;
///
/// async fn upload(mut multipart: Multipart) -> Result<String, ApiError> {
///     while let Some(field) = multipart.next_field().await? {
///         let name = field.name().unwrap_or("unknown").to_string();
///         let data = field.bytes().await?;
///         println!("Received field '{}' with {} bytes", name, data.len());
///     }
///     Ok("Upload complete".to_string())
/// }
/// ```
pub struct Multipart {
    fields: Vec<MultipartField>,
    current_index: usize,
}

impl Multipart {
    /// Create a new Multipart from raw data
    fn new(fields: Vec<MultipartField>) -> Self {
        Self {
            fields,
            current_index: 0,
        }
    }

    /// Get the next field from the multipart form
    pub async fn next_field(&mut self) -> Result<Option<MultipartField>> {
        if self.current_index >= self.fields.len() {
            return Ok(None);
        }
        let field = self.fields.get(self.current_index).cloned();
        self.current_index += 1;
        Ok(field)
    }

    /// Collect all fields into a vector
    pub fn into_fields(self) -> Vec<MultipartField> {
        self.fields
    }

    /// Get the number of fields
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

/// A single field from a multipart form
#[derive(Clone)]
pub struct MultipartField {
    name: Option<String>,
    file_name: Option<String>,
    content_type: Option<String>,
    data: Bytes,
}

impl MultipartField {
    /// Create a new multipart field
    pub fn new(
        name: Option<String>,
        file_name: Option<String>,
        content_type: Option<String>,
        data: Bytes,
    ) -> Self {
        Self {
            name,
            file_name,
            content_type,
            data,
        }
    }

    /// Get the field name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the original filename (if this is a file upload)
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// Get the content type of the field
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Check if this field is a file upload
    pub fn is_file(&self) -> bool {
        self.file_name.is_some()
    }

    /// Get the field data as bytes
    pub async fn bytes(&self) -> Result<Bytes> {
        Ok(self.data.clone())
    }

    /// Get the field data as a string (UTF-8)
    pub async fn text(&self) -> Result<String> {
        String::from_utf8(self.data.to_vec())
            .map_err(|e| ApiError::bad_request(format!("Invalid UTF-8 in field: {}", e)))
    }

    /// Get the size of the field data in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Save the file to disk
    ///
    /// # Arguments
    ///
    /// * `path` - The directory to save the file to
    /// * `filename` - Optional custom filename, uses original filename if None
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// field.save_to("./uploads", None).await?;
    /// // or with custom filename
    /// field.save_to("./uploads", Some("custom_name.txt")).await?;
    /// ```
    pub async fn save_to(&self, dir: impl AsRef<Path>, filename: Option<&str>) -> Result<String> {
        let dir = dir.as_ref();
        
        // Ensure directory exists
        tokio::fs::create_dir_all(dir).await.map_err(|e| {
            ApiError::internal(format!("Failed to create upload directory: {}", e))
        })?;

        // Determine filename
        let final_filename = filename
            .map(|s| s.to_string())
            .or_else(|| self.file_name.clone())
            .ok_or_else(|| ApiError::bad_request("No filename provided and field has no filename"))?;

        // Sanitize filename to prevent path traversal
        let safe_filename = sanitize_filename(&final_filename);
        let file_path = dir.join(&safe_filename);

        // Write file
        tokio::fs::write(&file_path, &self.data).await.map_err(|e| {
            ApiError::internal(format!("Failed to save file: {}", e))
        })?;

        Ok(file_path.to_string_lossy().to_string())
    }
}

/// Sanitize a filename to prevent path traversal attacks
fn sanitize_filename(filename: &str) -> String {
    // Remove path separators and parent directory references
    filename
        .replace(['/', '\\'], "_")
        .replace("..", "_")
        .trim_start_matches('.')
        .to_string()
}

impl FromRequest for Multipart {
    async fn from_request(req: &mut Request) -> Result<Self> {
        // Check content type
        let content_type = req
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::bad_request("Missing Content-Type header"))?;

        if !content_type.starts_with("multipart/form-data") {
            return Err(ApiError::bad_request(format!(
                "Expected multipart/form-data, got: {}",
                content_type
            )));
        }

        // Extract boundary
        let boundary = extract_boundary(content_type)
            .ok_or_else(|| ApiError::bad_request("Missing boundary in Content-Type"))?;

        // Get body
        let body = req
            .take_body()
            .ok_or_else(|| ApiError::internal("Body already consumed"))?;

        // Parse multipart
        let fields = parse_multipart(&body, &boundary)?;

        Ok(Multipart::new(fields))
    }
}

/// Extract boundary from Content-Type header
fn extract_boundary(content_type: &str) -> Option<String> {
    content_type
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            if part.starts_with("boundary=") {
                let boundary = part.trim_start_matches("boundary=").trim_matches('"');
                Some(boundary.to_string())
            } else {
                None
            }
        })
}

/// Parse multipart form data
fn parse_multipart(body: &Bytes, boundary: &str) -> Result<Vec<MultipartField>> {
    let mut fields = Vec::new();
    let delimiter = format!("--{}", boundary);
    let end_delimiter = format!("--{}--", boundary);

    // Convert body to string for easier parsing
    // Note: This is a simplified parser. For production, consider using multer crate.
    let body_str = String::from_utf8_lossy(body);
    
    // Split by delimiter
    let parts: Vec<&str> = body_str.split(&delimiter).collect();

    for part in parts.iter().skip(1) {
        // Skip empty parts and end delimiter
        let part = part.trim_start_matches("\r\n").trim_start_matches('\n');
        if part.is_empty() || part.starts_with("--") {
            continue;
        }

        // Find header/body separator (blank line)
        let header_body_split = if let Some(pos) = part.find("\r\n\r\n") {
            pos
        } else if let Some(pos) = part.find("\n\n") {
            pos
        } else {
            continue;
        };

        let headers_section = &part[..header_body_split];
        let body_section = &part[header_body_split..].trim_start_matches("\r\n\r\n").trim_start_matches("\n\n");

        // Remove trailing boundary markers from body
        let body_section = body_section
            .trim_end_matches(&end_delimiter)
            .trim_end_matches(&delimiter)
            .trim_end_matches("\r\n")
            .trim_end_matches('\n');

        // Parse headers
        let mut name = None;
        let mut filename = None;
        let mut content_type = None;

        for header_line in headers_section.lines() {
            let header_line = header_line.trim();
            if header_line.is_empty() {
                continue;
            }

            if let Some((key, value)) = header_line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim();

                match key.as_str() {
                    "content-disposition" => {
                        // Parse name and filename from Content-Disposition
                        for part in value.split(';') {
                            let part = part.trim();
                            if part.starts_with("name=") {
                                name = Some(
                                    part.trim_start_matches("name=")
                                        .trim_matches('"')
                                        .to_string(),
                                );
                            } else if part.starts_with("filename=") {
                                filename = Some(
                                    part.trim_start_matches("filename=")
                                        .trim_matches('"')
                                        .to_string(),
                                );
                            }
                        }
                    }
                    "content-type" => {
                        content_type = Some(value.to_string());
                    }
                    _ => {}
                }
            }
        }

        fields.push(MultipartField::new(
            name,
            filename,
            content_type,
            Bytes::copy_from_slice(body_section.as_bytes()),
        ));
    }

    Ok(fields)
}

/// Configuration for multipart form handling
#[derive(Clone)]
pub struct MultipartConfig {
    /// Maximum total size of the multipart form (default: 10MB)
    pub max_size: usize,
    /// Maximum number of fields (default: 100)
    pub max_fields: usize,
    /// Maximum size per file (default: 10MB)
    pub max_file_size: usize,
    /// Allowed content types for files (empty = all allowed)
    pub allowed_content_types: Vec<String>,
}

impl Default for MultipartConfig {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_FILE_SIZE,
            max_fields: DEFAULT_MAX_FIELDS,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            allowed_content_types: Vec::new(),
        }
    }
}

impl MultipartConfig {
    /// Create a new multipart config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum total size
    pub fn max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Set the maximum number of fields
    pub fn max_fields(mut self, count: usize) -> Self {
        self.max_fields = count;
        self
    }

    /// Set the maximum file size
    pub fn max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Set allowed content types for file uploads
    pub fn allowed_content_types(mut self, types: Vec<String>) -> Self {
        self.allowed_content_types = types;
        self
    }

    /// Add an allowed content type
    pub fn allow_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.allowed_content_types.push(content_type.into());
        self
    }
}

/// File data wrapper for convenient access to uploaded files
#[derive(Clone)]
pub struct UploadedFile {
    /// Original filename
    pub filename: String,
    /// Content type (MIME type)
    pub content_type: Option<String>,
    /// File data
    pub data: Bytes,
}

impl UploadedFile {
    /// Create from a multipart field
    pub fn from_field(field: &MultipartField) -> Option<Self> {
        field.file_name().map(|filename| Self {
            filename: filename.to_string(),
            content_type: field.content_type().map(|s| s.to_string()),
            data: field.data.clone(),
        })
    }

    /// Get file size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get file extension
    pub fn extension(&self) -> Option<&str> {
        self.filename.rsplit('.').next()
    }

    /// Save to disk with original filename
    pub async fn save_to(&self, dir: impl AsRef<Path>) -> Result<String> {
        let dir = dir.as_ref();
        
        tokio::fs::create_dir_all(dir).await.map_err(|e| {
            ApiError::internal(format!("Failed to create upload directory: {}", e))
        })?;

        let safe_filename = sanitize_filename(&self.filename);
        let file_path = dir.join(&safe_filename);

        tokio::fs::write(&file_path, &self.data).await.map_err(|e| {
            ApiError::internal(format!("Failed to save file: {}", e))
        })?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Save with a custom filename
    pub async fn save_as(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ApiError::internal(format!("Failed to create directory: {}", e))
            })?;
        }

        tokio::fs::write(path, &self.data).await.map_err(|e| {
            ApiError::internal(format!("Failed to save file: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_boundary() {
        let ct = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
        assert_eq!(
            extract_boundary(ct),
            Some("----WebKitFormBoundary7MA4YWxkTrZu0gW".to_string())
        );

        let ct_quoted = "multipart/form-data; boundary=\"----WebKitFormBoundary\"";
        assert_eq!(
            extract_boundary(ct_quoted),
            Some("----WebKitFormBoundary".to_string())
        );
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.txt"), "test.txt");
        assert_eq!(sanitize_filename("../../../etc/passwd"), "______etc_passwd");
        // ..\..\windows\system32 -> .._.._windows_system32 -> ____windows_system32
        assert_eq!(sanitize_filename("..\\..\\windows\\system32"), "____windows_system32");
        assert_eq!(sanitize_filename(".hidden"), "hidden");
    }

    #[test]
    fn test_parse_simple_multipart() {
        let boundary = "----WebKitFormBoundary";
        let body = format!(
            "------WebKitFormBoundary\r\n\
             Content-Disposition: form-data; name=\"field1\"\r\n\
             \r\n\
             value1\r\n\
             ------WebKitFormBoundary\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             file content\r\n\
             ------WebKitFormBoundary--\r\n"
        );

        let fields = parse_multipart(&Bytes::from(body), boundary).unwrap();
        assert_eq!(fields.len(), 2);

        assert_eq!(fields[0].name(), Some("field1"));
        assert!(!fields[0].is_file());

        assert_eq!(fields[1].name(), Some("file"));
        assert_eq!(fields[1].file_name(), Some("test.txt"));
        assert_eq!(fields[1].content_type(), Some("text/plain"));
        assert!(fields[1].is_file());
    }

    #[test]
    fn test_multipart_config() {
        let config = MultipartConfig::new()
            .max_size(20 * 1024 * 1024)
            .max_fields(50)
            .max_file_size(5 * 1024 * 1024)
            .allow_content_type("image/png")
            .allow_content_type("image/jpeg");

        assert_eq!(config.max_size, 20 * 1024 * 1024);
        assert_eq!(config.max_fields, 50);
        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
        assert_eq!(config.allowed_content_types.len(), 2);
    }
}
