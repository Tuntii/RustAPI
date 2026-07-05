//! Integration tests for cargo-rustapi CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Helper to get the cargo-rustapi binary
fn cargo_rustapi() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("cargo-rustapi")
}

mod new_command {
    use super::*;

    #[test]
    fn test_new_help() {
        cargo_rustapi()
            .arg("new")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Create a new RustAPI project"));
    }

    #[test]
    fn test_new_minimal_template() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-minimal-project";
        let project_path = dir.path().join(project_name);

        // Change to temp directory and create project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Verify project structure
        assert!(project_path.exists(), "Project directory should exist");
        assert!(
            project_path.join("Cargo.toml").exists(),
            "Cargo.toml should exist"
        );
        assert!(
            project_path.join("src/main.rs").exists(),
            "src/main.rs should exist"
        );

        // Verify Cargo.toml content
        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(
            cargo_content.contains("rustapi-rs"),
            "Cargo.toml should depend on rustapi-rs"
        );
    }

    #[test]
    fn test_new_api_template() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-api-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "api", "--yes"])
            .assert()
            .success();

        // Verify API project structure
        assert!(project_path.join("src/handlers").is_dir());
        assert!(project_path.join("src/models").is_dir());
        assert!(project_path.join("src/handlers/mod.rs").exists());
        assert!(project_path.join("src/handlers/items.rs").exists());
        assert!(project_path.join("src/models/mod.rs").exists());
    }

    #[test]
    fn test_new_with_features() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-features-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args([
                "new",
                project_name,
                "--template",
                "minimal",
                "--features",
                "extras-jwt,extras-cors",
                "--yes",
            ])
            .assert()
            .success();

        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(
            cargo_content.contains("extras-jwt") && cargo_content.contains("extras-cors"),
            "Cargo.toml should include extras-jwt and extras-cors features"
        );
    }

    #[test]
    fn test_new_with_prod_api_preset() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-prod-preset-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--preset", "prod-api", "--yes"])
            .assert()
            .success();

        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(cargo_content.contains("extras-config"));
        assert!(cargo_content.contains("extras-cors"));
        assert!(cargo_content.contains("extras-rate-limit"));
        assert!(cargo_content.contains("extras-security-headers"));
        assert!(cargo_content.contains("extras-structured-logging"));
        assert!(cargo_content.contains("extras-timeout"));
    }

    #[test]
    fn test_new_with_ai_api_preset() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-ai-preset-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--preset", "ai-api", "--yes"])
            .assert()
            .success();

        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(cargo_content.contains("protocol-toon"));
        assert!(cargo_content.contains("extras-config"));
        assert!(cargo_content.contains("extras-timeout"));
        assert!(cargo_content.contains("extras-structured-logging"));
    }

    #[test]
    fn test_new_with_realtime_api_preset() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-realtime-preset-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--preset", "realtime-api", "--yes"])
            .assert()
            .success();

        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(cargo_content.contains("protocol-ws"));
        assert!(cargo_content.contains("extras-cors"));
        assert!(cargo_content.contains("extras-timeout"));
        assert!(cargo_content.contains("extras-structured-logging"));
    }

    #[test]
    fn test_new_existing_directory_fails() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "existing-dir";

        // Create the directory first
        fs::create_dir(dir.path().join(project_name)).expect("Failed to create dir");

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
    }

    #[test]
    fn test_new_invalid_name_fails() {
        let dir = tempdir().expect("Failed to create temp dir");

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "invalid/name", "--template", "minimal", "--yes"])
            .assert()
            .failure();
    }
}

mod doctor_command {
    use super::*;

    #[test]
    fn test_doctor_help() {
        cargo_rustapi()
            .arg("doctor")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("environment health"));
    }

    #[test]
    fn test_doctor_runs() {
        // Doctor should run and check for tools
        // It will succeed even if some tools are missing (just warns)
        cargo_rustapi().arg("doctor").assert().success();
    }

    #[test]
    fn test_doctor_checks_rust() {
        let output = cargo_rustapi()
            .arg("doctor")
            .output()
            .expect("Failed to run doctor");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Rust compiler") || stdout.contains("rustc"),
            "Doctor should check for Rust compiler"
        );
    }
}

