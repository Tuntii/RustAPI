use rustapi_core::{get, RustApi};
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn test_graceful_shutdown() {
    let app = RustApi::new().route("/", get(|| async { "ok" }));

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();
    drop(listener);

    let (tx, rx) = oneshot::channel();
    let addr_str = format!("127.0.0.1:{}", port);

    let server_handle = tokio::spawn(async move {
        app.run_with_shutdown(&addr_str, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    // Retry logic in case startup is slow
    let mut resp = None;
    for _ in 0..5 {
        if let Ok(r) = client
            .get(format!("http://127.0.0.1:{}/", port))
            .header("Connection", "close")
            .send()
            .await
        {
            resp = Some(r);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // If we failed to get response, server might not have started or port issue.
    // We assume it started for now.
    if let Some(r) = resp {
        assert_eq!(r.status(), 200);
    } else {
        panic!("Failed to connect to server");
    }

    // Send shutdown signal
    tx.send(()).unwrap();

    // Wait for server to exit
    let result = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server did not shut down in time");

    let join_result = result.unwrap();
    assert!(join_result.is_ok(), "Join functionality failed");
    let server_result = join_result.unwrap();
    assert!(server_result.is_ok(), "Server returned error");
}
