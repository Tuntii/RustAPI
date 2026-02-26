//! Run command for development server

use super::watch::{self, WatchArgs};
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

    /// Watch for changes and auto-reload (like FastAPI's --reload)
    #[arg(short, long, visible_alias = "reload", alias = "hot")]
    pub watch: bool,

    /// Package to run (for workspace projects)
    #[arg(short = 'P', long)]
    pub package: Option<String>,
}

/// Run the development server
pub async fn run_dev(args: RunArgs) -> Result<()> {
    // Set environment variables
    std::env::set_var("PORT", args.port.to_string());
    std::env::set_var("RUSTAPI_ENV", "development");

    if args.watch {
        println!(
            "{}",
            style("🔄 Starting RustAPI in hot-reload mode...")
                .bold()
                .cyan()
        );
        println!(
            "{}",
            style("   Changes to source files will trigger automatic rebuild").dim()
        );
        println!();

        // Delegate to native watcher
        let watch_args = WatchArgs {
            command: "run".to_string(),
            clear: false,
            extensions: "rs,toml,html,css,sql".to_string(),
            watch_paths: vec![
                "src".to_string(),
                "templates".to_string(),
                "migrations".to_string(),
            ],
            ignore_paths: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
            ],
            delay: 300,
            quiet: false,
            no_restart_on_fail: false,
            poll: false,
            features: args.features,
            release: args.release,
            package: args.package,
        };
        watch::watch(watch_args).await
    } else {
        println!(
            "{}",
            style("🚀 Starting RustAPI development server...").bold()
        );
        println!();
        run_cargo(&args).await
    }
}

async fn run_cargo(args: &RunArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");

    if args.release {
        cmd.arg("--release");
    }

    if let Some(pkg) = &args.package {
        cmd.arg("-p").arg(pkg);
    }

    if let Some(features) = &args.features {
        cmd.arg("--features").arg(features.join(","));
    }

    cmd.env("RUSTAPI_ENV", "development");

    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("cargo run failed");
    }

    Ok(())
}
