//! Action registry and dispatch for server actions.

use crate::error::ApiError;
use crate::extract::{Body, Headers, Path};
use crate::response::IntoResponse;
use crate::response::Response;
use bytes::Bytes;
use http::{header, HeaderMap};
use inventory::collect;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;

/// The route path used for action dispatch.
pub const ACTIONS_PATH: &str = "/__actions/{action_id}";

/// A request payload passed to action handlers.
#[derive(Debug, Clone)]
pub struct ActionRequest {
    pub body: Bytes,
}

/// A boxed future returned by action handlers.
pub type ActionFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

/// Function pointer for action handlers.
pub type ActionHandlerFn = fn(ActionRequest) -> ActionFuture;

/// Registry entry for a server action.
pub struct ActionDefinition {
    pub id: &'static str,
    pub handler: ActionHandlerFn,
}

/// Convert an incoming request body into a typed input payload.
pub fn decode_action_input<T: DeserializeOwned>(body: Bytes) -> Result<T, ApiError> {
    serde_json::from_slice(&body)
        .map_err(|err| ApiError::bad_request(format!("Invalid action payload: {}", err)))
}

collect!(ActionDefinition);

/// Find a registered action by id.
pub fn find_action(id: &str) -> Option<&'static ActionDefinition> {
    inventory::iter::<ActionDefinition>
        .into_iter()
        .find(|action| action.id == id)
}

/// Handle an action POST request.
pub async fn action_handler(
    Path(action_id): Path<String>,
    Headers(headers): Headers,
    Body(body): Body,
) -> Response {
    if let Err(err) = enforce_csrf(&headers) {
        return err.into_response();
    }

    let Some(action) = find_action(&action_id) else {
        return ApiError::not_found(format!("Action '{}' not found", action_id)).into_response();
    };

    (action.handler)(ActionRequest { body }).await
}

fn enforce_csrf(headers: &HeaderMap) -> Result<(), ApiError> {
    let header_token = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let cookie_token = extract_cookie(headers, "csrf_token");

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if cookie == header => Ok(()),
        _ => Err(ApiError::forbidden("Invalid CSRF token")),
    }
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    cookie_header
        .split(';')
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if key.trim() == name {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
}
