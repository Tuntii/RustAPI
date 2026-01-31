use rustapi_extras::insight::export::{WebhookConfig, WebhookExporter, InsightExporter};
use rustapi_extras::insight::InsightData;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_webhook_blocking_behavior() {
    // Start a dummy server that sleeps
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = [0; 1024];
                let _ = socket.read(&mut buf).await;
                // Simulate slow processing
                tokio::time::sleep(Duration::from_millis(500)).await;
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    // Configure exporter with batch size 1 to trigger send immediately
    let config = WebhookConfig::new(format!("http://{}", addr))
        .batch_size(1)
        .timeout(2);

    let exporter = WebhookExporter::new(config);
    let insight = InsightData::new("test", "GET", "/");

    let start = Instant::now();
    // This should trigger a send because batch_size is 1.
    // In current implementation, it blocks waiting for response.
    match exporter.export(&insight) {
        Ok(_) => println!("Export successful"),
        Err(e) => println!("Export failed: {:?}", e),
    }
    let duration = start.elapsed();

    println!("Export took: {:?}", duration);

    // If it blocks, it should take at least 500ms (due to 500ms server sleep or timeout)
    if duration.as_millis() >= 400 {
        panic!("Performance regression: Export blocked for {:?}. Expected non-blocking behavior.", duration);
    }
}
