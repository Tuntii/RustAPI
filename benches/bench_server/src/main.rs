//! RustAPI Benchmark Server
//!
//! A minimal server for HTTP load testing (hey, wrk, etc.)
//! Optimized for maximum performance benchmarks.
//!
//! Run with: cargo run --release -p bench-server
//! Then test with: hey -n 100000 -c 50 http://127.0.0.1:8080/

use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct HelloResponse {
    message: &'static str,
}

#[derive(Serialize, Schema)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
    created_at: &'static str,
    is_active: bool,
}

#[derive(Serialize, Schema)]
struct UsersListResponse {
    users: Vec<UserResponse>,
    total: usize,
    page: usize,
}

#[derive(Serialize, Schema)]
struct PostResponse {
    post_id: i64,
    title: &'static str,
    content: &'static str,
}

#[derive(Deserialize, Validate, Schema)]
struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(email)]
    email: String,
}

// ============================================
// Handlers - Optimized for benchmarks
// ============================================

/// Plain text response - baseline (zero allocation)
#[rustapi_rs::get("/")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("Plain text hello")]
async fn hello() -> &'static str {
    "Hello, World!"
}

/// Simple JSON response - pre-serialized bytes
#[rustapi_rs::get("/json")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("JSON hello")]
async fn json_hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, World!",
    })
}

/// JSON response with path parameter
#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("Get user by ID")]
async fn get_user(Path(id): Path<i64>) -> Json<UserResponse> {
    Json(UserResponse {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
        created_at: "2024-01-01T00:00:00Z",
        is_active: true,
    })
}

/// JSON response with path parameter
#[rustapi_rs::get("/posts/{id}")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("Get post by ID")]
async fn get_post(Path(id): Path<i64>) -> Json<PostResponse> {
    Json(PostResponse {
        post_id: id,
        title: "Benchmark Post",
        content: "This is a test post for benchmarking",
    })
}

/// JSON request body parsing with validation
#[rustapi_rs::post("/create-user")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("Create user with validation")]
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUser>) -> Json<UserResponse> {
    Json(UserResponse {
        id: 1,
        name: body.name,
        email: body.email,
        created_at: "2024-01-01T00:00:00Z",
        is_active: true,
    })
}

/// Larger JSON response (10 users)
#[rustapi_rs::get("/users-list")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("List users (10 items)")]
async fn list_users() -> Json<UsersListResponse> {
    let users: Vec<UserResponse> = (1..=10)
        .map(|id| UserResponse {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
            created_at: "2024-01-01T00:00:00Z",
            is_active: id % 2 == 0,
        })
        .collect();

    Json(UsersListResponse {
        total: 100,
        page: 1,
        users,
    })
}

// ============================================
// Main - Optimized minimal server
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Minimal output for benchmarks
    eprintln!("🚀 RustAPI Benchmark Server @ http://127.0.0.1:8080");

    RustApi::new()
        .mount_route(hello_route())
        .mount_route(json_hello_route())
        .mount_route(get_user_route())
        .mount_route(get_post_route())
        .mount_route(create_user_route())
        .mount_route(list_users_route())
        .run("127.0.0.1:8080")
        .await
}
