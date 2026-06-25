//! Steps 3-4 of the verification plan when invoked with VERIFY_SCRATCH set.
//! Boots real HTTP server, POST multipart deploy, polls to live, then runs
//! `cargo run -p cargo-rustapi --features cloud -- deploy status <id>` twice.
//! Writes deploy-flow.log and cli-status.log only after full success.
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::Context;

use diesel_async::RunQueryDsl;
use reqwest::header::HeaderMap;
use rustapi_cloud::auth::jwt;
use rustapi_cloud::config::Config;
use rustapi_cloud::db::create_pool;
use rustapi_cloud::models::NewUser;
use rustapi_cloud::schema::users;
use tokio::time::sleep;

fn push_http_transcript(
    flow: &mut Vec<String>,
    label: &str,
    method: &str,
    url: &str,
    request_headers: &[(&str, &str)],
    status: reqwest::StatusCode,
    response_headers: &HeaderMap,
    body: &str,
) {
    let mut transcript = String::new();
    let _ = writeln!(transcript, "--- {label} ---");
    let _ = writeln!(transcript, ">>> REQUEST");
    let _ = writeln!(transcript, "{method} {url}");
    for (name, value) in request_headers {
        let _ = writeln!(transcript, "{name}: {value}");
    }
    let _ = writeln!(transcript);
    let _ = writeln!(transcript, "<<< RESPONSE");
    let _ = writeln!(transcript, "HTTP/1.1 {status}");
    for (name, value) in response_headers {
        let _ = writeln!(
            transcript,
            "{}: {}",
            name,
            value.to_str().unwrap_or("<non-utf8>")
        );
    }
    let _ = writeln!(transcript);
    let _ = writeln!(transcript, "{body}");
    flow.push(transcript);
}

