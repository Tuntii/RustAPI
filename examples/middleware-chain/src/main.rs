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
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

// Import middleware traits from rustapi_core since they're not re-exported
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};

// ============================================
// Custom Middleware
// ============================================

/// Request ID Middleware - Adds unique ID to each request
#[derive(Clone)]
struct RequestIdMiddleware;

impl RequestIdMiddleware {
    fn new() -> Self {
        Self
    }
}

impl MiddlewareLayer for RequestIdMiddleware {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        Box::pin(async move {
            let request_id = generate_request_id();
            println!(
                "📝 [{}] New request: {} {}",
                request_id,
                req.method(),
                req.uri()
            );

            // Call next middleware/handler
            let mut response = next(req).await;

            // Add request ID to response headers
            if let Ok(header_value) = request_id.parse() {
                response.headers_mut().insert("X-Request-ID", header_value);
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Timing Middleware - Logs request duration
#[derive(Clone)]
struct TimingMiddleware;

impl TimingMiddleware {
    fn new() -> Self {
        Self
    }
}

impl MiddlewareLayer for TimingMiddleware {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        Box::pin(async move {
            let start = Instant::now();
            let method = req.method().to_string();
            let uri = req.uri().to_string();

            let response = next(req).await;

            let duration = start.elapsed();
            println!("⏱️  {} {} - {}ms", method, uri, duration.as_millis());

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Custom Auth Middleware - Simple token validation
#[derive(Clone)]
struct CustomAuthMiddleware;

impl CustomAuthMiddleware {
    fn new() -> Self {
        Self
    }
}

impl MiddlewareLayer for CustomAuthMiddleware {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        Box::pin(async move {
            let path = req.uri().path();

            // Check if route requires auth
            if path.starts_with("/api/protected") {
                // Validate auth header
                if let Some(auth_header) = req.headers().get("Authorization") {
                    if let Ok(auth_str) = auth_header.to_str() {
                        if let Some(token) = auth_str.strip_prefix("Bearer ") {
                            if token == "token123" {
                                println!("✅ Auth successful for {}", path);
                                return next(req).await;
                            }
                        }
                    }
                }

                println!("❌ Auth failed for {}", path);
                // Return 401 Unauthorized
                use http::StatusCode;
                return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            }

            next(req).await
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

// ============================================
// Helper Functions
// ============================================

/// Generate a simple request ID
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{:x}-{:x}", timestamp, count)
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
        .layer(RequestIdMiddleware::new())
        .layer(TimingMiddleware::new())
        .layer(CustomAuthMiddleware::new())
        .run("127.0.0.1:8080")
        .await
}
