# Structured Logging & Tracing

RustAPI provides a powerful structured logging system via `rustapi-extras` that integrates with the `tracing` ecosystem. This allows you to output logs in various formats (JSON, Datadog, Splunk) with correlation IDs, request timing, and automatic field redaction.

## Dependencies

Ensure `rustapi-extras` with the `structured-logging` feature is enabled.

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["extras-structured-logging"] }
# OR
rustapi-extras = { version = "0.1.335", features = ["structured-logging"] }
```

## Basic Usage

To enable structured logging, add the `StructuredLoggingLayer` to your application.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::structured_logging::{StructuredLoggingConfig, StructuredLoggingLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Create configuration (default is JSON format)
    let config = StructuredLoggingConfig::default();

    // 2. Add the layer to your app
    RustApi::new()
        .layer(StructuredLoggingLayer::new(config))
        .route("/", get(handler))
        .run("127.0.0.1:8080")
        .await
}

async fn handler() -> &'static str {
    // These logs will be formatted as JSON
    tracing::info!("Handling request");
    "Hello World"
}
```

## Configuration Presets

The configuration builder provides several presets for common environments.

### Development (Pretty Printing)
Useful for local debugging with human-readable logs.

```rust
let config = StructuredLoggingConfig::development();
// Outputs: "2023-10-01T12:00:00Z INFO [rustapi] Request completed path=/ status=200 duration_ms=2"
```

### Production (JSON)
Optimized for log aggregators like ELK, CloudWatch, etc.

```rust
let config = StructuredLoggingConfig::production_json();
// Outputs: {"timestamp":"...","level":"INFO","message":"Request completed","path":"/","status":200,"duration_ms":2}
```

### Datadog
Formats logs specifically for Datadog ingestion.

```rust
let config = StructuredLoggingConfig::datadog();
```

## Advanced Configuration

You can customize almost every aspect of the logging behavior using the builder.

```rust
use rustapi_rs::extras::structured_logging::{StructuredLoggingConfig, LogOutputFormat};

let config = StructuredLoggingConfig::builder()
    .format(LogOutputFormat::Json)
    .service_name("payment-service")
    .service_version("1.2.0")
    .environment("production")

    // Include extra request details
    .include_request_headers(true)
    .include_caller_info(true) // File and line number

    // Redact sensitive headers
    .redact_header("x-api-key")
    .redact_header("authorization")

    // Add static fields to every log
    .static_field("region", "us-east-1")

    // Correlation ID
    .correlation_id_header("x-request-id")
    .generate_correlation_id(true)

    .build();
```

## Key Features

### Correlation IDs
The layer automatically extracts a correlation ID from the request header (default `x-correlation-id`) or generates a new UUID if missing. This ID is attached to every log message generated within the request scope, allowing you to trace a request across microservices.

### Field Redaction
Security is critical. The logger automatically redacts sensitive headers like `Authorization`, `Cookie`, `X-Api-Key` by default. You can add more headers to the redaction list via configuration.

### Performance
The logging layer uses the `tracing` ecosystem which is designed for high performance. JSON serialization is handled efficiently. However, enabling `include_request_body` or `include_response_body` can have a performance impact and should generally be avoided in production unless debugging.

## Integration with OpenTelemetry

Structured logging works well alongside OpenTelemetry (Otel). While structured logging handles the "logs" signal, Otel handles "traces" and "metrics". You can enable both `extras-structured-logging` and `extras-otel` features for a complete observability stack.
