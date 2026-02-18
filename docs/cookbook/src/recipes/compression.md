# Response Compression

Response compression reduces payload size, improving load times for clients on slow networks. RustAPI provides a `CompressionLayer` that supports Gzip, Deflate, and optional Brotli compression.

## Dependencies

Enable the `compression` feature in `Cargo.toml`. For Brotli support, enable `compression-brotli`.

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["compression"] }
# For Brotli support:
# rustapi-rs = { version = "0.1", features = ["compression-brotli"] }
```

## Basic Usage

The easiest way to enable compression is via the `RustApi` builder.

```rust
use rustapi_rs::prelude::*;

#[tokio::main]
async fn main() {
    RustApi::new()
        // Enable default compression (Gzip/Deflate, Level 6, Min size 1KB)
        .compression()
        .route("/users", get(list_users))
        .run("127.0.0.1:8080")
        .await
        .unwrap();
}

async fn list_users() -> Json<Vec<String>> {
    // Large response will be compressed automatically
    Json(vec!["user1".to_string(); 1000])
}
```

## Custom Configuration

You can fine-tune the compression settings using `CompressionConfig`.

```rust
use rustapi_rs::prelude::*;
use rustapi_core::middleware::CompressionConfig;

#[tokio::main]
async fn main() {
    let config = CompressionConfig::new()
        .level(9)               // Maximum compression
        .min_size(512)          // Compress responses > 512 bytes
        .gzip(true)             // Enable Gzip
        .deflate(false)         // Disable Deflate
        .add_content_type("application/vnd.api+json"); // Add custom type

    RustApi::new()
        .compression_with_config(config)
        .route("/", get(handler))
        .run("127.0.0.1:8080")
        .await
        .unwrap();
}
```

## How It Works

The middleware:
1. Checks the `Accept-Encoding` header sent by the client.
2. Selects the best supported algorithm (Brotli > Gzip > Deflate).
3. Checks if the response `Content-Type` is compressible (e.g., JSON, HTML, XML).
4. Checks if the response body size exceeds `min_size`.
5. Compresses the body and sets `Content-Encoding` header.
6. Removes `Content-Length` header as it changes.

## Supported Algorithms

- **Gzip**: Standard, widely supported. Good balance of speed and ratio.
- **Deflate**: Slightly faster, less common.
- **Brotli**: (Optional) Better compression ratio than Gzip, but slower to compress. Requires `compression-brotli` feature.

## Pitfalls

- **Double Compression**: Do not compress already compressed data like images (JPEG, PNG) or archives (ZIP). The middleware excludes common image types by default, but be careful with custom binary formats.
- **BREACH Attack**: Compressing encrypted secrets (like CSRF tokens) in the same response as user-controlled data can lead to security vulnerabilities. Avoid compressing responses containing secrets.
