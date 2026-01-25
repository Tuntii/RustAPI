use rustapi_rs::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

/// A simple demo to showcase the RustAPI Status Page.
///
/// Run with: `cargo run --example status_demo`
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
