//! Run command for development server

use anyhow::Result;
use clap::Args;
use console::style;
use std::process::Stdio;
use tokio::process::Command;

/// Arguments for the `run` command
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Port to run on
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Additional features to enable
    #[arg(short, long, value_delimiter = ',')]
    pub features: Option<Vec<String>>,

    /// Release mode
    #[arg(long)]
    pub release: bool,

    /// Watch for changes and auto-reload
    #[arg(short, long)]
    pub watch: bool,
}

/// Run the development server
pub async fn run_dev(args: RunArgs) -> Result<()> {
    // Set environment variables
    std::env::set_var("PORT", args.port.to_string());
    std::env::set_var("RUSTAPI_ENV", "development");

    println!("{}", style("Starting RustAPI development server...").bold());
    println!();

    if args.watch {
        // Use cargo-watch if available
        run_with_watch(&args).await
    } else {
        run_cargo(&args).await
    }
}

async fn run_cargo(args: &RunArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");

    if args.release {
        cmd.arg("--release");
    }

    if let Some(features) = &args.features {
        cmd.arg("--features").arg(features.join(","));
    }

    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("cargo run failed");
    }

    Ok(())
}

async fn run_with_watch(args: &RunArgs) -> Result<()> {
    // Check if cargo-watch is installed
    let check = Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .await;

    if check.is_err() || !check.unwrap().status.success() {
        println!("{}", style("cargo-watch not found. Installing...").yellow());
        
        let install = Command::new("cargo")
            .args(["install", "cargo-watch"])
            .status()
            .await?;

        if !install.success() {
            println!("{}", style("Failed to install cargo-watch. Running without watch mode.").yellow());
            return run_cargo(args).await;
        }
    }

    let mut cmd = Command::new("cargo");
    cmd.args(["watch", "-x"]);

    let mut run_cmd = String::from("run");
    if args.release {
        run_cmd.push_str(" --release");
    }
    if let Some(features) = &args.features {
        run_cmd.push_str(&format!(" --features {}", features.join(",")));
    }

    cmd.arg(run_cmd);
    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("cargo watch failed");
    }

    Ok(())
}
