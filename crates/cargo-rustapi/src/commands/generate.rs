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
    ensure_handlers_module(handlers_dir, name).await?;

    let handler_content = format!(
        r#"//! {} handlers

use rustapi_rs::prelude::*;
use serde::{{Deserialize, Serialize}};

/// List all {}
#[rustapi_rs::get("/{name}")]
pub async fn list() -> Json<Vec<{type_name}Response>> {{
    // TODO: Implement list
    Json(vec![])
}}

/// Get a single {singular}
#[rustapi_rs::get("/{name}/{{id}}")]
pub async fn get(Path(id): Path<i64>) -> Result<Json<{type_name}Response>> {{
    // TODO: Implement get
    Err(ApiError::not_found("{singular}"))
}}

/// Create a new {singular}
#[rustapi_rs::post("/{name}")]
pub async fn create(Json(body): Json<Create{type_name}>) -> Result<Created<Json<{type_name}Response>>> {{
    // TODO: Implement create
    Err(ApiError::internal("Not implemented"))
}}

/// Update a {singular}
#[rustapi_rs::put("/{name}/{{id}}")]
pub async fn update(
    Path(id): Path<i64>,
    Json(body): Json<Update{type_name}>,
) -> Result<Json<{type_name}Response>> {{
    // TODO: Implement update
    Err(ApiError::not_found("{singular}"))
}}

/// Delete a {singular}
#[rustapi_rs::delete("/{name}/{{id}}")]
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
    print_route_registration_hints(name);

    Ok(())
}

async fn generate_model(name: &str) -> Result<()> {
    let models_dir = Path::new("src/models");
    ensure_models_module(models_dir, name).await?;

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
    let type_name = to_pascal_case(&singularize(name));
    let table = name.to_lowercase();

    println!(
        "{}",
        style(format!("Generating SQLx CRUD for '{}'...", name)).bold()
    );
    println!();

    ensure_crud_dependencies().await?;
    ensure_db_module(&table).await?;
    generate_sqlx_model(name, &type_name).await?;
    generate_sqlx_handler(name, &type_name, &table).await?;

    println!();
    println!("Next steps:");
    println!("  1. Add to src/main.rs:");
    println!("     mod db;");
    println!("     mod handlers;");
    println!("     mod models;");
    println!("  2. Initialize the pool and share state:");
    println!(
        "     let pool = db::init_pool(\"sqlite:{}.db\").await?;",
        table
    );
    println!("     RustApi::auto().state(pool).run(\"127.0.0.1:8080\").await?;");

    Ok(())
}

async fn ensure_crud_dependencies() -> Result<()> {
    let cargo_path = Path::new("Cargo.toml");
    if !cargo_path.exists() {
        anyhow::bail!("Cargo.toml not found — run this command from your project root");
    }

    let mut content = fs::read_to_string(cargo_path).await?;
    let mut deps_to_add = Vec::new();

    if !content
        .lines()
        .any(|line| line.trim_start().starts_with("sqlx "))
    {
        deps_to_add.push(
            "sqlx = { version = \"0.8\", default-features = false, features = [\"runtime-tokio\", \"sqlite\", \"derive\"] }",
        );
    }

    if !content
        .lines()
        .any(|line| line.trim_start().starts_with("chrono "))
    {
        deps_to_add.push("chrono = { version = \"0.4\", features = [\"serde\"] }");
    }

    if !deps_to_add.is_empty() {
        let injection = format!("{}\n", deps_to_add.join("\n"));
        if let Some(idx) = content.find("[dependencies]") {
            let insert_at = idx + "[dependencies]".len();
            if content[insert_at..].starts_with('\n') {
                content.insert_str(insert_at + 1, &injection);
            } else {
                content.insert_str(insert_at, &format!("\n{injection}"));
            }
        } else {
            content.push_str(&format!("\n[dependencies]\n{injection}"));
        }
        fs::write(cargo_path, content).await?;
        println!(
            "{} Updated Cargo.toml with sqlx + chrono dependencies",
            style("✓").green()
        );
    }

    Ok(())
}

