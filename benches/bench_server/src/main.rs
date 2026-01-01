//! RustAPI Benchmark Server
//!
//! A minimal server for HTTP load testing (hey, wrk, etc.)
//!
//! Run with: cargo run --release -p bench-server
//! Then test with: hey -n 100000 -c 50 http://127.0.0.1:8080/

use rustapi_rs::prelude::*;

// ============================================
// Response types
// ============================================

#[derive(Serialize, Schema)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize, Schema)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
    created_at: String,
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
    title: String,
    content: String,
}

#[derive(Deserialize, Validate, Schema)]
struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(email)]
    email: String,
}

// ============================================
// Handlers
// ============================================

/// Plain text response - baseline
#[rustapi_rs::get("/")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("Plain text hello")]
async fn hello() -> &'static str {
    "Hello, World!"
}

/// Simple JSON response
#[rustapi_rs::get("/json")]
#[rustapi_rs::tag("Benchmark")]
#[rustapi_rs::summary("JSON hello")]
async fn json_hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, World!".to_string(),
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
        created_at: "2024-01-01T00:00:00Z".to_string(),
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
        title: "Benchmark Post".to_string(),
        content: "This is a test post for benchmarking".to_string(),
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
        created_at: "2024-01-01T00:00:00Z".to_string(),
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
            created_at: "2024-01-01T00:00:00Z".to_string(),
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
// Main
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 RustAPI Benchmark Server");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("📊 Benchmark Endpoints:");
    println!("  GET  /                        - Plain text (baseline)");
    println!("  GET  /json                    - Simple JSON");
    println!("  GET  /users/:id               - JSON + path param");
    println!("  GET  /posts/:id               - JSON + path param (alt)");
    println!("  POST /create-user             - JSON parsing + validation");
    println!("  GET  /users-list              - Large JSON (10 users)");
    println!();
    println!("🔧 Load Test Commands (install hey: go install github.com/rakyll/hey@latest):");
    println!("  hey -n 100000 -c 50 http://127.0.0.1:8080/");
    println!("  hey -n 100000 -c 50 http://127.0.0.1:8080/json");
    println!("  hey -n 100000 -c 50 http://127.0.0.1:8080/users/123");
    println!("  hey -n 50000 -c 50 -m POST -H \"Content-Type: application/json\" \\");
    println!("      -d '{{\"name\":\"Test\",\"email\":\"test@example.com\"}}' http://127.0.0.1:8080/create-user");
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("🌐 Server running at: http://127.0.0.1:8080");
    println!();

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
