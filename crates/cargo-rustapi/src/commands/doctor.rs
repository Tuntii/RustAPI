//! Doctor command to check environment health

use anyhow::{bail, Context, Result};
use clap::Args;
use console::{style, Emoji};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use walkdir::WalkDir;

#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Project or workspace path to inspect.
    #[arg(long, default_value = ".", value_name = "PATH")]
    pub path: PathBuf,
    /// Exit with a non-zero code when warnings are found.
    #[arg(long, default_value_t = false)]
    pub strict: bool,
}

static CHECK: Emoji<'_, '_> = Emoji("✅ ", "+ ");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");
static ERROR: Emoji<'_, '_> = Emoji("❌ ", "x ");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
struct DoctorCheck {
    status: DoctorStatus,
    name: &'static str,
    detail: String,
}

impl DoctorCheck {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status: DoctorStatus::Pass,
            name,
            detail: detail.into(),
        }
    }

    fn warn(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status: DoctorStatus::Warn,
            name,
            detail: detail.into(),
        }
    }

    fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status: DoctorStatus::Fail,
            name,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct WorkspaceSignals {
    production_defaults: bool,
    env_production: bool,
    health_endpoints: bool,
    health_checks: bool,
    request_id: bool,
    tracing: bool,
    shutdown: bool,
    shutdown_hooks: bool,
    structured_logging: bool,
    otel: bool,
    rate_limit: bool,
    security_headers: bool,
    timeout: bool,
    cors: bool,
    body_limit: bool,
}

pub async fn doctor(args: DoctorArgs) -> Result<()> {
    println!("{}", style("Checking environment health...").bold());
    println!();

    let mut checks = Vec::new();

    println!("{}", style("Toolchain").bold());
    checks.push(check_tool("rustc", &["--version"], "Rust compiler", true).await);
    checks.push(check_tool("cargo", &["--version"], "Cargo package manager", true).await);
    checks.push(
        check_tool(
            "cargo",
            &["watch", "--version"],
            "cargo-watch (for hot reload)",
            false,
        )
        .await,
    );
    checks.push(
        check_tool(
            "docker",
            &["--version"],
            "Docker (for containerization)",
            false,
        )
        .await,
    );
    checks.push(
        check_tool(
            "sqlx",
            &["--version"],
            "sqlx-cli (for database migrations)",
            false,
        )
        .await,
    );

    for check in &checks {
        print_check(check);
    }

    let inspect_path = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()
            .context("failed to determine current directory")?
            .join(&args.path)
    };

    println!();
    println!("{}", style("Production checklist alignment").bold());

    if let Some(workspace_root) = find_workspace_root(&inspect_path) {
        print_check(&DoctorCheck::pass(
            "Workspace root",
            format!("found {}", workspace_root.display()),
        ));

        let project_checks = build_project_checks(&workspace_root)?;
        for check in &project_checks {
            print_check(check);
        }
        checks.extend(project_checks);
    } else {
        checks.push(DoctorCheck::warn(
            "Workspace root",
            format!(
                "No Cargo.toml found above {} — skipped project-level checklist checks",
                inspect_path.display()
            ),
        ));
        print_check(checks.last().unwrap());
    }

    println!();

    let failures = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Warn)
        .count();

    if failures == 0 && warnings == 0 {
        println!("{}", style("Doctor check passed cleanly.").green());
        return Ok(());
    }

    if failures > 0 {
        println!(
            "{}",
            style(format!(
                "Doctor found {} failure(s) and {} warning(s).",
                failures, warnings
            ))
            .red()
        );
        bail!("doctor found {failures} failure(s)");
    }

    if args.strict {
        println!(
            "{}",
            style(format!(
                "Doctor found {} warning(s) and strict mode is enabled.",
                warnings
            ))
            .yellow()
        );
        bail!("doctor found {warnings} warning(s) in strict mode");
    }

    println!(
        "{}",
        style(format!(
            "Doctor completed with {} warning(s). Use --strict to fail on warnings.",
            warnings
        ))
        .yellow()
    );

    Ok(())
}

