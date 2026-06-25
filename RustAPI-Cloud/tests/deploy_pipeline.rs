mod common;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use common::JWT_SECRET;
use rustapi_cloud::auth::jwt;
use rustapi_cloud::config::{Config, DeploySettings};
use rustapi_cloud::db::create_pool;
use rustapi_cloud::deploy::DeployService;
use rustapi_cloud::models::{DeployStatus, NewUser};
use rustapi_cloud::schema::users;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tempfile::tempdir;
use tokio::time::sleep;

fn listener_binary() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/port-listener/Cargo.toml");
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--manifest-path",
                manifest.to_str().unwrap(),
            ])
            .status()
            .expect("failed to build port-listener fixture");
        assert!(status.success(), "port-listener build failed");

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("fixtures/port-listener/target/release/port-listener");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        assert!(path.exists(), "fixture binary missing at {}", path.display());
        path
    })
    .clone()
}

async fn setup_pool() -> rustapi_cloud::db::DbPool {
    let url = common::database_url().await;
    create_pool(&url).await
}

async fn fetch_deploy_error(pool: &rustapi_cloud::db::DbPool, deploy_id: &str) -> Option<String> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use rustapi_cloud::schema::deploys;

    let mut conn = pool.get().await.ok()?;
    deploys::table
        .filter(deploys::id.eq(deploy_id))
        .select(deploys::error_message)
        .first::<Option<String>>(&mut conn)
        .await
        .ok()
        .flatten()
}

async fn insert_test_user(pool: &rustapi_cloud::db::DbPool) -> (String, String) {
    let mut conn = pool.get().await.expect("db connection");
    let suffix = uuid::Uuid::new_v4().as_simple().to_string();
    let login = format!("deploy-test-{}", &suffix[..8]);
    let github_id = (uuid::Uuid::new_v4().as_u128() & i64::MAX as u128) as i64;
    let user = NewUser::from_github(
        github_id,
        login.clone(),
        None,
        Some(format!("{}@example.com", login)),
    );
    let user_id = user.id.clone();
    diesel::insert_into(users::table)
        .values(&user)
        .execute(&mut conn)
        .await
        .expect("insert user");
    let (token, _) = jwt::create_token(
        &user_id,
        &login,
        None,
        "hobby",
        JWT_SECRET,
        24,
    )
    .expect("jwt");
    (user_id, token)
}

#[tokio::test]
async fn create_from_upload_persists_deploy_and_returns_queued() {
    let pool = setup_pool().await;
    let (user_id, token) = insert_test_user(&pool).await;
    let storage = tempdir().expect("tempdir");
    let service = DeployService::new(pool.clone(), storage.path(), DeploySettings::default());

    let binary = listener_binary();
    let bytes = std::fs::read(&binary).expect("read fixture binary");

    let created = service
        .create_from_upload(&user_id, "listener-app", &bytes)
        .await
        .expect("create deploy");

    assert_eq!(created.status, DeployStatus::Queued.as_str());
    assert!(created.url.is_none());
    assert!(!created.deploy_id.is_empty());

    let stored = service
        .get_status(&user_id, &created.deploy_id)
        .await
        .expect("status");
    assert_eq!(stored.deploy_id, created.deploy_id);
    assert_eq!(stored.status, DeployStatus::Queued.as_str());

    for _ in 0..40 {
        sleep(Duration::from_millis(250)).await;
        let current = service
            .get_status(&user_id, &created.deploy_id)
            .await
            .expect("poll status");
        if current.status == DeployStatus::Running.as_str()
            || current.status == DeployStatus::Live.as_str()
        {
            assert!(current.url.is_some(), "running deploy must have url");
            let url = current.url.unwrap();
            assert!(url.starts_with("http://127.0.0.1:"));
            return;
        }
        if current.status == DeployStatus::Failed.as_str() {
            panic!("deploy failed unexpectedly");
        }
    }

    panic!("deploy did not reach running/live state in time");
}

#[tokio::test]
async fn get_status_rejects_other_users_deploy() {
    let pool = setup_pool().await;
    let (owner_id, _) = insert_test_user(&pool).await;
    let (other_id, _) = insert_test_user(&pool).await;
    let storage = tempdir().expect("tempdir");
    let service = DeployService::new(pool, storage.path(), DeploySettings::default());

    let created = service
        .create_from_upload(&owner_id, "private-app", b"not-a-real-binary")
        .await
        .expect("create");

    let err = service
        .get_status(&other_id, &created.deploy_id)
        .await
        .expect_err("must not leak deploy");
    assert!(err.to_string().contains("not found") || err.to_string().contains("Not found"));
}

