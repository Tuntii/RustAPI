//! Middleware Chain Example for RustAPI
//!
//! This example demonstrates:
//! - Custom middleware composition
//! - Request ID tracking
//! - Logging middleware
//! - Authentication middleware
//! - Error handling middleware
//! - Middleware execution order
//!
//! Run with: cargo run -p middleware-chain
//! Then test: curl -H "Authorization: Bearer token123" http://127.0.0.1:8080/api/protected

use rustapi_rs::prelude::*;
use std::time::Instant;
use uuid::Uuid;

// ============================================
// Custom Middleware
// ============================================

/// Request ID Middleware - Adds unique ID to each request
struct RequestIdMiddleware;

impl RequestIdMiddleware {
    fn new() -> Self {
        Self
    }

    async fn handle<B>(&self, req: Request<B>, next: Next<B>) -> Response {
        let request_id = Uuid::new_v4().to_string();
        println!(
            "📝 [{}] New request: {} {}",
            request_id,
            req.method(),
            req.uri()
        );

        // Add request ID to headers
        let mut response = next.run(req).await;
        response
            .headers_mut()
            .insert("X-Request-ID", request_id.parse().unwrap());
        response
    }
}

/// Timing Middleware - Logs request duration
struct TimingMiddleware;

impl TimingMiddleware {
    fn new() -> Self {
        Self
    }

    async fn handle<B>(&self, req: Request<B>, next: Next<B>) -> Response {
        let start = Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();

        let response = next.run(req).await;

        let duration = start.elapsed();
        println!("⏱️  {} {} - {}ms", method, uri, duration.as_millis());

        response
    }
}

/// Custom Auth Middleware - Simple token validation
struct CustomAuthMiddleware;

impl CustomAuthMiddleware {
    fn new() -> Self {
        Self
    }

    async fn handle<B>(&self, req: Request<B>, next: Next<B>) -> Response {
        // Check if route requires auth
        let path = req.uri().path();
        if path.starts_with("/api/protected") {
            // Validate auth header
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        if token == "token123" {
                            println!("✅ Auth successful for {}", path);
                            return next.run(req).await;
                        }
                    }
                }
            }

            println!("❌ Auth failed for {}", path);
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }

        next.run(req).await
    }
}

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
    println!("🚀 Starting Middleware Chain Demo...");
    println!("📍 Swagger UI: http://127.0.0.1:8080/docs");
    println!("\n🔗 Middleware Order:");
    println!("   1. Request ID - Adds unique ID");
    println!("   2. Timing - Logs duration");
    println!("   3. Auth - Validates token for /api/protected");
    println!("\n🧪 Test with:");
    println!("   curl http://127.0.0.1:8080/api/public");
    println!("   curl -H 'Authorization: Bearer token123' http://127.0.0.1:8080/api/protected");
    println!("   curl http://127.0.0.1:8080/api/protected  (should fail)");

    RustApi::auto()
        // Middleware are executed in order
        .middleware(RequestIdMiddleware::new())
        .middleware(TimingMiddleware::new())
        .middleware(CustomAuthMiddleware::new())
        .run("127.0.0.1:8080")
        .await
}
