use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// User entity
#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct User {
    /// Unique identifier
    id: i32,
    /// Username
    username: String,
    /// Email address
    email: Option<String>,
    /// User role
    role: UserRole,
}

/// User Role
#[derive(Debug, Serialize, Deserialize, ToSchema)]
enum UserRole {
    Admin,
    User,
    Guest,
}

/// Search parameters for listing users
#[derive(Debug, Deserialize, IntoParams)]
struct SearchParams {
    /// Search query
    query: String,
    /// Page number (default: 1)
    #[serde(default)]
    page: Option<usize>,
    /// Items per page (default: 10)
    #[serde(default)]
    limit: Option<usize>,
}

/// List users
#[rustapi::get("/users")]
async fn list_users(Query(params): Query<SearchParams>) -> Json<Vec<User>> {
    println!("Searching: {:?}", params);
    Json(vec![User {
        id: 1,
        username: "alice".to_string(),
        email: Some("alice@example.com".to_string()),
        role: UserRole::Admin,
    }])
}

/// Create a new user
#[rustapi::post("/users")]
async fn create_user(Json(user): Json<User>) -> Created<Json<User>> {
    Created(Json(user))
}

#[tokio::main]
async fn main() {
    // initialize logger
    tracing_subscriber::fmt::init();

    println!("Building RustAPI application...");

    let app = RustApi::auto()
        .openapi_info("Demo API", "1.0.0", Some("A demo for Native OpenAPI"))
        .register_schema::<User>() // Register explicit schemas if needed, though auto-routes handle it usually
        .docs("/docs");

    // Print the generated OpenAPI spec to stdout for verification
    println!("\n--- Generated OpenAPI Spec ---\n");
    let spec = app.openapi_spec();
    let json_spec = serde_json::to_string_pretty(&spec.to_json()).unwrap();
    println!("{}", json_spec);

    println!("\n------------------------------\n");
    println!("Server would run here. To run locally:");
    println!("  app.run(\"127.0.0.1:8080\").await");
}
