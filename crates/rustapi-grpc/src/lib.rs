//! # rustapi-grpc
//!
//! gRPC integration helpers for RustAPI using [`tonic`].
//!
//! This crate keeps RustAPI's facade approach: your app code stays simple while you can
//! run a RustAPI HTTP server and a Tonic gRPC server side-by-side in the same process.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use rustapi_rs::grpc::{run_rustapi_and_grpc, tonic};
//! use rustapi_rs::prelude::*;
//!
//! #[rustapi_rs::get("/health")]
//! async fn health() -> &'static str { "ok" }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let http_app = RustApi::new().route("/health", get(health));
//!
//!     let grpc_addr = "127.0.0.1:50051".parse()?;
//!     let grpc_server = tonic::transport::Server::builder()
//!         .add_service(MyGreeterServer::new(MyGreeter::default()))
//!         .serve(grpc_addr);
//!
//!     run_rustapi_and_grpc(http_app, "127.0.0.1:8080", grpc_server).await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use rustapi_core::RustApi;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::watch;

/// Boxed error used by this crate.
pub type BoxError = Box<dyn Error + Send + Sync>;

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, BoxError>;

/// Shutdown future type used by gRPC server builders.
pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Re-export `tonic` so users can use a single dependency surface from `rustapi-rs`.
pub use tonic;

/// Re-export `prost` for protobuf message derives and runtime types.
pub use prost;

fn to_boxed_error<E>(err: E) -> BoxError
where
    E: Error + Send + Sync + 'static,
{
    Box::new(err)
}

/// Run two independent servers/tasks concurrently.
///
/// This is useful for running a RustAPI HTTP server together with a Tonic gRPC server.
///
/// The function returns when one of the futures returns an error, or when both complete successfully.
pub async fn run_concurrently<HF, GF, HE, GE>(http_future: HF, grpc_future: GF) -> Result<()>
where
    HF: Future<Output = std::result::Result<(), HE>> + Send,
    GF: Future<Output = std::result::Result<(), GE>> + Send,
    HE: Error + Send + Sync + 'static,
    GE: Error + Send + Sync + 'static,
{
    let http_task = async move { http_future.await.map_err(to_boxed_error) };
    let grpc_task = async move { grpc_future.await.map_err(to_boxed_error) };

    let (_http_ok, _grpc_ok) = tokio::try_join!(http_task, grpc_task)?;
    Ok(())
}

/// Run a `RustApi` HTTP server and any gRPC future side-by-side.
///
/// `grpc_future` is typically a Tonic server future:
/// `tonic::transport::Server::builder().add_service(...).serve(addr)`.
pub async fn run_rustapi_and_grpc<GF, GE>(
    app: RustApi,
    http_addr: impl AsRef<str>,
    grpc_future: GF,
) -> Result<()>
where
    GF: Future<Output = std::result::Result<(), GE>> + Send,
    GE: Error + Send + Sync + 'static,
{
    let http_addr = http_addr.as_ref().to_string();
    let http_task = async move { app.run(&http_addr).await };
    let grpc_task = async move { grpc_future.await.map_err(to_boxed_error) };

    let (_http_ok, _grpc_ok) = tokio::try_join!(http_task, grpc_task)?;
    Ok(())
}

/// Run RustAPI HTTP and gRPC servers together with a shared shutdown signal.
///
/// This helper lets you provide a single shutdown signal (for example `tokio::signal::ctrl_c()`)
/// and uses it for both servers.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_rs::grpc::{run_rustapi_and_grpc_with_shutdown, tonic};
/// use rustapi_rs::prelude::*;
///
/// #[rustapi_rs::get("/health")]
/// async fn health() -> &'static str { "ok" }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let app = RustApi::new().route("/health", get(health));
///     let grpc_addr = "127.0.0.1:50051".parse()?;
///
///     run_rustapi_and_grpc_with_shutdown(
///         app,
///         "127.0.0.1:8080",
///         tokio::signal::ctrl_c(),
///         move |shutdown| {
///             tonic::transport::Server::builder()
///                 .add_service(MyGreeterServer::new(MyGreeter::default()))
///                 .serve_with_shutdown(grpc_addr, shutdown)
///         },
///     ).await?;
///
///     Ok(())
/// }
/// ```
pub async fn run_rustapi_and_grpc_with_shutdown<GF, GE, SF, F>(
    app: RustApi,
    http_addr: impl AsRef<str>,
    shutdown_signal: SF,
    grpc_with_shutdown: F,
) -> Result<()>
where
    GF: Future<Output = std::result::Result<(), GE>> + Send,
    GE: Error + Send + Sync + 'static,
    SF: Future<Output = ()> + Send + 'static,
    F: FnOnce(ShutdownFuture) -> GF,
{
    let http_addr = http_addr.as_ref().to_string();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Fan out a single shutdown signal to both servers.
    let shutdown_dispatch = tokio::spawn(async move {
        shutdown_signal.await;
        let _ = shutdown_tx.send(true);
    });

    let http_shutdown = shutdown_notifier(shutdown_rx.clone());
    let grpc_shutdown = shutdown_notifier(shutdown_rx);

    let http_task = async move { app.run_with_shutdown(&http_addr, http_shutdown).await };
    let grpc_task = async move {
        grpc_with_shutdown(Box::pin(grpc_shutdown))
            .await
            .map_err(to_boxed_error)
    };

    let joined = tokio::try_join!(http_task, grpc_task).map(|_| ());

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

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_core::get;
    use std::io;
    use tokio::sync::oneshot;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn run_concurrently_returns_ok_when_both_succeed() {
        let http = async { Ok::<(), io::Error>(()) };
        let grpc = async { Ok::<(), io::Error>(()) };

        let result = run_concurrently(http, grpc).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_concurrently_returns_err_when_any_fails() {
        let http = async { Err::<(), _>(io::Error::other("http failed")) };

        let grpc = async {
            sleep(Duration::from_millis(20)).await;
            Ok::<(), io::Error>(())
        };

        let result = run_concurrently(http, grpc).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_rustapi_and_grpc_with_shutdown_stops_both_servers() {
        async fn health() -> &'static str {
            "ok"
        }

        let app = RustApi::new().route("/health", get(health));
        let grpc_addr = "127.0.0.1:0".parse().expect("valid socket addr");
        let (tx, rx) = oneshot::channel::<()>();

        let run_future = run_rustapi_and_grpc_with_shutdown(
            app,
            "127.0.0.1:0",
            async move {
                let _ = rx.await;
            },
            move |shutdown| {
                let (_reporter, health_service) = tonic_health::server::health_reporter();
                tonic::transport::Server::builder()
                    .add_service(health_service)
                    .serve_with_shutdown(grpc_addr, shutdown)
            },
        );

        tokio::spawn(async move {
            sleep(Duration::from_millis(75)).await;
            let _ = tx.send(());
        });

        let result = timeout(Duration::from_secs(3), run_future).await;
        assert!(result.is_ok(), "servers should stop before timeout");
        assert!(
            result.expect("timeout checked").is_ok(),
            "graceful shutdown should succeed"
        );
    }
}
