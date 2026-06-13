//! Concurrent execution helpers to run a normal RustAPI HTTP server
//! side-by-side with an MCP server (on a separate address).
//!
//! This is modeled after `rustapi-grpc` for consistency.

use crate::server::McpServer;
use rustapi_core::RustApi;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::watch;

/// Boxed error type used by runner helpers.
pub type BoxError = Box<dyn Error + Send + Sync>;

/// Result type for runner helpers.
pub type Result<T> = std::result::Result<T, BoxError>;

/// Shutdown future type.
pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

fn to_boxed_error<E>(err: E) -> BoxError
where
    E: Error + Send + Sync + 'static,
{
    Box::new(err)
}

/// Run two independent futures concurrently (HTTP + MCP).
pub async fn run_concurrently<HF, MF, HE, ME>(http_future: HF, mcp_future: MF) -> Result<()>
where
    HF: Future<Output = std::result::Result<(), HE>> + Send,
    MF: Future<Output = std::result::Result<(), ME>> + Send,
    HE: Error + Send + Sync + 'static,
    ME: Error + Send + Sync + 'static,
{
    let http_task = async move { http_future.await.map_err(to_boxed_error) };
    let mcp_task = async move { mcp_future.await.map_err(to_boxed_error) };

    let (_http_ok, _mcp_ok) = tokio::try_join!(http_task, mcp_task)?;
    Ok(())
}

/// Run a `RustApi` HTTP server and an `McpServer` side-by-side on separate addresses.
///
/// This is the primary helper for running your normal API and the MCP endpoint
/// (for LLM/agent clients) at the same time.
pub async fn run_rustapi_and_mcp(
    app: RustApi,
    http_addr: impl AsRef<str>,
    mcp: McpServer,
    mcp_addr: impl AsRef<str>,
) -> Result<()>
where
{
    let http_addr = http_addr.as_ref().to_string();
    let mcp_addr = mcp_addr.as_ref().to_string();

    // Automatically configure the MCP server to proxy tool calls back to the main HTTP API.
    // This makes end-to-end tool invocation work out of the box.
    let http_base = format!(
        "http://127.0.0.1:{}",
        extract_port_or_default(&http_addr, 8080)
    );
    let mcp = mcp.with_http_base(http_base);

    let http_task = async move { app.run(&http_addr).await };
    let mcp_task = async move { mcp.serve(&mcp_addr).await.map_err(to_boxed_error) };

    let (_http_ok, _mcp_ok) = tokio::try_join!(http_task, mcp_task)?;
    Ok(())
}

/// Run RustAPI HTTP + MCP with a shared shutdown signal.
///
/// Useful with `tokio::signal::ctrl_c()` so that Ctrl+C stops both servers cleanly.
pub async fn run_rustapi_and_mcp_with_shutdown<SF>(
    app: RustApi,
    http_addr: impl AsRef<str>,
    mcp: McpServer,
    mcp_addr: impl AsRef<str>,
    shutdown_signal: SF,
) -> Result<()>
where
    SF: Future<Output = ()> + Send + 'static,
{
    let http_addr = http_addr.as_ref().to_string();
    let mcp_addr = mcp_addr.as_ref().to_string();

    let http_base = format!(
        "http://127.0.0.1:{}",
        extract_port_or_default(&http_addr, 8080)
    );
    let mcp = mcp.with_http_base(http_base);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let shutdown_dispatch = tokio::spawn(async move {
        shutdown_signal.await;
        let _ = shutdown_tx.send(true);
    });

    let http_shutdown = shutdown_notifier(shutdown_rx.clone());
    let mcp_shutdown = shutdown_notifier(shutdown_rx);

    let http_task = async move { app.run_with_shutdown(&http_addr, http_shutdown).await };

    let mcp_task = async move {
        mcp.serve_with_shutdown(&mcp_addr, mcp_shutdown)
            .await
            .map_err(to_boxed_error)
    };

    let joined = tokio::try_join!(http_task, mcp_task).map(|_| ());

    shutdown_dispatch.abort();
    let _ = shutdown_dispatch.await;

    joined
}

async fn shutdown_notifier(mut rx: watch::Receiver<bool>) {
    if *rx.borrow() {
        return;
    }
    while rx.changed().await.is_ok() {
        if *rx.borrow() {
            break;
        }
    }
}

/// Very small helper to turn "0.0.0.0:9090" or "[::]:9090" into a port for the localhost base URL.
fn extract_port_or_default(addr: &str, default: u16) -> u16 {
    // Try to find the last ':' and parse what follows as port
    if let Some(colon) = addr.rfind(':') {
        let after = &addr[colon + 1..];
        // strip any trailing path or query (shouldn't be there for addr)
        let port_str = after
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("");
        if let Ok(p) = port_str.parse::<u16>() {
            return p;
        }
    }
    default
}
