//! New project command

use anyhow::{Context, Result};
use clap::Args;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;
use tokio::fs;

use crate::templates::{self, ProjectTemplate};

/// Arguments for the `new` command
#[derive(Args, Debug)]
pub struct NewArgs {
    /// Project name
    pub name: Option<String>,

    /// Project template
    #[arg(short, long, value_enum)]
    pub template: Option<ProjectTemplate>,

    /// Features to enable
    #[arg(short, long, value_delimiter = ',')]
    pub features: Option<Vec<String>>,

    /// Skip interactive prompts
    #[arg(long)]
    pub yes: bool,

    /// Initialize git repository
    #[arg(long, default_value = "true")]
    pub git: bool,
}

/// Create a new RustAPI project
pub async fn new_project(mut args: NewArgs) -> Result<()> {
    let theme = ColorfulTheme::default();

    // Get project name
    let name = if let Some(name) = args.name.take() {
        name
    } else {
        Input::with_theme(&theme)
            .with_prompt("Project name")
            .default("my-rustapi-app".to_string())
            .interact_text()?
    };

    // Validate project name
    validate_project_name(&name)?;

    // Check if directory exists
    let project_path = Path::new(&name);
    if project_path.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Get template
    let template = if let Some(template) = args.template {
        template
    } else if args.yes {
        ProjectTemplate::Minimal
    } else {
        let templates = ["minimal - Bare minimum app", "api - REST API with CRUD", "web - Web app with templates", "full - Full-featured app"];
        let selection = Select::with_theme(&theme)
            .with_prompt("Select a template")
            .items(&templates)
            .default(0)
            .interact()?;

        match selection {
            0 => ProjectTemplate::Minimal,
            1 => ProjectTemplate::Api,
            2 => ProjectTemplate::Web,
            3 => ProjectTemplate::Full,
            _ => ProjectTemplate::Minimal,
        }
    };

    // Get features
    let features = if let Some(features) = args.features {
        features
    } else if args.yes {
        vec![]
    } else {
        let available = ["jwt", "cors", "rate-limit", "config", "toon", "ws", "view"];
        let defaults = match template {
            ProjectTemplate::Full => vec![true, true, true, true, false, false, false],
            ProjectTemplate::Web => vec![false, false, false, false, false, false, true],
            _ => vec![false; available.len()],
        };

        let selections = dialoguer::MultiSelect::with_theme(&theme)
            .with_prompt("Select features (space to toggle)")
            .items(&available)
            .defaults(&defaults)
            .interact()?;

        selections.iter().map(|&i| available[i].to_string()).collect()
    };

    // Confirm
    if !args.yes {
        println!();
        println!("{}", style("Project configuration:").bold());
        println!("  Name:     {}", style(&name).cyan());
        println!("  Template: {}", style(format!("{:?}", template)).cyan());
        println!("  Features: {}", style(if features.is_empty() { "none".to_string() } else { features.join(", ") }).cyan());
        println!();

        if !Confirm::with_theme(&theme)
            .with_prompt("Create project?")
            .default(true)
            .interact()?
        {
            println!("{}", style("Aborted").yellow());
            return Ok(());
        }
    }

    // Create project
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    pb.set_message("Creating project directory...");
    fs::create_dir_all(&name).await?;

    pb.set_message("Generating project files...");
    templates::generate_project(&name, template, &features).await?;

    if args.git {
        pb.set_message("Initializing git repository...");
        init_git(&name).await.ok(); // Don't fail if git isn't available
    }

    pb.finish_and_clear();

    // Success message
    println!();
    println!("{}", style("✨ Project created successfully!").green().bold());
    println!();
    println!("Next steps:");
    println!("  {} {}", style("cd").cyan(), name);
    println!("  {} {}", style("cargo").cyan(), "run");
    println!();
    println!("Then open {} in your browser.", style("http://localhost:8080").cyan());
    
    if features.iter().any(|f| f == "swagger-ui") || template == ProjectTemplate::Full {
        println!("API docs available at {}", style("http://localhost:8080/docs").cyan());
    }

    Ok(())
}

/// Validate project name
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }

    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("Project name cannot contain path separators");
    }

    // Check for valid Rust crate name characters
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        anyhow::bail!("Project name can only contain alphanumeric characters, hyphens, and underscores");
    }

    if name.starts_with('-') || name.starts_with('_') {
        anyhow::bail!("Project name cannot start with a hyphen or underscore");
    }

    Ok(())
}

/// Initialize a git repository
async fn init_git(path: &str) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .await
        .context("Failed to run git init")?;

    if !output.status.success() {
        anyhow::bail!("git init failed");
    }

    Ok(())
}