#[tokio::test]
async fn deploy_routes_require_auth_over_http() {
    use bytes::Bytes;
    use http::Method;
    use http_body_util::BodyExt;
    use rustapi_core::{BodyVariant, PathParams, Request};

    let pool = setup_pool().await;
    let storage = tempdir().expect("tempdir");
    let config = Config {
        database_url: common::database_url().await,
        host: "127.0.0.1".into(),
        port: 0,
        jwt_secret: JWT_SECRET.into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        github_redirect_uri: "http://localhost/auth/callback".into(),
        storage_root: storage.path().to_string_lossy().into(),
        deploy: DeploySettings::default(),
    };

    let app = rustapi_cloud::build_app(config, pool);
    let dispatcher = app.request_dispatcher();

    let boundary = "test-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"project_name\"\r\n\r\nx\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"binary\"; filename=\"x.bin\"\r\nContent-Type: application/octet-stream\r\n\r\nabc\r\n--{boundary}--\r\n"
    );

    let http_req = http::Request::builder()
        .method(Method::POST)
        .uri("/deploy")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(())
        .expect("request");
    let (parts, _) = http_req.into_parts();
    let request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::from(body)),
        dispatcher.state_ref(),
        PathParams::new(),
    );

    let response = dispatcher.dispatch(request).await;
    assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deploy_schema_has_required_columns() {
    let schema = common::dump_deploy_schema().await;
    println!("{}", schema);
    for column in [
        "projects.user_id",
        "projects.name",
        "deploys.user_id",
        "deploys.binary_path",
        "deploys.status",
        "deploys.url",
        "deploys.created_at",
        "deploys.updated_at",
    ] {
        assert!(
            schema.contains(column),
            "missing column {} in schema dump:\n{}",
            column,
            schema
        );
    }
}

#[tokio::test]
async fn deploy_create_over_http_with_auth_returns_deploy_id() {
    use bytes::Bytes;
    use http::Method;
    use http_body_util::BodyExt;
    use rustapi_core::{BodyVariant, PathParams, Request};

    let pool = setup_pool().await;
    let (user_id, token) = insert_test_user(&pool).await;
    let storage = tempdir().expect("tempdir");
    let config = Config {
        database_url: common::database_url().await,
        host: "127.0.0.1".into(),
        port: 0,
        jwt_secret: JWT_SECRET.into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        github_redirect_uri: "http://localhost/auth/callback".into(),
        storage_root: storage.path().to_string_lossy().into(),
        deploy: DeploySettings::default(),
    };

    let app = rustapi_cloud::build_app(config, pool);
    let dispatcher = app.request_dispatcher();

    let binary = listener_binary();
    let bytes = std::fs::read(&binary).expect("read fixture");

    let boundary = "auth-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"project_name\"\r\n\r\nlistener-app\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"binary\"; filename=\"listener.bin\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n",
    );
    let mut payload = body.into_bytes();
    payload.extend_from_slice(&bytes);
    payload.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let http_req = http::Request::builder()
        .method(Method::POST)
        .uri("/deploy")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(())
        .expect("request");
    let (parts, _) = http_req.into_parts();
    let request = Request::new(
        parts,
        BodyVariant::Buffered(Bytes::from(payload)),
        dispatcher.state_ref(),
        PathParams::new(),
    );

    let response = dispatcher.dispatch(request).await;
    assert_eq!(response.status(), http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
    assert!(json.get("deploy_id").and_then(|v| v.as_str()).is_some());
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("queued"));

    let deploy_id = json["deploy_id"].as_str().unwrap().to_string();

    for attempt in 0..40 {
        sleep(Duration::from_millis(250)).await;
        let status_req = http::Request::builder()
            .method(Method::GET)
            .uri(format!("/deploy/{}/status", deploy_id))
            .header("authorization", format!("Bearer {}", token))
            .body(())
            .expect("status request");
        let (parts, _) = status_req.into_parts();
        let status_request = Request::new(
            parts,
            BodyVariant::Buffered(Bytes::new()),
            dispatcher.state_ref(),
            PathParams::new(),
        );
        let status_response = dispatcher.dispatch(status_request).await;
        assert_eq!(status_response.status(), http::StatusCode::OK);
        let status_body = status_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let status_json: serde_json::Value =
            serde_json::from_slice(&status_body).expect("status json");
        let status = status_json["status"].as_str().unwrap_or("");
        if status == "live" || status == "running" {
            assert!(status_json["url"].as_str().is_some());
            println!(
                "HTTP_DEPLOY_STATUS_OK attempt={} deploy_id={} status={}",
                attempt, deploy_id, status
            );
            let _ = user_id;
            return;
        }
        if status == "failed" {
            let err = fetch_deploy_error(&setup_pool().await, &deploy_id).await;
            panic!("deploy failed: {} db_error={:?}", status_json, err);
        }
    }

    panic!("deploy did not reach running/live via HTTP handlers");
}

