//! Full-featured project template

use super::common;
use anyhow::Result;
use tokio::fs;

pub async fn generate(name: &str, features: &[String]) -> Result<()> {
    // Add recommended features for full template
    let mut all_features: Vec<String> = vec![
        "jwt".to_string(),
        "cors".to_string(),
        "rate-limit".to_string(),
        "config".to_string(),
    ];
    
    // Add user-specified features
    for f in features {
        if !all_features.contains(f) {
            all_features.push(f.clone());
        }
    }

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
rustapi-rs = {{ version = "0.1"{features} }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
uuid = {{ version = "1", features = ["v4"] }}
"#,
        name = name,
        features = common::features_to_cargo(&all_features),
    );
    fs::write(format!("{name}/Cargo.toml"), cargo_toml).await?;

    // Create directories
    fs::create_dir_all(format!("{name}/src/handlers")).await?;
    fs::create_dir_all(format!("{name}/src/models")).await?;
    fs::create_dir_all(format!("{name}/src/middleware")).await?;

    // main.rs
    let main_rs = r#"mod handlers;
mod models;
mod middleware;

use rustapi_rs::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type AppState = Arc<RwLock<models::Store>>;

#[rustapi::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load environment variables
    load_dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Get configuration
    let env = Environment::from_env();
    let host = env_or("HOST", "127.0.0.1");
    let port = env_or("PORT", "8080");
    let addr = format!("{}:{}", host, port);

    // Create shared state
    let state: AppState = Arc::new(RwLock::new(models::Store::new()));

    tracing::info!("🚀 Starting server in {:?} mode", env);
    tracing::info!("📡 Listening on http://{}", addr);
    tracing::info!("📚 API docs at http://{}/docs", addr);

    RustApi::new()
        .state(state)
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(RateLimitLayer::new(100, std::time::Duration::from_secs(60)))
        // Health check
        .route("/health", get(handlers::health))
        // Auth endpoints
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/me", get(handlers::auth::me))
        // Protected items endpoints (require JWT)
        .mount(handlers::items::list)
        .mount(handlers::items::get)
        .mount(handlers::items::create)
        .mount(handlers::items::update)
        .mount(handlers::items::delete)
        // Documentation
        .docs_with_info("/docs", ApiInfo {
            title: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some("Full-featured RustAPI application".to_string()),
        })
        .run(&addr)
        .await
}
"#;
    fs::write(format!("{name}/src/main.rs"), main_rs).await?;

    // handlers/mod.rs
    let handlers_mod = r#"//! Request handlers

pub mod auth;
pub mod items;

use rustapi_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize, Schema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub environment: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: std::env::var("RUSTAPI_ENV").unwrap_or_else(|_| "development".to_string()),
    })
}
"#;
    fs::write(format!("{name}/src/handlers/mod.rs"), handlers_mod).await?;

    // handlers/auth.rs
    let handlers_auth = r#"//! Authentication handlers

use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Schema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Schema)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserClaims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
}

/// Login and get a JWT token
#[rustapi::post("/auth/login")]
#[rustapi::tag("Authentication")]
#[rustapi::summary("Login with username and password")]
pub async fn login(Json(body): Json<LoginRequest>) -> Result<Json<LoginResponse>> {
    // TODO: Validate credentials against your database
    if body.username == "admin" && body.password == "password" {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev-secret-change-in-production".to_string());
        
        let claims = UserClaims {
            sub: "1".to_string(),
            username: body.username,
            exp: (chrono_now() + 86400) as usize, // 24 hours
        };
        
        let token = create_token(&claims, &jwt_secret)?;
        
        Ok(Json(LoginResponse {
            token,
            token_type: "Bearer".to_string(),
        }))
    } else {
        Err(ApiError::unauthorized("Invalid credentials"))
    }
}

/// Get current user info
#[rustapi::get("/auth/me")]
#[rustapi::tag("Authentication")]
#[rustapi::summary("Get current authenticated user")]
pub async fn me(auth: AuthUser<UserClaims>) -> Json<UserClaims> {
    Json(auth.claims)
}

fn chrono_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
"#;
    fs::write(format!("{name}/src/handlers/auth.rs"), handlers_auth).await?;

    // handlers/items.rs
    let handlers_items = r#"//! Item handlers

