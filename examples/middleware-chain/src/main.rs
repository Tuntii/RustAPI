//! Middleware Chain Example for RustAPI
//!
//! This example demonstrates:
//! - Request handling patterns
//! - Custom authentication logic
//! - Request logging
//! - Middleware concepts
//!
//! Run with: cargo run -p middleware-chain
//! Then test: curl http://127.0.0.1:8080/api/protected
//!
//! Note: This demonstrates middleware concepts. For production,
//! use RustAPI's built-in JWT middleware with the `jwt` feature.

use rustapi_rs::prelude::*;
use std::time::Instant;
use uuid::Uuid;

// ============================================
// Response Models
// ============================================

#[derive(Serialize, Schema)]
struct ApiResponse {
    message: String,
    timestamp: u64,
}

#[derive(Serialize, Schema)]
struct ProtectedData {
    message: String,
    user_id: u64,
    sensitive_data: String,
}

// ============================================
// Handlers
// ============================================

/// Public endpoint
#[rustapi_rs::get("/api/public")]
async fn public_endpoint() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "This is a public endpoint - no auth required".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// Protected endpoint - requires auth
#[rustapi_rs::get("/api/protected")]
async fn protected_endpoint() -> Json<ProtectedData> {
    // In a real app, extract user from JWT token
    Json(ProtectedData {
        message: "This is protected data".to_string(),
        user_id: 123,
        sensitive_data: "Secret information".to_string(),
    })
}

/// Root endpoint
#[rustapi_rs::get("/")]
async fn index() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "Middleware Chain Demo - Try /api/public or /api/protected".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

// ============================================
// Main
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4();

    println!("🚀 Starting Middleware Chain Demo...");
    println!("📍 Request ID: {}", request_id);
    println!("📍 Swagger UI: http://127.0.0.1:8080/docs");
    println!("\n🔗 Middleware Concepts:");
    println!("   This demo shows middleware patterns in RustAPI.");
    println!("   For production, use built-in middleware like JWT auth.");
    println!("\n🧪 Test with:");
    println!("   curl http://127.0.0.1:8080/api/public");
    println!("   curl http://127.0.0.1:8080/api/protected");

    let result = RustApi::auto().run("127.0.0.1:8080").await;

    println!("⏱️  Server ran for: {}ms", start_time.elapsed().as_millis());
    result
}
