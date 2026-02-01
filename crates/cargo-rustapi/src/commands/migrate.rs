//! Database migration commands
//!
//! Provides a wrapper around sqlx-cli for database migrations.
//! Supports creating, running, reverting, and checking migration status.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;

static CHECK: Emoji<'_, '_> = Emoji("✅ ", "+ ");
static WARN: Emoji<'_, '_> = Emoji("⚠️  ", "! ");
static ERROR: Emoji<'_, '_> = Emoji("❌ ", "x ");
static DB: Emoji<'_, '_> = Emoji("🗄️  ", "# ");
static ARROW: Emoji<'_, '_> = Emoji("➡️  ", "-> ");

/// Database migration commands
#[derive(Subcommand, Debug)]
pub enum MigrateArgs {
    /// Run all pending migrations
    Run(MigrateRunArgs),

    /// Revert the last migration (or N migrations)
    Revert(MigrateRevertArgs),

    /// Show migration status
    Status(MigrateStatusArgs),

    /// Create a new migration
    Create(MigrateCreateArgs),

    /// Reset database (drop, create, run all migrations)
    Reset(MigrateResetArgs),
}

#[derive(Args, Debug)]
pub struct MigrateRunArgs {
    /// Database URL (overrides DATABASE_URL env var)
    #[arg(long)]
    pub database_url: Option<String>,

    /// Run in dry-run mode (don't actually apply)
    #[arg(long)]
    pub dry_run: bool,

    /// Migrations directory
    #[arg(long, default_value = "migrations")]
    pub source: String,
}

#[derive(Args, Debug)]
pub struct MigrateRevertArgs {
    /// Database URL (overrides DATABASE_URL env var)
    #[arg(long)]
    pub database_url: Option<String>,

    /// Number of migrations to revert
    #[arg(short, long, default_value = "1")]
    pub count: u32,

    /// Run in dry-run mode
    #[arg(long)]
    pub dry_run: bool,

    /// Migrations directory
    #[arg(long, default_value = "migrations")]
    pub source: String,
}

#[derive(Args, Debug)]
pub struct MigrateStatusArgs {
    /// Database URL (overrides DATABASE_URL env var)
    #[arg(long)]
    pub database_url: Option<String>,

    /// Migrations directory
    #[arg(long, default_value = "migrations")]
    pub source: String,
}

#[derive(Args, Debug)]
pub struct MigrateCreateArgs {
    /// Migration name (e.g., "create_users_table")
    pub name: String,

    /// Create reversible migration (with up.sql and down.sql)
    #[arg(short, long)]
    pub reversible: bool,

    /// Migrations directory
    #[arg(long, default_value = "migrations")]
    pub source: String,

    /// Create migration with timestamp prefix instead of sequential
    #[arg(long)]
    pub timestamp: bool,
}

#[derive(Args, Debug)]
pub struct MigrateResetArgs {
    /// Database URL (overrides DATABASE_URL env var)
    #[arg(long)]
    pub database_url: Option<String>,

    /// Migrations directory
    #[arg(long, default_value = "migrations")]
    pub source: String,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

/// Execute migration commands
pub async fn migrate(args: MigrateArgs) -> Result<()> {
    // Check if sqlx-cli is installed
    ensure_sqlx_installed().await?;

    match args {
        MigrateArgs::Run(args) => run_migrations(args).await,
        MigrateArgs::Revert(args) => revert_migrations(args).await,
        MigrateArgs::Status(args) => show_status(args).await,
        MigrateArgs::Create(args) => create_migration(args).await,
        MigrateArgs::Reset(args) => reset_database(args).await,
    }
}

/// Ensure sqlx-cli is installed
async fn ensure_sqlx_installed() -> Result<()> {
    let output = Command::new("sqlx").arg("--version").output().await;

    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => {
            println!(
                "{}",
                style("sqlx-cli is not installed. Installing...").yellow()
            );
            println!(
                "{}",
                style("This may take a few minutes...").dim()
            );

            let status = Command::new("cargo")
                .args(["install", "sqlx-cli", "--no-default-features", "--features", "postgres,mysql,sqlite"])
                .status()
                .await
                .context("Failed to run cargo install")?;

            if !status.success() {
                anyhow::bail!(
                    "Failed to install sqlx-cli. Please install it manually:\n\
                     cargo install sqlx-cli --features postgres,mysql,sqlite"
                );
            }

            println!("{} sqlx-cli installed successfully!", CHECK);
            Ok(())
        }
    }
}

