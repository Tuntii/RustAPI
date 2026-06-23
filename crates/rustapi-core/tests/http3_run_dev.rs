#![cfg(feature = "http3-dev")]

use rustapi_core::RustApi;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn run_http3_dev_with_shutdown_runs_lifecycle_hooks() {
    let on_start = Arc::new(AtomicBool::new(false));
    let on_shutdown = Arc::new(AtomicBool::new(false));
    let on_start_flag = on_start.clone();
    let on_shutdown_flag = on_shutdown.clone();

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);

    let addr = format!("127.0.0.1:{port}");
    let app = RustApi::new()
        .health_endpoints()
        .on_start(move || {
            let on_start_flag = on_start_flag.clone();
            async move {
                on_start_flag.store(true, Ordering::SeqCst);
            }
        })
        .on_shutdown(move || {
            let on_shutdown_flag = on_shutdown_flag.clone();
            async move {
                on_shutdown_flag.store(true, Ordering::SeqCst);
            }
        });

    let (tx, rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        app.run_http3_dev_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        on_start.load(Ordering::SeqCst),
        "on_start should run via prepare_for_serve before HTTP/3 accept loop"
    );

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(3), server).await;
    assert!(
        on_shutdown.load(Ordering::SeqCst),
        "on_shutdown should run after HTTP/3 shutdown signal"
    );
}