mod bench_command {
    use super::*;

    #[test]
    fn test_bench_help() {
        cargo_rustapi()
            .args(["bench", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("benchmark workflow"));
    }
}

mod observability_command {
    use super::*;

    #[test]
    fn test_observability_help() {
        cargo_rustapi()
            .args(["observability", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("observability"));
    }

    #[test]
    fn test_observability_runs_against_repo() {
        cargo_rustapi()
            .args(["observability", "--path", "."])
            .assert()
            .success()
            .stdout(predicate::str::contains("Observability workflow assets"));
    }
}

#[cfg(feature = "replay")]
mod replay_command {
    use super::*;

    #[test]
    fn test_replay_help() {
        cargo_rustapi()
            .args(["replay", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Replay debugging commands"));
    }
}

mod generate_command {
    use super::*;

    #[test]
    fn test_generate_help() {
        cargo_rustapi()
            .args(["generate", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate code from templates"));
    }

    #[test]
    fn test_generate_handler() {
        let dir = tempdir().expect("Failed to create temp dir");

        // First create a minimal project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-gen", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Generate a handler
        cargo_rustapi()
            .current_dir(dir.path().join("test-gen"))
            .args(["generate", "handler", "users"])
            .assert()
            .success();

        // Verify handler was created
        let handler_path = dir.path().join("test-gen/src/handlers/users.rs");
        assert!(handler_path.exists(), "Handler file should be created");

        let content = fs::read_to_string(&handler_path).expect("Failed to read handler");
        assert!(content.contains("pub async fn list"));
        assert!(content.contains("pub async fn get"));
        assert!(content.contains("pub async fn create"));
    }

    #[test]
    fn test_generate_crud_sqlx_compiles() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-crud-sqlx";
        let project_path = dir.path().join(project_name);
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let workspace_version = env!("CARGO_PKG_VERSION");

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .success();

        cargo_rustapi()
            .current_dir(&project_path)
            .args(["generate", "crud", "items"])
            .assert()
            .success();

        let handler_path = project_path.join("src/handlers/items.rs");
        let handler = fs::read_to_string(&handler_path).expect("read handler");
        assert!(
            !handler.contains("TODO: Implement"),
            "SQLx CRUD handler must not contain TODO stubs"
        );
        assert!(handler.contains("sqlx::query_as"));
        assert!(
            handler.contains("crate::db::items::{SINGULAR, TABLE}"),
            "generated handler must use per-table db module constants"
        );
        let db_rs = fs::read_to_string(project_path.join("src/db.rs")).expect("read db.rs");
        assert!(
            db_rs.contains("pub mod items"),
            "db.rs must define a per-resource items module"
        );

        let rustapi_path = workspace_root
            .join("crates/rustapi-rs")
            .display()
            .to_string()
            .replace('\\', "/");
        let testing_path = workspace_root
            .join("crates/rustapi-testing")
            .display()
            .to_string()
            .replace('\\', "/");

        let cargo_toml_path = project_path.join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");
        assert!(
            cargo_toml.contains(&format!("version = \"{workspace_version}\"")),
            "template must pin rustapi-rs to workspace version {workspace_version}"
        );
        cargo_toml = cargo_toml.replace(
            &format!("rustapi-rs = {{ version = \"{workspace_version}\""),
            &format!("rustapi-rs = {{ path = \"{rustapi_path}\""),
        );
        if !cargo_toml.contains("[lib]") {
            cargo_toml.push_str(&format!(
                r#"
[lib]
path = "src/lib.rs"

[dev-dependencies]
rustapi-testing = {{ path = "{testing_path}" }}
reqwest = {{ version = "0.12", default-features = false, features = ["json", "rustls-tls"] }}
"#
            ));
        }
        fs::write(&cargo_toml_path, cargo_toml).expect("write Cargo.toml");

        let lib_rs = r#"pub mod db;
pub mod handlers;
pub mod models;
"#;
        fs::write(project_path.join("src/lib.rs"), lib_rs).expect("write lib.rs");

        let main_rs = r#"use test_crud_sqlx::{db, handlers, models};
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let pool = db::init_pool("sqlite:items.db").await?;
    RustApi::auto()
        .state(pool)
        .run("127.0.0.1:8080")
        .await
}
"#;
        fs::write(project_path.join("src/main.rs"), main_rs).expect("write main.rs");

        fs::create_dir_all(project_path.join("tests")).expect("create tests dir");
        let e2e_test = r#"use rustapi_rs::prelude::*;
use test_crud_sqlx::db;
use test_crud_sqlx::handlers::items::{create, delete, get as get_one, list, update};
use std::time::Duration;
use tokio::sync::oneshot;

#[tokio::test]
async fn generated_items_routes_create_and_list_via_server() {
    let pool = db::init_pool("sqlite::memory:").await.expect("pool");
    let app = RustApi::new()
        .state(pool)
        .route("/items", get(list).post(create))
        .route("/items/{id}", get(get_one).put(update).delete(delete));

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);

    let addr = format!("127.0.0.1:{port}");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            shutdown_rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(400)).await;

    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");

    let create_res = client
        .post(format!("{base}/items"))
        .header("content-type", "application/json")
        .body("{\"name\":\"widget\",\"description\":\"demo\"}")
        .send()
        .await
        .expect("create request");
    assert_eq!(create_res.status(), 201, "POST /items should return 201");

    let list_res = client
        .get(format!("{base}/items"))
        .send()
        .await
        .expect("list request");
    assert_eq!(list_res.status(), 200, "GET /items should return 200");
    let body = list_res.text().await.expect("list body");
    assert!(body.contains("widget"), "list response must include created item");

    shutdown_tx.send(()).ok();
    server.await.expect("server task").expect("server run");
}
"#;
        fs::write(project_path.join("tests/crud_e2e.rs"), e2e_test).expect("write e2e test");

        let output = std::process::Command::new("cargo")
            .current_dir(&project_path)
            .args(["test", "--test", "crud_e2e", "--", "--nocapture"])
            .output()
            .expect("cargo test status");
        assert!(
            output.status.success(),
            "generated CRUD project must pass server e2e route test:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_generate_crud_multiple_resources_share_db_module() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-crud-multi";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .success();

        cargo_rustapi()
            .current_dir(&project_path)
            .args(["generate", "crud", "items"])
            .assert()
            .success();

        cargo_rustapi()
            .current_dir(&project_path)
            .args(["generate", "crud", "products"])
            .assert()
            .success();

        let db_rs = fs::read_to_string(project_path.join("src/db.rs")).expect("read db.rs");
        assert!(
            db_rs.contains("pub mod items"),
            "db.rs must track items table"
        );
        assert!(
            db_rs.contains("pub mod products"),
            "db.rs must track products table"
        );
        assert!(
            db_rs.matches("ensure_table(&pool").count() >= 2,
            "init_pool must ensure every generated table"
        );

        let products_handler = fs::read_to_string(project_path.join("src/handlers/products.rs"))
            .expect("read handler");
        assert!(
            products_handler.contains("crate::db::products::{SINGULAR, TABLE}"),
            "second resource must use its own db module"
        );
    }

    #[test]
    fn test_generate_model() {
        let dir = tempdir().expect("Failed to create temp dir");

        // First create a minimal project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-model", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Generate a model (model name is used as-is, should be PascalCase)
        cargo_rustapi()
            .current_dir(dir.path().join("test-model"))
            .args(["generate", "model", "User"])
            .assert()
            .success();

        // Model file is lowercase
        let model_path = dir.path().join("test-model/src/models/user.rs");
        assert!(model_path.exists(), "Model file should be created");

        let content = fs::read_to_string(&model_path).expect("Failed to read model");
        // The generate command uses the name as-is for struct name
        assert!(content.contains("struct User"));
        assert!(content.contains("impl User"));
    }
}

mod watch_command {
    use super::*;

