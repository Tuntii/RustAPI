use rustapi_core::health::HealthEndpointConfig;
use rustapi_core::{get, ProductionDefaultsConfig, RustApi};
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn test_production_defaults_enable_request_id_and_health_probes() {
    async fn hello() -> &'static str {
        "ok"
    }

    let app = RustApi::new()
        .production_defaults("users-api")
        .route("/hello", get(hello));

    assert_eq!(
        app.layers().len(),
        2,
        "request ID and tracing layers should be installed"
    );

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
        .get(format!("{}/hello", base_url))
        .send()
        .await
        .expect("hello request failed");
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("x-request-id").is_some());

    let res = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("health request failed");
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(
        body.get("version").is_none(),
        "version should be omitted when no version is configured"
    );
    assert!(body.get("checks").is_some());

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[tokio::test]
async fn test_production_defaults_custom_config_applies_version_and_custom_paths() {
    let config = ProductionDefaultsConfig::new("billing-api")
        .version("1.2.3")
        .health_endpoint_config(
            HealthEndpointConfig::new()
                .health_path("/healthz")
                .readiness_path("/readyz")
                .liveness_path("/livez"),
        );

    let app = RustApi::new().production_defaults_with_config(config);

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
            .expect("probe request failed");
        assert_eq!(res.status(), 200);
    }

    let res = client
        .get(format!("{}/healthz", base_url))
        .send()
        .await
        .expect("healthz request failed");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        body.get("version"),
        Some(&serde_json::Value::String("1.2.3".to_string()))
    );

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[test]
fn test_production_defaults_can_disable_optional_parts() {
    let app = RustApi::new().production_defaults_with_config(
        ProductionDefaultsConfig::new("minimal-api")
            .request_id(false)
            .tracing(false)
            .health_endpoints(false),
    );

    assert_eq!(app.layers().len(), 0);
}
