//! Watch command for development with hot-reload

use anyhow::Result;
use clap::Args;
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tokio::process::Command;

static WATCH: Emoji<'_, '_> = Emoji("👀 ", "* ");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "> ");
static GEAR: Emoji<'_, '_> = Emoji("⚙️  ", "# ");

#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Command to run (default: "run")
    #[arg(short = 'x', long, default_value = "run")]
    pub command: String,

    /// Clear screen before each run
    #[arg(short = 'c', long)]
    pub clear: bool,

    /// File extensions to watch (comma-separated)
    #[arg(short, long, default_value = "rs,toml,html,css,sql")]
    pub extensions: String,

    /// Paths to watch (can be specified multiple times)
    #[arg(short = 'w', long = "watch-path", default_values_t = vec!["src".to_string(), "templates".to_string(), "migrations".to_string()])]
    pub watch_paths: Vec<String>,

    /// Paths to ignore (can be specified multiple times)
    #[arg(short = 'i', long = "ignore", default_values_t = vec![".git".to_string(), "target".to_string(), "node_modules".to_string()])]
    pub ignore_paths: Vec<String>,

    /// Delay before restarting (in milliseconds)
    #[arg(short, long, default_value = "500")]
    pub delay: u32,

    /// Enable quiet mode (less output)
    #[arg(short, long)]
    pub quiet: bool,

    /// Don't restart if build fails
    #[arg(long)]
    pub no_restart_on_fail: bool,

    /// Poll for changes instead of using filesystem events
    #[arg(long)]
    pub poll: bool,
}

pub async fn watch(args: WatchArgs) -> Result<()> {
    // Print banner unless quiet
    if !args.quiet {
        println!();
        println!(
            "{}",
            style("╔════════════════════════════════════════╗")
                .cyan()
                .bold()
        );
        println!(
            "{}",
            style("║     RustAPI Watch Mode                 ║")
                .cyan()
                .bold()
        );
        println!(
            "{}",
            style("╚════════════════════════════════════════╝")
                .cyan()
                .bold()
        );
        println!();
    }

    // Check if cargo-watch is installed
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message("Checking for cargo-watch...");

    let version_check = Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .await;

    if version_check.is_err() || !version_check.unwrap().status.success() {
        pb.set_message("Installing cargo-watch...");
        println!(
            "\n{}",
            style("cargo-watch is not installed. Installing...").yellow()
        );

        let install_status = Command::new("cargo")
            .args(["install", "cargo-watch"])
            .status()
            .await?;

        if !install_status.success() {
            pb.finish_and_clear();
            anyhow::bail!("Failed to install cargo-watch. Please install it manually: cargo install cargo-watch");
        }
    }

    pb.finish_and_clear();

    // Print configuration
    if !args.quiet {
        println!("{} {} {}", GEAR, style("Command:").bold(), args.command);
        println!(
            "{} {} {}",
            WATCH,
            style("Extensions:").bold(),
            args.extensions
        );
        println!(
            "{} {} {}",
            WATCH,
            style("Watching:").bold(),
            args.watch_paths.join(", ")
        );
        println!(
            "{} {} {}",
            WATCH,
            style("Ignoring:").bold(),
            args.ignore_paths.join(", ")
        );
        println!("{} {} {}ms", GEAR, style("Delay:").bold(), args.delay);
        println!();
        println!("{}", style("Press Ctrl+C to stop watching.").dim());
        println!();
        println!(
            "{} {}",
            ROCKET,
            style("Starting watch mode...").green().bold()
        );
        println!();
    }

    // Build cargo-watch command
    let mut cmd = Command::new("cargo");
    cmd.arg("watch");

    // Clear screen option
    if args.clear {
        cmd.arg("-c");
    }

    // Delay option
    cmd.arg("-d").arg(format!("{}", args.delay as f64 / 1000.0));

    // Extension filter
    for ext in args.extensions.split(',') {
        let ext = ext.trim();
        if !ext.is_empty() {
            cmd.arg("-e").arg(ext);
        }
    }

    // Watch paths
    for path in &args.watch_paths {
        // Only add if path exists
        if std::path::Path::new(path).exists() {
            cmd.arg("-w").arg(path);
        }
    }

    // Ignore paths
    for path in &args.ignore_paths {
        cmd.arg("-i").arg(path);
    }

    // Polling mode
    if args.poll {
        cmd.arg("--poll");
    }

    // No restart on fail
    if args.no_restart_on_fail {
        cmd.arg("--no-restart");
    }

    // Quiet mode for cargo-watch
    if args.quiet {
        cmd.arg("-q");
    }

    // The command to execute
    cmd.arg("-x").arg(&args.command);

    // Run the watch process
    let mut child = cmd.spawn()?;
    child.wait().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = WatchArgs {
            command: "run".to_string(),
            clear: false,
            extensions: "rs,toml".to_string(),
            watch_paths: vec!["src".to_string()],
            ignore_paths: vec![".git".to_string()],
            delay: 500,
            quiet: false,
            no_restart_on_fail: false,
            poll: false,
        };

        assert_eq!(args.command, "run");
        assert_eq!(args.delay, 500);
        assert!(!args.clear);
    }

    #[test]
    fn test_extension_parsing() {
        let extensions = "rs,toml,html,css";
        let parsed: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
        assert_eq!(parsed, vec!["rs", "toml", "html", "css"]);
    }
}
