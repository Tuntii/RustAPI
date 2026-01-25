# Performance Philosophy

RustAPI is built on a simple premise: **Abstractions shouldn't cost you runtime performance.**

We leverage Rust's unique ownership system and modern async ecosystem (Tokio, Hyper) to deliver performance that rivals C++ servers, while maintaining developer safe-guards.

## The Pillars of Speed

### 1. Zero-Copy Networking
Where possible, RustAPI avoids copying memory. When you receive a large JSON payload or file upload, we aim to pass pointers to the underlying memory buffer rather than cloning the data.

- **`Bytes` over `Vec<u8>`**: We use the `bytes` crate extensively. Passing a `Bytes` object around is `O(1)` (it's just a reference-counted pointer and length), whereas cloning a `Vec<u8>` is `O(n)`.
- **String View**: Extractors like `Path` and `Query` often leverage `Cow<'str, str>` (Clone on Write) to avoid allocations if the data doesn't need to be modified.

### 2. Multi-Core Async Runtime
RustAPI runs on **Tokio**, a work-stealing, multi-threaded runtime.

- **Non-blocking I/O**: A single thread can handle thousands of concurrent idle connections (e.g., WebSockets waiting for messages) with minimal memory overhead.
- **Work Stealing**: If one CPU core is overloaded with tasks, other idle cores will "steal" work from its queue, ensuring balanced utilization of your hardware.

### 3. Compile-Time Router
Our router (`matchit`) is based on a **Radix Trie** structure.

- **O(log n) Lookup**: Route matching speed depends on the length of the URL, *not* the number of routes defined. Having 10 routes or 10,000 routes has negligible impact on routing latency.
- **Allocation-Free Matching**: For standard paths, routing decisions happen without heap allocations.

## Memory Management

### Stack vs. Heap
RustAPI encourages stack allocation for small, short-lived data.
- **Extractors** are often allocated on the stack.
- **Response bodies** are streamed, meaning a 1GB file download doesn't require 1GB of RAM. It flows through a small, constant-sized buffer.

### Connection Pooling
For database performance, we strongly recommend using connection pooling (e.g., `sqlx::Pool`).
- **Reuse**: Establishing a TCP connection and performing a simplified SSL handshake for every request is slow. Pooling keeps connections open and ready.
- **Multiplexing**: Some drivers allow multiple queries to be in-flight on a single connection simultaneously.

## Optimizing Your App

To get the most out of RustAPI, follow these guidelines:

1. **Avoid Blocking the Async Executor**: Never run CPU-intensive tasks (cryptography, image processing) or blocking I/O (std::fs::read) directly in an async handler.
    - *Solution*: Use `tokio::task::spawn_blocking` to offload these to a dedicated thread pool.
    
    ```rust
    // BAD: Blocks the thread, potentially stalling other requests
    fn handler() {
        let digest = tough_crypto_hash(data); 
    }
    
    // GOOD: Runs on a thread meant for blocking work
    async fn handler() {
        let digest = tokio::task::spawn_blocking(move || {
            tough_crypto_hash(data)
        }).await.unwrap();
    }
    ```

2. **JSON Serialization**: While `serde` is fast, JSON text processing is CPU heavy.
    - For extremely high-throughput endpoints, consider binary formats like **Protobuf** or **MessagePack** if the client supports it.

3. **Keep `State` Light**: Your `State` struct is cloned for every request. Wrap large shared data in `Arc<T>` so only the pointer is cloned, not the data itself.

```rust
// Fast
#[derive(Clone)]
struct AppState {
    db: PgPool,                // Internally uses Arc
    config: Arc<Config>,       // Wrapped in Arc manually
}
```

## Benchmarking

Performance is not a guessing game. Below are results from our internal benchmarks on reference hardware.

### Comparative Benchmarks

| Framework | Requests/sec | Latency (avg) | Memory |
|-----------|--------------|---------------|--------|
| **RustAPI** | **~185,000** | **~0.54ms** | **~8MB** |
| **RustAPI + simd-json** | **~220,000** | **~0.45ms** | **~8MB** |
| Actix-web | ~178,000 | ~0.56ms | ~10MB |
| Axum | ~165,000 | ~0.61ms | ~12MB |
| Rocket | ~95,000 | ~1.05ms | ~15MB |
| FastAPI (Python) | ~12,000 | ~8.3ms | ~45MB |

<details>
<summary>🔬 Test Configuration</summary>

- **Hardware**: Intel i7-12700K, 32GB RAM
- **Method**: `wrk -t12 -c400 -d30s http://127.0.0.1:8080/api/users`
- **Scenario**: JSON serialization of 100 user objects
- **Build**: `cargo build --release`

Results may vary based on hardware and workload. Run your own benchmarks:
```bash
cd benches
./run_benchmarks.ps1
```
</details>

### Why So Fast?

| Optimization | Description |
|--------------|-------------|
| ⚡ **SIMD-JSON** | 2-4x faster JSON parsing with `simd-json` feature |
| 🔄 **Zero-copy parsing** | Direct memory access for path/query params |
| 📦 **SmallVec PathParams** | Stack-optimized path parameters |
| 🎯 **Compile-time dispatch** | All extractors resolved at compile time |
| 🌊 **Streaming bodies** | Handle large uploads without memory bloat |

Remember: RustAPI provides the *capability* for high performance, but your application logic ultimately dictates the speed. Use tools like `wrk`, `k6`, or `drill` to stress-test your specific endpoints.
