//! # Native OpenAPI 3.1 Demo
//!
//! This example demonstrates RustAPI's native OpenAPI 3.1 generator,
//! which replaced the `utoipa` dependency in v0.1.203.
//!
//! ## Features Demonstrated:
//! - `#[derive(Schema)]` for automatic JSON Schema generation
//! - `RustApiSchema` trait for custom types
//! - OpenAPI 3.1 nullable types (`type: ["string", "null"]`)
//! - Nested type schema generation with `$ref`
//! - Query parameter schemas via `field_schemas()`
//!
//! Run with: `cargo run -p rustapi-rs --example native_openapi_demo`

use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// DATA MODELS WITH NATIVE SCHEMA DERIVATION
// ============================================================================

/// A user in the system.
/// The Schema derive generates OpenAPI 3.1 compatible JSON Schema.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct User {
    /// Unique identifier
    pub id: i64,

    /// Display name
    pub name: String,

    /// Email address (optional field generates nullable type)
    pub email: Option<String>,

    /// User's roles - generates array schema
    pub roles: Vec<String>,

    /// User's profile information - nested schema with $ref
    pub profile: Option<UserProfile>,
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct UserProfile {
    /// Profile bio
    pub bio: Option<String>,

    /// Profile avatar URL
    pub avatar_url: Option<String>,

    /// Social links - generates object with additionalProperties
    pub social_links: HashMap<String, String>,
}

/// Request body for creating a user
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct CreateUserRequest {
    /// User's name (required)
    pub name: String,

    /// User's email (optional)
    pub email: Option<String>,

    /// Initial roles
    pub roles: Vec<String>,
}

/// Request body for updating a user
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct UpdateUserRequest {
    /// New name (optional)
    pub name: Option<String>,

    /// New email (optional)
    pub email: Option<String>,

    /// Updated profile
    pub profile: Option<UserProfile>,
}

/// Query parameters for listing users
#[derive(Debug, Clone, Deserialize, Schema)]
pub struct ListUsersQuery {
    /// Page number (1-indexed)
    pub page: Option<i32>,

    /// Items per page
    pub limit: Option<i32>,

    /// Search by name
    pub search: Option<String>,

    /// Filter by role
    pub role: Option<String>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Schema)]
pub struct UserListResponse {
    /// List of users
    pub data: Vec<User>,

    /// Total count
    pub total: i64,

    /// Current page
    pub page: i32,

    /// Items per page
    pub limit: i32,
}

/// API Error response
#[derive(Debug, Clone, Serialize, Schema)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,

    /// Human-readable message
    pub message: String,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List all users with pagination and filtering
#[rustapi_rs::get("/users")]
async fn list_users(Query(query): Query<ListUsersQuery>) -> Json<UserListResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);

    // Demo data
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            roles: vec!["admin".to_string()],
            profile: Some(UserProfile {
                bio: Some("Software developer".to_string()),
                avatar_url: None,
                social_links: HashMap::from([(
                    "github".to_string(),
                    "https://github.com/alice".to_string(),
                )]),
            }),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: None,
            roles: vec!["user".to_string()],
            profile: None,
        },
    ];

    Json(UserListResponse {
        data: users,
        total: 2,
        page,
        limit,
    })
}

/// Get a user by ID
#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i64>) -> Result<Json<User>, rustapi_rs::Response> {
    // Demo: return user if id is 1, otherwise not found
    if id == 1 {
        Ok(Json(User {
            id: 1,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            roles: vec!["admin".to_string()],
            profile: None,
        }))
    } else {
        Err(rustapi_rs::StatusCode::NOT_FOUND.into_response())
    }
}

/// Create a new user
#[rustapi_rs::post("/users")]
async fn create_user(Json(body): Json<CreateUserRequest>) -> Json<User> {
    let user = User {
        id: 42, // Demo: generated ID
        name: body.name,
        email: body.email,
        roles: body.roles,
        profile: None,
    };

    Json(user)
}

/// Update an existing user
#[rustapi_rs::put("/users/{id}")]
async fn update_user(
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<User>, rustapi_rs::Response> {
    if id != 1 {
        return Err(rustapi_rs::StatusCode::NOT_FOUND.into_response());
    }

    Ok(Json(User {
        id,
        name: body.name.unwrap_or_else(|| "Alice".to_string()),
        email: body.email,
        roles: vec!["admin".to_string()],
        profile: body.profile,
    }))
}

/// Delete a user
#[rustapi_rs::delete("/users/{id}")]
async fn delete_user(Path(id): Path<i64>) -> rustapi_rs::Response {
    if id != 1 {
        return rustapi_rs::StatusCode::NOT_FOUND.into_response();
    }
    rustapi_rs::StatusCode::NO_CONTENT.into_response()
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    println!("🚀 Native OpenAPI 3.1 Demo");
    println!("==========================\n");

    println!("📖 OpenAPI Features Demonstrated:");
    println!("   • OpenAPI 3.1.0 specification");
    println!("   • JSON Schema 2020-12 dialect");
    println!("   • Native nullable types: type: [\"string\", \"null\"]");
    println!("   • Nested schemas with $ref");
    println!("   • Array schemas for Vec<T>");
    println!("   • Object schemas for HashMap<String, T>");
    println!("   • Query parameter extraction\n");

    println!("🔗 Endpoints:");
    println!("   GET    /users        - List users with pagination");
    println!("   GET    /users/{{id}}   - Get user by ID");
    println!("   POST   /users        - Create new user");
    println!("   PUT    /users/{{id}}   - Update user");
    println!("   DELETE /users/{{id}}   - Delete user\n");

    println!("📚 Documentation:");
    println!("   Swagger UI:   http://localhost:8080/docs");
    println!("   OpenAPI JSON: http://localhost:8080/openapi.json\n");

    println!("Starting server on http://localhost:8080...\n");

    // RustApi::auto() automatically discovers and mounts all routes
    // defined with #[get], #[post], etc. macros in this binary
    RustApi::auto().run("127.0.0.1:8080").await.unwrap();
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_openapi::schema::{RustApiSchema, SchemaCtx, SchemaRef};

    #[test]
    fn test_user_schema_generation() {
        let mut ctx = SchemaCtx::new();
        let schema = User::schema(&mut ctx);

        // Should return a $ref since User has a component name
        match schema {
            SchemaRef::Ref { reference } => {
                assert_eq!(reference, "#/components/schemas/User");
            }
            _ => panic!("Expected $ref for User schema"),
        }

        // Check that User was added to components
        assert!(ctx.components.contains_key("User"));
    }

    #[test]
    fn test_optional_field_nullable() {
        let mut ctx = SchemaCtx::new();
        let _ = User::schema(&mut ctx);

        // Get the User schema from components
        let user_schema = ctx
            .components
            .get("User")
            .expect("User schema should exist");

        // Check properties exist
        assert!(user_schema.properties.is_some());
        let props = user_schema.properties.as_ref().unwrap();

        // email should be in properties (nullable handling is in the type)
        assert!(props.contains_key("email"));
    }

    #[test]
    fn test_nested_schema_ref() {
        let mut ctx = SchemaCtx::new();
        let _ = User::schema(&mut ctx);

        // UserProfile should also be registered due to nested reference
        assert!(ctx.components.contains_key("UserProfile"));
    }

    #[test]
    fn test_component_name() {
        assert_eq!(User::component_name(), Some("User"));
        assert_eq!(UserProfile::component_name(), Some("UserProfile"));
        assert_eq!(
            CreateUserRequest::component_name(),
            Some("CreateUserRequest")
        );
    }
}
