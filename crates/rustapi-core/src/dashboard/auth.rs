//! Bearer-token authentication for dashboard admin endpoints.

use crate::response::{Body, Response};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::json;

/// Check the `Authorization: Bearer <token>` header against the expected token.
///
/// Returns `Ok(())` if the bearer token is present and matches `expected`.
/// Returns an HTTP 401 response on failure.
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
            Some(value) => {
                let Some(token) = bearer_token(value) else {
                    return Err(json_response(
                        StatusCode::UNAUTHORIZED,
                        json!({
                            "error": "unauthorized",
                            "message": "Expected 'Authorization: Bearer <token>'"
                        }),
                    ));
                };

                if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
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

fn bearer_token(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();

    for idx in 0..max_len {
        let left_byte = left.get(idx).copied().unwrap_or(0);
        let right_byte = right.get(idx).copied().unwrap_or(0);
        diff |= (left_byte ^ right_byte) as usize;
    }

    diff == 0
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CACHE_CONTROL, "no-store")
        .header(http::header::REFERRER_POLICY, "no-referrer")
        .header("x-content-type-options", "nosniff")
        .body(Body::Full(Full::new(Bytes::from(bytes))))
        .unwrap()
}