    #[test]
    fn test_watch_help() {
        cargo_rustapi()
            .arg("watch")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Watch for changes"));
    }

    #[test]
    fn test_watch_accepts_command_flag() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--command"))
            .stdout(predicate::str::contains("--clear"));
    }

    #[test]
    fn test_watch_accepts_extension_filter() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--extensions"));
    }

    #[test]
    fn test_watch_accepts_path_filter() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--watch-path"));
    }
}

mod migrate_command {
    use super::*;

    #[test]
    fn test_migrate_help() {
        cargo_rustapi()
            .args(["migrate", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Database migration"));
    }

    #[test]
    fn test_migrate_run_help() {
        cargo_rustapi()
            .args(["migrate", "run", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("pending migrations"));
    }

    #[test]
    fn test_migrate_status_help() {
        cargo_rustapi()
            .args(["migrate", "status", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("migration status"));
    }

    #[test]
    fn test_migrate_create_help() {
        cargo_rustapi()
            .args(["migrate", "create", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("new migration"));
    }

    #[test]
    fn test_migrate_create_generates_files() {
        let dir = tempdir().expect("Failed to create temp dir");

        // Create a project first
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-migrate", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Create a migration
        cargo_rustapi()
            .current_dir(dir.path().join("test-migrate"))
            .args(["migrate", "create", "create_users_table"])
            .assert()
            .success();

        // Check migrations directory exists
        let migrations_dir = dir.path().join("test-migrate/migrations");
        assert!(migrations_dir.exists(), "migrations directory should exist");

        // Check that migration files were created
        let entries: Vec<_> = fs::read_dir(&migrations_dir)
            .expect("Failed to read migrations dir")
            .collect();
        assert!(!entries.is_empty(), "Migration files should be created");
    }

    #[test]
    fn test_migrate_revert_help() {
        cargo_rustapi()
            .args(["migrate", "revert", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Revert"));
    }
}

#[cfg(feature = "cloud")]
mod deploy_command {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    fn spawn_mock_deploy_status_server(deploy_id: &str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock cloud");
        let port = listener.local_addr().expect("addr").port();
        let cloud_url = format!("http://127.0.0.1:{port}");
        let id = deploy_id.to_string();
        let hits = Arc::new(AtomicUsize::new(0));

        let handle = thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while hits.load(Ordering::SeqCst) < 2 && std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 4096];
                    let _ = std::io::Read::read(&mut stream, &mut buf);
                    let body = format!(
                        r#"{{"deploy_id":"{id}","status":"live","url":"http://127.0.0.1:59999"}}"#
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    hits.fetch_add(1, Ordering::SeqCst);
                } else {
                    thread::sleep(Duration::from_millis(20));
                }
            }
        });

        (cloud_url, handle)
    }

    fn write_cloud_config_file(path: &std::path::Path, cloud_url: &str, token: &str) {
        let config = serde_json::json!({
            "token": token,
            "cloud_url": cloud_url,
            "user": { "login": "cli-test", "tier": "hobby" }
        });
        fs::write(path, serde_json::to_string_pretty(&config).expect("json"))
            .expect("write config");
    }

    #[test]
    fn test_deploy_status_help() {
        cargo_rustapi()
            .args(["deploy", "status", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Deploy ID"));
    }

    #[test]
    fn test_deploy_cloud_help() {
        cargo_rustapi()
            .args(["deploy", "cloud", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("RustAPI Cloud"));
    }

    #[test]
    fn test_deploy_status_fetches_live_response_from_cloud() {
        let deploy_id = "cli-mock-deploy-1";
        let (cloud_url, server) = spawn_mock_deploy_status_server(deploy_id);
        let dir = tempdir().expect("config dir");
        let config_path = dir.path().join("cloud-config.json");
        write_cloud_config_file(&config_path, &cloud_url, "mock-jwt-token");

        cargo_rustapi()
            .env("RUSTAPI_CONFIG_PATH", &config_path)
            .args(["deploy", "status", deploy_id])
            .assert()
            .success()
            .stdout(predicate::str::contains(deploy_id))
            .stdout(predicate::str::contains("live"))
            .stdout(predicate::str::contains("URL:"))
            .stdout(predicate::str::contains("http://127.0.0.1:59999"));

        server.join().expect("mock server thread");
    }
}
