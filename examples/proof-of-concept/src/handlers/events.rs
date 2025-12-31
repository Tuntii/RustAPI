//! SSE event handlers

use rustapi_rs::prelude::*;
use std::sync::Arc;

use crate::models::{Claims, HealthResponse};
use crate::stores::AppState;

/// SSE event stream endpoint - placeholder for now
/// Real SSE implementation would require ResponseModifier for Sse type
async fn events(
    State(_state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser<Claims>,
) -> Json<HealthResponse> {
    // TODO: Implement proper SSE streaming once ResponseModifier is available for Sse
    Json(HealthResponse {
        status: "connected".to_string(),
        version: "SSE endpoint - use EventSource to connect".to_string(),
    })
}

// Route function
pub fn events_route() -> Route {
    get_route("/events", events)
}
