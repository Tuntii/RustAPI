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
use crate::stream::StreamingBody;
use bytes::Bytes;
use futures_util::stream;
use http::StatusCode;
use std::error::Error as _;
use std::path::Path;
use tokio::io::AsyncWriteExt;

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

/// Streaming multipart extractor for large file uploads.
///
/// Unlike [`Multipart`], this extractor does not buffer the entire request body in memory before
/// parsing. It consumes the request body as a stream and yields one field at a time.
///
/// If a [`MultipartConfig`] is present in app state, its size and content-type limits are applied.
pub struct StreamingMultipart {
    inner: multer::Multipart<'static>,
    config: MultipartConfig,
    field_count: usize,
}

impl StreamingMultipart {
    fn new(stream: StreamingBody, boundary: String, config: MultipartConfig) -> Self {
        Self {
            inner: multer::Multipart::new(stream, boundary),
            config,
            field_count: 0,
        }
    }

    /// Get the next field from the multipart stream.
    ///
    /// Consume or drop the previously returned field before calling this again.
    pub async fn next_field(&mut self) -> Result<Option<StreamingMultipartField<'static>>> {
        let field = self.inner.next_field().await.map_err(map_multer_error)?;
        let Some(field) = field else {
            return Ok(None);
        };

        self.field_count += 1;
        if self.field_count > self.config.max_fields {
            return Err(ApiError::bad_request(format!(
                "Multipart field count exceeded limit of {}",
                self.config.max_fields
            )));
        }

        validate_streaming_field(&field, &self.config)?;

        Ok(Some(StreamingMultipartField::new(
            field,
            self.config.max_file_size,
        )))
    }

    /// Number of fields yielded so far.
    pub fn field_count(&self) -> usize {
        self.field_count
    }
}

impl FromRequest for StreamingMultipart {
    async fn from_request(req: &mut Request) -> Result<Self> {
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

        let boundary = extract_boundary(content_type)
            .ok_or_else(|| ApiError::bad_request("Missing boundary in Content-Type"))?;

        let config = req
            .state()
            .get::<MultipartConfig>()
            .cloned()
            .unwrap_or_default();

        let stream = request_body_stream(req, config.max_size)?;
        Ok(Self::new(stream, boundary, config))
    }
}

/// A single streaming field from a multipart form.
///
/// This field is one-shot: once you call [`chunk`](Self::chunk), [`bytes`](Self::bytes),
/// [`text`](Self::text), or one of the save helpers, the underlying stream is consumed.
pub struct StreamingMultipartField<'a> {
    inner: multer::Field<'a>,
    max_file_size: usize,
    bytes_read: usize,
}

impl<'a> StreamingMultipartField<'a> {
    fn new(inner: multer::Field<'a>, max_file_size: usize) -> Self {
        Self {
            inner,
            max_file_size,
            bytes_read: 0,
        }
    }