async fn check_tool(cmd: &str, args: &[&str], name: &'static str, required: bool) -> DoctorCheck {
    let output = Command::new(cmd).args(args).output().await;

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            DoctorCheck::pass(name, version)
        }
        Ok(_) => {
            if required {
                DoctorCheck::fail(name, "installed but returned error")
            } else {
                DoctorCheck::warn(name, "installed but returned error")
            }
        }
        Err(_) => {
            let msg = if cmd == "cargo" && args[0] == "watch" {
                "(install with: cargo install cargo-watch)"
            } else if cmd == "sqlx" {
                "(install with: cargo install sqlx-cli)"
            } else if cmd == "docker" {
                "(install Docker Desktop or Docker Engine)"
            } else {
                "(not found)"
            };

            if required {
                DoctorCheck::fail(name, msg)
            } else {
                DoctorCheck::warn(name, msg)
            }
        }
    }
}

fn print_check(check: &DoctorCheck) {
    let icon = match check.status {
        DoctorStatus::Pass => CHECK,
        DoctorStatus::Warn => WARN,
        DoctorStatus::Fail => ERROR,
    };

    let detail = match check.status {
        DoctorStatus::Pass => style(&check.detail).dim(),
        DoctorStatus::Warn => style(&check.detail).yellow(),
        DoctorStatus::Fail => style(&check.detail).red(),
    };

    println!("{} {} {}", icon, style(check.name).bold(), detail);
}

fn build_project_checks(workspace_root: &Path) -> Result<Vec<DoctorCheck>> {
    let signals = scan_workspace_signals(workspace_root)?;
    let mut checks = Vec::new();

    if workspace_root.join("scripts/check_quality.ps1").exists() {
        checks.push(DoctorCheck::pass(
            "Quality gate",
            "scripts/check_quality.ps1 is available",
        ));
    } else {
        checks.push(DoctorCheck::warn(
            "Quality gate",
            "scripts/check_quality.ps1 was not found; run your equivalent build/test gate before deploy",
        ));
    }

    checks.push(if signals.production_defaults {
        DoctorCheck::pass("Application baseline", "production_defaults usage detected")
    } else {
        DoctorCheck::warn(
            "Application baseline",
            "No production_defaults(...) or production_defaults_with_config(...) call detected",
        )
    });

    checks.push(if signals.env_production {
        DoctorCheck::pass(
            "Production environment",
            "RUSTAPI_ENV=production detected in project files",
        )
    } else {
        DoctorCheck::warn(
            "Production environment",
            "RUSTAPI_ENV=production was not detected in scanned config files",
        )
    });

    checks.push(if signals.production_defaults || signals.health_endpoints || signals.health_checks {
        DoctorCheck::pass(
            "Health and readiness",
            "Health endpoint configuration detected",
        )
    } else {
        DoctorCheck::warn(
            "Health and readiness",
            "No .health_endpoints(...), .with_health_check(...), or production preset usage detected",
        )
    });

    checks.push(if signals.shutdown || signals.shutdown_hooks {
        DoctorCheck::pass(
            "Graceful shutdown",
            "Shutdown flow or on_shutdown hook detected",
        )
    } else {
        DoctorCheck::warn(
            "Graceful shutdown",
            "No run_with_shutdown(...) or .on_shutdown(...) usage detected",
        )
    });

    checks.push(
        if (signals.production_defaults || signals.request_id)
            && (signals.production_defaults || signals.tracing)
        {
            DoctorCheck::pass(
                "Request IDs and tracing",
                "Request ID and tracing signals detected",
            )
        } else {
            DoctorCheck::warn(
                "Request IDs and tracing",
                "RequestIdLayer/tracing signals were not clearly detected",
            )
        },
    );

    checks.push(if signals.structured_logging || signals.otel {
        DoctorCheck::pass(
            "Observability",
            "Structured logging or OpenTelemetry configuration detected",
        )
    } else {
        DoctorCheck::warn(
            "Observability",
            "No StructuredLoggingLayer/structured_logging(...) or OtelLayer/otel(...) usage detected",
        )
    });

    checks.push(
        if signals.rate_limit || signals.security_headers || signals.timeout || signals.cors {
            DoctorCheck::pass(
                "Edge protections",
                "Detected timeout, rate limit, CORS, or security header configuration",
            )
        } else {
            DoctorCheck::warn(
                "Edge protections",
                "No timeout, rate limit, CORS, or security header configuration was detected",
            )
        },
    );

    checks.push(if signals.body_limit {
        DoctorCheck::pass(
            "Payload management",
            "Body limit configuration detected",
        )
    } else {
        DoctorCheck::warn(
            "Payload management",
            "No body limit override detected; validate that the default 1 MB limit matches your traffic",
        )
    });

    Ok(checks)
}

