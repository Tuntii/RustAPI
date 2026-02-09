# Replay: Time-Travel Debugging

Record HTTP request/response pairs and replay them against different environments for debugging and regression testing.

> **Security Notice**: The replay system is designed for **development and staging environments only**. See [Security](#security) for details.

## Quick Start

Add the `replay` feature to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["replay"] }
```

Add the `ReplayLayer` middleware to your application:

```rust,ignore
use rustapi_rs::prelude::*;
use rustapi_rs::replay::{ReplayLayer, InMemoryReplayStore};
use rustapi_core::replay::ReplayConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let replay = ReplayLayer::new(
        ReplayConfig::new()
            .enabled(true)
            .admin_token("my-secret-token")
            .ttl_secs(3600)
    );

    RustApi::new()
        .layer(replay)
        .route("/api/users", get(list_users))
        .run("127.0.0.1:8080")
        .await
}

async fn list_users() -> Json<Vec<String>> {
    Json(vec!["Alice".into(), "Bob".into()])
}
```

## How It Works

1. **Record**: The `ReplayLayer` middleware captures HTTP request/response pairs as they flow through your application
2. **List**: Query recorded entries via the admin API or CLI
3. **Replay**: Re-send a recorded request against any target URL
4. **Diff**: Compare the replayed response against the original to detect regressions

## Admin API

All admin endpoints require a bearer token in the `Authorization` header:

```
Authorization: Bearer <admin_token>
```

| Method | Path | Description |
|--------|------|-------------|
| GET | `/__rustapi/replays` | List recorded entries |
| GET | `/__rustapi/replays/{id}` | Show a single entry |
| POST | `/__rustapi/replays/{id}/run?target=URL` | Replay against target |
| POST | `/__rustapi/replays/{id}/diff?target=URL` | Replay and compute diff |
| DELETE | `/__rustapi/replays/{id}` | Delete an entry |

### Query Parameters for List

- `limit` - Maximum number of entries to return
- `method` - Filter by HTTP method (GET, POST, etc.)
- `path` - Filter by path substring
- `status_min` - Minimum status code filter

### Example: cURL

```bash
# List entries
curl -H "Authorization: Bearer my-secret-token" \
     http://localhost:8080/__rustapi/replays?limit=10

# Show a specific entry
curl -H "Authorization: Bearer my-secret-token" \
     http://localhost:8080/__rustapi/replays/<id>

# Replay against staging
curl -X POST -H "Authorization: Bearer my-secret-token" \
     "http://localhost:8080/__rustapi/replays/<id>/run?target=http://staging:8080"

# Replay and diff
curl -X POST -H "Authorization: Bearer my-secret-token" \
     "http://localhost:8080/__rustapi/replays/<id>/diff?target=http://staging:8080"
```

## CLI Usage

Install with the `replay` feature:

```bash
cargo install cargo-rustapi --features replay
```

### Commands

```bash
# List recorded entries
cargo rustapi replay list -s http://localhost:8080 -t my-secret-token

# List with filters
cargo rustapi replay list -t my-secret-token --method GET --limit 20

# Show entry details
cargo rustapi replay show <id> -t my-secret-token

# Replay against a target URL
cargo rustapi replay run <id> -T http://staging:8080 -t my-secret-token

# Replay and diff
cargo rustapi replay diff <id> -T http://staging:8080 -t my-secret-token
```

The `--token` (`-t`) parameter can also be set via the `RUSTAPI_REPLAY_TOKEN` environment variable:

```bash
export RUSTAPI_REPLAY_TOKEN=my-secret-token
cargo rustapi replay list
```

## Configuration

### ReplayConfig

```rust,ignore
use rustapi_core::replay::ReplayConfig;

