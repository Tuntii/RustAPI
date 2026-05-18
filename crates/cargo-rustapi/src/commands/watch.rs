//! Watch command for development with hot-reload
//!
//! Uses a std-only polling watcher by default and the `notify` crate when the
//! `native-watch` feature is enabled.
//! Detects file changes, rebuilds the project, and restarts the server
//! automatically — no external tools (cargo-watch) required.

use anyhow::{Context, Result};
use clap::Args;
use console::{style, Emoji};
#[cfg(feature = "native-watch")]
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "native-watch")]
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};
use tokio::process::{Child, Command};
use tokio::sync::mpsc as tokio_mpsc;

static WATCH: Emoji<'_, '_> = Emoji("👀 ", "* ");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "> ");
static GEAR: Emoji<'_, '_> = Emoji("⚙️  ", "# ");
static CHECK: Emoji<'_, '_> = Emoji("✅ ", "+ ");
static CROSS: Emoji<'_, '_> = Emoji("❌ ", "x ");
static RELOAD: Emoji<'_, '_> = Emoji("🔄 ", "~ ");

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
    #[arg(short, long, default_value = "300")]
    pub delay: u32,

    /// Enable quiet mode (less output)
    #[arg(short, long)]
    pub quiet: bool,

    /// Don't restart if build fails
    #[arg(long)]
    pub no_restart_on_fail: bool,

    /// Poll for changes instead of using native filesystem events.
    ///
    /// This is the default when cargo-rustapi is built without the `native-watch` feature.
    #[arg(long)]
    pub poll: bool,

    /// Additional features to enable during build
    #[arg(short, long, value_delimiter = ',')]
    pub features: Option<Vec<String>>,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Package to run (for workspace projects)
    #[arg(short = 'p', long)]
    pub package: Option<String>,
}

/// Check if a path has a watched extension
fn is_watched_extension(path: &std::path::Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.iter().any(|e| e == ext))
        .unwrap_or(false)
}

/// Check if a path should be ignored
fn is_ignored(path: &std::path::Path, ignore_paths: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    ignore_paths.iter().any(|ignored| {
        path_str.contains(ignored)
            || path
                .components()
                .any(|c| c.as_os_str().to_string_lossy() == *ignored)
    })
}

fn collect_watch_snapshot(
    watch_paths: &[String],
    ignore_paths: &[String],
    extensions: &[String],
) -> Result<(usize, HashMap<PathBuf, SystemTime>)> {
    let mut roots_watched = 0;
    let mut snapshot = HashMap::new();

    for watch_path in watch_paths {
        let path = PathBuf::from(watch_path);
        if path.exists() {
            roots_watched += 1;
            collect_path_snapshot(&path, ignore_paths, extensions, &mut snapshot)?;
        }
    }

    Ok((roots_watched, snapshot))
}

fn collect_path_snapshot(
    path: &Path,
    ignore_paths: &[String],
    extensions: &[String],
    snapshot: &mut HashMap<PathBuf, SystemTime>,
) -> Result<()> {
    if is_ignored(path, ignore_paths) {
        return Ok(());
    }

    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };

    if metadata.is_dir() {
        for entry in
            fs::read_dir(path).with_context(|| format!("Failed to scan {}", path.display()))?
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            collect_path_snapshot(&entry.path(), ignore_paths, extensions, snapshot)?;
        }
    } else if metadata.is_file() && is_watched_extension(path, extensions) {
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        snapshot.insert(path.to_path_buf(), modified);
    }

    Ok(())
}

fn setup_polling_watcher(
    args: &WatchArgs,
    extensions: &[String],
    debounce_duration: Duration,
) -> Result<tokio_mpsc::Receiver<()>> {
    let (paths_watched, mut snapshot) =
        collect_watch_snapshot(&args.watch_paths, &args.ignore_paths, extensions)?;

    if paths_watched == 0 {
        anyhow::bail!(
            "No valid paths to watch. Ensure at least one of [{}] exists.",
            args.watch_paths.join(", ")
        );
    }

    let watch_paths = args.watch_paths.clone();
    let ignore_paths = args.ignore_paths.clone();
    let extensions = extensions.to_vec();
    let poll_interval = debounce_duration.max(Duration::from_millis(100));
    let (async_tx, async_rx) = tokio_mpsc::channel::<()>(1);

    std::thread::spawn(move || loop {
        if async_tx.is_closed() {
            break;
        }

        std::thread::sleep(poll_interval);

        let Ok((_, next_snapshot)) =
            collect_watch_snapshot(&watch_paths, &ignore_paths, &extensions)
        else {
            continue;
        };

        if next_snapshot != snapshot {
            snapshot = next_snapshot;
            if async_tx.blocking_send(()).is_err() {
                break;
            }
        }
    });

    Ok(async_rx)
}