fn scan_workspace_signals(workspace_root: &Path) -> Result<WorkspaceSignals> {
    let mut signals = WorkspaceSignals::default();

    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|entry| should_scan(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_scannable_file(entry.path()) {
            continue;
        }

        let contents = match fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        signals.production_defaults |= contains_any(
            &contents,
            &[".production_defaults(", ".production_defaults_with_config("],
        );
        signals.health_endpoints |= contains_any(
            &contents,
            &[
                ".health_endpoints(",
                ".health_endpoint_config(",
                "HealthEndpointConfig",
            ],
        );
        signals.health_checks |= contents.contains(".with_health_check(");
        signals.request_id |= contents.contains("RequestIdLayer");
        signals.tracing |= contains_any(&contents, &["TracingLayer", "tracing_subscriber"]);
        signals.shutdown |= contents.contains("run_with_shutdown(");
        signals.shutdown_hooks |= contents.contains(".on_shutdown(");
        signals.structured_logging |= contains_any(
            &contents,
            &["StructuredLoggingLayer", "structured_logging("],
        );
        signals.otel |= contains_any(&contents, &["OtelLayer", "otel("]);
        signals.rate_limit |= contains_any(&contents, &["RateLimitLayer", "rate_limit("]);
        signals.security_headers |=
            contains_any(&contents, &["SecurityHeadersLayer", "security_headers("]);
        signals.timeout |= contains_any(&contents, &["TimeoutLayer", "timeout("]);
        signals.cors |= contains_any(&contents, &["CorsLayer", "cors("]);
        signals.body_limit |= contains_any(&contents, &["BodyLimitLayer", ".body_limit("]);
        signals.env_production |= contains_any(
            &contents,
            &[
                "RUSTAPI_ENV=production",
                "RUSTAPI_ENV: production",
                "RUSTAPI_ENV = \"production\"",
                "RUSTAPI_ENV','production",
                "RUSTAPI_ENV\", \"production\"",
            ],
        );
    }

    Ok(signals)
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

fn contains_any(haystack: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| haystack.contains(pattern))
}

fn should_scan(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };

    !matches!(
        name,
        ".git" | "target" | "node_modules" | ".next" | "dist" | "build"
    )
}

fn is_scannable_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if matches!(name, ".env" | ".env.example" | "Dockerfile") {
        return true;
    }

    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "toml" | "md" | "yml" | "yaml" | "env")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_workspace_root_walks_upwards() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
        let nested = dir.path().join("crates").join("app").join("src");
        fs::create_dir_all(&nested).unwrap();

        let root = find_workspace_root(&nested).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn scan_workspace_signals_detects_production_patterns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(dir.path().join(".env"), "RUSTAPI_ENV=production\n").unwrap();

        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            src_dir.join("main.rs"),
            r#"
            use rustapi_rs::prelude::*;

            fn app() {
                let _app = RustApi::new()
                    .production_defaults("svc")
                    .with_health_check(|| async { Ok(()) })
                    .on_shutdown(|| async {})
                    .layer(StructuredLoggingLayer::default())
                    .layer(RateLimitLayer::new(100, std::time::Duration::from_secs(60)))
                    .layer(BodyLimitLayer::new(2 * 1024 * 1024));
            }
            "#,
        )
        .unwrap();

        let signals = scan_workspace_signals(dir.path()).unwrap();
        assert!(signals.production_defaults);
        assert!(signals.env_production);
        assert!(signals.health_checks);
        assert!(signals.shutdown_hooks);
        assert!(signals.structured_logging);
        assert!(signals.rate_limit);
        assert!(signals.body_limit);
    }

    #[test]
    fn build_project_checks_warns_when_signals_are_missing() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "fn main() {}\n").unwrap();

        let checks = build_project_checks(dir.path()).unwrap();
        assert!(checks
            .iter()
            .any(|check| check.status == DoctorStatus::Warn));
        assert!(checks
            .iter()
            .any(|check| check.name == "Application baseline"));
    }
}