/// Run pending migrations
async fn run_migrations(args: MigrateRunArgs) -> Result<()> {
    println!("{} {} Running migrations...", DB, style("migrate run").cyan().bold());
    println!();

    // Ensure migrations directory exists
    if !Path::new(&args.source).exists() {
        println!(
            "{} {} No migrations directory found at '{}'",
            WARN,
            style("Warning:").yellow(),
            args.source
        );
        println!(
            "{}",
            style("Create one with: cargo rustapi migrate create <name>").dim()
        );
        return Ok(());
    }

    let mut cmd = Command::new("sqlx");
    cmd.args(["migrate", "run"]);

    if let Some(url) = &args.database_url {
        cmd.arg("--database-url").arg(url);
    }

    cmd.arg("--source").arg(&args.source);

    if args.dry_run {
        cmd.arg("--dry-run");
        println!("{} Running in dry-run mode", style("Note:").yellow());
    }

    let status = cmd.status().await.context("Failed to run sqlx migrate")?;

    if status.success() {
        println!();
        println!("{} Migrations applied successfully!", CHECK);
    } else {
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

/// Revert migrations
async fn revert_migrations(args: MigrateRevertArgs) -> Result<()> {
    println!(
        "{} {} Reverting {} migration(s)...",
        DB,
        style("migrate revert").cyan().bold(),
        args.count
    );
    println!();

    let mut cmd = Command::new("sqlx");
    cmd.args(["migrate", "revert"]);

    if let Some(url) = &args.database_url {
        cmd.arg("--database-url").arg(url);
    }

    cmd.arg("--source").arg(&args.source);

    if args.dry_run {
        cmd.arg("--dry-run");
        println!("{} Running in dry-run mode", style("Note:").yellow());
    }

    // Revert N times
    for i in 0..args.count {
        println!("{} Reverting migration {}...", ARROW, i + 1);
        let status = cmd.status().await.context("Failed to run sqlx migrate revert")?;
        if !status.success() {
            anyhow::bail!("Failed to revert migration {}", i + 1);
        }
    }

    println!();
    println!("{} {} migration(s) reverted!", CHECK, args.count);

    Ok(())
}

/// Show migration status
async fn show_status(args: MigrateStatusArgs) -> Result<()> {
    println!("{} {} Checking migration status...", DB, style("migrate status").cyan().bold());
    println!();

    // Check if migrations directory exists
    if !Path::new(&args.source).exists() {
        println!(
            "{} {} No migrations directory found",
            WARN,
            style("Warning:").yellow()
        );
        return Ok(());
    }

    // List local migrations
    let mut local_migrations = Vec::new();
    let mut entries = fs::read_dir(&args.source).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                local_migrations.push(name.to_string_lossy().to_string());
            }
        }
    }
    local_migrations.sort();

    if local_migrations.is_empty() {
        println!("{} No migrations found in '{}'", WARN, args.source);
        return Ok(());
    }

    println!("{}", style("Local migrations:").bold());
    for migration in &local_migrations {
        println!("  {} {}", ARROW, migration);
    }
    println!();

    // Run sqlx migrate info if database is available
    if args.database_url.is_some() || std::env::var("DATABASE_URL").is_ok() {
        let mut cmd = Command::new("sqlx");
        cmd.args(["migrate", "info"]);

        if let Some(url) = &args.database_url {
            cmd.arg("--database-url").arg(url);
        }

        cmd.arg("--source").arg(&args.source);

        println!("{}", style("Database migration status:").bold());
        let _ = cmd.status().await;
    } else {
        println!(
            "{} {} Set DATABASE_URL to see applied migrations",
            WARN,
            style("Tip:").yellow()
        );
    }

    Ok(())
}

