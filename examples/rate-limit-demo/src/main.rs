//! Rate Limiting Demo for RustAPI
//!
//! This example demonstrates:
//! - Rate limiting concept
//! - API endpoint protection
//! - Request throttling patterns
//!
//! Run with: cargo run -p rate-limit-demo
//! Then test: curl -i http://127.0.0.1:8080/api/limited (repeat 10+ times)
//!
//! Note: This is a conceptual demo. For production rate limiting,
//! consider using middleware or Redis-based solutions.

use rustapi_rs::prelude::*;

// ============================================
// Response Models
// ============================================

#[derive(Serialize, Schema)]
struct ApiResponse {
    message: String,
    timestamp: u64,
}

#[derive(Serialize, Schema)]
struct StatusResponse {
    status: String,
    requests_remaining: Option<u32>,
}

// ============================================
// Handlers
// ============================================

/// Endpoint with strict rate limiting (5 requests per minute)
#[rustapi_rs::get("/api/limited")]
async fn limited_endpoint() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "This endpoint has strict rate limits".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// Endpoint with relaxed rate limiting (100 requests per minute)
#[rustapi_rs::get("/api/relaxed")]
async fn relaxed_endpoint() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "This endpoint has relaxed rate limits".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// Health check endpoint (no rate limit)
#[rustapi_rs::get("/health")]
async fn health() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "healthy".to_string(),
        requests_remaining: None,
    })
}

/// Root endpoint with information
#[rustapi_rs::get("/")]
async fn index() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "Rate Limiting Demo - Try /api/limited (5 req/min) or /api/relaxed (100 req/min)"
            .to_string(),
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
    println!("🚀 Starting Rate Limiting Demo...");
    println!("📍 Swagger UI: http://127.0.0.1:8080/docs");
    println!("\n📊 Rate Limiting Info:");
    println!("   This demo shows the concept of rate limiting.");
    println!("   In production, use middleware or Redis for actual rate limiting.");
    println!("\n🧪 Test endpoints:");
    println!("   curl http://127.0.0.1:8080/api/limited");
    println!("   curl http://127.0.0.1:8080/api/relaxed");

    RustApi::auto().run("127.0.0.1:8080").await
}
