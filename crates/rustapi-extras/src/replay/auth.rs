//! Admin authentication for replay endpoints.

use bytes::Bytes;
use http_body_util::Full;
use rustapi_core::ResponseBody;
use rustapi_core::Response;
use serde_json::json;

/// Admin authentication check for replay endpoints.
pub struct ReplayAdminAuth;

impl ReplayAdminAuth {
    /// Check the admin bearer token from the Authorization header.
    ///
    /// Returns `Ok(())` if the token is valid, or an `Err(Response)` with
    /// a 401 JSON error if the token is missing or invalid.
    #[allow(clippy::result_large_err)]
    pub fn check(
        headers: &http::HeaderMap,
        expected_token: &str,
    ) -> Result<(), Response> {
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok());

        let expected = format!("Bearer {}", expected_token);

        match auth {
            Some(value) if value == expected => Ok(()),
            _ => {
                let body = json!({
                    "error": "unauthorized",
                    "message": "Missing or invalid admin token. Use Authorization: Bearer <token>"
                });
                let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
                let response = http::Response::builder()
                    .status(http::StatusCode::UNAUTHORIZED)
                    .header("content-type", "application/json")
                    .body(ResponseBody::Full(Full::new(Bytes::from(body_bytes))))
                    .unwrap();
                Err(response)
            }
        }
    }
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
    fn test_missing_token() {
        let headers = HeaderMap::new();
        assert!(ReplayAdminAuth::check(&headers, "my-token").is_err());
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