/// Create a new migration
async fn create_migration(args: MigrateCreateArgs) -> Result<()> {
    println!(
        "{} {} Creating migration '{}'...",
        DB,
        style("migrate create").cyan().bold(),
        args.name
    );
    println!();

    // Create migrations directory if it doesn't exist
    if !Path::new(&args.source).exists() {
        fs::create_dir_all(&args.source).await?;
        println!(
            "{} Created migrations directory: {}",
            CHECK,
            style(&args.source).cyan()
        );
    }

    // Generate timestamp or sequential prefix
    let timestamp = chrono_timestamp();
    let migration_dir = format!("{}/{}_{}", args.source, timestamp, args.name);

    fs::create_dir_all(&migration_dir).await?;

    if args.reversible {
        // Create up.sql and down.sql
        let up_content = format!(
            "-- Migration: {}\n-- Created at: {}\n\n-- Write your UP migration here\n",
            args.name, timestamp
        );
        let down_content = format!(
            "-- Migration: {} (revert)\n-- Created at: {}\n\n-- Write your DOWN migration here\n",
            args.name, timestamp
        );

        fs::write(format!("{}/up.sql", migration_dir), up_content).await?;
        fs::write(format!("{}/down.sql", migration_dir), down_content).await?;

        println!("{} Created reversible migration:", CHECK);
        println!("   {} {}/up.sql", ARROW, style(&migration_dir).cyan());
        println!("   {} {}/down.sql", ARROW, style(&migration_dir).cyan());
    } else {
        // Create single migration file
        let content = format!(
            "-- Migration: {}\n-- Created at: {}\n\n-- Write your migration here\n",
            args.name, timestamp
        );

        // For simple migrations, sqlx expects just a .sql file in the migrations dir
        let migration_file = format!("{}/{}_{}.sql", args.source, timestamp, args.name);
        
        // Remove the directory we created and use file instead
        fs::remove_dir(&migration_dir).await.ok();
        fs::write(&migration_file, content).await?;

        println!("{} Created migration:", CHECK);
        println!("   {} {}", ARROW, style(&migration_file).cyan());
    }

    println!();
    println!(
        "{}",
        style("Edit the migration file(s), then run:").dim()
    );
    println!("   cargo rustapi migrate run");

    Ok(())
}

/// Reset database
async fn reset_database(args: MigrateResetArgs) -> Result<()> {
    println!(
        "{} {} This will DROP and recreate your database!",
        ERROR,
        style("WARNING:").red().bold()
    );
    println!();

    if !args.yes {
        use dialoguer::{theme::ColorfulTheme, Confirm};

        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you sure you want to reset the database?")
            .default(false)
            .interact()?
        {
            println!("{}", style("Aborted").yellow());
            return Ok(());
        }
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.red} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    // Drop database
    pb.set_message("Dropping database...");
    let mut drop_cmd = Command::new("sqlx");
    drop_cmd.args(["database", "drop", "-y"]);
    if let Some(url) = &args.database_url {
        drop_cmd.arg("--database-url").arg(url);
    }
    let _ = drop_cmd.status().await; // Ignore error if DB doesn't exist

    // Create database
    pb.set_message("Creating database...");
    let mut create_cmd = Command::new("sqlx");
    create_cmd.args(["database", "create"]);
    if let Some(url) = &args.database_url {
        create_cmd.arg("--database-url").arg(url);
    }
    create_cmd
        .status()
        .await
        .context("Failed to create database")?;

    // Run migrations
    pb.set_message("Running migrations...");
    let mut migrate_cmd = Command::new("sqlx");
    migrate_cmd.args(["migrate", "run"]);
    if let Some(url) = &args.database_url {
        migrate_cmd.arg("--database-url").arg(url);
    }
    migrate_cmd.arg("--source").arg(&args.source);
    migrate_cmd
        .status()
        .await
        .context("Failed to run migrations")?;

    pb.finish_and_clear();

    println!();
    println!("{} Database reset complete!", CHECK);

    Ok(())
}

/// Generate a timestamp for migration names
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    
    // Format: YYYYMMDDHHMMSS
    let secs = duration.as_secs();
    
    // Simple conversion (not timezone aware, but good enough for migration ordering)
    let days = secs / 86400;
    let _years_since_1970 = days / 365;
    
    // For simplicity, just use the unix timestamp
    // This ensures unique, sortable names
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_generation() {
        let ts1 = chrono_timestamp();
        let ts2 = chrono_timestamp();
        
        // Timestamps should be numeric
        assert!(ts1.chars().all(|c| c.is_ascii_digit()));
        
        // Should be reasonably close (within same second usually)
        let diff: i64 = ts2.parse::<i64>().unwrap() - ts1.parse::<i64>().unwrap();
        assert!(diff.abs() <= 1);
    }

    #[test]
    fn test_migrate_create_args() {
        let args = MigrateCreateArgs {
            name: "create_users".to_string(),
            reversible: true,
            source: "migrations".to_string(),
            timestamp: false,
        };
        
        assert_eq!(args.name, "create_users");
        assert!(args.reversible);
    }
}
