//! Example: Pins API with UUID path parameters and validation
//!
//! This example demonstrates:
//! - Automatic UUID schema detection for Path<Uuid>
//! - ValidatedJson for input validation
//! - Created response type
//! - State management
//!
//! Run with: cargo run -p rustapi-rs --example pins_api
//! Then visit: http://localhost:8080/docs

use rustapi_rs::prelude::*;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// =============================================================================
// State
// =============================================================================

#[derive(Clone)]
pub struct AppState {
    pub pins: Arc<Mutex<Vec<Pin>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pins: Arc::new(Mutex::new(vec![])),
        }
    }
}

// =============================================================================
// Models
// =============================================================================

/// A pin resource
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct Pin {
    pub id: String, // UUID as string for simplicity
    pub title: String,
    pub description: Option<String>,
}

/// Request body for creating a pin
#[derive(Debug, Clone, Deserialize, Validate, Schema)]
pub struct CreatePin {
    #[validate(length(min = 1, max = 100, message = "Title must be between 1 and 100 characters"))]
    pub title: String,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

// =============================================================================
// Handlers
// =============================================================================

/// List all pins
#[rustapi_rs::get("/pins")]
#[rustapi_rs::tag("Pins")]
#[rustapi_rs::summary("List pins")]
#[rustapi_rs::description("Returns all pins.")]
async fn list_pins(State(state): State<AppState>) -> Json<Pin> {
    let pins = state.pins.lock().unwrap();
    let pin = pins.first().cloned().unwrap_or(Pin {
        id: "default".to_string(),
        title: "No pins".to_string(),
        description: None,
    });
    Json(pin)
}

/// Get a pin by ID
#[rustapi_rs::get("/pins/{id}")]
#[rustapi_rs::tag("Pins")]
#[rustapi_rs::summary("Get pin")]
#[rustapi_rs::description("Returns a pin by its UUID.")]
async fn get_pin(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Pin>> {
    let pins = state.pins.lock().unwrap();
    let pin = pins.iter().find(|p| p.id == id.to_string()).cloned();

    match pin {
        Some(p) => Ok(Json(p)),
        None => Err(ApiError::not_found(format!("Pin with id {} not found", id))),
    }
}

/// Create a new pin
#[rustapi_rs::post("/pins")]
#[rustapi_rs::tag("Pins")]
#[rustapi_rs::summary("Create pin")]
#[rustapi_rs::description("Creates a new pin with validation.")]
async fn create_pin(
    ValidatedJson(payload): ValidatedJson<CreatePin>,
) -> Json<Pin> {
    let pin = Pin {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        description: payload.description,
    };

    Json(pin)
}

/// Delete a pin
#[rustapi_rs::delete("/pins/{id}")]
#[rustapi_rs::tag("Pins")]
#[rustapi_rs::summary("Delete pin")]
#[rustapi_rs::description("Deletes a pin by its UUID.")]
async fn delete_pin(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Pin>> {
    let mut pins = state.pins.lock().unwrap();
    let pos = pins.iter().position(|p| p.id == id.to_string());

    match pos {
        Some(i) => {
            let pin = pins.remove(i);
            Ok(Json(pin))
        }
        None => Err(ApiError::not_found(format!("Pin with id {} not found", id))),
    }
}

// =============================================================================
// Main
// =============================================================================

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Starting Pins API server...");
    println!("📚 Swagger UI: http://localhost:8080/docs");
    println!("📄 OpenAPI JSON: http://localhost:8080/docs/openapi.json");

    let state = AppState::new();

    RustApi::auto()
        .state(state)
        .openapi_info("Pins API", "1.0.0", Some("A simple pins API demonstrating UUID path parameters and validation"))
        .run("127.0.0.1:8080")
        .await
}
