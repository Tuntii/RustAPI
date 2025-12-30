//! Hello World example for RustAPI
//!
//! Run with: cargo run --example hello-world
//! Or from examples/hello-world: cargo run
//!
//! Then visit:
//! - http://127.0.0.1:8080 - Hello World
//! - http://127.0.0.1:8080/docs - Swagger UI
//! - http://127.0.0.1:8080/openapi.json - OpenAPI spec

use rustapi_rs::prelude::*;
use validator::Validate as ValidatorValidate;

#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: i64,
    name: String,
    email: Option<String>,
}

/// Request body for creating a user - with validation!
#[derive(Deserialize, ValidatorValidate)]
struct CreateUserRequest {
    #[validate(length(min = 1, max = 100))]
    name: String,
    
    #[validate(email)]
    email: String,
    
    #[validate(range(min = 18, max = 120))]
    age: u8,
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
        email: None,
    })
}

/// Create user with validation!
/// 
/// This endpoint demonstrates:
/// - JSON body parsing
/// - Automatic validation with 422 errors
/// - Custom error messages
///
/// Try sending invalid data:
/// ```powershell
/// Invoke-RestMethod -Uri http://127.0.0.1:8080/users -Method POST -ContentType "application/json" -Body '{"name":"","email":"not-email","age":15}'
/// ```
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUserRequest>) -> impl IntoResponse {
    // If we reach here, validation passed!
    created(UserResponse {
        id: 1,
        name: body.name,
        email: Some(body.email),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // RustAPI: DX-first web framework for Rust! 🚀
    info!("Starting RustAPI server at http://127.0.0.1:8080");
    
    RustApi::new()
        // Configure OpenAPI documentation
        .openapi(
            OpenApiDoc::new("Hello World API", "1.0.0")
                .description("A sample RustAPI application demonstrating validation and OpenAPI")
                .server("http://127.0.0.1:8080")
        )
        // Enable Swagger UI at /docs
        .docs("/docs")
        // Define routes
        .route("/", get(hello))
        .route("/health", get(health))
        .route("/users/{id}", get(get_user))
        .route("/users", post(create_user))
        .run("127.0.0.1:8080")
        .await
}
