# HTTP/3 (QUIC) Support

RustAPI supports HTTP/3 (QUIC), the next generation of the HTTP protocol, providing lower latency, better performance over unstable networks, and improved security.

## Enabling HTTP/3

HTTP/3 support is optional and can be enabled via feature flags in `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = { version = "0.1.275", features = ["http3"] }
# For development with self-signed certificates
rustapi-rs = { version = "0.1.275", features = ["http3", "http3-dev"] }
```

## Running an HTTP/3 Server

Since HTTP/3 requires TLS (even for local development), RustAPI provides helpers to make this easy.

### Development (Self-Signed Certs)

For local development, you can use `run_http3_dev` which automatically generates self-signed certificates.

```rust,no_run
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/")]
async fn hello() -> &'static str {
    "Hello from HTTP/3!"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Requires "http3-dev" feature
    RustApi::auto()
        .run_http3_dev("127.0.0.1:8080")
        .await
}
```

### Production (QUIC)

For production, you should provide valid certificates.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_core::http3::Http3Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Http3Config::new("cert.pem", "key.pem");
    
    RustApi::auto()
        .run_http3(config)
        .await
}
```

### Dual Stack (HTTP/1.1 + HTTP/3)

You can serve both HTTP/1.1 and HTTP/3 on the same port (via Alt-Svc header promotion) or different ports.

```rust,no_run
use rustapi_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Run HTTP/1.1 on port 8080 and HTTP/3 on port 4433 (or same port if supported)
    RustApi::auto()
        .run_dual_stack("127.0.0.1:8080")
        .await
}
```

## How It Works

HTTP/3 in RustAPI is built on top of `quinn` and `h3`. When enabled:

1.  **UDP Binding**: The server binds to a UDP socket (in addition to TCP if dual-stack).
2.  **TLS**: QUIC requires TLS 1.3. RustAPI handles the TLS configuration.
3.  **Optimization**: Responses are optimized for QUIC streams.

## Testing

You can test HTTP/3 support using `curl` with HTTP/3 support:

```bash
curl --http3 -k https://localhost:8080/
```

Or using online tools like [http3check.net](https://http3check.net/).
