//! Observability workflow command.

use anyhow::{bail, Context, Result};
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};

/// Surface observability assets and recommended baseline inputs.
#[derive(Args, Debug, Clone)]
pub struct ObservabilityArgs {
    /// Project or workspace path to inspect.
    #[arg(long, default_value = ".", value_name = "PATH")]
    pub path: PathBuf,
    /// Exit with a non-zero code when expected observability assets are missing.
    #[arg(long, default_value_t = false)]
    pub check: bool,
}

pub async fn observability(args: ObservabilityArgs) -> Result<()> {
    let inspect_path = resolve_path(&args.path)?;
    let workspace_root = find_workspace_root(&inspect_path)
        .with_context(|| format!("No Cargo.toml found above {}", inspect_path.display()))?;

    let assets = [
        (
            "Production baseline",
            workspace_root.join("docs").join("PRODUCTION_BASELINE.md"),
        ),
        (
            "Observability cookbook",
            workspace_root
                .join("docs")
                .join("cookbook")
                .join("src")
                .join("recipes")
                .join("observability.md"),
        ),
        (
            "Benchmark workflow",
            workspace_root.join("scripts").join("bench.ps1"),
        ),
        (
            "Quality gate",
            workspace_root.join("scripts").join("check_quality.ps1"),
        ),
    ];

    println!("{}", style("Observability workflow assets").bold());
    println!();

    let mut missing = 0usize;
    for (label, path) in assets {
        if path.exists() {
            println!(
                "{} {}",
                style(format!("• {label}:")).bold(),
                style(path.display()).cyan()
            );
        } else {
            missing += 1;
            println!(
                "{} {}",
                style(format!("• {label}:")).bold(),
                style(format!("missing at {}", path.display())).yellow()
            );
        }
    }

    println!();
    println!("{}", style("Recommended feature block").bold());
    println!("  extras-otel");
    println!("  extras-structured-logging");
    println!("  extras-insight");
    println!("  extras-timeout");
    println!("  extras-cors");
    println!();
    println!("{}", style("Suggested CLI flow").bold());
    println!("  1. cargo rustapi doctor --strict");
    println!("  2. cargo rustapi observability --check");
    println!("  3. cargo rustapi bench");

    if missing > 0 && args.check {
        bail!("observability workflow is missing {missing} required asset(s)");
    }

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
