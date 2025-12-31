//! Request handlers for the Bookmark Manager POC

pub mod auth;
pub mod bookmarks;
pub mod categories;
pub mod events;

use rustapi_rs::prelude::*;

use crate::models::HealthResponse;

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub fn health_route() -> Route {
    get_route("/health", health)
}
