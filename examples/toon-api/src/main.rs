//! TOON Format API Example
//!
//! This example demonstrates the use of TOON (Token-Oriented Object Notation)
//! for LLM-optimized API endpoints. TOON reduces token usage by 20-40% compared
//! to JSON, making it ideal for AI/LLM communication.
//!
//! ## Running
//!
//! ```bash
//! cargo run --example toon-api
//! ```
//!
//! ## Testing
//!
//! ### JSON endpoint (for comparison):
//! ```bash
//! curl http://localhost:8080/json/users
//! ```
//!
//! ### TOON endpoint:
//! ```bash
//! curl http://localhost:8080/toon/users
//! ```
//!
//! ### Content Negotiation (automatic format selection):
//! ```bash
//! # Request JSON (default)
//! curl http://localhost:8080/users
//!
//! # Request TOON format
//! curl -H "Accept: application/toon" http://localhost:8080/users
//!
//! # Request JSON explicitly
//! curl -H "Accept: application/json" http://localhost:8080/users
//! ```
//!
//! ### Create user with TOON:
//! ```bash
//! curl -X POST http://localhost:8080/toon/users \
//!   -H "Content-Type: application/toon" \
//!   -d 'name: Alice
//! email: alice@example.com'
//! ```
//!
//! ## Token Savings Example
//!
//! **JSON (16 tokens, 40 bytes):**
//! ```json
//! {
//!   "users": [
//!     { "id": 1, "name": "Alice" },
//!     { "id": 2, "name": "Bob" }
//!   ]
//! }
//! ```
//!
//! **TOON (13 tokens, 28 bytes) - 18.75% token savings:**
//! ```text
//! users[2]{id,name}:
//!   1,Alice
//!   2,Bob
//! ```

use rustapi_rs::prelude::*;
use rustapi_rs::toon::{api_description_with_toon, LlmResponse, Toon, TOON_FORMAT_DESCRIPTION};

// --- Data Models ---

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct User {
    id: u64,
    name: String,
    email: String,
    role: String,
}

#[derive(Debug, Deserialize, Schema)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct UsersResponse {
    users: Vec<User>,
    total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct Message {
    content: String,
    format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct ComparisonResult {
    json_bytes: usize,
    toon_bytes: usize,
    bytes_saved: usize,
    savings_percent: String,
    note: String,
}

// --- Sample Data ---

fn get_sample_users() -> Vec<User> {
    vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            role: "admin".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            role: "user".to_string(),
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            role: "user".to_string(),
        },
    ]
}

// --- JSON Handlers (for comparison) ---

/// Get all users as JSON
async fn get_users_json() -> Json<UsersResponse> {
    let users = get_sample_users();
    let total = users.len();
    Json(UsersResponse { users, total })
}

/// Create a user (JSON input)
async fn create_user_json(Json(input): Json<CreateUser>) -> Created<User> {
    let user = User {
        id: 4,
        name: input.name,
        email: input.email,
        role: "user".to_string(),
    };
    Created(user)
}

// --- TOON Handlers ---

/// Get all users as TOON format
///
/// Returns users in TOON format, reducing token count for LLM processing.
async fn get_users_toon() -> Toon<UsersResponse> {
    let users = get_sample_users();
    let total = users.len();
    Toon(UsersResponse { users, total })
}

/// Get a single user as TOON
async fn get_user_toon(id: u64) -> Result<Toon<User>> {
    let users = get_sample_users();
    let user = users
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;
    Ok(Toon(user))
}

/// Create a user (TOON input) -> TOON output
///
/// Demonstrates full TOON round-trip: parse TOON request, return TOON response.
async fn create_user_toon(Toon(input): Toon<CreateUser>) -> Toon<User> {
    let user = User {
        id: 4,
        name: input.name,
        email: input.email,
        role: "user".to_string(),
    };
    Toon(user)
}

// --- Content Negotiation Handlers ---

/// Get users with automatic content negotiation
///
/// Returns JSON or TOON based on the client's Accept header:
/// - `Accept: application/json` → JSON response
/// - `Accept: application/toon` → TOON response
/// - Default → JSON response
async fn get_users_negotiate(accept: AcceptHeader) -> Negotiate<UsersResponse> {
    let users = get_sample_users();
    let total = users.len();
    Negotiate::new(UsersResponse { users, total }, accept.preferred)
}

/// Get a single user with content negotiation
async fn get_user_negotiate(id: u64, accept: AcceptHeader) -> Result<Negotiate<User>> {
    let users = get_sample_users();
    let user = users
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;
    Ok(Negotiate::new(user, accept.preferred))
}

// --- LLM-Optimized Handlers (with token counting) ---

/// Get users with LLM optimization and token counting headers
///
/// Returns response with headers:
/// - `X-Token-Count-JSON`: Estimated tokens in JSON format
/// - `X-Token-Count-TOON`: Estimated tokens in TOON format
/// - `X-Token-Savings`: Percentage saved with TOON
/// - `X-Format-Used`: Which format was returned
async fn get_users_llm(accept: AcceptHeader) -> LlmResponse<UsersResponse> {
    let users = get_sample_users();
    let total = users.len();
    LlmResponse::new(UsersResponse { users, total }, accept.preferred)
}