async fn ensure_db_module(table: &str) -> Result<()> {
    let db_path = Path::new("src/db.rs");
    if db_path.exists() {
        return Ok(());
    }

    let singular = singularize(table);
    let content = format!(
        r#"//! Database bootstrap for generated CRUD resources.

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS {table} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)";

/// Open a SQLite pool and ensure the `{table}` table exists.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {{
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    sqlx::query(SCHEMA).execute(&pool).await?;
    Ok(pool)
}}

/// Table name used by generated CRUD handlers.
pub const TABLE: &str = "{table}";

/// Singular resource label for error messages.
pub const SINGULAR: &str = "{singular}";
"#,
        table = table,
        singular = singular,
    );

    fs::write(db_path, content).await?;
    println!(
        "{} Generated database module: {}",
        style("✓").green(),
        db_path.display()
    );
    Ok(())
}

async fn generate_sqlx_model(name: &str, type_name: &str) -> Result<()> {
    let models_dir = Path::new("src/models");
    ensure_models_module(models_dir, name).await?;

    let model_content = format!(
        r#"//! {} model (SQLx)

use rustapi_rs::Schema;
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize, Schema, sqlx::FromRow)]
pub struct {type_name} {{
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}}

#[derive(Debug, Deserialize, Schema)]
pub struct Create{type_name} {{
    pub name: String,
    pub description: Option<String>,
}}

#[derive(Debug, Deserialize, Schema)]
pub struct Update{type_name} {{
    pub name: Option<String>,
    pub description: Option<String>,
}}
"#,
        capitalize(name),
        type_name = type_name,
    );

    let model_path = models_dir.join(format!("{}.rs", name));
    fs::write(&model_path, model_content).await?;

    println!(
        "{} Generated SQLx model: {}",
        style("✓").green(),
        model_path.display()
    );

    Ok(())
}

