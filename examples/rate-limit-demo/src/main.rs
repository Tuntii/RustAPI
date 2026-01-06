//! Rate Limiting Demo for RustAPI
//!
//! This example demonstrates:
//! - IP-based rate limiting
//! - Custom rate limit configurations
//! - Per-endpoint rate limits
//! - Rate limit headers (X-RateLimit-*)
//!
//! Run with: cargo run -p rate-limit-demo
//! Then test: curl -i http://127.0.0.1:8080/api/limited (repeat 10+ times)

use rustapi_rs::prelude::*;
use rustapi_rs::extras::ratelimit::{RateLimit, RateLimitConfig};
use std::time::Duration;

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
        message: "Rate Limiting Demo - Try /api/limited (5 req/min) or /api/relaxed (100 req/min)".to_string(),
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
    // Configure rate limiting
    let strict_config = RateLimitConfig {
        max_requests: 5,
        window: Duration::from_secs(60),
        burst_size: 2, // Allow burst of 2 extra requests
    };

    let relaxed_config = RateLimitConfig {
        max_requests: 100,
        window: Duration::from_secs(60),
        burst_size: 10,
    };

    println!("🚀 Starting Rate Limiting Demo...");
    println!("📍 Swagger UI: http://127.0.0.1:8080/docs");
    println!("\n📊 Rate Limits:");
    println!("   - /api/limited: 5 requests/minute (burst: 2)");
    println!("   - /api/relaxed: 100 requests/minute (burst: 10)");
    println!("   - /health: No limit");
    println!("\n🧪 Test with:");
    println!("   for i in {{1..10}}; do curl -i http://127.0.0.1:8080/api/limited; done");

    RustApi::auto()
        .middleware(RateLimit::new(strict_config))
        .run("127.0.0.1:8080")
        .await
}
