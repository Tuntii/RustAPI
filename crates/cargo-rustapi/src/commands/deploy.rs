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
#[derive(Args, Debug)]
pub struct CloudArgs {
    /// Project name (defaults to Cargo.toml package name)
    #[arg(short, long)]
    pub name: Option<String>,

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
async fn deploy_cloud(args: CloudArgs) -> Result<()> {
    let config = load_config()?;

    let token = config
        .token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not logged in. Run `rustapi login` first."))?;

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

    // Build
    println!("  🔨 Building {} (release)...", project_name);

    let build = std::process::Command::new("cargo")
        .args(["build", "--release"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start cargo build")?;

    let output = build
        .wait_with_output()
        .context("Failed to wait for cargo build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Build failed:\n{}", stderr));
    }

    println!("  ✅ Build complete");

    // Find binary
    let binary_path = PathBuf::from("target/release")
        .join(&project_name)
        .with_extension(std::env::consts::EXE_SUFFIX);

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

    let client = cloud_http_client()?;
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
        .context("Failed to connect to RustAPI Cloud")?
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
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to build HTTP client")
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
    use super::DeployResponse;

    #[test]
    fn deploy_response_matches_cloud_api_shape() {
        let json = r#"{"deploy_id":"d-1","status":"live","url":"http://127.0.0.1:30001"}"#;
        let parsed: DeployResponse = serde_json::from_str(json).expect("shape");
        assert_eq!(parsed.deploy_id, "d-1");
        assert_eq!(parsed.status, "live");
        assert_eq!(parsed.url.as_deref(), Some("http://127.0.0.1:30001"));
    }
}

#[cfg(feature = "cloud")]
async fn deploy_status(args: DeployStatusArgs) -> Result<()> {
    let config = load_config()?;

    let token = config
        .token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not logged in. Run `rustapi login` first."))?;

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
