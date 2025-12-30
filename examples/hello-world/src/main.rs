//! Hello World example for RustAPI
//!
//! Run with: cargo run --example hello-world
//! Or from examples/hello-world: cargo run
//!
//! Then visit: http://127.0.0.1:8080

use rustapi_rs::prelude::*;

#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: i64,
    name: String,
}

/// Hello World endpoint
async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, World!".to_string(),
    })
}

/// Health check endpoint
async fn health() -> &'static str {
    "OK"
}

/// Get user by ID (demonstrates path parameters)
async fn get_user(Path(id): Path<i64>) -> Json<UserResponse> {
    Json(UserResponse {
        id,
        name: format!("User {}", id),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // RustAPI: 5 lines to a working API! 🚀
    RustApi::new()
        .route("/", get(hello))
        .route("/health", get(health))
        .route("/users/{id}", get(get_user))
        .run("127.0.0.1:8080")
        .await
}
