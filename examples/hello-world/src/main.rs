//! Hello World example for RustAPI
//!
//! Run with: cargo run -p hello-world
//!
//! Then visit: http://127.0.0.1:8080

use rustapi_rs::prelude::*;

// ============================================
// Response types
// ============================================

#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
}

/// Request body with validation
#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    name: String,
    
    #[validate(email)]
    email: String,
}

// ============================================
// Handlers using attribute macros
// ============================================

/// Hello World endpoint
#[rustapi_rs::get("/")]
async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, World!".to_string(),
    })
}

/// Health check endpoint
#[rustapi_rs::get("/health")]
async fn health() -> &'static str {
    "OK"
}

/// Get user by ID
#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i64>) -> Json<UserResponse> {
    Json(UserResponse {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    })
}

/// Create a new user with validation
/// 
/// Validates that:
/// - name is 1-100 characters
/// - email is a valid email format
/// 
/// Returns 422 with field errors if validation fails
#[rustapi_rs::post("/users")]
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUser>) -> Json<UserResponse> {
    Json(UserResponse {
        id: 1,
        name: body.name,
        email: body.email,
    })
}

// ============================================
// Main entry point
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 RustAPI Example Server");
    println!("Routes:");
    println!("  GET  /          - Hello World");
    println!("  GET  /health    - Health check");
    println!("  GET  /users/:id - Get user by ID");
    println!("  POST /users     - Create user (validates name & email)");
    println!();
    
    RustApi::new()
        .mount_route(hello_route())
        .mount_route(health_route())
        .mount_route(get_user_route())
        .mount_route(create_user_route())
        .run("127.0.0.1:8080")
        .await
}
