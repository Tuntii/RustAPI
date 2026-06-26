//! Deployment Commands
//!
//! Generate deployment configurations and deploy to various platforms.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::load_config;

/// Arguments for deployment commands
#[derive(Subcommand, Debug)]
pub enum DeployArgs {
    /// Deploy to RustAPI Cloud (managed hosting)
    #[cfg(feature = "cloud")]
    Cloud(CloudArgs),

    /// Check status of a RustAPI Cloud deployment
    #[cfg(feature = "cloud")]
    Status(DeployStatusArgs),

    /// Generate a Dockerfile for the project
    Docker(DockerArgs),

    /// Deploy to Fly.io
    Fly(FlyArgs),

    /// Deploy to Railway
    Railway(RailwayArgs),

    /// Deploy to Shuttle.rs
    Shuttle(ShuttleArgs),
}

#[cfg(feature = "cloud")]
#[derive(Args, Debug)]
pub struct DeployStatusArgs {
    /// Deploy ID returned by `deploy cloud`
    pub deploy_id: String,
}

#[cfg(feature = "cloud")]
const CLOUD_LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";

#[cfg(feature = "cloud")]
#[derive(Args, Debug)]
pub struct CloudArgs {
    /// Project name (defaults to Cargo.toml package name)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Cross-compile target (default: x86_64-unknown-linux-gnu when not on Linux)
    #[arg(long)]
    pub target: Option<String>,

    /// Do not wait for deployment to complete
    #[arg(long)]
    pub no_wait: bool,
}

#[derive(Args, Debug)]
pub struct DockerArgs {
    /// Output path for Dockerfile
    #[arg(short, long, default_value = "./Dockerfile")]
    pub output: PathBuf,

    /// Rust toolchain version
    #[arg(long, default_value = "1.78")]
    pub rust_version: String,

    /// Binary name (defaults to package name)
    #[arg(short, long)]
    pub binary: Option<String>,

    /// Port to expose
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
}

#[derive(Args, Debug)]
pub struct FlyArgs {
    /// Application name
    #[arg(short, long)]
    pub app: Option<String>,

    /// Region to deploy to
    #[arg(short, long, default_value = "iad")]
    pub region: String,

    /// Initialize only (don't deploy)
    #[arg(long)]
    pub init_only: bool,
}

#[derive(Args, Debug)]
pub struct RailwayArgs {
    /// Project name
    #[arg(short, long)]
    pub project: Option<String>,

    /// Environment (production, staging)
    #[arg(short, long, default_value = "production")]
    pub environment: String,
}

#[derive(Args, Debug)]
pub struct ShuttleArgs {
    /// Project name
    #[arg(short, long)]
    pub project: Option<String>,

    /// Initialize only
    #[arg(long)]
    pub init_only: bool,
}

/// Execute deployment command
pub async fn deploy(args: DeployArgs) -> Result<()> {
    match args {
        #[cfg(feature = "cloud")]
        DeployArgs::Cloud(cloud_args) => deploy_cloud(cloud_args).await,
        #[cfg(feature = "cloud")]
        DeployArgs::Status(status_args) => deploy_status(status_args).await,
        DeployArgs::Docker(docker_args) => generate_dockerfile(docker_args).await,
        DeployArgs::Fly(fly_args) => deploy_fly(fly_args).await,
        DeployArgs::Railway(railway_args) => deploy_railway(railway_args).await,
        DeployArgs::Shuttle(shuttle_args) => deploy_shuttle(shuttle_args).await,
    }
}