    /// Get the field name.
    pub fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    /// Get the original filename when this field is a file upload.
    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }

    /// Get the content type of the field.
    pub fn content_type(&self) -> Option<&str> {
        self.inner.content_type().map(|mime| mime.essence_str())
    }

    /// Check whether this field represents a file upload.
    pub fn is_file(&self) -> bool {
        self.file_name().is_some()
    }

    /// Number of bytes consumed from this field so far.
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    /// Read the next chunk from the field stream.
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        let chunk = self.inner.chunk().await.map_err(map_multer_error)?;
        let Some(chunk) = chunk else {
            return Ok(None);
        };

        self.bytes_read += chunk.len();
        if self.bytes_read > self.max_file_size {
            return Err(file_size_limit_error(self.max_file_size));
        }

        Ok(Some(chunk))
    }

    /// Collect the full field into memory.
    pub async fn bytes(&mut self) -> Result<Bytes> {
        let mut buffer = bytes::BytesMut::new();
        while let Some(chunk) = self.chunk().await? {
            buffer.extend_from_slice(&chunk);
        }
        Ok(buffer.freeze())
    }

    /// Collect the field as UTF-8 text.
    pub async fn text(&mut self) -> Result<String> {
        String::from_utf8(self.bytes().await?.to_vec())
            .map_err(|e| ApiError::bad_request(format!("Invalid UTF-8 in field: {}", e)))
    }

    /// Save the field to a directory using either the provided filename or the uploaded name.
    pub async fn save_to(
        &mut self,
        dir: impl AsRef<Path>,
        filename: Option<&str>,
    ) -> Result<String> {
        let dir = dir.as_ref();

        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create upload directory: {}", e)))?;

        let final_filename = filename
            .map(|value| value.to_string())
            .or_else(|| self.file_name().map(|value| value.to_string()))
            .ok_or_else(|| {
                ApiError::bad_request("No filename provided and field has no filename")
            })?;

        let safe_filename = sanitize_filename(&final_filename);
        let file_path = dir.join(&safe_filename);
        self.save_as(&file_path).await?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Save the field contents to an explicit file path without buffering the full file in memory.
    pub async fn save_as(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to create directory: {}", e)))?;
        }

        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create file: {}", e)))?;

        while let Some(chunk) = self.chunk().await? {
            file.write_all(&chunk)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;
        }

        file.flush()
            .await
            .map_err(|e| ApiError::internal(format!("Failed to flush file: {}", e)))?;

        Ok(())
    }

    /// Collect the field into an [`UploadedFile`] for APIs that still expect the buffered wrapper.
    pub async fn into_uploaded_file(mut self) -> Result<UploadedFile> {
        let filename = self
            .file_name()
            .ok_or_else(|| ApiError::bad_request("Field is not a file upload"))?
            .to_string();
        let content_type = self.content_type().map(|value| value.to_string());
        let data = self.bytes().await?;

        Ok(UploadedFile {
            filename,
            content_type,
            data,
        })
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
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create upload directory: {}", e)))?;

        // Determine filename
        let final_filename = filename
            .map(|s| s.to_string())
            .or_else(|| self.file_name.clone())
            .ok_or_else(|| {
                ApiError::bad_request("No filename provided and field has no filename")
            })?;

        // Sanitize filename to prevent path traversal
        let safe_filename = sanitize_filename(&final_filename);
        let file_path = dir.join(&safe_filename);

        // Write file
        tokio::fs::write(&file_path, &self.data)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;

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

fn request_body_stream(req: &mut Request, limit: usize) -> Result<StreamingBody> {
    if let Some(stream) = req.take_stream() {
        return Ok(StreamingBody::new(stream, Some(limit)));
    }

    if let Some(body) = req.take_body() {
        let stream = stream::once(async move { Ok::<Bytes, ApiError>(body) });
        return Ok(StreamingBody::from_stream(stream, Some(limit)));
    }

    Err(ApiError::internal("Body already consumed"))
}

fn validate_streaming_field(field: &multer::Field<'_>, config: &MultipartConfig) -> Result<()> {
    if field.file_name().is_none() || config.allowed_content_types.is_empty() {
        return Ok(());
    }

    let content_type = field
        .content_type()
        .map(|mime| mime.essence_str().to_string())
        .ok_or_else(|| ApiError::bad_request("Uploaded file is missing Content-Type"))?;

    if config
        .allowed_content_types
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&content_type))
    {
        return Ok(());
    }

    Err(ApiError::bad_request(format!(
        "Unsupported content type '{}'",
        content_type
    )))
}

fn file_size_limit_error(limit: usize) -> ApiError {
    ApiError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload_too_large",
        format!("Multipart field exceeded limit of {} bytes", limit),
    )
}

fn map_multer_error(error: multer::Error) -> ApiError {
    if let Some(source) = error.source() {
        if let Some(api_error) = source.downcast_ref::<ApiError>() {
            return api_error.clone();
        }
    }

    let message = error.to_string();
    if message.to_ascii_lowercase().contains("size limit") {
        return ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", message);
    }

    ApiError::bad_request(format!("Invalid multipart body: {}", message))
}

