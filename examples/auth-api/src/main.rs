//! Authentication API Example for RustAPI
//!
//! This example demonstrates:
//! - JWT authentication middleware
//! - Protected routes
//! - Rate limiting
//!
//! Run with: cargo run -p auth-api
//! Then visit: http://127.0.0.1:8080/docs
//!
//! ## Testing the API
//!
//! 1. Login to get a token:
//!    ```bash
//!    curl -X POST http://127.0.0.1:8080/auth/login \
//!      -H "Content-Type: application/json" \
//!      -d '{"username": "admin", "password": "secret"}'
//!    ```
//!
//! 2. Access protected route with token:
//!    ```bash
//!    curl http://127.0.0.1:8080/protected/profile \
//!      -H "Authorization: Bearer <your-token>"
//!    ```

use rustapi_rs::prelude::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================
// Configuration
// ============================================

/// JWT secret key (in production, use environment variable)
const JWT_SECRET: &str = "super-secret-key-change-in-production";

/// Token expiration time (1 hour)
const TOKEN_EXPIRY_SECS: u64 = 3600;

// ============================================
// Data Models
// ============================================

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Expiration timestamp
    pub exp: u64,
}

/// Login request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
    #[validate(length(min = 1, max = 100))]
    pub password: String,
}

/// Login response with JWT token
#[derive(Debug, Serialize, Schema)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// User profile response
#[derive(Debug, Serialize, Schema)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

/// Public message response
#[derive(Debug, Serialize, Schema)]
pub struct Message {
    pub message: String,
}

// ============================================
// Public Handlers (No Auth Required)
// ============================================

/// Public endpoint - no authentication required
#[rustapi_rs::get("/")]
#[rustapi_rs::tag("Public")]
#[rustapi_rs::summary("Welcome")]
async fn welcome() -> Json<Message> {
    Json(Message {
        message: "Welcome to the Auth API! Login at /auth/login".to_string(),
    })
}

/// Health check endpoint
#[rustapi_rs::get("/health")]
#[rustapi_rs::tag("Public")]
#[rustapi_rs::summary("Health Check")]
async fn health() -> &'static str {
    "OK"
}

/// Login endpoint - returns JWT token
#[rustapi_rs::post("/auth/login")]
#[rustapi_rs::tag("Authentication")]
#[rustapi_rs::summary("Login")]
#[rustapi_rs::description("Authenticate with username and password to receive a JWT token.")]
async fn login(ValidatedJson(body): ValidatedJson<LoginRequest>) -> Result<Json<LoginResponse>, ApiError> {
    // In production, verify credentials against database
    // For demo, accept admin/secret
    if body.username != "admin" || body.password != "secret" {
        return Err(ApiError::unauthorized("Invalid username or password"));
    }

    // Calculate expiration time
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + TOKEN_EXPIRY_SECS;

    // Create claims
    let claims = Claims {
        sub: "user-123".to_string(),
        username: body.username,
        role: "admin".to_string(),
        exp,
    };

    // Generate token
    let token = create_token(&claims, JWT_SECRET)
        .map_err(|e| ApiError::internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(LoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: TOKEN_EXPIRY_SECS,
    }))
}


// ============================================
// Protected Handlers (Auth Required)
// ============================================

/// Get current user's profile (requires authentication)
async fn get_profile(AuthUser(claims): AuthUser<Claims>) -> Json<UserProfile> {
    Json(UserProfile {
        user_id: claims.sub,
        username: claims.username,
        role: claims.role,
    })
}

/// Admin-only endpoint
async fn admin_only(AuthUser(claims): AuthUser<Claims>) -> Result<Json<Message>, ApiError> {
    if claims.role != "admin" {
        return Err(ApiError::forbidden("Admin access required"));
    }

    Ok(Json(Message {
        message: format!("Hello admin {}! You have full access.", claims.username),
    }))
}

/// Protected data endpoint
async fn get_protected_data(AuthUser(claims): AuthUser<Claims>) -> Json<Message> {
    Json(Message {
        message: format!("Secret data for user: {}", claims.username),
    })
}

// ============================================
// Main
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔐 Authentication API Example");
    println!();
    println!("Public Endpoints:");
    println!("  GET  /           - Welcome message");
    println!("  GET  /health     - Health check");
    println!("  POST /auth/login - Login (username: admin, password: secret)");
    println!();
    println!("Protected Endpoints (require JWT token):");
    println!("  GET /protected/profile - Get user profile");
    println!("  GET /protected/admin   - Admin only");
    println!("  GET /protected/data    - Protected data");
    println!();
    println!("Documentation:");
    println!("  GET /docs - Swagger UI (Basic Auth: docs / docs123)");
    println!();
    println!("Server running at http://127.0.0.1:8080");

    // Create the app with JWT middleware for protected routes
    // Public routes (/health, /auth/login, /) are excluded from JWT validation
    // Docs has its own Basic Auth protection
    let app = RustApi::new()
        .body_limit(1024 * 1024) // 1MB limit
        .layer(RequestIdLayer::new())
        .layer(TracingLayer::new())
        // Rate limiting: 100 requests per minute
        .layer(RateLimitLayer::new(100, Duration::from_secs(60)))
        // JWT middleware - skip public paths (docs has its own auth)
        .layer(JwtLayer::<Claims>::new(JWT_SECRET)
            .skip_paths(vec!["/health", "/docs", "/auth/login", "/"]))
        .register_schema::<LoginRequest>()
        .register_schema::<LoginResponse>()
        .register_schema::<UserProfile>()
        .register_schema::<Message>()
        // Public routes
        .mount_route(welcome_route())
        .mount_route(health_route())
        .mount_route(login_route())
        // Protected routes
        .route("/protected/profile", get(get_profile))
        .route("/protected/admin", get(admin_only))
        .route("/protected/data", get(get_protected_data))
        // Docs with Basic Auth protection
        .docs_with_auth("/docs", "docs", "docs123");

    app.run("127.0.0.1:8080").await
}