#[cfg(feature = "native-watch")]
fn setup_native_watcher(
    args: &WatchArgs,
    extensions: &[String],
    debounce_duration: Duration,
) -> Result<(tokio_mpsc::Receiver<()>, Box<dyn std::any::Any>)> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer =
        new_debouncer(debounce_duration, tx).context("Failed to create file watcher")?;

    let mut paths_watched = 0;
    for watch_path in &args.watch_paths {
        let path = PathBuf::from(watch_path);
        if path.exists() {
            debouncer
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch path: {watch_path}"))?;
            paths_watched += 1;
        }
    }

    if paths_watched == 0 {
        anyhow::bail!(
            "No valid paths to watch. Ensure at least one of [{}] exists.",
            args.watch_paths.join(", ")
        );
    }

    let (async_tx, async_rx) = tokio_mpsc::channel::<()>(1);
    let ignore_paths = args.ignore_paths.clone();
    let ext_clone = extensions.to_vec();
    std::thread::spawn(move || {
        for result in rx {
            match result {
                Ok(events) => {
                    let has_relevant = events.iter().any(|event| {
                        event.kind == DebouncedEventKind::Any
                            && !is_ignored(&event.path, &ignore_paths)
                            && is_watched_extension(&event.path, &ext_clone)
                    });
                    if has_relevant && async_tx.blocking_send(()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("File watcher error: {e}");
                }
            }
        }
    });

    Ok((async_rx, Box::new(debouncer)))
}

/// Build the project, returning (success, duration, error_output)
async fn build_project(args: &WatchArgs) -> (bool, Duration, String) {
    let start = Instant::now();

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--message-format=short");

    if args.release {
        cmd.arg("--release");
    }

    if let Some(pkg) = &args.package {
        cmd.arg("-p").arg(pkg);
    }

    if let Some(features) = &args.features {
        cmd.arg("--features").arg(features.join(","));
    }

    let output = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    let duration = start.elapsed();

    match output {
        Ok(output) => {
            let success = output.status.success();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (success, duration, stderr)
        }
        Err(e) => (false, duration, format!("Build failed to start: {e}")),
    }
}

/// Start the server process
async fn start_server(args: &WatchArgs) -> Result<Child> {
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

    // Mark process as being watched so .hot_reload() can detect it
    cmd.env("RUSTAPI_HOT_RELOAD", "1");
    cmd.env("RUSTAPI_ENV", "development");

    cmd.stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);

    let child = cmd.spawn().context("Failed to start server process")?;
    Ok(child)
}

