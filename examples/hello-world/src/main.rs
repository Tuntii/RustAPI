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

#[derive(Serialize, Schema)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
}

/// Request body with validation
#[derive(Deserialize, Validate, Schema)]
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
#[rustapi_rs::tag("General")]
#[rustapi_rs::summary("Hello World")]
async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, World!".to_string(),
    })
}

/// Health check endpoint
#[rustapi_rs::get("/health")]
#[rustapi_rs::tag("General")]
#[rustapi_rs::summary("Health Check")]
#[rustapi_rs::description("Returns 'OK' if the server is healthy")]
async fn health() -> &'static str {
    "OK"
}

/// Get user by ID
#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Get User")]
#[rustapi_rs::description("Retrieves a user by their unique ID")]
async fn get_user(Path(id): Path<i64>) -> Json<UserResponse> {
    Json(UserResponse {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    })
}

/// Create a new user with validation
#[rustapi_rs::post("/users")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Create User")]
#[rustapi_rs::description("Creates a new user. Validates name (1-100 chars) and email format. Returns 422 on validation failure.")]
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
    println!("  GET  /docs      - Swagger UI");
    println!();
    
    RustApi::new()
        .register_schema::<UserResponse>()
        .register_schema::<CreateUser>()
        .mount_route(hello_route())
        .mount_route(health_route())
        .mount_route(get_user_route())
        .mount_route(create_user_route())
        .docs("/docs")  // Enable Swagger UI!
        .run("127.0.0.1:8080")
        .await
}
