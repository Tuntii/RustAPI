//! Actix-web benchmark server for comparison
//!
//! Run with: cargo run --release -p actix-bench-server
//! Then test with: hey -n 100000 -c 50 http://127.0.0.1:8081/

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

// ============================================
// Response types (same as RustAPI)
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
    created_at: String,
    is_active: bool,
}

#[derive(Serialize)]
struct UsersListResponse {
    users: Vec<UserResponse>,
    total: usize,
    page: usize,
}

#[derive(Serialize)]
struct PostResponse {
    user_id: i64,
    post_id: i64,
    title: String,
    content: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

// ============================================
// Handlers
// ============================================

#[get("/")]
async fn hello() -> impl Responder {
    "Hello, World!"
}

#[get("/json")]
async fn json_hello() -> impl Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello, World!".to_string(),
    })
}

#[get("/users/{id}")]
async fn get_user(path: web::Path<i64>) -> impl Responder {
    let id = path.into_inner();
    HttpResponse::Ok().json(UserResponse {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        is_active: true,
    })
}

#[get("/users/{user_id}/posts/{post_id}")]
async fn get_user_post(path: web::Path<(i64, i64)>) -> impl Responder {
    let (user_id, post_id) = path.into_inner();
    HttpResponse::Ok().json(PostResponse {
        user_id,
        post_id,
        title: "Benchmark Post".to_string(),
        content: "This is a test post for benchmarking".to_string(),
    })
}

#[post("/users")]
async fn create_user(body: web::Json<CreateUser>) -> impl Responder {
    HttpResponse::Ok().json(UserResponse {
        id: 1,
        name: body.name.clone(),
        email: body.email.clone(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        is_active: true,
    })
}

#[get("/users")]
async fn list_users() -> impl Responder {
    let users: Vec<UserResponse> = (1..=10)
        .map(|id| UserResponse {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: id % 2 == 0,
        })
        .collect();

    HttpResponse::Ok().json(UsersListResponse {
        total: 100,
        page: 1,
        users,
    })
}

// ============================================
// Main
// ============================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Actix-web Benchmark Server (for comparison)");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("📊 Benchmark Endpoints:");
    println!("  GET  /                        - Plain text (baseline)");
    println!("  GET  /json                    - Simple JSON");
    println!("  GET  /users/:id               - JSON + path param");
    println!("  GET  /users/:uid/posts/:pid   - JSON + 2 path params");
    println!("  POST /users                   - JSON parsing");
    println!("  GET  /users                   - Large JSON (10 users)");
    println!();
    println!("🔧 Load Test Commands:");
    println!("  hey -n 100000 -c 50 http://127.0.0.1:8081/");
    println!("  hey -n 100000 -c 50 http://127.0.0.1:8081/json");
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("🌐 Server running at: http://127.0.0.1:8081");
    println!();

    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(json_hello)
            .service(get_user)
            .service(get_user_post)
            .service(create_user)
            .service(list_users)
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}
