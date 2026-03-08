//! Benchmark workflow command.

use anyhow::{bail, Context, Result};
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Run the repository benchmark workflow.
#[derive(Args, Debug, Clone)]
pub struct BenchArgs {
    /// Project or workspace path to inspect.
    #[arg(long, default_value = ".", value_name = "PATH")]
    pub path: PathBuf,
    /// Override performance snapshot warmup iterations.
    #[arg(long)]
    pub warmup: Option<u32>,
    /// Override performance snapshot measured iterations.
    #[arg(long)]
    pub iterations: Option<u32>,
}

pub async fn bench(args: BenchArgs) -> Result<()> {
    let inspect_path = resolve_path(&args.path)?;
    let workspace_root = find_workspace_root(&inspect_path)
        .with_context(|| format!("No Cargo.toml found above {}", inspect_path.display()))?;
    let script_path = workspace_root.join("scripts").join("bench.ps1");

    if !script_path.exists() {
        bail!(
            "Benchmark script was not found at {}",
            script_path.display()
        );
    }

    let shell = if cfg!(windows) { "powershell" } else { "pwsh" };

    println!(
        "{} {}",
        style("Running benchmark workflow from").bold(),
        style(script_path.display()).cyan()
    );

    let mut command = Command::new(shell);
    if cfg!(windows) {
        command.args(["-ExecutionPolicy", "Bypass", "-File"]);
    } else {
        command.arg("-File");
    }
    command.arg(&script_path).current_dir(&workspace_root);

    if let Some(warmup) = args.warmup {
        command.env("RUSTAPI_PERF_WARMUP", warmup.to_string());
    }
    if let Some(iterations) = args.iterations {
        command.env("RUSTAPI_PERF_ITERS", iterations.to_string());
    }

    let status = command
        .status()
        .await
        .context("Failed to launch benchmark workflow")?;
    if !status.success() {
        bail!("Benchmark workflow exited with status {}", status);
    }

    println!("{}", style("Benchmark workflow finished.").green());
    Ok(())
}

fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("failed to determine current directory")?
            .join(path))
    }
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}