#[tokio::test]
async fn full_stack_deploy_flow_over_tcp() {
    use std::net::TcpListener;

    let pool = setup_pool().await;
    let (user_id, token) = insert_test_user(&pool).await;
    let storage = tempdir().expect("tempdir");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let config = Config {
        database_url: common::database_url().await,
        host: "127.0.0.1".into(),
        port,
        jwt_secret: JWT_SECRET.into(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        github_redirect_uri: "http://localhost/auth/callback".into(),
        storage_root: storage.path().to_string_lossy().into(),
        deploy: DeploySettings::default(),
    };

    let pool_for_errors = pool.clone();
    let app = rustapi_cloud::build_app(config, pool);
    let addr = format!("127.0.0.1:{}", port);
    let cloud_url = format!("http://{}", addr);
    let server_addr = addr.clone();
    let server = tokio::spawn(async move {
        app.run(&server_addr).await.expect("server run");
    });
    tokio::time::sleep(Duration::from_millis(500)).await;

    let binary = listener_binary();
    let binary_data = std::fs::read(&binary).expect("read fixture");
    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .text("project_name", "listener-app")
        .part(
            "binary",
            reqwest::multipart::Part::bytes(binary_data).file_name("listener.bin"),
        );

    let deploy_resp = client
        .post(format!("{}/deploy", cloud_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .expect("post deploy");
    assert!(deploy_resp.status().is_success(), "deploy POST failed");
    let created: serde_json::Value = deploy_resp.json().await.expect("deploy json");
    println!("DEPLOY_CREATE_RESPONSE: {}", created);
    let deploy_id = created["deploy_id"]
        .as_str()
        .expect("deploy_id")
        .to_string();
    assert_eq!(created["status"].as_str(), Some("queued"));

    let mut final_status = None;
    for attempt in 0..40 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let status_resp = client
            .get(format!("{}/deploy/{}/status", cloud_url, deploy_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("status get");
        assert!(status_resp.status().is_success(), "status GET failed");
        let status_json: serde_json::Value = status_resp.json().await.expect("status json");
        println!("DEPLOY_STATUS_POLL_{}: {}", attempt, status_json);
        let status = status_json["status"].as_str().unwrap_or("");
        if status == "live" || status == "running" {
            assert!(status_json["url"].as_str().is_some());
            final_status = Some(status_json);
            break;
        }
        if status == "failed" {
            let err = fetch_deploy_error(&pool_for_errors, &deploy_id).await;
            panic!("deploy failed: {} db_error={:?}", status_json, err);
        }
    }

    let final_status = final_status.expect("deploy never reached running/live");
    println!(
        "DEPLOY_FLOW_OK deploy_id={} status={} url={}",
        deploy_id,
        final_status["status"].as_str().unwrap(),
        final_status["url"].as_str().unwrap()
    );

    let _ = user_id;
    let _ = token;
    server.abort();
}

#[tokio::test]
async fn deploy_returns_public_https_url_when_production_host_configured() {
    let pool = setup_pool().await;
    let (user_id, _) = insert_test_user(&pool).await;
    let storage = tempdir().expect("tempdir");
    let map_dir = storage.path().join("nginx-map");
    let service = DeployService::new(
        pool.clone(),
        storage.path(),
        DeploySettings {
            public_host: Some("rustapi.tunayinbayramharcligi.com".into()),
            url_scheme: "https".into(),
            nginx_map_dir: Some(map_dir.to_string_lossy().into_owned()),
        },
    );

    let binary = listener_binary();
    let bytes = std::fs::read(&binary).expect("read fixture binary");
    let created = service
        .create_from_upload(&user_id, "listener-app", &bytes)
        .await
        .expect("create deploy");

    let user_prefix: String = user_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    let expected_url = format!(
        "https://listener-app-{user_prefix}.rustapi.tunayinbayramharcligi.com"
    );

    for _ in 0..40 {
        sleep(Duration::from_millis(250)).await;
        let current = service
            .get_status(&user_id, &created.deploy_id)
            .await
            .expect("poll status");
        if current.status == DeployStatus::Live.as_str() {
            assert_eq!(current.url.as_deref(), Some(expected_url.as_str()));
            let map_file = map_dir.join(format!("listener-app-{user_prefix}.conf"));
            let hostname = expected_url.trim_start_matches("https://");
            let map_content = std::fs::read_to_string(&map_file).unwrap_or_else(|err| {
                panic!("nginx map file {} missing: {err}", map_file.display())
            });
            assert!(map_content.contains(hostname));
            assert!(map_content.contains(';'));
            return;
        }
        if current.status == DeployStatus::Failed.as_str() {
            panic!("deploy failed unexpectedly");
        }
    }

    panic!("deploy did not reach live with public URL");
}