//! Data models for the Bookmark Manager POC
//!
//! Contains all domain entities, DTOs, and validation rules.

use chrono::{DateTime, Utc};
use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================
// Domain Entities
// ============================================

/// User entity stored in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Bookmark entity stored in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: u64,
    pub user_id: u64,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub category_id: Option<u64>,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Category entity stored in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: u64,
    pub user_id: u64,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================
// JWT Claims
// ============================================

/// JWT claims structure for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Expiration timestamp
    pub exp: u64,
}

// ============================================
// Authentication DTOs
// ============================================

/// Registration request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

/// Login request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

/// Authentication response with JWT token
#[derive(Debug, Serialize, Schema)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
}

/// User information returned in auth responses
#[rustapi_rs::schema]
#[derive(Debug, Serialize, Schema)]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub email: String,
}

// ============================================
// Bookmark DTOs
// ============================================

/// Create bookmark request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct CreateBookmarkRequest {
    #[validate(url)]
    pub url: String,
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub category_id: Option<u64>,
    pub is_favorite: Option<bool>,
}

/// Update bookmark request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct UpdateBookmarkRequest {
    #[validate(url)]
    pub url: Option<String>,
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub category_id: Option<u64>,
    pub is_favorite: Option<bool>,
}

/// Bookmark response
#[derive(Debug, Clone, Serialize, Schema)]
pub struct BookmarkResponse {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub category_id: Option<u64>,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Bookmark> for BookmarkResponse {
    fn from(b: &Bookmark) -> Self {
        Self {
            id: b.id,
            url: b.url.clone(),
            title: b.title.clone(),
            description: b.description.clone(),
            category_id: b.category_id,
            is_favorite: b.is_favorite,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}

/// Query parameters for listing bookmarks
#[derive(Debug, Deserialize, Default, IntoParams)]
pub struct BookmarkListParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub category_id: Option<u64>,
    pub is_favorite: Option<bool>,
    pub search: Option<String>,
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Schema)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

// ============================================
// Category DTOs
// ============================================

/// Create category request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(length(equal = 7))]
    pub color: Option<String>,
}

/// Update category request body
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct UpdateCategoryRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: Option<String>,
    #[validate(length(equal = 7))]
    pub color: Option<String>,
}

/// Category list response wrapper
#[derive(Debug, Serialize, Schema)]
pub struct CategoryListResponse {
    pub categories: Vec<CategoryResponse>,
}

/// Category response
#[derive(Debug, Serialize, Schema)]
pub struct CategoryResponse {
    pub id: u64,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&Category> for CategoryResponse {
    fn from(c: &Category) -> Self {
        Self {
            id: c.id,
            name: c.name.clone(),
            color: c.color.clone(),
            created_at: c.created_at,
        }
    }
}

// ============================================
// Export/Import DTOs
// ============================================

/// Bookmark export format
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct BookmarkExport {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub is_favorite: bool,
    pub category_name: Option<String>,
}

/// Export response wrapper
#[derive(Debug, Serialize, Schema)]
pub struct ExportResponse {
    pub bookmarks: Vec<BookmarkExport>,
}

/// Import request body
#[derive(Debug, Deserialize, Schema)]
pub struct ImportBookmarksRequest {
    pub bookmarks: Vec<BookmarkExport>,
}

/// Import response
#[derive(Debug, Serialize, Schema)]
pub struct ImportResponse {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// ============================================
// Error Response
// ============================================

/// Standard error response format
#[derive(Debug, Serialize, Schema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

/// Error detail structure
#[derive(Debug, Serialize, Schema)]
pub struct ErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
}

/// Field-level validation error
#[derive(Debug, Serialize, Schema)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

// ============================================
// SSE Events
// ============================================

/// Bookmark event types for SSE
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum BookmarkEvent {
    #[serde(rename = "bookmark_created")]
    Created { bookmark: BookmarkResponse },
    #[serde(rename = "bookmark_updated")]
    Updated { bookmark: BookmarkResponse },
    #[serde(rename = "bookmark_deleted")]
    Deleted { id: u64 },
}

// ============================================
// Health Check
// ============================================

/// Health check response
#[derive(Debug, Serialize, Schema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
