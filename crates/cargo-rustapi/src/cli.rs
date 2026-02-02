//! CLI argument parsing

use crate::commands::{
    self, AddArgs, ClientArgs, DeployArgs, DoctorArgs, GenerateArgs, MigrateArgs, NewArgs, RunArgs,
    WatchArgs,
};
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

    /// Watch for changes and auto-reload (dedicated)
    Watch(WatchArgs),

    /// Add a feature or dependency
    Add(AddArgs),

    /// Check environment health
    Doctor(DoctorArgs),

    /// Generate code from templates
    #[command(subcommand)]
    Generate(GenerateArgs),

    /// Database migration commands
    #[command(subcommand)]
    Migrate(MigrateArgs),

    /// Open API documentation in browser
    Docs {
        /// Port to check for running server
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Generate API client from OpenAPI spec
    Client(ClientArgs),

    /// Deploy to various platforms
    #[command(subcommand)]
    Deploy(DeployArgs),
}

impl Cli {
    /// Execute the CLI command
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::New(args) => commands::new_project(args).await,
            Commands::Run(args) => commands::run_dev(args).await,
            Commands::Watch(args) => commands::watch(args).await,
            Commands::Add(args) => commands::add(args).await,
            Commands::Doctor(args) => commands::doctor(args).await,
            Commands::Generate(args) => commands::generate(args).await,
            Commands::Migrate(args) => commands::migrate(args).await,
            Commands::Docs { port } => commands::open_docs(port).await,
            Commands::Client(args) => commands::client(args).await,
            Commands::Deploy(args) => commands::deploy(args).await,
        }
    }
}