async fn generate_dockerfile(args: DockerArgs) -> Result<()> {
    println!("🐳 Generating Dockerfile...");

    // Try to get package name from Cargo.toml
    let binary_name = args
        .binary
        .unwrap_or_else(|| get_package_name().unwrap_or_else(|_| "app".to_string()));

    let dockerfile = format!(
        r#"# Build stage
FROM rust:{rust_version}-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates

# Build dependencies (this layer will be cached)
RUN mkdir src && echo "fn main() {{}}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/{binary_name} /usr/local/bin/app

# Expose port
EXPOSE {port}

# Set environment variables
ENV RUST_LOG=info
ENV PORT={port}

# Run the application
CMD ["app"]
"#,
        rust_version = args.rust_version,
        binary_name = binary_name,
        port = args.port
    );

    fs::write(&args.output, dockerfile).context("Failed to write Dockerfile")?;

    println!("✅ Dockerfile generated at: {}", args.output.display());
    println!();
    println!("Build and run with:");
    println!("  docker build -t myapp .");
    println!("  docker run -p {}:{} myapp", args.port, args.port);

    Ok(())
}

async fn deploy_fly(args: FlyArgs) -> Result<()> {
    println!("✈️  Deploying to Fly.io...");

    let app_name = args
        .app
        .unwrap_or_else(|| get_package_name().unwrap_or_else(|_| "rustapi-app".to_string()));

    // Generate fly.toml
    let fly_toml = format!(
        r#"# Fly.io configuration
# Generated by RustAPI CLI

app = "{app_name}"
primary_region = "{region}"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[vm]]
  memory = "256mb"
  cpu_kind = "shared"
  cpus = 1
"#,
        app_name = app_name,
        region = args.region
    );

    fs::write("fly.toml", &fly_toml).context("Failed to write fly.toml")?;

    println!("✅ fly.toml generated");

    if args.init_only {
        println!();
        println!("To deploy, run:");
        println!("  fly launch");
        println!("  fly deploy");
    } else {
        println!();
        println!("To complete deployment:");
        println!("  1. Install flyctl: curl -L https://fly.io/install.sh | sh");
        println!("  2. Login: fly auth login");
        println!("  3. Launch: fly launch");
        println!("  4. Deploy: fly deploy");
    }

    Ok(())
}

async fn deploy_railway(args: RailwayArgs) -> Result<()> {
    println!("🚂 Deploying to Railway...");

    let project_name = args
        .project
        .unwrap_or_else(|| get_package_name().unwrap_or_else(|_| "rustapi-app".to_string()));

    // Generate railway.toml
    let railway_toml = r#"# Railway configuration
# Generated by RustAPI CLI

[build]
builder = "dockerfile"
dockerfilePath = "Dockerfile"

[deploy]
numReplicas = 1
healthcheckPath = "/health"
healthcheckTimeout = 100
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 3
"#
    .to_string();

    fs::write("railway.toml", &railway_toml).context("Failed to write railway.toml")?;

    println!("✅ railway.toml generated for: {}", project_name);
    println!();
    println!("To deploy:");
    println!("  1. Install Railway CLI: npm i -g @railway/cli");
    println!("  2. Login: railway login");
    println!("  3. Link project: railway link");
    println!("  4. Deploy: railway up");

    Ok(())
}

async fn deploy_shuttle(args: ShuttleArgs) -> Result<()> {
    println!("🚀 Setting up Shuttle.rs...");

    let project_name = args
        .project
        .unwrap_or_else(|| get_package_name().unwrap_or_else(|_| "rustapi-app".to_string()));

    // Generate Shuttle.toml
    let shuttle_toml = format!(
        r#"# Shuttle configuration
# Generated by RustAPI CLI

name = "{project_name}"
"#
    );

    fs::write("Shuttle.toml", &shuttle_toml).context("Failed to write Shuttle.toml")?;

    println!("✅ Shuttle.toml generated");
    println!();
    println!("⚠️  Note: Shuttle requires code modifications to use their runtime.");
    println!();
    println!("To deploy:");
    println!("  1. Install Shuttle CLI: cargo install cargo-shuttle");
    println!("  2. Login: cargo shuttle login");
    println!("  3. Init: cargo shuttle init");
    println!("  4. Deploy: cargo shuttle deploy");

    Ok(())
}

