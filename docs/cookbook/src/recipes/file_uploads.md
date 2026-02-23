# File Uploads

Handling file uploads is a common requirement. RustAPI provides a `Multipart` extractor to parse `multipart/form-data` requests.

## Dependencies

Add `uuid` and `tokio` with `fs` features to your `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = "0.1.335"
tokio = { version = "1", features = ["fs", "io-util"] }
uuid = { version = "1", features = ["v4"] }
```

## Basic File Upload

The `Multipart` extractor allows you to iterate over fields and save files to disk.

**⚠️ IMPORTANT: Buffering Warning**
RustAPI's `Multipart` extractor currently buffers the **entire request body into memory** before parsing. This makes it simple to use but unsuitable for very large files (e.g., video uploads > 100MB) on memory-constrained servers.
For large files, see the [Streaming & Large Files](#streaming--large-files) section below.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extract::{Multipart, DefaultBodyLimit};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure uploads directory exists
    tokio::fs::create_dir_all("./uploads").await?;

    println!("Starting Upload Server at http://127.0.0.1:8080");

    RustApi::new()
        // Increase body limit to 50MB (default is 2MB)
        .body_limit(50 * 1024 * 1024)
        .route("/upload", post(upload_handler))
        .run("127.0.0.1:8080")
        .await
}

#[derive(Serialize, Schema)]
struct UploadResponse {
    message: String,
    files: Vec<FileResult>,
}

#[derive(Serialize, Schema)]
struct FileResult {
    original_name: String,
    stored_name: String,
    content_type: String,
    size: usize,
}

async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    let mut uploaded_files = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::bad_request("Invalid multipart"))? {
        
        // Skip non-file fields
        if !field.is_file() {
            continue;
        }

        let file_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = field.bytes().await.map_err(|e| ApiError::internal(e.to_string()))?;

        // 1. Manual Validation
        if data.len() > 10 * 1024 * 1024 { // 10MB limit per file
            return Err(ApiError::bad_request(format!("File {} is too large", file_name)));
        }

        if !content_type.starts_with("image/") {
            return Err(ApiError::bad_request(format!("File {} is not an image", file_name)));
        }

        // 2. Generate Safe Filename
        // Never trust the user-provided filename directly!
        let safe_filename = format!("{}-{}", uuid::Uuid::new_v4(), file_name);
        let path = Path::new("./uploads").join(&safe_filename);

        // 3. Save to Disk
        tokio::fs::write(&path, &data).await.map_err(|e| ApiError::internal(e.to_string()))?;

        println!("Saved file: {} -> {:?}", file_name, path);

        uploaded_files.push(FileResult {
            original_name: file_name,
            stored_name: safe_filename,
            content_type,
            size: data.len(),
        });
    }

    Ok(Json(UploadResponse {
        message: "Upload successful".into(),
        files: uploaded_files,
    }))
}
```

## Manual Validation

The `Multipart` extractor returns raw fields, so you cannot use the `#[derive(Validate)]` macro directly on them. Instead, you must perform manual checks inside your handler loop:

1.  **File Size**: Check `data.len()` before saving.
2.  **Content Type**: Check `field.content_type()`. Note that this header is set by the client and can be spoofed.
3.  **Magic Bytes**: For true security, inspect the first few bytes of `data` to verify the file format (e.g., using the `infer` crate).

```rust
// Example using `infer` crate (add to Cargo.toml)
// let kind = infer::get(&data).ok_or(ApiError::bad_request("Unknown file type"))?;
// if kind.mime_type() != "image/png" { ... }
```

## Streaming & Large Files

Since `Multipart` buffers content, you might run out of memory with concurrent large uploads.

If you need to handle large files (e.g., 1GB+) or high concurrency:

1.  **Do NOT use `Multipart` extractor.**
2.  **Do NOT use `Json` or `Bytes` extractors.**
3.  You must implement a custom extractor or use `hyper::body::Body` directly (if exposed) to stream the request.

Currently, `rustapi-core` focuses on ergonomic, buffered access. For high-performance streaming uploads, consider handling the `Request<Body>` manually in a custom middleware or by bypassing the high-level router for specific paths.

### Configuration

You must explicitly increase the body limit for any upload route, as the default is 2MB to prevent DoS attacks.

```rust
RustApi::new()
    .body_limit(1024 * 1024 * 1024) // 1GB limit
    // ...
```

## Testing with cURL

```bash
curl -X POST http://localhost:8080/upload \
  -F "file1=@./image.png" \
  -F "file2=@./document.pdf"
```

Response:
```json
{
  "message": "Upload successful",
  "files": [
    {
      "original_name": "image.png",
      "stored_name": "550e8400-e29b-41d4-a716-446655440000-image.png",
      "content_type": "image/png",
      "size": 12345
    }
  ]
}
```