/// Get a single user with LLM optimization
async fn get_user_llm(id: u64, accept: AcceptHeader) -> Result<LlmResponse<User>> {
    let users = get_sample_users();
    let user = users
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;
    Ok(LlmResponse::new(user, accept.preferred))
}

/// Get users optimized for LLM - always TOON format
async fn get_users_llm_toon() -> LlmResponse<UsersResponse> {
    let users = get_sample_users();
    let total = users.len();
    LlmResponse::toon(UsersResponse { users, total })
}

// --- Info/Comparison Handlers ---

/// Compare JSON vs TOON for the same data
async fn compare_formats() -> Json<ComparisonResult> {
    let users = get_sample_users();
    let response = UsersResponse {
        users,
        total: 3,
    };

    // Serialize to both formats
    let json_str = serde_json::to_string_pretty(&response).unwrap();
    let toon_str = rustapi_rs::toon::encode_default(&response).unwrap_or_else(|_| "Error".to_string());

    let json_bytes = json_str.len();
    let toon_bytes = toon_str.len();
    let savings_percent = ((json_bytes as f64 - toon_bytes as f64) / json_bytes as f64) * 100.0;

    Json(ComparisonResult {
        json_bytes,
        toon_bytes,
        bytes_saved: json_bytes - toon_bytes,
        savings_percent: format!("{:.2}%", savings_percent),
        note: "TOON typically saves 20-40% tokens when processed by LLMs".to_string(),
    })
}

/// API info
async fn index() -> Json<Message> {
    Json(Message {
        content: "TOON Format API Example - Use /compare to see JSON vs TOON comparison".to_string(),
        format: "json".to_string(),
    })
}

/// Get TOON format documentation
async fn toon_docs() -> Html<String> {
    // Convert markdown to simple HTML
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>TOON Format Documentation</title>
    <style>
        body {{ font-family: system-ui, sans-serif; max-width: 800px; margin: 2em auto; padding: 0 1em; line-height: 1.6; }}
        h1, h2, h3 {{ color: #333; }}
        pre {{ background: #f4f4f4; padding: 1em; overflow-x: auto; border-radius: 4px; }}
        code {{ background: #f4f4f4; padding: 0.2em 0.4em; border-radius: 3px; }}
        .savings {{ color: #2e7d32; font-weight: bold; }}
    </style>
</head>
<body>
    <h1>🚀 TOON Format Documentation</h1>
    <pre>{}</pre>
    <p><a href="/docs">Back to API Documentation</a></p>
</body>
</html>"#,
        TOON_FORMAT_DESCRIPTION.replace('<', "&lt;").replace('>', "&gt;")
    );
    Html(html)
}

// --- Main ---

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting TOON API example...");
    info!("Server running at http://127.0.0.1:8080");
    info!("");
    info!("Endpoints:");
    info!("  GET  /               - API info");
    info!("  GET  /docs           - Swagger UI (API documentation)");
    info!("  GET  /toon-docs      - TOON format documentation");
    info!("  GET  /compare        - Compare JSON vs TOON");
    info!("  GET  /json/users     - Get users (JSON)");
    info!("  POST /json/users     - Create user (JSON)");
    info!("  GET  /toon/users     - Get users (TOON)");
    info!("  GET  /toon/users/:id - Get user by ID (TOON)");
    info!("  POST /toon/users     - Create user (TOON)");
    info!("  GET  /users          - Get users (content negotiation)");
    info!("  GET  /users/:id      - Get user by ID (content negotiation)");
    info!("  GET  /llm/users      - Get users (LLM optimized with token headers)");
    info!("  GET  /llm/users/:id  - Get user by ID (LLM optimized)");
    info!("  GET  /llm/toon/users - Get users (always TOON format)");
    info!("");
    info!("Content Negotiation Examples:");
    info!("  curl http://localhost:8080/users                        # JSON (default)");
    info!("  curl -H 'Accept: application/toon' http://localhost:8080/users  # TOON");

    // Build API description with TOON support notice
    let description = api_description_with_toon(
        "TOON Format API Example demonstrating LLM-optimized data serialization."
    );

    RustApi::new()
        // Info endpoints
        .route("/", get(index))
        .route("/toon-docs", get(toon_docs))
        .route("/compare", get(compare_formats))
        // JSON endpoints (for comparison)
        .route("/json/users", get(get_users_json))
        .route("/json/users", post(create_user_json))
        // TOON endpoints
        .route("/toon/users", get(get_users_toon))
        .route("/toon/users/:id", get(get_user_toon))
        .route("/toon/users", post(create_user_toon))
        // Content negotiation endpoints
        .route("/users", get(get_users_negotiate))
        .route("/users/:id", get(get_user_negotiate))
        // LLM-optimized endpoints (with token counting headers)
        .route("/llm/users", get(get_users_llm))
        .route("/llm/users/:id", get(get_user_llm))
        .route("/llm/toon/users", get(get_users_llm_toon))
        // OpenAPI/Swagger documentation
        .docs("/docs")
        .run("127.0.0.1:8080")
        .await?;

    Ok(())
}
