//! SSE event handlers

use rustapi_rs::prelude::*;
use std::sync::Arc;

use crate::models::{Claims, HealthResponse};
use crate::stores::AppState;

/// SSE event stream endpoint
/// Returns a Server-Sent Events stream for real-time updates
#[rustapi_rs::get("/events")]
#[rustapi_rs::tag("Events")]
#[rustapi_rs::summary("SSE Events")]
async fn events(
    State(_state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser<Claims>,
) -> Json<HealthResponse> {
    // For this example, we return a simple JSON response
    // For real SSE streaming, use rustapi_core::sse::Sse with a stream
    Json(HealthResponse {
        status: "connected".to_string(),
        version: "SSE endpoint - use EventSource to connect".to_string(),
    })
}