fn get_package_name() -> Result<String> {
    let cargo_toml = fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;

    for line in cargo_toml.lines() {
        if line.starts_with("name") {
            if let Some(name) = line.split('=').nth(1) {
                return Ok(name.trim().trim_matches('"').to_string());
            }
        }
    }

    anyhow::bail!("Could not find package name in Cargo.toml")
}

// --- Cloud Deploy ---

#[cfg(feature = "cloud")]
#[derive(Deserialize)]
struct DeployResponse {
    deploy_id: String,
    status: String,
    #[serde(default)]
    url: Option<String>,
}

#[cfg(feature = "cloud")]
fn resolve_cloud_build_target(override_target: Option<&str>) -> Result<Option<String>> {
    if let Some(target) = override_target {
        return Ok(Some(target.to_string()));
    }
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Ok(None)
    } else {
        Ok(Some(CLOUD_LINUX_TARGET.to_string()))
    }
}

#[cfg(feature = "cloud")]
fn ensure_rustup_target(target: &str) -> Result<()> {
    let output = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to run rustup")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("rustup target list failed"));
    }

    let installed = String::from_utf8_lossy(&output.stdout);
    if installed.lines().any(|line| line.trim() == target) {
        return Ok(());
    }

    println!("  📥 Installing Rust target {target}...");
    let status = std::process::Command::new("rustup")
        .args(["target", "add", target])
        .status()
        .context("Failed to run rustup target add")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to install target {target}"));
    }

    Ok(())
}

