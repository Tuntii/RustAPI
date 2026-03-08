use rustapi_core::health::{HealthCheckBuilder, HealthEndpointConfig, HealthStatus};
use rustapi_core::RustApi;
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn test_default_health_endpoints_are_available() {
    let app = RustApi::new().health_endpoints();

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
    let base_url = format!("http://127.0.0.1:{}", port);

    for path in ["/health", "/ready", "/live"] {
        let res = client
            .get(format!("{}{}", base_url, path))
            .send()
            .await
            .expect("health endpoint request failed");

        assert_eq!(res.status(), 200, "{} should return 200", path);

        let body: serde_json::Value = res.json().await.unwrap();
        assert!(
            body.get("status").is_some(),
            "{} should include status",
            path
        );
        assert!(
            body.get("timestamp").is_some(),
            "{} should include timestamp",
            path
        );
    }

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[tokio::test]
async fn test_unhealthy_readiness_returns_503() {
    let health = HealthCheckBuilder::new(false)
        .add_check("database", || async {
            HealthStatus::unhealthy("database offline")
        })
        .build();

    let app = RustApi::new().with_health_check(health);

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
    let base_url = format!("http://127.0.0.1:{}", port);

    let res = client
        .get(format!("{}/ready", base_url))
        .send()
        .await
        .expect("readiness endpoint request failed");

    assert_eq!(res.status(), 503);

    let body = res.text().await.unwrap();
    assert!(body.contains("database offline"));

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[tokio::test]
async fn test_custom_health_endpoint_paths() {
    let config = HealthEndpointConfig::new()
        .health_path("/healthz")
        .readiness_path("/readyz")
        .liveness_path("/livez");

    let app = RustApi::new().health_endpoints_with_config(config);

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
    let base_url = format!("http://127.0.0.1:{}", port);

    for path in ["/healthz", "/readyz", "/livez"] {
        let res = client
            .get(format!("{}{}", base_url, path))
            .send()
            .await
            .expect("custom health endpoint request failed");

        assert_eq!(res.status(), 200, "{} should return 200", path);
    }

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}
