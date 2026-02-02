# Automatic Status Page

RustAPI comes with a built-in, zero-configuration status page that gives you instant visibility into your application's health and performance.

## Enabling the Status Page

To enable the status page, simply call `.status_page()` on your `RustApi` builder:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> Result<()> {
    RustApi::auto()
        .status_page() // <--- Enable Status Page
        .run("127.0.0.1:8080")
        .await
}
```

By default, the status page is available at `/status`.

## Full Example

Here is a complete, runnable example that demonstrates how to set up the status page and generate some traffic to see the metrics in action.

You can find this example in `crates/rustapi-rs/examples/status_demo.rs`.

```rust
use rustapi_rs::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

/// A simple demo to showcase the RustAPI Status Page.
///
/// Run with: `cargo run -p rustapi-rs --example status_demo`
/// Then verify:
/// - Status Page: http://127.0.0.1:3000/status
/// - Generate Traffic: http://127.0.0.1:3000/api/fast
/// - Generate Errors: http://127.0.0.1:3000/api/slow
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Define some handlers to generate metrics

    // A fast endpoint
    async fn fast_handler() -> &'static str {
        "Fast response!"
    }

    // A slow endpoint with random delay to show latency
    async fn slow_handler() -> &'static str {
        sleep(Duration::from_millis(500)).await;
        "Slow response... sleepy..."
    }

    // An endpoint that sometimes fails
    async fn flaky_handler() -> Result<&'static str, rustapi_rs::Response> {
        use std::sync::atomic::{AtomicBool, Ordering};
        static FAILURE: AtomicBool = AtomicBool::new(false);

        // Toggle failure every call
        let fail = FAILURE.fetch_xor(true, Ordering::Relaxed);

        if !fail {
            Ok("Success!")
        } else {
            Err(rustapi_rs::StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }

    // 2. Build the app with status page enabled
    println!("Starting Status Page Demo...");
    println!(" -> Open http://127.0.0.1:3000/status to see the dashboard");
    println!(" -> Visit http://127.0.0.1:3000/fast to generate traffic");
    println!(" -> Visit http://127.0.0.1:3000/slow to generate latency");
    println!(" -> Visit http://127.0.0.1:3000/flaky to generate errors");

    RustApi::auto()
        .status_page() // <--- Enable Status Page
        .route("/fast", get(fast_handler))
        .route("/slow", get(slow_handler))
        .route("/flaky", get(flaky_handler))
        .run("127.0.0.1:3000")
        .await
}
```

## Dashboard Overview

The status page provides a comprehensive real-time view of your system.

### 1. Global System Stats
At the top of the dashboard, you'll see high-level metrics for the entire application:
- **System Uptime**: How long the server has been running.
- **Total Requests**: The aggregate number of requests served across all endpoints.
- **Active Endpoints**: The number of distinct routes that have received traffic.
- **Auto-Refresh**: The page automatically updates every 5 seconds, so you can keep it open on a second monitor.

### 2. Endpoint Metrics Grid
The main section is a detailed table showing granular performance data for every endpoint:

| Metric | Description |
|--------|-------------|
| **Endpoint** | The path of the route (e.g., `/api/users`). |
| **Requests** | Total number of hits this specific route has received. |
| **Success Rate** | Visual indicator of health. <br>🟢 **Green**: ≥95% success <br>🔴 **Red**: <95% success |
| **Avg Latency** | The average time (in milliseconds) it takes to serve a request. |
| **Last Access** | Timestamp of the most recent request to this endpoint. |

### 3. Visual Design
The dashboard is built with a "zero-dependency" philosophy. It renders a single, self-contained HTML page directly from the binary.
- **Modern UI**: Clean, card-based layout using system fonts.
- **Responsive**: Adapts perfectly to mobile and desktop screens.
- **Lightweight**: No external CSS/JS files to manage or load.

## Custom Configuration

If you need more control, you can customize the path and title of the status page:

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::status::StatusConfig;

#[rustapi_rs::main]
async fn main() -> Result<()> {
    // Configure the status page
    let config = StatusConfig::new()
        .path("/admin/health")      // Change URL to /admin/health
        .title("Production Node 1"); // Custom title for easy identification

    RustApi::auto()
        .status_page_with_config(config)
        .run("127.0.0.1:8080")
        .await
}
```
