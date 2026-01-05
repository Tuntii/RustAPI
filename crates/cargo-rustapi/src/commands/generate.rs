//! Code generation command

use anyhow::Result;
use clap::Subcommand;
use console::style;
use std::path::Path;
use tokio::fs;

/// Arguments for the `generate` command
#[derive(Subcommand, Debug)]
pub enum GenerateArgs {
    /// Generate a handler module
    Handler {
        /// Handler name (e.g., "users", "products")
        name: String,
    },

    /// Generate a model struct
    Model {
        /// Model name (e.g., "User", "Product")
        name: String,
    },

    /// Generate CRUD handlers for a resource
    Crud {
        /// Resource name (e.g., "users", "products")
        name: String,
    },
}

/// Execute code generation
pub async fn generate(args: GenerateArgs) -> Result<()> {
    match args {
        GenerateArgs::Handler { name } => generate_handler(&name).await,
        GenerateArgs::Model { name } => generate_model(&name).await,
        GenerateArgs::Crud { name } => generate_crud(&name).await,
    }
}

async fn generate_handler(name: &str) -> Result<()> {
    let handlers_dir = Path::new("src/handlers");

    // Create handlers directory if it doesn't exist
    if !handlers_dir.exists() {
        fs::create_dir_all(handlers_dir).await?;

        // Create mod.rs
        let mod_content = format!("pub mod {};\n", name);
        fs::write(handlers_dir.join("mod.rs"), mod_content).await?;
    } else {
        // Append to existing mod.rs
        let mod_path = handlers_dir.join("mod.rs");
        if mod_path.exists() {
            let mut content = fs::read_to_string(&mod_path).await?;
            if !content.contains(&format!("mod {};", name)) {
                content.push_str(&format!("pub mod {};\n", name));
                fs::write(&mod_path, content).await?;
            }
        }
    }

    // Generate handler file
    let handler_content = format!(
        r#"//! {} handlers

use rustapi_rs::prelude::*;
use serde::{{Deserialize, Serialize}};

/// List all {}
#[rustapi::get("/{name}")]
pub async fn list() -> Json<Vec<{type_name}Response>> {{
    // TODO: Implement list
    Json(vec![])
}}

/// Get a single {singular}
#[rustapi::get("/{name}/{{id}}")]
pub async fn get(Path(id): Path<i64>) -> Result<Json<{type_name}Response>> {{
    // TODO: Implement get
    Err(ApiError::not_found("{singular}"))
}}

/// Create a new {singular}
#[rustapi::post("/{name}")]
pub async fn create(Json(body): Json<Create{type_name}>) -> Result<Created<Json<{type_name}Response>>> {{
    // TODO: Implement create
    Err(ApiError::internal("Not implemented"))
}}

/// Update a {singular}
#[rustapi::put("/{name}/{{id}}")]
pub async fn update(
    Path(id): Path<i64>,
    Json(body): Json<Update{type_name}>,
) -> Result<Json<{type_name}Response>> {{
    // TODO: Implement update
    Err(ApiError::not_found("{singular}"))
}}

/// Delete a {singular}
#[rustapi::delete("/{name}/{{id}}")]
pub async fn delete(Path(id): Path<i64>) -> Result<NoContent> {{
    // TODO: Implement delete
    Err(ApiError::not_found("{singular}"))
}}

// Request/Response types
#[derive(Debug, Serialize, Schema)]
pub struct {type_name}Response {{
    pub id: i64,
    // TODO: Add fields
}}

#[derive(Debug, Deserialize, Schema)]
pub struct Create{type_name} {{
    // TODO: Add fields
}}

#[derive(Debug, Deserialize, Schema)]
pub struct Update{type_name} {{
    // TODO: Add fields
}}
"#,
        capitalize(name),
        name,
        name = name,
        type_name = to_pascal_case(name),
        singular = singularize(name),
    );

    let handler_path = handlers_dir.join(format!("{}.rs", name));
    fs::write(&handler_path, handler_content).await?;

    println!(
        "{} Generated handler: {}",
        style("✓").green(),
        handler_path.display()
    );
    println!();
    println!("Don't forget to register the routes in main.rs:");
    println!(
        "  {}",
        style(format!(".mount(handlers::{}::list)", name)).cyan()
    );
    println!(
        "  {}",
        style(format!(".mount(handlers::{}::get)", name)).cyan()
    );
    println!(
        "  {}",
        style(format!(".mount(handlers::{}::create)", name)).cyan()
    );
    println!(
        "  {}",
        style(format!(".mount(handlers::{}::update)", name)).cyan()
    );
    println!(
        "  {}",
        style(format!(".mount(handlers::{}::delete)", name)).cyan()
    );

    Ok(())
}

async fn generate_model(name: &str) -> Result<()> {
    let models_dir = Path::new("src/models");

    // Create models directory if it doesn't exist
    if !models_dir.exists() {
        fs::create_dir_all(models_dir).await?;

        // Create mod.rs
        let mod_content = format!(
            "mod {};\npub use {}::*;\n",
            name.to_lowercase(),
            name.to_lowercase()
        );
        fs::write(models_dir.join("mod.rs"), mod_content).await?;
    } else {
        // Append to existing mod.rs
        let mod_path = models_dir.join("mod.rs");
        if mod_path.exists() {
            let mut content = fs::read_to_string(&mod_path).await?;
            let lower_name = name.to_lowercase();
            if !content.contains(&format!("mod {};", lower_name)) {
                content.push_str(&format!(
                    "mod {};\npub use {}::*;\n",
                    lower_name, lower_name
                ));
                fs::write(&mod_path, content).await?;
            }
        }
    }

    // Generate model file
    let model_content = format!(
        r#"//! {} model

use serde::{{Deserialize, Serialize}};
use rustapi_rs::Schema;

/// {} entity
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct {} {{
    /// Unique identifier
    pub id: i64,
    
    /// Creation timestamp
    pub created_at: String,
    
    /// Last update timestamp
    pub updated_at: String,
    
    // TODO: Add your fields here
}}

impl {} {{
    /// Create a new {} instance
    pub fn new(id: i64) -> Self {{
        let now = chrono::Utc::now().to_rfc3339();
        Self {{
            id,
            created_at: now.clone(),
            updated_at: now,
        }}
    }}
}}
"#,
        name,
        name,
        name,
        name,
        name.to_lowercase(),
    );

    let model_path = models_dir.join(format!("{}.rs", name.to_lowercase()));
    fs::write(&model_path, model_content).await?;

    println!(
        "{} Generated model: {}",
        style("✓").green(),
        model_path.display()
    );

    Ok(())
}

async fn generate_crud(name: &str) -> Result<()> {
    // Generate both handler and model
    let type_name = to_pascal_case(name);

    println!(
        "{}",
        style(format!("Generating CRUD for '{}'...", name)).bold()
    );
    println!();

    generate_model(&type_name).await?;
    generate_handler(name).await?;

    Ok(())
}

// Helper functions
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split(&['-', '_'][..]).map(capitalize).collect()
}

fn singularize(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix("ies") {
        format!("{}y", stripped)
    } else if let Some(stripped) = s.strip_suffix('s') {
        if !s.ends_with("ss") {
            stripped.to_string()
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    }
}
