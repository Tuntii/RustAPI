# File Uploads

Handling file uploads efficiently is crucial for modern applications. RustAPI provides a `Multipart` extractor that allows you to stream uploads, enabling you to handle large files (e.g., 1GB+) without consuming proportional RAM.

## Dependencies

Add `uuid` and `tokio` with `fs` features to your `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = "0.1.335"
tokio = { version = "1", features = ["fs", "io-util"] }
uuid = { version = "1", features = ["v4"] }
```

## Streaming Upload Example

Here is a complete, runnable example of a file upload server that streams files to a `./uploads` directory.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extract::{Multipart, DefaultBodyLimit};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure uploads directory exists
    tokio::fs::create_dir_all("./uploads").await?;

    println!("Starting Upload Server at http://127.0.0.1:8080");

    RustApi::new()
        .route("/upload", post(upload_handler))
        // Increase body limit to 1GB (default is usually 2MB)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024))
        .run("127.0.0.1:8080")
        .await
}

async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    let mut uploaded_files = Vec::new();

    // Iterate over the fields in the multipart form
    while let Some(mut field) = multipart.next_field().await.map_err(|_| ApiError::bad_request("Invalid multipart"))? {
        
        let file_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        // ⚠️ Security: Never trust the user-provided filename directly!
        // It could contain paths like "../../../etc/passwd".
        // Always generate a safe filename or sanitize inputs.
        let safe_filename = format!("{}-{}", uuid::Uuid::new_v4(), file_name);
        let path = Path::new("./uploads").join(&safe_filename);

        println!("Streaming file: {} -> {:?}", file_name, path);

        // Open destination file
        let mut file = File::create(&path).await.map_err(|e| ApiError::internal(e.to_string()))?;

        // Stream the field content chunk-by-chunk
        // This is memory efficient even for large files.
        while let Some(chunk) = field.chunk().await.map_err(|_| ApiError::bad_request("Stream error"))? {
             file.write_all(&chunk).await.map_err(|e| ApiError::internal(e.to_string()))?;
        }

        uploaded_files.push(FileResult {
            original_name: file_name,
            stored_name: safe_filename,
            content_type,
        });
    }

    Ok(Json(UploadResponse {
        message: "Upload successful".into(),
        files: uploaded_files,
    }))
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
}
```

## Key Concepts

### 1. Streaming vs Buffering
By default, some frameworks load the entire file into RAM. RustAPI's `Multipart` allows you to process the stream incrementally using `field.chunk()`.
- **Buffering**: `field.bytes().await` (Load all into RAM - simple but dangerous for large files)
- **Streaming**: `field.chunk().await` (Load small chunks - scalable)

### 2. Body Limits
The default request body limit is often small (e.g., 2MB) to prevent DoS attacks. You must explicitly increase this limit for file upload routes using `DefaultBodyLimit::max(size)`.

### 3. Security
- **Path Traversal**: Malicious users can send filenames like `../../system32/cmd.exe`. Always rename files or sanitize filenames strictly.
- **Content Type Validation**: The `Content-Type` header is client-controlled and can be spoofed. Do not rely on it for security execution checks (e.g., preventing `.php` execution).
- **Executable Permissions**: Store uploads in a directory where script execution is disabled.

## Testing with cURL

You can test this endpoint using `curl`:

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
      "content_type": "image/png"
    },
    ...
  ]
}
```