#[cfg(feature = "cloud")]
fn cargo_subcommand_available(subcommand: &str) -> bool {
    std::process::Command::new("cargo")
        .arg(subcommand)
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(feature = "cloud")]
fn zig_runs(path: &str) -> bool {
    std::process::Command::new(path)
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(feature = "cloud")]
fn locate_zig() -> Option<String> {
    if zig_runs("zig") {
        return Some("zig".to_string());
    }

    #[cfg(windows)]
    {
        let mut candidates = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(format!(r"{local}\Microsoft\WinGet\Links\zig.exe"));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            candidates.push(format!(r"{home}\scoop\shims\zig.exe"));
        }
        for candidate in candidates {
            if zig_runs(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(feature = "cloud")]
fn prepend_path(cmd: &mut std::process::Command, dir: &std::path::Path) {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let existing = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let merged = if existing.is_empty() {
        dir.display().to_string()
    } else {
        format!("{}{}{}", dir.display(), sep, existing)
    };
    cmd.env("PATH", merged);
}

#[cfg(feature = "cloud")]
fn docker_daemon_ready() -> bool {
    std::process::Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(all(feature = "cloud", windows))]
fn wsl_has_cargo() -> bool {
    std::process::Command::new("wsl")
        .args(["cargo", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(all(feature = "cloud", windows))]
fn windows_path_for_wsl(path: &std::path::Path) -> Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Failed to resolve project path {}", path.display()))?;
    let text = canonical.display().to_string();
    let drive = text
        .chars()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid project path"))?
        .to_ascii_lowercase();
    let rest = text.get(2..).unwrap_or("").replace('\\', "/");
    Ok(format!("/mnt/{drive}{rest}"))
}

#[cfg(feature = "cloud")]
struct CloudBuildResult {
    success: bool,
    stderr: String,
    /// `Some(target)` for cross-compile output layout; `None` for Linux-native (Docker/WSL).
    binary_target: Option<String>,
}

#[cfg(feature = "cloud")]
fn run_cargo_command(args: &[&str], binary: &str) -> Result<std::process::Output> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(args).args(["--bin", binary]);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.output()
        .with_context(|| format!("Failed to run cargo {}", args.first().unwrap_or(&"")))
}

#[cfg(feature = "cloud")]
fn run_zigbuild(target: &str, zig: &str, binary: &str) -> Result<std::process::Output> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["zigbuild", "--release", "--target", target, "--bin", binary]);
    if zig != "zig" {
        prepend_path(
            &mut cmd,
            std::path::Path::new(zig)
                .parent()
                .unwrap_or_else(|| ".".as_ref()),
        );
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.output().context("Failed to run cargo zigbuild")
}

#[cfg(feature = "cloud")]
fn run_docker_linux_build(
    project_dir: &std::path::Path,
    binary: &str,
) -> Result<std::process::Output> {
    let mount = project_dir
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", project_dir.display()))?;
    let script = format!(
        "set -euo pipefail; \
        apt-get update -qq && apt-get install -y -qq pkg-config libssl-dev >/dev/null; \
        cargo build --release --bin {binary}"
    );
    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/app", mount.display()),
            "-w",
            "/app",
            "rust:1-bookworm",
            "bash",
            "-lc",
            &script,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("Failed to run docker build")?;
    Ok(output)
}

#[cfg(all(feature = "cloud", windows))]
fn run_wsl_linux_build(
    project_dir: &std::path::Path,
    binary: &str,
) -> Result<std::process::Output> {
    let wsl_path = windows_path_for_wsl(project_dir)?;
    let script = format!("cd '{wsl_path}' && cargo build --release --bin {binary}");
    std::process::Command::new("wsl")
        .args(["bash", "-lc", &script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("Failed to run WSL cargo build")
}

#[cfg(feature = "cloud")]
fn run_cloud_release_build(target: Option<&str>, binary: &str) -> Result<CloudBuildResult> {
    if target.is_none() {
        let output = run_cargo_command(&["build", "--release"], binary)?;
        return Ok(CloudBuildResult {
            success: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            binary_target: None,
        });
    }

    let target = target.unwrap();
    let mut errors = Vec::new();

    if cargo_subcommand_available("zigbuild") {
        if let Some(zig) = locate_zig() {
            println!("  🦎 Cross-compiling with Zig ({target})...");
            let output = run_zigbuild(target, &zig, binary)?;
            if output.status.success() {
                return Ok(CloudBuildResult {
                    success: true,
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    binary_target: Some(target.to_string()),
                });
            }
            errors.push(format!(
                "Zig cross-compile failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        } else {
            errors.push(
                "Zig not found in PATH (install: winget install zig.zig, or https://ziglang.org/download/)"
                    .to_string(),
            );
        }
    } else {
        errors.push("cargo-zigbuild not installed (cargo install cargo-zigbuild)".to_string());
    }

    if docker_daemon_ready() {
        println!("  🐳 Building Linux binary via Docker...");
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let output = run_docker_linux_build(&cwd, binary)?;
        if output.status.success() {
            return Ok(CloudBuildResult {
                success: true,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                binary_target: None,
            });
        }
        errors.push(format!(
            "Docker build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    #[cfg(windows)]
    if wsl_has_cargo() {
        println!("  🐧 Building Linux binary via WSL...");
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let output = run_wsl_linux_build(&cwd, binary)?;
        if output.status.success() {
            return Ok(CloudBuildResult {
                success: true,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                binary_target: None,
            });
        }
        errors.push(format!(
            "WSL build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(CloudBuildResult {
        success: false,
        stderr: errors.join("\n\n"),
        binary_target: Some(target.to_string()),
    })
}

#[cfg(feature = "cloud")]
fn cross_compile_hint(target: Option<&str>) -> &'static str {
    if target.is_none() {
        return "";
    }
    "\nHint: RustAPI Cloud needs a Linux binary. From Windows/macOS we try automatically:\n\
     • Zig cross-compile (cargo-zigbuild + zig in PATH)\n\
     • Docker (start Docker Desktop if installed)\n\
     • WSL with Rust installed\n\
     Projects using OpenSSL (reqwest native-tls) usually need Docker or WSL.\n\
     Or deploy from a Linux machine."
}

#[cfg(feature = "cloud")]
fn cloud_binary_path(project_name: &str, target: Option<&str>) -> PathBuf {
    let mut path = PathBuf::from("target");
    if let Some(t) = target {
        path.push(t);
    }
    path.push("release");
    path.push(project_name);
    path
}

#[cfg(feature = "cloud")]
async fn deploy_cloud(args: CloudArgs) -> Result<()> {
    let config = load_config()?;

    let token = config.token.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Not logged in. Run `cargo rustapi login` or `cargo-rustapi login` first.")
    })?;

    let cloud_url = config
        .cloud_url
        .as_deref()
        .unwrap_or("https://api.rustapi.cloud")
        .trim_end_matches('/');

    let project_name = args
        .name
        .unwrap_or_else(|| get_package_name().unwrap_or_else(|_| "rustapi-app".to_string()));

    // Verify project uses RustAPI
    let cargo_toml =
        fs::read_to_string("Cargo.toml").context("Not in a Rust project (no Cargo.toml found)")?;

    if !cargo_toml.contains("rustapi") {
        println!("  ⚠️  This project doesn't appear to use RustAPI.");
        println!("  Continuing anyway...");
    }

    let build_target = resolve_cloud_build_target(args.target.as_deref())?;
    if let Some(target) = &build_target {
        ensure_rustup_target(target)?;
        println!("  🔨 Building {} for {} (release)...", project_name, target);
    } else {
        println!("  🔨 Building {} (release)...", project_name);
    }

    let build = run_cloud_release_build(build_target.as_deref(), &project_name)?;

    if !build.success {
        let hint = cross_compile_hint(build_target.as_deref());
        return Err(anyhow::anyhow!("Build failed:\n{}{}", build.stderr, hint));
    }

    println!("  ✅ Build complete");

    let binary_path = cloud_binary_path(&project_name, build.binary_target.as_deref());
    if !binary_path.exists() {
        return Err(anyhow::anyhow!(
            "Binary not found at {}. Make sure the package name matches the binary name.",
            binary_path.display()
        ));
    }

    // Package binary
    println!("  📦 Packaging...");
    let binary_data = fs::read(&binary_path).context("Failed to read binary")?;

    // Upload
    println!("  ☁️  Uploading to RustAPI Cloud...");

    let upload_mb = binary_data.len() as f64 / (1024.0 * 1024.0);
    if upload_mb > 1.0 {
        println!("  📤 Uploading {:.1} MB...", upload_mb);
    }

    let client = cloud_upload_client(binary_data.len())?;
    let form = reqwest::multipart::Form::new()
        .text("project_name", project_name.clone())
        .part(
            "binary",
            reqwest::multipart::Part::bytes(binary_data).file_name(format!("{}.bin", project_name)),
        );

    let deploy_resp: DeployResponse = client
        .post(format!("{}/deploy", cloud_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to upload to RustAPI Cloud ({cloud_url}/deploy). \
                 If the connection timed out, check your upload speed or try again."
            )
        })?
        .json()
        .await
        .context("Invalid response from deploy endpoint")?;

    let deploy_id = deploy_resp.deploy_id;

    if args.no_wait {
        println!("  ✅ Deploy queued: {}", deploy_id);
        println!("  Check status: cargo rustapi deploy status {}", deploy_id);
        return Ok(());
    }

    // Poll for status
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("  {spinner} Deploying...").unwrap());
    spinner.enable_steady_tick(Duration::from_millis(100));

    for _ in 0..120 {
        tokio::time::sleep(Duration::from_secs(3)).await;

        let status_resp: DeployResponse =
            match fetch_deploy_status(cloud_url, token, &deploy_id).await {
                Ok(body) => body,
                Err(_) => continue,
            };

        match status_resp.status.as_str() {
            "running" | "live" => {
                spinner.finish_and_clear();
                println!(
                    "  🚀 Deployed: {}",
                    status_resp.url.as_deref().unwrap_or("(url pending)")
                );
                return Ok(());
            }
            "failed" => {
                spinner.finish_with_message("Deploy failed");
                return Err(anyhow::anyhow!(
                    "Deployment failed. Check logs in the dashboard."
                ));
            }
            _ => continue,
        }
    }

    spinner.finish_with_message("Deploy timed out");
    println!("  Deploy ID: {} (still processing)", deploy_id);
    println!("  Check: cargo rustapi deploy status {}", deploy_id);

    Ok(())
}

#[cfg(feature = "cloud")]
fn cloud_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")
}

#[cfg(feature = "cloud")]
fn upload_timeout_secs(binary_bytes: usize) -> u64 {
    // ~13 MB needs more than 10s on typical home uplinks; scale with payload size.
    let min_secs = 120u64;
    let size_secs = (binary_bytes as u64 / (256 * 1024)).max(1);
    min_secs.max(size_secs).min(600)
}

#[cfg(feature = "cloud")]
fn cloud_upload_client(binary_bytes: usize) -> Result<reqwest::Client> {
    let timeout_secs = upload_timeout_secs(binary_bytes);
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to build HTTP upload client")
}

#[cfg(feature = "cloud")]
async fn fetch_deploy_status(
    cloud_url: &str,
    token: &str,
    deploy_id: &str,
) -> Result<DeployResponse> {
    let client = cloud_http_client()?;
    let response = client
        .get(format!("{}/deploy/{}/status", cloud_url, deploy_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to connect to RustAPI Cloud")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Deploy status request failed ({}): {}",
            status,
            body
        ));
    }

    response
        .json::<DeployResponse>()
        .await
        .context("Invalid response from deploy status endpoint")
}

#[cfg(all(feature = "cloud", test))]
mod status_tests {
    use super::{cloud_binary_path, upload_timeout_secs, DeployResponse, CLOUD_LINUX_TARGET};

    #[test]
    fn cloud_binary_path_uses_target_dir_for_cross_compile() {
        let path = cloud_binary_path("hello-rustapi", Some(CLOUD_LINUX_TARGET));
        assert!(path.to_string_lossy().contains("x86_64-unknown-linux-gnu"));
        assert!(path.to_string_lossy().ends_with("hello-rustapi"));
    }

    #[test]
    fn deploy_response_matches_cloud_api_shape() {
        let json = r#"{"deploy_id":"d-1","status":"live","url":"http://127.0.0.1:30001"}"#;
        let parsed: DeployResponse = serde_json::from_str(json).expect("shape");
        assert_eq!(parsed.deploy_id, "d-1");
        assert_eq!(parsed.status, "live");
        assert_eq!(parsed.url.as_deref(), Some("http://127.0.0.1:30001"));
    }

    #[test]
    fn upload_timeout_scales_with_binary_size() {
        assert_eq!(upload_timeout_secs(1024), 120);
        assert_eq!(upload_timeout_secs(13 * 1024 * 1024), 120);
        assert!(upload_timeout_secs(200 * 1024 * 1024) <= 600);
    }
}

#[cfg(feature = "cloud")]
async fn deploy_status(args: DeployStatusArgs) -> Result<()> {
    let config = load_config()?;

    let token = config.token.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Not logged in. Run `cargo rustapi login` or `cargo-rustapi login` first.")
    })?;

    let cloud_url = config
        .cloud_url
        .as_deref()
        .unwrap_or("https://api.rustapi.cloud")
        .trim_end_matches('/');

    let status = fetch_deploy_status(cloud_url, token, &args.deploy_id).await?;

    println!("  Deploy ID: {}", status.deploy_id);
    println!("  Status:    {}", status.status);
    if let Some(url) = status.url {
        println!("  URL:       {}", url);
    }

    Ok(())
}