let config = ReplayConfig::new()
    // Enable recording (default: false)
    .enabled(true)
    // Required: admin bearer token
    .admin_token("my-secret-token")
    // Max entries in store (default: 500)
    .store_capacity(1000)
    // Entry TTL in seconds (default: 3600 = 1 hour)
    .ttl_secs(7200)
    // Sampling rate 0.0-1.0 (default: 1.0 = all requests)
    .sample_rate(0.5)
    // Max request body capture size (default: 64KB)
    .max_request_body(131_072)
    // Max response body capture size (default: 256KB)
    .max_response_body(524_288)
    // Only record specific paths
    .record_path("/api/users")
    .record_path("/api/orders")
    // Or skip specific paths
    .skip_path("/health")
    .skip_path("/metrics")
    // Add headers to redact
    .redact_header("x-custom-secret")
    // Add body fields to redact
    .redact_body_field("password")
    .redact_body_field("ssn")
    .redact_body_field("credit_card")
    // Custom admin route prefix (default: "/__rustapi/replays")
    .admin_route_prefix("/__admin/replays");
```

### Default Redacted Headers

The following headers are redacted by default (values replaced with `[REDACTED]`):

- `authorization`
- `cookie`
- `x-api-key`
- `x-auth-token`

### Body Field Redaction

JSON body fields are recursively redacted. For example, with `.redact_body_field("password")`:

```json
// Before redaction
{"user": {"name": "alice", "password": "secret123"}}

// After redaction
{"user": {"name": "alice", "password": "[REDACTED]"}}
```

## Custom Store

### File-System Store

For persistent storage across restarts:

```rust,ignore
use rustapi_rs::replay::{ReplayLayer, FsReplayStore, FsReplayStoreConfig};
use rustapi_core::replay::ReplayConfig;

let config = ReplayConfig::new()
    .enabled(true)
    .admin_token("my-secret-token");

let fs_store = FsReplayStore::new(FsReplayStoreConfig {
    directory: "./replay-data".into(),
    max_file_size: Some(10 * 1024 * 1024), // 10MB per file
    create_if_missing: true,
});

let layer = ReplayLayer::new(config).with_store(fs_store);
```

### Implementing a Custom Store

Implement the `ReplayStore` trait for custom backends (Redis, database, etc.):

```rust,ignore
use async_trait::async_trait;
use rustapi_core::replay::{
    ReplayEntry, ReplayQuery, ReplayStore, ReplayStoreResult,
};

struct MyCustomStore {
    // your fields
}

#[async_trait]
impl ReplayStore for MyCustomStore {
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()> {
        // Store the entry
        Ok(())
    }

    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>> {
        // Retrieve by ID
        Ok(None)
    }

    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>> {
        // List with filtering
        Ok(vec![])
    }

    async fn delete(&self, id: &str) -> ReplayStoreResult<bool> {
        // Delete by ID
        Ok(false)
    }

    async fn count(&self) -> ReplayStoreResult<usize> {
        Ok(0)
    }

    async fn clear(&self) -> ReplayStoreResult<()> {
        Ok(())
    }

    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize> {
        // Delete entries older than timestamp
        Ok(0)
    }

    fn clone_store(&self) -> Box<dyn ReplayStore> {
        Box::new(self.clone())
    }
}
```

## Security

The replay system has multiple security layers built in:

1. **Disabled by default**: Recording is off (`enabled: false`) until explicitly enabled
2. **Admin token required**: All `/__rustapi/replays` endpoints require a valid bearer token. Requests without the token get a `401 Unauthorized` response
3. **Header redaction**: `authorization`, `cookie`, `x-api-key`, and `x-auth-token` values are replaced with `[REDACTED]` before storage
4. **Body field redaction**: Sensitive JSON fields (e.g., `password`, `ssn`) can be configured for redaction
5. **TTL enforcement**: Entries are automatically deleted after the configured TTL (default: 1 hour)
6. **Body size limits**: Request (64KB) and response (256KB) bodies are truncated to prevent memory issues
7. **Bounded storage**: The in-memory store uses a ring buffer with FIFO eviction

**Recommendations**:

- Use only in development/staging environments
- Use a strong, unique admin token
- Keep TTL short
- Add application-specific sensitive fields to the redaction list
- Monitor memory usage when using the in-memory store with large capacity values