use crate::handlers::auth::UserClaims;
use crate::models::{Item, CreateItem, UpdateItem};
use crate::AppState;
use rustapi_rs::prelude::*;

/// List all items
#[rustapi::get("/items")]
#[rustapi::tag("Items")]
#[rustapi::summary("List all items")]
pub async fn list(
    _auth: AuthUser<UserClaims>,
    State(state): State<AppState>,
) -> Json<Vec<Item>> {
    let store = state.read().await;
    Json(store.items.values().cloned().collect())
}

/// Get an item by ID
#[rustapi::get("/items/{id}")]
#[rustapi::tag("Items")]
#[rustapi::summary("Get item by ID")]
pub async fn get(
    _auth: AuthUser<UserClaims>,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Item>> {
    let store = state.read().await;
    store.items
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("Item {} not found", id)))
}

/// Create a new item
#[rustapi::post("/items")]
#[rustapi::tag("Items")]
#[rustapi::summary("Create a new item")]
pub async fn create(
    auth: AuthUser<UserClaims>,
    State(state): State<AppState>,
    Json(body): Json<CreateItem>,
) -> Result<Created<Json<Item>>> {
    let item = Item::new(body.name, body.description, auth.claims.sub.clone());
    
    let mut store = state.write().await;
    store.items.insert(item.id.clone(), item.clone());
    
    tracing::info!("User {} created item {}", auth.claims.username, item.id);
    
    Ok(Created(Json(item)))
}

/// Update an item
#[rustapi::put("/items/{id}")]
#[rustapi::tag("Items")]
#[rustapi::summary("Update an item")]
pub async fn update(
    _auth: AuthUser<UserClaims>,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateItem>,
) -> Result<Json<Item>> {
    let mut store = state.write().await;
    
    let item = store.items
        .get_mut(&id)
        .ok_or_else(|| ApiError::not_found(format!("Item {} not found", id)))?;
    
    if let Some(name) = body.name {
        item.name = name;
    }
    if let Some(description) = body.description {
        item.description = description;
    }
    item.updated_at = chrono_now();
    
    Ok(Json(item.clone()))
}

/// Delete an item
#[rustapi::delete("/items/{id}")]
#[rustapi::tag("Items")]
#[rustapi::summary("Delete an item")]
pub async fn delete(
    auth: AuthUser<UserClaims>,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<NoContent> {
    let mut store = state.write().await;
    
    store.items
        .remove(&id)
        .ok_or_else(|| ApiError::not_found(format!("Item {} not found", id)))?;
    
    tracing::info!("User {} deleted item {}", auth.claims.username, id);
    
    Ok(NoContent)
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
"#;
    fs::write(format!("{name}/src/handlers/items.rs"), handlers_items).await?;

    // models/mod.rs
    let models_mod = r#"//! Data models

use serde::{Deserialize, Serialize};
use rustapi_rs::Schema;
use std::collections::HashMap;

pub struct Store {
    pub items: HashMap<String, Item>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Item {
    pub fn new(name: String, description: Option<String>, created_by: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            created_by,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Deserialize, Schema)]
pub struct CreateItem {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Schema)]
pub struct UpdateItem {
    pub name: Option<String>,
    pub description: Option<String>,
}
"#;
    fs::write(format!("{name}/src/models/mod.rs"), models_mod).await?;

    // middleware/mod.rs
    let middleware_mod = r#"//! Custom middleware

// Add your custom middleware here
// Example:
// pub mod logging;
// pub mod auth_check;
"#;
    fs::write(format!("{name}/src/middleware/mod.rs"), middleware_mod).await?;

    // .env.example with JWT secret
    let env_example = r#"# Server configuration
HOST=127.0.0.1
PORT=8080

# Environment (development, production)
RUSTAPI_ENV=development

# JWT Secret (CHANGE THIS IN PRODUCTION!)
JWT_SECRET=your-super-secret-key-change-in-production

# Rate limiting
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_WINDOW_SECS=60

# Logging
RUST_LOG=info
"#;
    fs::write(format!("{name}/.env.example"), env_example).await?;

    // Copy .env.example to .env for development
    fs::copy(
        format!("{name}/.env.example"),
        format!("{name}/.env"),
    ).await?;

    // .gitignore
    common::generate_gitignore(name).await?;

    Ok(())
}
