//! Admin authentication for replay endpoints.

use bytes::Bytes;
use http_body_util::Full;
use rustapi_core::Response;
use rustapi_core::ResponseBody;
use serde_json::json;

/// Admin authentication check for replay endpoints.
pub struct ReplayAdminAuth;

impl ReplayAdminAuth {
    /// Check the admin bearer token from the Authorization header.
    ///
    /// Returns `Ok(())` if the token is valid, or an `Err(Response)` with
    /// a 401 JSON error if the token is missing or invalid.
    #[allow(clippy::result_large_err)]
    pub fn check(headers: &http::HeaderMap, expected_token: &str) -> Result<(), Response> {
        let auth = headers.get("authorization").and_then(|v| v.to_str().ok());

        match auth {
            Some(value) if bearer_token_matches(value, expected_token) => Ok(()),
            _ => {
                let body = json!({
                    "error": "unauthorized",
                    "message": "Missing or invalid admin token. Use Authorization: Bearer <token>"
                });
                let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
                let response = http::Response::builder()
                    .status(http::StatusCode::UNAUTHORIZED)
                    .header("content-type", "application/json")
                    .header(http::header::CACHE_CONTROL, "no-store")
                    .header(http::header::REFERRER_POLICY, "no-referrer")
                    .header("x-content-type-options", "nosniff")
                    .body(ResponseBody::Full(Full::new(Bytes::from(body_bytes))))
                    .unwrap();
                Err(response)
            }
        }
    }
}

fn bearer_token_matches(value: &str, expected_token: &str) -> bool {
    let Some((scheme, token)) = value.split_once(' ') else {
        return false;
    };

    scheme.eq_ignore_ascii_case("Bearer")
        && constant_time_eq(token.trim().as_bytes(), expected_token.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn test_valid_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer my-token".parse().unwrap());
        assert!(ReplayAdminAuth::check(&headers, "my-token").is_ok());
    }

    #[test]
    fn test_valid_token_accepts_case_insensitive_bearer_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "bearer my-token".parse().unwrap());
        assert!(ReplayAdminAuth::check(&headers, "my-token").is_ok());
    }

    #[test]
    fn test_missing_token() {
        let headers = HeaderMap::new();
        let response = ReplayAdminAuth::check(&headers, "my-token").unwrap_err();
        assert_eq!(response.status(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(http::header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert_eq!(
            response
                .headers()
                .get(http::header::REFERRER_POLICY)
                .unwrap(),
            "no-referrer"
        );
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[test]
    fn test_wrong_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer wrong-token".parse().unwrap());
        assert!(ReplayAdminAuth::check(&headers, "my-token").is_err());
    }

    #[test]
    fn test_no_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "my-token".parse().unwrap());
        assert!(ReplayAdminAuth::check(&headers, "my-token").is_err());
    }
}
