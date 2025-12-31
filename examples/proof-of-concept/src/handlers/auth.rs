//! Authentication handlers

use chrono::Utc;
use rustapi_rs::prelude::*;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{AuthResponse, Claims, LoginRequest, RegisterRequest, User, UserInfo};
use crate::stores::AppState;
use crate::{JWT_SECRET, TOKEN_EXPIRY_SECS};

/// Register a new user
async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<Created<AuthResponse>, ApiError> {
    // Create user (password stored as-is for POC, use bcrypt in production)
    let user = User {
        id: 0, // Will be set by store
        username: body.username,
        email: body.email,
        password_hash: body.password, // In production: hash with bcrypt/argon2
        created_at: Utc::now(),
    };

    let created_user = state
        .users
        .create(user)
        .await
        .ok_or_else(|| ApiError::bad_request("Email already registered"))?;

    // Generate JWT token
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + TOKEN_EXPIRY_SECS;

    let claims = Claims {
        sub: created_user.id.to_string(),
        username: created_user.username.clone(),
        email: created_user.email.clone(),
        exp,
    };

    let token = create_token(&claims, JWT_SECRET)
        .map_err(|e| ApiError::internal(format!("Failed to create token: {}", e)))?;

    Ok(Created(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: TOKEN_EXPIRY_SECS,
        user: UserInfo {
            id: created_user.id,
            username: created_user.username,
            email: created_user.email,
        },
    }))
}

/// Login with email and password
async fn login(
    State(state): State<Arc<AppState>>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Find user by email
    let user = state
        .users
        .find_by_email(&body.email)
        .await
        .ok_or_else(|| ApiError::unauthorized("Invalid email or password"))?;

    // Verify password (plain comparison for POC, use bcrypt in production)
    if user.password_hash != body.password {
        return Err(ApiError::unauthorized("Invalid email or password"));
    }

    // Generate JWT token
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + TOKEN_EXPIRY_SECS;

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        email: user.email.clone(),
        exp,
    };

    let token = create_token(&claims, JWT_SECRET)
        .map_err(|e| ApiError::internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(AuthResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: TOKEN_EXPIRY_SECS,
        user: UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
        },
    }))
}


// Route functions
pub fn register_route() -> Route {
    post_route("/auth/register", register)
}

pub fn login_route() -> Route {
    post_route("/auth/login", login)
}
