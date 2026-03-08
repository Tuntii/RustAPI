# Graceful Shutdown

Graceful shutdown lets your API stop accepting new work, drain in-flight requests, and clean up resources before the process exits. In production, the missing piece is usually **draining**: marking the instance unready so upstream load balancers stop sending traffic before shutdown completes.

## Problem

When you stop a server (for example with `Ctrl+C` or `SIGTERM`), you usually want all of the following:

1. The process stops receiving new traffic.
2. Existing requests are allowed to finish.
3. Readiness flips to unhealthy during the drain window.
4. Cleanup hooks run in a predictable order.

## Solution

RustAPI provides `run_with_shutdown(...)`, which accepts a future. When that future resolves, the server begins graceful shutdown. If you also wire readiness to shared state, you can make the instance report `503` during the drain window before the future returns.

### Basic Example

```rust
use rustapi_rs::prelude::*;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let app = RustApi::new().route("/", get(hello));

    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    println!("Server running... Press CTRL+C to stop.");
    app.run_with_shutdown("127.0.0.1:3000", shutdown_signal).await?;

    println!("Server stopped gracefully.");
    Ok(())
}

async fn hello() -> &'static str {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    "Hello, World!"
}
```

### Production Example with Draining

In orchestrated environments you usually want to:

1. listen for `SIGTERM` as well as `Ctrl+C`,
2. mark the instance as draining,
3. wait for a short drain window, and only then
4. let `run_with_shutdown(...)` finish the shutdown.

```rust
use rustapi_rs::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    signal,
    time::{sleep, Duration},
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let draining = Arc::new(AtomicBool::new(false));
    let readiness_flag = draining.clone();

    let health = HealthCheckBuilder::new(true)
        .add_check("draining", move || {
            let readiness_flag = readiness_flag.clone();
            async move {
                if readiness_flag.load(Ordering::SeqCst) {
                    HealthStatus::unhealthy("draining")
                } else {
                    HealthStatus::healthy()
                }
            }
        })
        .build();

    let app = RustApi::new()
        .with_health_check(health)
        .on_shutdown(|| async {
            tracing::info!("shutdown cleanup finished");
        })
        .route("/", get(hello));

    app.run_with_shutdown("0.0.0.0:3000", shutdown_signal(draining)).await?;

    Ok(())
}

async fn shutdown_signal(draining: Arc<AtomicBool>) {
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

    draining.store(true, Ordering::SeqCst);
    sleep(Duration::from_secs(15)).await;
}

async fn hello() -> &'static str {
    sleep(Duration::from_secs(2)).await;
    "Hello, World!"
}
```

## Discussion

- **Active requests**: RustAPI waits for in-flight requests to complete as shutdown proceeds.
- **Drain window**: The sleep inside `shutdown_signal(...)` gives your ingress or load balancer time to observe readiness failure and stop sending new traffic.
- **Readiness semantics**: By wiring readiness to shared state, `/ready` can return `503 Service Unavailable` while `/live` still reports that the process is alive.
- **Cleanup hooks**: `on_shutdown(...)` hooks are executed after the shutdown signal future resolves, making them a good place for final flush/cleanup work.
- **Detached tasks**: `tokio::spawn` tasks are still detached. For critical work, coordinate them explicitly or move the work into a durable queue such as `rustapi-jobs`.
- **Forceful shutdown**: If your platform requires a hard upper bound, combine this approach with a platform-level termination grace period and an application-level timeout policy.

## Recommended production pattern

For most deployments:

1. Receive `SIGTERM`.
2. Mark the instance as draining.
3. Let readiness fail.
4. Wait 10–30 seconds, depending on your proxy and traffic pattern.
5. Allow graceful shutdown to complete.
6. Run shutdown hooks.

Pair this with the cookbook [Deployment](deployment.md) recipe and the docs [Production Checklist](../../../PRODUCTION_CHECKLIST.md).
