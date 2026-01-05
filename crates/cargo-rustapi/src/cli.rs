//! CLI argument parsing

use crate::commands::{self, GenerateArgs, NewArgs, RunArgs};
use clap::{Parser, Subcommand};

/// RustAPI CLI - Project scaffolding and development utilities
#[derive(Parser, Debug)]
#[command(name = "cargo-rustapi")]
#[command(bin_name = "cargo rustapi")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new RustAPI project
    New(NewArgs),

    /// Run the development server
    Run(RunArgs),

    /// Generate code from templates
    #[command(subcommand)]
    Generate(GenerateArgs),

    /// Open API documentation in browser
    Docs {
        /// Port to check for running server
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

impl Cli {
    /// Execute the CLI command
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::New(args) => commands::new_project(args).await,
            Commands::Run(args) => commands::run_dev(args).await,
            Commands::Generate(args) => commands::generate(args).await,
            Commands::Docs { port } => commands::open_docs(port).await,
        }
    }
}
