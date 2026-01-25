use rustapi_core::{get, RustApi};
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn test_status_page() {
    async fn task_handler() -> &'static str {
        "ok"
    }

    // Setup app with status page
    let app = RustApi::new()
        .status_page() // Enable status page
        .route("/task", get(task_handler));

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

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);

    // 1. Check initial status page
    let res = client
        .get(format!("{}/status", base_url))
        .send()
        .await
        .expect("Failed to get status");
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("System Status"));
    assert!(body.contains("Total Requests"));

    // 2. Make some requests to generate metrics
    for _ in 0..2 {
        let res = client
            .get(format!("{}/task", base_url))
            .send()
            .await
            .expect("Failed to get task");
        assert_eq!(res.status(), 200);
    }

    // 3. Check updated status page
    let res = client
        .get(format!("{}/status", base_url))
        .send()
        .await
        .expect("Failed to get status");
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();

    // Should show path /task
    assert!(body.contains("/task"));

    // Send shutdown signal
    tx.send(()).unwrap();

    // Wait for server to exit
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}
