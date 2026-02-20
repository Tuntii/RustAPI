# Graceful Shutdown

Graceful shutdown is essential for production applications to ensure that in-flight requests are completed before the server stops. This prevents data loss and provides a smooth experience for users during deployments or restarts.

## How it Works

When a shutdown signal (like `SIGTERM` or `Ctrl+C`) is received:
1. The server stops accepting new connections.
2. Existing connections are allowed to finish their current request.
3. Background tasks are notified (if configured).
4. The application exits once all connections are closed or a timeout is reached.

## Implementation

RustAPI supports graceful shutdown via the `.run_with_shutdown()` method, which accepts a `Future` that resolves when shutdown should occur.

### Basic Example

Here is how to implement graceful shutdown listening for `Ctrl+C`:

```rust
use rustapi_rs::prelude::*;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = RustApi::auto();

    println!("Server running on http://127.0.0.1:3000");

    app.run_with_shutdown("127.0.0.1:3000", shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Shutdown signal received, starting graceful shutdown...");
}
```

## Shutdown with Background Tasks

If you have background tasks (like `rustapi-jobs` workers), you should coordinate their shutdown as well. You can use `tokio_util::sync::CancellationToken` or a broadcast channel.

```rust
use rustapi_rs::prelude::*;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let token = CancellationToken::new();

    // Start a background task
    let child_token = token.child_token();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = child_token.cancelled() => {
                    println!("Background task shutting down");
                    return;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                    println!("Working...");
                }
            }
        }
    });

    // Run server
    RustApi::auto()
        .run_with_shutdown("0.0.0.0:3000", async {
            shutdown_signal().await;
            token.cancel(); // Notify background tasks
        })
        .await?;

    println!("Server exited successfully");
    Ok(())
}
```

## Platform Specifics

### Docker

When stopping a container, Docker sends `SIGTERM`. If your app doesn't exit within a timeout (default 10s), it sends `SIGKILL`. Graceful shutdown ensures your app handles `SIGTERM` correctly.

### Kubernetes

Kubernetes also sends `SIGTERM` before terminating a pod. It waits for the `terminationGracePeriodSeconds` (default 30s) before force-killing.

Ensure your shutdown logic completes within these timeframes.