/// Extract boundary from Content-Type header
fn extract_boundary(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|part| {
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
        let body_section = &part[header_body_split..]
            .trim_start_matches("\r\n\r\n")
            .trim_start_matches("\n\n");

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

        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create upload directory: {}", e)))?;

        let safe_filename = sanitize_filename(&self.filename);
        let file_path = dir.join(&safe_filename);

        tokio::fs::write(&file_path, &self.data)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Save with a custom filename
    pub async fn save_as(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::internal(format!("Failed to create directory: {}", e)))?;
        }

        tokio::fs::write(path, &self.data)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to save file: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    fn chunked_body_stream(
        body: Bytes,
        chunk_size: usize,
    ) -> impl futures_util::Stream<Item = Result<Bytes>> + Send + 'static {
        let chunks = body
            .chunks(chunk_size)
            .map(Bytes::copy_from_slice)
            .map(Ok)
            .collect::<Vec<_>>();
        stream::iter(chunks)
    }

    fn streaming_multipart_from_body(
        body: Bytes,
        boundary: &str,
        config: MultipartConfig,
    ) -> StreamingMultipart {
        let stream =
            StreamingBody::from_stream(chunked_body_stream(body, 7), Some(config.max_size));
        StreamingMultipart::new(stream, boundary.to_string(), config)
    }

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
        assert_eq!(
            sanitize_filename("..\\..\\windows\\system32"),
            "____windows_system32"
        );
        assert_eq!(sanitize_filename(".hidden"), "hidden");
    }

    #[test]
    fn test_parse_simple_multipart() {
        let boundary = "----WebKitFormBoundary";
        let body = "------WebKitFormBoundary\r\n\
             Content-Disposition: form-data; name=\"field1\"\r\n\
             \r\n\
             value1\r\n\
             ------WebKitFormBoundary\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             file content\r\n\
             ------WebKitFormBoundary--\r\n"
            .to_string();

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

    #[tokio::test]
    async fn streaming_multipart_reads_chunked_body() {
        let boundary = "----RustApiBoundary";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"title\"\r\n\
             \r\n\
             hello\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"demo.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             streamed-content\r\n\
             --{boundary}--\r\n"
        );

        let mut multipart = streaming_multipart_from_body(
            Bytes::from(body),
            boundary,
            MultipartConfig::new().max_size(1024).max_file_size(1024),
        );

        let mut title = multipart.next_field().await.unwrap().unwrap();
        assert_eq!(title.name(), Some("title"));
        assert_eq!(title.text().await.unwrap(), "hello");
        drop(title);

        let mut file = multipart.next_field().await.unwrap().unwrap();
        assert_eq!(file.file_name(), Some("demo.txt"));
        assert_eq!(file.content_type(), Some("text/plain"));
        assert_eq!(file.bytes().await.unwrap(), Bytes::from("streamed-content"));
        drop(file);

        assert!(multipart.next_field().await.unwrap().is_none());
        assert_eq!(multipart.field_count(), 2);
    }

    #[tokio::test]
    async fn streaming_multipart_enforces_per_file_limit() {
        let boundary = "----RustApiBoundary";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"demo.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             way-too-large\r\n\
             --{boundary}--\r\n"
        );

        let mut multipart = streaming_multipart_from_body(
            Bytes::from(body),
            boundary,
            MultipartConfig::new().max_size(1024).max_file_size(4),
        );

        let mut file = multipart.next_field().await.unwrap().unwrap();
        let error = file.bytes().await.unwrap_err();
        assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(error.message.contains("4"));
    }

    #[tokio::test]
    async fn streaming_multipart_enforces_field_count_limit() {
        let boundary = "----RustApiBoundary";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"first\"\r\n\
             \r\n\
             one\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"second\"\r\n\
             \r\n\
             two\r\n\
             --{boundary}--\r\n"
        );

        let mut multipart = streaming_multipart_from_body(
            Bytes::from(body),
            boundary,
            MultipartConfig::new().max_size(1024).max_fields(1),
        );

        assert!(multipart.next_field().await.unwrap().is_some());
        let next = multipart.next_field().await;
        assert!(next.is_err());
        let error = next.err().unwrap();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(error.message.contains("field count exceeded"));
    }

    #[tokio::test]
    async fn streaming_multipart_save_to_writes_incrementally() {
        let boundary = "----RustApiBoundary";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"demo.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             persisted\r\n\
             --{boundary}--\r\n"
        );

        let mut multipart = streaming_multipart_from_body(
            Bytes::from(body),
            boundary,
            MultipartConfig::new().max_size(1024).max_file_size(1024),
        );

        let mut file = multipart.next_field().await.unwrap().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("rustapi-streaming-upload-{}", uuid::Uuid::new_v4()));
        let saved_path = file.save_to(&temp_dir, None).await.unwrap();
        let saved = tokio::fs::read_to_string(&saved_path).await.unwrap();

        assert_eq!(saved, "persisted");

        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }
}
