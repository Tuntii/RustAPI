# RustAPI Release History

## v0.1.275 - Time-Travel Debugging (2026-02-06)

### 🎯 Major Feature: Replay System

Introducing **Replay** - a comprehensive time-travel debugging system that records HTTP request/response pairs and allows you to replay them against different environments for debugging and regression testing.

#### Core Capabilities

- **Automatic Recording**: `ReplayLayer` middleware captures all HTTP traffic
- **Flexible Storage**: In-memory (dev) and filesystem (production) stores
- **Smart Diffing**: JSON-aware diff algorithm that highlights actual changes
- **Security First**: Disabled by default, admin token auth, automatic sensitive data redaction
- **CLI Tooling**: Full `cargo rustapi replay` command suite
- **Retention Management**: Automatic cleanup with configurable TTL

#### Architecture

```
rustapi-core       → Pure types & traits (no IO)
rustapi-extras     → Middleware, stores, HTTP routes
cargo-rustapi      → CLI commands
```

#### Quick Example

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::replay::{ReplayLayer, InMemoryReplayStore};
use rustapi_core::replay::ReplayConfig;

let replay = ReplayLayer::new(
    ReplayConfig::new()
        .enabled(true)
        .admin_token("secret")
        .ttl_secs(3600)
);

RustApi::new()
    .layer(replay)
    .route("/api/users", get(list_users))
    .run("127.0.0.1:8080")
    .await
```

#### Admin API Endpoints

- `GET /__rustapi/replays` - List entries
- `GET /__rustapi/replays/{id}` - Show details
- `POST /__rustapi/replays/{id}/run?target=URL` - Replay request
- `POST /__rustapi/replays/{id}/diff?target=URL` - Compare responses
- `DELETE /__rustapi/replays/{id}` - Delete entry

#### CLI Commands

**Installation** (requires `replay` feature):
```bash
cargo install cargo-rustapi --features replay
```

**Usage**:
```bash
cargo rustapi replay list
cargo rustapi replay show <id>
cargo rustapi replay run <id> --target http://staging:8080
cargo rustapi replay diff <id> --target http://staging:8080
cargo rustapi replay delete <id>
```

### 📦 What's Changed

- **28 files changed**: 4,113 insertions across rustapi-core, rustapi-extras, and cargo-rustapi
- Comprehensive cookbook recipe with security guidelines
- Full test coverage for all replay components

### 🔧 Improvements

- Fixed broken intra-doc link in replay module documentation
- Removed unused `check_diff.py` script

### 🎓 Learn More

- [Cookbook: Replay Recipe](docs/cookbook/src/recipes/replay.md)
- [PR #98: Time-Travel Debugging](https://github.com/Tuntii/RustAPI/pull/98)

---

## v0.1.202 - Performance Revolution (2026-01-26)

### 🚀 Performance Improvements

This release delivers a **12x performance improvement**, bringing RustAPI from ~8K req/s to **~92K req/s** - now within striking distance of Actix-web.

#### Benchmark Results

| Framework | Requests/sec | Latency (avg) |
|-----------|-------------|---------------|
| **RustAPI** | ~92,000 | ~1.1ms |
| Actix-web 4 | ~105,000 | ~0.95ms |
| Axum | ~100,000 | ~1.0ms |

*Tested with `hey -n 100000 -c 100` on Windows 11, Ryzen 9 5900X*

### ✨ Server Optimizations

- **TCP_NODELAY**: Disabled Nagle's algorithm for lower latency
- **Pipeline Flush**: Enabled HTTP/1.1 pipeline flushing for better throughput
- **ConnectionService**: Reduced Arc cloning overhead per connection
- **HandleRequestFuture**: Custom future implementation for request handling
- **Ultra-Fast Path**: New routing path that bypasses both middleware AND interceptors for maximum performance

### 📦 JSON Optimizations

- **simd-json Serialization**: Extended simd-json support from parsing-only to full serialization
- Added `to_vec` and `to_vec_with_capacity` using simd-json when feature is enabled

### 🔧 Build Profile Optimizations

```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
panic = "abort"
strip = true
```

### 📚 Documentation

- Updated README.md with accurate benchmark numbers
- Removed inflated performance claims
- Added TechEmpower-based comparison data

### 🧹 Cleanup

- Removed unused static variables from bench_server
- Code formatted with `cargo fmt --all`

---

## v0.1.201 - Previous Release

*See CHANGELOG.md for historical releases*

---

## Performance Roadmap

For planned optimizations to reach and exceed Actix performance, see [BEAT_ACTIX_ROADMAP.md](memories/BEAT_ACTIX_ROADMAP.md).

**Target: 105-115K req/s** through:
- Stack-allocated futures (remove Box::pin)
- Zero-copy path handling
- Pre-compiled middleware stack
- Response header pooling
