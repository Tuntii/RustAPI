//! cargo-rustapi CLI tool
//!
//! Provides project scaffolding and development utilities for RustAPI.

mod cli;
#[cfg(feature = "cloud")]
mod cloud;
mod commands;
mod config;
mod templates;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("cargo_rustapi=info".parse().unwrap()),
        )
        .without_time()
        .init();

    // Parse CLI arguments (`cargo rustapi` may forward a leading `rustapi` argv on Windows)
    let cli = Cli::parse_from(normalize_argv());

    // Execute command
    cli.execute().await
}

/// Strip `rustapi` when `cargo rustapi <cmd>` forwards it as argv\[1\].
pub(crate) fn strip_cargo_forwarded_subcommand(
    mut args: Vec<std::ffi::OsString>,
) -> Vec<std::ffi::OsString> {
    if args.len() > 2 && args[1].to_string_lossy() == "rustapi" {
        args.remove(1);
    }
    args
}

fn normalize_argv() -> Vec<std::ffi::OsString> {
    strip_cargo_forwarded_subcommand(std::env::args_os().collect())
}

#[cfg(test)]
mod tests {
    use super::strip_cargo_forwarded_subcommand;

    #[test]
    fn strips_leading_rustapi_subcommand_arg() {
        let args: Vec<_> = ["cargo-rustapi", "rustapi", "login", "--help"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect();
        let normalized: Vec<String> = strip_cargo_forwarded_subcommand(args)
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert_eq!(normalized, vec!["cargo-rustapi", "login", "--help"]);
    }

    #[test]
    fn leaves_direct_invocation_untouched() {
        let args: Vec<_> = ["cargo-rustapi", "login"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect();
        let normalized: Vec<String> = strip_cargo_forwarded_subcommand(args)
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert_eq!(normalized, vec!["cargo-rustapi", "login"]);
    }
}
