# Response Compression

RustAPI supports automatic response compression (Gzip, Deflate, Brotli) via the `CompressionLayer`. This middleware negotiates the best compression algorithm based on the client's `Accept-Encoding` header.

## Dependencies

To use compression, you must enable the `compression` feature in `rustapi-core` (or `rustapi-rs`). For Brotli support, enable `compression-brotli`.

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["compression", "compression-brotli"] }
```

## Basic Usage

The simplest way to enable compression is to add the layer to your application:

```rust
use rustapi_rs::prelude::*;
use rustapi_core::middleware::CompressionLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .layer(CompressionLayer::new())
        .route("/", get(hello))
        .run("127.0.0.1:8080")
        .await
}

async fn hello() -> &'static str {
    "Hello, World! This response will be compressed if the client supports it."
}
```

## Configuration

You can customize the compression behavior using `CompressionConfig`:

```rust
use rustapi_rs::prelude::*;
use rustapi_core::middleware::{CompressionLayer, CompressionConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = CompressionConfig::new()
        .min_size(1024)       // Only compress responses larger than 1KB
        .level(6)             // Compression level (0-9)
        .gzip(true)           // Enable Gzip
        .deflate(false)       // Disable Deflate
        .brotli(true)         // Enable Brotli (if feature enabled)
        .add_content_type("application/custom-json"); // Add custom type

    RustApi::new()
        .layer(CompressionLayer::with_config(config))
        .route("/data", get(get_large_data))
        .run("127.0.0.1:8080")
        .await
}
```

## Default Configuration

By default, `CompressionLayer` is configured with:
- `min_size`: 1024 bytes (1KB)
- `level`: 6
- `gzip`: enabled
- `deflate`: enabled
- `brotli`: enabled (if feature is present)
- `content_types`: `text/*`, `application/json`, `application/javascript`, `application/xml`, `image/svg+xml`

## Best Practices

### 1. Don't Compress Already Compressed Data
Images (JPEG, PNG), Videos, and Archives (ZIP) are already compressed. Compressing them again wastes CPU cycles and might even increase the file size. The default configuration excludes most binary formats, but be careful with custom types.

### 2. Set Minimum Size
Compressing very small responses (e.g., "OK") can actually make them larger due to framing overhead. The default 1KB threshold is a good starting point.

### 3. Order of Middleware
Compression should usually be one of the *last* layers added (outermost), so it compresses the final response after other middleware (like logging or headers) have run.

```rust
RustApi::new()
    .layer(CompressionLayer::new()) // Runs last on response (first on request)
    .layer(LoggingLayer::new())     // Runs before compression on response
```
