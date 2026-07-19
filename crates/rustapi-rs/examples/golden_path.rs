//! Golden path — smallest service shape we recommend shipping.
//!
//! Story in one command:
//! 1. handler + `Schema` → typed JSON
//! 2. `RustApi::auto()` → OpenAPI + Swagger UI at `/docs`
//! 3. `production_defaults` → request IDs, tracing, `/live` `/ready` `/health`
//! 4. graceful shutdown on Ctrl-C
//!
//! Run (from repo root):
//! ```bash
//! cargo run -p rustapi-rs --example golden_path
//! ```
//!
//! Verify:
//! ```bash
//! curl -s http://127.0.0.1:8080/api/v1/ping
//! curl -s http://127.0.0.1:8080/live
//! curl -s http://127.0.0.1:8080/docs/openapi.json | head
//! # browser: http://127.0.0.1:8080/docs
//! ```
//!
//! Guide: [docs/GOLDEN_PATH.md](../../../docs/GOLDEN_PATH.md)
//! Before real traffic: [Production Checklist](../../../docs/PRODUCTION_CHECKLIST.md)

use rustapi_rs::prelude::*;
use tokio::signal;

#[derive(Debug, Serialize, Schema)]
struct PingResponse {
    status: &'static str,
}

/// Liveness-style app check. Probes come from `production_defaults`.
#[rustapi_rs::get("/api/v1/ping")]
#[rustapi_rs::tag("public")]
#[rustapi_rs::summary("Ping - golden path app check")]
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
        .run_with_shutdown("127.0.0.1:8080", async {
            signal::ctrl_c().await.ok();
        })
        .await
}