fn redact_bearer(token: &str) -> String {
    if token.len() <= 16 {
        return "Bearer <redacted>".into();
    }
    format!("Bearer {}...{}", &token[..8], &token[token.len() - 8..])
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let scratch = std::env::var("VERIFY_SCRATCH").expect("VERIFY_SCRATCH must be set");
    let scratch = PathBuf::from(scratch);
    std::fs::create_dir_all(&scratch)?;

    let mut flow = Vec::new();

    flow.push("==> verify-deploy-e2e: boot real server (build_app.run)".into());
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            flow.push(format!("DATABASE_URL={url} (from env)"));
            url
        }
        Err(_) => {
            flow.push("DATABASE_URL unset - starting pg-embed".into());
            let url = rustapi_cloud::verify_db::embedded_database_url().await?;
            flow.push(format!("DATABASE_URL={url} (pg-embed)"));
            url
        }
    };

    let schema = rustapi_cloud::verify_db::dump_schema(&database_url).await?;
    let schema_path = scratch.join("db-schema.txt");
    if schema_path.exists() {
        let existing = std::fs::read_to_string(&schema_path)?;
        std::fs::write(
            &schema_path,
            format!("{existing}\n-- pg-embed schema dump (step 3)\n{schema}"),
        )?;
    } else {
        std::fs::write(&schema_path, format!("-- pg-embed schema dump\n{schema}"))?;
    }

    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into());
    let storage = scratch.join("verify-storage");
    std::fs::create_dir_all(&storage)?;

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let config = Config {
        database_url: database_url.clone(),
        host: "127.0.0.1".into(),
        port,
        jwt_secret: jwt_secret.clone(),
        github_client_id: "test".into(),
        github_client_secret: "test".into(),
        github_redirect_uri: format!("http://127.0.0.1:{port}/auth/callback"),
        storage_root: storage.to_string_lossy().into(),
    };

    let pool = create_pool(&database_url).await;
    let (user_id, token) = insert_user_and_mint_token(&pool, &jwt_secret).await?;
    flow.push(format!("MINTED_JWT user_id={user_id}"));

    let app = rustapi_cloud::build_app(config, pool);
    let addr = format!("127.0.0.1:{port}");
    let cloud_url = format!("http://{addr}");
    flow.push(format!("SERVER_ADDR={cloud_url}"));

    let server = tokio::spawn(async move {
        app.run(&addr).await.expect("server run");
    });
    sleep(Duration::from_millis(800)).await;

    let fixture = build_fixture_binary()?;
    let bytes = std::fs::read(&fixture)?;
    flow.push(format!(
        "FIXTURE_BYTES={} path={}",
        bytes.len(),
        fixture.display()
    ));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let form = reqwest::multipart::Form::new()
        .text("project_name", "listener-app")
        .part(
            "binary",
            reqwest::multipart::Part::bytes(bytes).file_name("listener.bin"),
        );

    let deploy_url = format!("{cloud_url}/deploy");
    let auth_header = redact_bearer(&token);
    let created = client
        .post(&deploy_url)
        .header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .send()
        .await?;
    let create_status = created.status();
    let create_headers = created.headers().clone();
    let create_body = created.text().await?;
    push_http_transcript(
        &mut flow,
        "POST /deploy",
        "POST",
        &deploy_url,
        &[
            ("Authorization", &auth_header),
            (
                "Content-Type",
                "multipart/form-data; fields=project_name,binary",
            ),
        ],
        create_status,
        &create_headers,
        &create_body,
    );

    let created_json: serde_json::Value = serde_json::from_str(&create_body)?;
    let deploy_id = created_json["deploy_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing deploy_id"))?
        .to_string();

    let mut final_status = None;
    for attempt in 0..40 {
        sleep(Duration::from_millis(500)).await;
        let status_url = format!("{cloud_url}/deploy/{deploy_id}/status");
        let status_resp = client
            .get(&status_url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;
        let poll_status = status_resp.status();
        let poll_headers = status_resp.headers().clone();
        let poll_body = status_resp.text().await?;

        push_http_transcript(
            &mut flow,
            &format!("GET /deploy/{{id}}/status (poll {attempt})"),
            "GET",
            &status_url,
            &[("Authorization", &auth_header)],
            poll_status,
            &poll_headers,
            &poll_body,
        );

        let status_json: serde_json::Value = serde_json::from_str(&poll_body)?;
        let status = status_json["status"].as_str().unwrap_or("");
        if status == "live" || status == "running" {
            flow.push(format!(
                "POLL_STOPPED_AT attempt={attempt} reason=status_{status} (no further polls required)"
            ));
            final_status = Some(status_json);
            break;
        }
        if status == "failed" {
            anyhow::bail!("deploy failed: {poll_body}");
        }
    }
    let final_status = final_status.ok_or_else(|| anyhow::anyhow!("deploy timed out"))?;
    flow.push(format!(
        "DEPLOY_FLOW_OK deploy_id={deploy_id} status={} url={}",
        final_status["status"].as_str().unwrap_or("?"),
        final_status["url"].as_str().unwrap_or("?")
    ));

    std::fs::write(
        scratch.join("deploy-state.json"),
        serde_json::json!({
            "deploy_id": deploy_id,
            "status": final_status["status"],
            "url": final_status["url"],
            "cloud_url": cloud_url,
            "token": token,
        })
        .to_string(),
    )?;

    let mut cli_log = String::new();
    let rustapi_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .context("resolve RustAPI repo root from in-tree RustAPI-Cloud mirror")?;
    let cli_config = scratch.join("cli-config.json");
    std::fs::write(
        &cli_config,
        serde_json::to_string_pretty(&serde_json::json!({
            "token": token,
            "cloud_url": cloud_url,
            "user": { "login": "verify-e2e", "tier": "hobby" }
        }))?,
    )?;

    let build = Command::new("cargo")
        .args(["build", "-p", "cargo-rustapi", "--features", "cloud"])
        .current_dir(&rustapi_root)
        .output()?;
    writeln!(cli_log, "cargo build cargo-rustapi exit={}", build.status)?;
    if !build.status.success() {
        anyhow::bail!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
    }

    for run in 1..=2 {
        writeln!(
            cli_log,
            "=== cargo run -p cargo-rustapi --features cloud -- deploy status {deploy_id} (run {run}) ==="
        )?;
        let output = Command::new("cargo")
            .args([
                "run",
                "-p",
                "cargo-rustapi",
                "--features",
                "cloud",
                "--",
                "deploy",
                "status",
                &deploy_id,
            ])
            .current_dir(&rustapi_root)
            .stdin(Stdio::null())
            .env("RUSTAPI_CONFIG_PATH", &cli_config)
            .output()?;
        writeln!(cli_log, "exit={}", output.status)?;
        writeln!(
            cli_log,
            "stdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )?;
        writeln!(
            cli_log,
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )?;
        if !output.status.success() {
            anyhow::bail!("cargo run deploy status run {run} failed");
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains(&deploy_id) || !stdout.contains("URL:") || !stdout.contains("live") {
            anyhow::bail!("unexpected cli stdout run {run}");
        }
    }

    server.abort();
    std::fs::write(scratch.join("deploy-flow.log"), flow.join("\n"))?;
    std::fs::write(scratch.join("cli-status.log"), cli_log)?;
    std::fs::write(
        scratch.join("verify-e2e-console.log"),
        format!("VERIFY_E2E_OK scratch={}\n", scratch.display()),
    )?;
    println!("VERIFY_E2E_OK scratch={}", scratch.display());
    Ok(())
}

async fn insert_user_and_mint_token(
    pool: &rustapi_cloud::db::DbPool,
    jwt_secret: &str,
) -> anyhow::Result<(String, String)> {
    let mut conn = pool.get().await?;
    let user = NewUser::from_github(
        424242,
        "verify-e2e".into(),
        None,
        Some("verify@example.com".into()),
    );
    let user_id = user.id.clone();
    let _ = diesel::insert_into(users::table)
        .values(&user)
        .execute(&mut conn)
        .await;
    let (token, _) = jwt::create_token(&user_id, "verify-e2e", None, "hobby", jwt_secret, 24)?;
    Ok((user_id, token))
}

fn build_fixture_binary() -> anyhow::Result<PathBuf> {
    let manifest =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/port-listener/Cargo.toml");
    let status = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(&manifest)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("fixture build failed");
    }
    let mut path = manifest
        .parent()
        .unwrap()
        .join("target/release/port-listener");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    Ok(path)
}