//! Golden-path production baseline — the smallest service shape we recommend.
//!
//! Run:
//! ```bash
//! cargo run -p rustapi-rs --example golden_path
//! ```
//!
//! Verify:
//! - `GET http://127.0.0.1:8080/live`   → 200
//! - `GET http://127.0.0.1:8080/ready`  → 200
//! - `GET http://127.0.0.1:8080/health` → 200
//! - `GET http://127.0.0.1:8080/api/v1/ping` → `{"status":"ok"}`
//!
//! See [Production Checklist](../../../docs/PRODUCTION_CHECKLIST.md) before real traffic.

use rustapi_rs::prelude::*;
use tokio::signal;

#[derive(Debug, Serialize, Schema)]
struct PingResponse {
    status: &'static str,
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse { status: "ok" })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    RustApi::auto()
        .production_defaults("golden-path")
        .on_shutdown(|| async {
            tracing::info!("graceful shutdown complete");
        })
        .route("/api/v1/ping", get(ping))
        .run_with_shutdown("127.0.0.1:8080", async {
            signal::ctrl_c().await.ok();
        })
        .await
}
