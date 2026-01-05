//! cargo-rustapi CLI tool
//!
//! Provides project scaffolding and development utilities for RustAPI.

mod cli;
mod commands;
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

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    cli.execute().await
}
