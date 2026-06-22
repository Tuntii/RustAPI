#![cfg(feature = "http3-dev")]

use rustapi_core::RustApi;
use std::net::UdpSocket;
use std::time::Duration;

#[tokio::test]
async fn run_http3_dev_entrypoint_prepares_health_routes() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);

    let addr = format!("127.0.0.1:{port}");
    let app = RustApi::new().health_endpoints();

    let server = tokio::spawn(async move { app.run_http3_dev(&addr).await });

    tokio::time::sleep(Duration::from_millis(500)).await;
    server.abort();
    let _ = server.await;
}
