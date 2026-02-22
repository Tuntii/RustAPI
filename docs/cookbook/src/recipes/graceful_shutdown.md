# Graceful Shutdown

Graceful shutdown allows your API to stop accepting new connections and finish processing active requests before terminating. This is crucial for avoiding data loss and ensuring a smooth deployment process.

## Problem

When you stop a server (e.g., via `CTRL+C` or `SIGTERM`), you want to ensure that:
1.  The server stops listening on the port.
2.  Ongoing requests are allowed to complete.
3.  Resources (database connections, background jobs) are cleaned up properly.

## Solution

RustAPI provides the `run_with_shutdown` method, which accepts a `Future`. When this future completes, the server initiates the shutdown process.

### Basic Example (CTRL+C)

```rust
use rustapi_rs::prelude::*;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Define your application
    let app = RustApi::new().route("/", get(hello));

    // 2. Define the shutdown signal
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    // 3. Run with shutdown
    println!("Server running... Press CTRL+C to stop.");
    app.run_with_shutdown("127.0.0.1:3000", shutdown_signal).await?;

    println!("Server stopped gracefully.");
    Ok(())
}

async fn hello() -> &'static str {
    // Simulate some work
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    "Hello, World!"
}
```

### Production Example (Unix Signals)

In a production environment (like Kubernetes or Docker), you need to handle `SIGTERM` as well as `SIGINT`.

```rust
use rustapi_rs::prelude::*;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustApi::new().route("/", get(hello));

    app.run_with_shutdown("0.0.0.0:3000", shutdown_signal()).await?;

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
        _ = ctrl_c => println!("Received Ctrl+C"),
        _ = terminate => println!("Received SIGTERM"),
    }
}
```

## Discussion

-   **Active Requests**: RustAPI (via Hyper) will wait for active requests to complete.
-   **Timeout**: You might want to wrap the server execution in a timeout if you want to force shutdown after a certain period (though Hyper usually handles connection draining well).
-   **Background Tasks**: If you have spawned background tasks using `tokio::spawn`, they are detached and will be aborted when the runtime shuts down. For critical background work, consider using a dedicated job queue (like `rustapi-jobs`) or a `CancellationToken` to coordinate shutdown.