async fn generate_sqlx_handler(name: &str, type_name: &str, table: &str) -> Result<()> {
    let handlers_dir = Path::new("src/handlers");
    ensure_handlers_module(handlers_dir, name).await?;

    let singular = singularize(name);
    let handler_content = format!(
        r#"//! {} handlers (SQLx SQLite)

use crate::models::{{Create{type_name}, Update{type_name}, {type_name}}};
use rustapi_rs::prelude::*;
use sqlx::SqlitePool;

fn now_ts() -> String {{
    chrono::Utc::now().to_rfc3339()
}}

fn db_error(err: sqlx::Error) -> ApiError {{
    ApiError::internal(err.to_string())
}}

/// List all {name}
#[rustapi_rs::get("/{name}")]
#[rustapi_rs::tag("{type_name}")]
#[rustapi_rs::summary("List all {name}")]
pub async fn list(State(pool): State<SqlitePool>) -> Result<Json<Vec<{type_name}>>> {{
    let rows = sqlx::query_as::<_, {type_name}>(
        "SELECT id, name, description, created_at, updated_at FROM {table} ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;
    Ok(Json(rows))
}}

/// Get a single {singular}
#[rustapi_rs::get("/{name}/{{id}}")]
#[rustapi_rs::tag("{type_name}")]
#[rustapi_rs::summary("Get {singular} by ID")]
pub async fn get(Path(id): Path<i64>, State(pool): State<SqlitePool>) -> Result<Json<{type_name}>> {{
    let row = sqlx::query_as::<_, {type_name}>(
        "SELECT id, name, description, created_at, updated_at FROM {table} WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| ApiError::not_found(format!("{singular} {{id}} not found", id = id)))?;
    Ok(Json(row))
}}

/// Create a new {singular}
#[rustapi_rs::post("/{name}")]
#[rustapi_rs::tag("{type_name}")]
#[rustapi_rs::summary("Create {singular}")]
pub async fn create(
    State(pool): State<SqlitePool>,
    Json(body): Json<Create{type_name}>,
) -> Result<WithStatus<Json<{type_name}>, 201>> {{
    let now = now_ts();
    let result = sqlx::query(
        "INSERT INTO {table} (name, description, created_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .map_err(db_error)?;

    let id = result.last_insert_rowid();
    let row = sqlx::query_as::<_, {type_name}>(
        "SELECT id, name, description, created_at, updated_at FROM {table} WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;
    Ok(WithStatus(Json(row)))
}}

/// Update a {singular}
#[rustapi_rs::put("/{name}/{{id}}")]
#[rustapi_rs::tag("{type_name}")]
#[rustapi_rs::summary("Update {singular}")]
pub async fn update(
    Path(id): Path<i64>,
    State(pool): State<SqlitePool>,
    Json(body): Json<Update{type_name}>,
) -> Result<Json<{type_name}>> {{
    let existing = sqlx::query_as::<_, {type_name}>(
        "SELECT id, name, description, created_at, updated_at FROM {table} WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| ApiError::not_found(format!("{singular} {{id}} not found", id = id)))?;

    let name = body.name.unwrap_or(existing.name);
    let description = body.description.or(existing.description);
    let updated_at = now_ts();

    sqlx::query(
        "UPDATE {table} SET name = ?, description = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&description)
    .bind(&updated_at)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(db_error)?;

    let row = sqlx::query_as::<_, {type_name}>(
        "SELECT id, name, description, created_at, updated_at FROM {table} WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;
    Ok(Json(row))
}}

/// Delete a {singular}
#[rustapi_rs::delete("/{name}/{{id}}")]
#[rustapi_rs::tag("{type_name}")]
#[rustapi_rs::summary("Delete {singular}")]
pub async fn delete(Path(id): Path<i64>, State(pool): State<SqlitePool>) -> Result<NoContent> {{
    let result = sqlx::query("DELETE FROM {table} WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(db_error)?;
    if result.rows_affected() == 0 {{
        return Err(ApiError::not_found(format!("{singular} {{id}} not found", id = id)));
    }}
    Ok(NoContent)
}}
"#,
        capitalize(name),
        name = name,
        type_name = type_name,
        table = table,
        singular = singular,
    );

    let handler_path = handlers_dir.join(format!("{}.rs", name));
    fs::write(&handler_path, handler_content).await?;

    println!(
        "{} Generated SQLx handler: {}",
        style("✓").green(),
        handler_path.display()
    );

    Ok(())
}

async fn ensure_handlers_module(handlers_dir: &Path, name: &str) -> Result<()> {
    if !handlers_dir.exists() {
        fs::create_dir_all(handlers_dir).await?;
        let mod_content = format!("pub mod {};\n", name);
        fs::write(handlers_dir.join("mod.rs"), mod_content).await?;
    } else {
        let mod_path = handlers_dir.join("mod.rs");
        if mod_path.exists() {
            let mut content = fs::read_to_string(&mod_path).await?;
            if !content.contains(&format!("mod {};", name)) {
                content.push_str(&format!("pub mod {};\n", name));
                fs::write(&mod_path, content).await?;
            }
        } else {
            fs::write(mod_path, format!("pub mod {};\n", name)).await?;
        }
    }
    Ok(())
}

async fn ensure_models_module(models_dir: &Path, name: &str) -> Result<()> {
    let module_name = name.to_lowercase();
    if !models_dir.exists() {
        fs::create_dir_all(models_dir).await?;
        let mod_content = format!(
            "mod {module_name};\npub use {module_name}::*;\n",
            module_name = module_name
        );
        fs::write(models_dir.join("mod.rs"), mod_content).await?;
    } else {
        let mod_path = models_dir.join("mod.rs");
        if mod_path.exists() {
            let mut content = fs::read_to_string(&mod_path).await?;
            if !content.contains(&format!("mod {};", module_name)) {
                content.push_str(&format!(
                    "mod {module_name};\npub use {module_name}::*;\n",
                    module_name = module_name
                ));
                fs::write(&mod_path, content).await?;
            }
        } else {
            fs::write(
                mod_path,
                format!(
                    "mod {module_name};\npub use {module_name}::*;\n",
                    module_name = module_name
                ),
            )
            .await?;
        }
    }
    Ok(())
}

fn print_route_registration_hints(name: &str) {
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
