use rustapi_rs::prelude::*;

use crate::auth::jwt::{self, Claims};

/// Authenticated user extracted from `Authorization: Bearer <jwt>`.
#[derive(Debug, Clone)]
pub struct BearerAuth(pub Claims);

impl BearerAuth {
    pub fn user_id(&self) -> &str {
        &self.0.sub
    }

    pub fn from_headers(headers: &http::HeaderMap, jwt_secret: &str) -> Result<Self> {
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .or_else(|| auth_header.strip_prefix("bearer "))
            .ok_or_else(|| ApiError::unauthorized("Expected Bearer token"))?;

        let claims = jwt::verify_token(token, jwt_secret)
            .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

        Ok(Self(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    const JWT_SECRET: &str = "test-jwt-secret-bearer";

    fn auth_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        headers
    }

    #[test]
    fn rejects_missing_and_invalid_tokens() {
        let err = BearerAuth::from_headers(&HeaderMap::new(), JWT_SECRET).unwrap_err();
        assert!(err.to_string().contains("Missing Authorization"));

        let err = BearerAuth::from_headers(&auth_headers("bad"), JWT_SECRET).unwrap_err();
        assert!(err.to_string().contains("Invalid") || err.to_string().contains("expired"));
    }

    #[test]
    fn accepts_valid_token() {
        let (token, _) = crate::auth::jwt::create_token(
            "user-1",
            "alice",
            None,
            "hobby",
            JWT_SECRET,
            1,
        )
        .expect("token");

        let auth = BearerAuth::from_headers(&auth_headers(&token), JWT_SECRET).expect("valid");
        assert_eq!(auth.user_id(), "user-1");
    }
}