/// Gracefully stop the server process
async fn stop_server(mut child: Child) {
    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
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
            style("║     RustAPI Hot Reload                 ║")
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

    // Parse extensions
    let extensions: Vec<String> = args
        .extensions
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Print configuration
    if !args.quiet {
        println!("{} {} {}", GEAR, style("Command:").bold(), args.command);
        println!(
            "{} {} {}",
            WATCH,
            style("Extensions:").bold(),
            extensions.join(", ")
        );
        println!(
            "{} {} {}",
            WATCH,
            style("Watching:").bold(),
            args.watch_paths.join(", ")
        );
        println!("{} {} {}ms", WATCH, style("Debounce:").bold(), args.delay);
        println!();
        println!("{}", style("Press Ctrl+C to stop.").dim());
        println!();
    }

    // ─── Set up file watcher ────────────────────────────────────────────
    let debounce_duration = Duration::from_millis(args.delay as u64);
    let (mut async_rx, _watcher_guard): (tokio_mpsc::Receiver<()>, Option<Box<dyn std::any::Any>>) = {
        #[cfg(feature = "native-watch")]
        {
            if !args.poll {
                let (rx, guard) = setup_native_watcher(&args, &extensions, debounce_duration)?;
                (rx, Some(guard))
            } else {
                (
                    setup_polling_watcher(&args, &extensions, debounce_duration)?,
                    None,
                )
            }
        }

        #[cfg(not(feature = "native-watch"))]
        {
            (
                setup_polling_watcher(&args, &extensions, debounce_duration)?,
                None,
            )
        }
    };

    // ─── Initial build & start ──────────────────────────────────────────
    if !args.quiet {
        println!("{} {}", ROCKET, style("Initial build...").green().bold());
    }

    let (success, duration, output) = build_project(&args).await;
    if !success {
        println!(
            "{} {} ({:.1}s)",
            CROSS,
            style("Build FAILED").red().bold(),
            duration.as_secs_f64()
        );
        if !output.is_empty() {
            for line in output.lines() {
                if line.contains("error") || line.contains("warning") {
                    println!("  {line}");
                }
            }
        }
        if args.no_restart_on_fail {
            anyhow::bail!("Initial build failed");
        }
        println!("\n{}", style("Watching for changes to retry...").dim());
    }

    let mut server: Option<Child> = if success {
        if !args.quiet {
            println!(
                "{} {} ({:.1}s)",
                CHECK,
                style("Build OK").green().bold(),
                duration.as_secs_f64()
            );
            println!("{} {}", ROCKET, style("Starting server...").green().bold());
            println!();
        }
        Some(start_server(&args).await?)
    } else {
        None
    };

    let mut rebuild_count: u32 = 0;

    // ─── Watch loop ─────────────────────────────────────────────────────
    loop {
        tokio::select! {
            // File change detected
            Some(()) = async_rx.recv() => {
                rebuild_count += 1;

                if args.clear {
                    print!("\x1B[2J\x1B[1;1H");
                }

                if !args.quiet {
                    println!();
                    println!(
                        "{} {} (rebuild #{})",
                        RELOAD,
                        style("Change detected, rebuilding...").yellow().bold(),
                        rebuild_count
                    );
                }

                // Stop current server
                if let Some(child) = server.take() {
                    stop_server(child).await;
                }

                // Build
                let (success, duration, output) = build_project(&args).await;

                if success {
                    if !args.quiet {
                        println!(
                            "{} {} ({:.1}s)",
                            CHECK,
                            style("Build OK").green().bold(),
                            duration.as_secs_f64()
                        );
                        println!(
                            "{} {}",
                            ROCKET,
                            style("Restarting server...").green().bold()
                        );
                        println!();
                    }
                    server = Some(start_server(&args).await?);
                } else {
                    println!(
                        "{} {} ({:.1}s)",
                        CROSS,
                        style("Build FAILED").red().bold(),
                        duration.as_secs_f64()
                    );
                    if !output.is_empty() {
                        for line in output.lines() {
                            if line.contains("error") || line.contains("warning") {
                                println!("  {line}");
                            }
                        }
                    }
                    if !args.quiet {
                        println!(
                            "\n{}",
                            style("Watching for changes to retry...").dim()
                        );
                    }
                }
            }
            // Server process exited unexpectedly
            _ = async {
                if let Some(ref mut child) = server {
                    let _ = child.wait().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                server = None;
                if !args.quiet {
                    println!(
                        "\n{} {}",
                        style("⚠").yellow(),
                        style("Server process exited. Watching for changes to restart...").yellow()
                    );
                }
            }
            // Ctrl+C
            _ = tokio::signal::ctrl_c() => {
                if !args.quiet {
                    println!(
                        "\n{} {}",
                        style("👋").bold(),
                        style("Shutting down...").dim()
                    );
                }
                if let Some(child) = server.take() {
                    stop_server(child).await;
                }
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_watched_extension() {
        let extensions = vec!["rs".to_string(), "toml".to_string()];
        assert!(is_watched_extension(
            std::path::Path::new("src/main.rs"),
            &extensions
        ));
        assert!(is_watched_extension(
            std::path::Path::new("Cargo.toml"),
            &extensions
        ));
        assert!(!is_watched_extension(
            std::path::Path::new("README.md"),
            &extensions
        ));
        assert!(!is_watched_extension(
            std::path::Path::new("data.json"),
            &extensions
        ));
    }

    #[test]
    fn test_is_ignored() {
        let ignore = vec!["target".to_string(), ".git".to_string()];
        assert!(is_ignored(
            std::path::Path::new("target/debug/main"),
            &ignore
        ));
        assert!(is_ignored(std::path::Path::new(".git/HEAD"), &ignore));
        assert!(!is_ignored(std::path::Path::new("src/main.rs"), &ignore));
    }

    #[test]
    fn test_collect_watch_snapshot_skips_ignored_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let target = temp.path().join("target");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(target.join("generated.rs"), "fn ignored() {}").unwrap();

        let watch_paths = vec![temp.path().to_string_lossy().to_string()];
        let ignore_paths = vec!["target".to_string()];
        let extensions = vec!["rs".to_string()];

        let (roots, snapshot) =
            collect_watch_snapshot(&watch_paths, &ignore_paths, &extensions).unwrap();

        assert_eq!(roots, 1);
        assert!(snapshot.keys().any(|path| path.ends_with("src/main.rs")));
        assert!(!snapshot
            .keys()
            .any(|path| path.ends_with("target/generated.rs")));
    }

    #[test]
    fn test_default_args() {
        let args = WatchArgs {
            command: "run".to_string(),
            clear: false,
            extensions: "rs,toml".to_string(),
            watch_paths: vec!["src".to_string()],
            ignore_paths: vec![".git".to_string()],
            delay: 300,
            quiet: false,
            no_restart_on_fail: false,
            poll: false,
            features: None,
            release: false,
            package: None,
        };

        assert_eq!(args.command, "run");
        assert_eq!(args.delay, 300);
        assert!(!args.clear);
    }

    #[test]
    fn test_extension_parsing() {
        let extensions = "rs,toml,html,css";
        let parsed: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
        assert_eq!(parsed, vec!["rs", "toml", "html", "css"]);
    }
}
