# Production Tuning

**Problem**: Your API needs to handle extreme load (10k+ requests per second).

## Solution

### 1. Release Profile
Ensure `Cargo.toml` has optimal settings:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### 2. Runtime Config
Configure the Tokio runtime for high throughput in `main.rs`:

```rust
#[tokio::main(worker_threads = num_cpus::get())]
async fn main() {
    // ...
}
```

### 3. File Descriptors (Linux)
Increase the limit before running:

```bash
ulimit -n 100000
```

## Discussion

RustAPI is fast by default, but the OS often becomes the bottleneck using default settings. `panic = "abort"` reduces binary size and slightly improves performance by removing unwinding tables.
