# RustAPI Release History

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
