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

## Buffered Upload Example

RustAPI's `Multipart` extractor currently buffers the entire request body into memory before parsing. This means it is suitable for small to medium file uploads (e.g., images, documents) but care must be taken with very large files to avoid running out of RAM.

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
        // Increase body limit to 1GB (default is usually 1MB)
        .body_limit(1024 * 1024 * 1024)
        .route("/upload", post(upload_handler))
        // Increase body limit to 50MB (default is usually 2MB)
        // ⚠️ IMPORTANT: Since Multipart buffers the whole body,
        // setting this too high can exhaust server memory.
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
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
}

async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    let mut uploaded_files = Vec::new();

    // Iterate over the fields in the multipart form
    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::bad_request("Invalid multipart"))? {
        
        // Skip fields that are not files
        if !field.is_file() {
            continue;
        }

        let file_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        // ⚠️ Security: Never trust the user-provided filename directly!
        // It could contain paths like "../../../etc/passwd".
        // Always generate a safe filename or sanitize inputs.
        let safe_filename = format!("{}-{}", uuid::Uuid::new_v4(), sanitize_filename(&file_name));

        // Option 1: Use the helper method (sanitizes filename automatically)
        // field.save_to("./uploads", Some(&safe_filename)).await.map_err(|e| ApiError::internal(e.to_string()))?;

        // Option 2: Manual write (gives you full control)
        let data = field.bytes().await.map_err(|e| ApiError::internal(e.to_string()))?;
        let path = Path::new("./uploads").join(&safe_filename);

        tokio::fs::write(&path, &data).await.map_err(|e| ApiError::internal(e.to_string()))?;

        println!("Saved file: {} -> {:?}", file_name, path);

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

fn sanitize_filename(name: &str) -> String {
    std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}
```

## Key Concepts

### 1. Buffering
RustAPI loads the entire `multipart/form-data` body into memory.
- **Pros**: Simple API, easy to work with.
- **Cons**: High memory usage for concurrent large uploads.
- **Mitigation**: Set a reasonable `DefaultBodyLimit` (e.g., 10MB - 100MB) to prevent DoS attacks.

### 2. Body Limits
The default request body limit is small (2MB) to prevent attacks. You **must** explicitly increase this limit for file upload routes using `.layer(DefaultBodyLimit::max(size_in_bytes))`.

### 3. Security
- **Path Traversal**: Malicious users can send filenames like `../../system32/cmd.exe`. Always rename files or sanitize filenames strictly.
- **Content Type Validation**: The `Content-Type` header is client-controlled and can be spoofed. Do not rely on it for security execution checks (e.g., preventing `.php` execution).
- **Executable Permissions**: Store uploads in a directory where script execution is disabled.

## Handling Large Files (Streaming)

If you need to handle files larger than available RAM (e.g., video uploads), the built-in `Multipart` extractor is not suitable because it buffers the entire body.

Instead, you should access the request body stream directly and use a streaming multipart parser like `multer`.

### Alternative: Using `multer`

Add `multer` and `futures-util` to your dependencies:

```toml
[dependencies]
multer = "3.0"
futures-util = "0.3" # Required for StreamExt
uuid = { version = "1", features = ["v4"] }
```

Then, create a handler that consumes the body stream:

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::http::Request;
use rustapi_rs::body::Body;
use futures_util::stream::StreamExt;
use multer::Multipart;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[rustapi_rs::post("/upload/stream")]
async fn stream_upload(req: Request<Body>) -> Result<String, ApiError> {
    // 1. Extract the boundary from the Content-Type header
    let boundary = req
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .and_then(|ct| multer::parse_boundary(ct).ok())
        .ok_or_else(|| ApiError::bad_request("Missing multipart boundary"))?;

    // 2. Create a multer instance
    let mut multipart = Multipart::new(req.into_body(), boundary);

    // 3. Iterate over fields
    while let Some(mut field) = multipart.next_field().await.map_err(|e| ApiError::bad_request(e.to_string()))? {
        let file_name = field.file_name().map(|s| s.to_string());

        if let Some(filename) = file_name {
             // ⚠️ Security: Sanitize filename to prevent path traversal!
             let safe_filename = format!("{}-{}", Uuid::new_v4(), sanitize_filename(&filename));
             let path = std::path::Path::new("./uploads").join(&safe_filename);

             let mut file = File::create(&path).await.map_err(|e| ApiError::internal(e.to_string()))?;

             // Stream chunks to disk
             while let Some(chunk) = field.chunk().await.map_err(|e| ApiError::internal(e.to_string()))? {
                 file.write_all(&chunk).await.map_err(|e| ApiError::internal(e.to_string()))?;
             }
             println!("Saved stream: {:?}", path);
        }
    }

    Ok("Stream upload complete".to_string())
}

// Helper to strip directory components from filename
fn sanitize_filename(name: &str) -> String {
    std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}
```

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
