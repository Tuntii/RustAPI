# File Uploads

Handling file uploads is a common requirement. RustAPI provides a `Multipart` extractor to parse `multipart/form-data` requests.

See the runnable example: `crates/rustapi-rs/examples/file_upload.rs`

```bash
cargo run -p rustapi-rs --example file_upload
curl -X POST http://127.0.0.1:8080/upload -F "file=@./README.md"
```

## Dependencies

```toml
[dependencies]
rustapi-rs = "0.2.0"
tokio = { version = "1", features = ["fs", "io-util", "macros", "rt-multi-thread"] }
```

## Buffered Upload Example

RustAPI's `Multipart` extractor buffers the entire request body into memory before parsing. It suits small to medium uploads; set explicit body limits for larger files.

```rust
use rustapi_rs::prelude::*;
use std::path::PathBuf;

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
    size_bytes: usize,
}

#[rustapi_rs::post("/upload")]
async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    tokio::fs::create_dir_all("./uploads").await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let mut uploaded_files = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("Invalid multipart"))?
    {
        if !field.is_file() {
            continue;
        }

        let original_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let safe_filename = format!("upload-{}", original_name);
        let data = field.bytes().await.map_err(|e| ApiError::internal(e.to_string()))?;
        let size_bytes = data.len();

        let path = PathBuf::from("./uploads").join(&safe_filename);
        tokio::fs::write(&path, &data).await.map_err(|e| ApiError::internal(e.to_string()))?;

        uploaded_files.push(FileResult {
            original_name,
            stored_name: safe_filename,
            content_type,
            size_bytes,
        });
    }

    Ok(Json(UploadResponse {
        message: "Upload successful".into(),
        files: uploaded_files,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .body_limit(50 * 1024 * 1024)
        .layer(BodyLimitLayer::new(50 * 1024 * 1024))
        .route("/upload", post(upload_handler))
        .docs("/docs")
        .run("127.0.0.1:8080")
        .await
}
```

## Key Concepts

### 1. Buffering

RustAPI loads the entire `multipart/form-data` body into memory.

- **Pros**: Simple API, easy to work with.
- **Cons**: High memory usage for concurrent large uploads.
- **Mitigation**: Set a reasonable `BodyLimitLayer` (e.g., 10MB–100MB).

### 2. Body Limits

The default request body limit is 1MB. Increase it for upload routes via `.body_limit(...)` and/or `.layer(BodyLimitLayer::new(size))`.

### 3. Security

- **Path Traversal**: Never trust client-provided filenames. Generate safe names or sanitize strictly.
- **Content Type**: Client-controlled; do not rely on it for execution checks.
- **Executable Permissions**: Store uploads where script execution is disabled.

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
      "stored_name": "upload-image.png",
      "content_type": "image/png",
      "size_bytes": 12345
    }
  ]
}
```