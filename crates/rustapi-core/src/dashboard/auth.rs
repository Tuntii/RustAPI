//! Bearer-token authentication for dashboard admin endpoints.

use crate::response::{Body, Response};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::json;

/// Check the `Authorization: Bearer <token>` header against the expected token.
///
/// Returns `Ok(())` if the token is valid or if `expected` is `None` (no auth configured).
/// Returns an HTTP 401/403 response on failure.
pub struct DashboardAuth;

impl DashboardAuth {
    // Response is intentionally returned by value here; callers short-circuit
    // with it immediately, so the extra stack size never matters in practice.
    #[allow(clippy::result_large_err)]
    pub fn check(headers: &http::HeaderMap, expected: &str) -> Result<(), Response> {
        let auth_header = headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        match auth_header {
            Some(value) if value.starts_with("Bearer ") => {
                let token = &value["Bearer ".len()..];
                if token == expected {
                    Ok(())
                } else {
                    Err(json_response(
                        StatusCode::UNAUTHORIZED,
                        json!({
                            "error": "unauthorized",
                            "message": "Invalid admin token"
                        }),
                    ))
                }
            }
            Some(_) => Err(json_response(
                StatusCode::UNAUTHORIZED,
                json!({
                    "error": "unauthorized",
                    "message": "Expected 'Authorization: Bearer <token>'"
                }),
            )),
            None => Err(json_response(
                StatusCode::UNAUTHORIZED,
                json!({
                    "error": "unauthorized",
                    "message": "Authorization header missing"
                }),
            )),
        }
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::Full(Full::new(Bytes::from(bytes))))
        .unwrap()
}
