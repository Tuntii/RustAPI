# File Uploads

Handling file uploads efficiently is crucial. RustAPI allows you to stream `Multipart` data, meaning you can handle 1GB uploads without using 1GB of RAM.

## Dependencies

```toml
[dependencies]
rustapi-rs = "0.1.335"
tokio = { version = "1", features = ["fs", "io-util"] }
uuid = { version = "1", features = ["v4"] }
```

## Streaming Upload Handler

This handler reads the incoming stream part-by-part and writes it directly to disk (or S3).

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extract::Multipart;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

async fn upload_file(mut multipart: Multipart) -> Result<StatusCode, ApiError> {
    // Iterate over the fields
    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::BadRequest)? {
        
        let name = field.name().unwrap_or("file").to_string();
        let file_name = field.file_name().unwrap_or("unknown.bin").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        println!("Uploading: {} ({})", file_name, content_type);

        // Security: Create a safe random filename to prevent overwrites or path traversal
        let new_filename = format!("{}-{}", uuid::Uuid::new_v4(), file_name);
        let path = std::path::Path::new("./uploads").join(new_filename);

        // Open destination file
        let mut file = File::create(&path).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        // Write stream to file chunk by chunk
        let mut field_bytes = field; // field implements Stream itself (in some drivers) or we read chunks
        
        // In RustAPI/Axum multipart, `field.bytes()` loads the whole field into memory.
        // To stream, we use `field.chunk()`:
        
        while let Some(chunk) = field.chunk().await.map_err(|_| ApiError::BadRequest)? {
             file.write_all(&chunk).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        }
    }

    Ok(StatusCode::CREATED)
}
```

## Handling Constraints

You should always set limits to prevent DoS attacks.

```rust
use rustapi_rs::extract::DefaultBodyLimit;

let app = RustApi::new()
    .route("/upload", post(upload_file))
    // Limit request body to 10MB
    .layer(DefaultBodyLimit::max(10 * 1024 * 1024));
```

## Validating Content Type

Never trust the `Content-Type` header sent by the client implicitly for security (e.g., executing a PHP script uploaded as an image).

Verify the "magic bytes" of the file content itself if strictly needed, or ensure uploaded files are stored in a non-executable directory (or S3 bucket).

```rust
// Simple check on the header (not fully secure but good UX)
if let Some(ct) = field.content_type() {
    if !ct.starts_with("image/") {
        return Err(ApiError::BadRequest("Only images are allowed".into()));
    }
}
```
