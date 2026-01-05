//! API project template

use super::common;
use anyhow::Result;
use tokio::fs;

pub async fn generate(name: &str, features: &[String]) -> Result<()> {
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
        features = common::features_to_cargo(features),
    );
    fs::write(format!("{name}/Cargo.toml"), cargo_toml).await?;

    // Create directories
    fs::create_dir_all(format!("{name}/src/handlers")).await?;
    fs::create_dir_all(format!("{name}/src/models")).await?;

    // main.rs
    let main_rs = r#"mod handlers;
mod models;
mod error;

use rustapi_rs::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type AppState = Arc<RwLock<models::Store>>;

#[rustapi::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Create shared state
    let state: AppState = Arc::new(RwLock::new(models::Store::new()));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);

    tracing::info!("🚀 Server running at http://{}", addr);
    tracing::info!("📚 API docs at http://{}/docs", addr);

    RustApi::new()
        .state(state)
        // Health check
        .route("/health", get(handlers::health))
        // Items CRUD
        .mount(handlers::items::list)
        .mount(handlers::items::get)
        .mount(handlers::items::create)
        .mount(handlers::items::update)
        .mount(handlers::items::delete)
        // Documentation
        .docs("/docs")
        .run(&addr)
        .await
}
"#;
    fs::write(format!("{name}/src/main.rs"), main_rs).await?;

    // error.rs
    let error_rs = r#"//! Application error types

use rustapi_rs::prelude::*;

/// Application-specific errors
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InvalidInput(String),
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(msg) => ApiError::not_found(msg),
            AppError::InvalidInput(msg) => ApiError::bad_request(msg),
        }
    }
}
"#;
    fs::write(format!("{name}/src/error.rs"), error_rs).await?;

    // handlers/mod.rs
    let handlers_mod = r#"//! Request handlers

pub mod items;

use rustapi_rs::prelude::*;
use serde::Serialize;

/// Health check response
#[derive(Serialize, Schema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
"#;
    fs::write(format!("{name}/src/handlers/mod.rs"), handlers_mod).await?;

    // handlers/items.rs
    let handlers_items = r#"//! Item handlers

use crate::models::{Item, CreateItem, UpdateItem};
use crate::AppState;
use rustapi_rs::prelude::*;

/// List all items
#[rustapi::get("/items")]
#[rustapi::tag("Items")]
#[rustapi::summary("List all items")]
pub async fn list(State(state): State<AppState>) -> Json<Vec<Item>> {
    let store = state.read().await;
    Json(store.items.values().cloned().collect())
}

/// Get an item by ID
#[rustapi::get("/items/{id}")]
#[rustapi::tag("Items")]
#[rustapi::summary("Get item by ID")]
pub async fn get(
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
    State(state): State<AppState>,
    Json(body): Json<CreateItem>,
) -> Result<Created<Json<Item>>> {
    let item = Item::new(body.name, body.description);
    
    let mut store = state.write().await;
    store.items.insert(item.id.clone(), item.clone());
    
    Ok(Created(Json(item)))
}

/// Update an item
#[rustapi::put("/items/{id}")]
#[rustapi::tag("Items")]
#[rustapi::summary("Update an item")]
pub async fn update(
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
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<NoContent> {
    let mut store = state.write().await;
    
    store.items
        .remove(&id)
        .ok_or_else(|| ApiError::not_found(format!("Item {} not found", id)))?;
    
    Ok(NoContent)
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
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

/// In-memory data store
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

/// An item in the store
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Item {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Request to create an item
#[derive(Debug, Deserialize, Schema)]
pub struct CreateItem {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request to update an item
#[derive(Debug, Deserialize, Schema)]
pub struct UpdateItem {
    pub name: Option<String>,
    pub description: Option<String>,
}
"#;
    fs::write(format!("{name}/src/models/mod.rs"), models_mod).await?;

    // .gitignore and .env.example
    common::generate_gitignore(name).await?;
    common::generate_env_example(name).await?;

    Ok(())
}
