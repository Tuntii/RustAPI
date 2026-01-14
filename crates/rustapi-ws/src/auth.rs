//! WebSocket authentication support
//!
//! This module provides authentication infrastructure for WebSocket connections,
//! allowing token validation before the WebSocket upgrade completes.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_ws::auth::{WsAuthConfig, TokenExtractor, TokenValidator, Claims};
//! use async_trait::async_trait;
//!
//! struct MyTokenValidator;
//!
//! #[async_trait]
//! impl TokenValidator for MyTokenValidator {
//!     async fn validate(&self, token: &str) -> Result<Claims, AuthError> {
//!         // Validate JWT or other token format
//!         Ok(Claims::new("user_123"))
//!     }
//! }
//!
//! let config = WsAuthConfig::new(Box::new(MyTokenValidator))
//!     .extractor(TokenExtractor::Header("Authorization".to_string()));
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Error type for WebSocket authentication
#[derive(Error, Debug, Clone)]
pub enum AuthError {
    /// Token is missing from the request
    #[error("Authentication token missing")]
    TokenMissing,

    /// Token format is invalid
    #[error("Invalid token format: {0}")]
    InvalidFormat(String),

    /// Token has expired
    #[error("Token has expired")]
    TokenExpired,

    /// Token signature is invalid
    #[error("Invalid token signature")]
    InvalidSignature,

    /// Token validation failed
    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    /// Insufficient permissions
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
}

impl AuthError {
    /// Create a validation failed error
    pub fn validation_failed(msg: impl Into<String>) -> Self {
        Self::ValidationFailed(msg.into())
    }

    /// Create an invalid format error
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Create an insufficient permissions error
    pub fn insufficient_permissions(msg: impl Into<String>) -> Self {
        Self::InsufficientPermissions(msg.into())
    }
}

/// Claims extracted from a validated token
///
/// Contains the user identity and any additional claims from the token.
#[derive(Debug, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Additional claims as key-value pairs
    pub extra: HashMap<String, String>,
}

impl Claims {
    /// Create new claims with just a subject
    pub fn new(sub: impl Into<String>) -> Self {
        Self {
            sub: sub.into(),
            extra: HashMap::new(),
        }
    }

    /// Create claims with subject and extra data
    pub fn with_extra(sub: impl Into<String>, extra: HashMap<String, String>) -> Self {
        Self {
            sub: sub.into(),
            extra,
        }
    }

    /// Get the subject (user ID)
    pub fn subject(&self) -> &str {
        &self.sub
    }

    /// Get an extra claim by key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.extra.get(key).map(|s| s.as_str())
    }

    /// Add an extra claim
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.extra.insert(key.into(), value.into());
    }
}

/// Specifies where to extract the authentication token from
#[derive(Debug, Clone)]
pub enum TokenExtractor {
    /// Extract from a header (e.g., "Authorization")
    Header(String),
    /// Extract from a query parameter (e.g., "token")
    Query(String),
    /// Extract from the Sec-WebSocket-Protocol header
    Protocol,
}

impl Default for TokenExtractor {
    fn default() -> Self {
        Self::Header("Authorization".to_string())
    }
}

impl TokenExtractor {
    /// Create a header extractor
    pub fn header(name: impl Into<String>) -> Self {
        Self::Header(name.into())
    }

    /// Create a query parameter extractor
    pub fn query(name: impl Into<String>) -> Self {
        Self::Query(name.into())
    }

    /// Create a protocol extractor
    pub fn protocol() -> Self {
        Self::Protocol
    }

    /// Extract the token from an HTTP request
    pub fn extract<B>(&self, req: &http::Request<B>) -> Option<String> {
        match self {
            TokenExtractor::Header(name) => {
                req.headers()
                    .get(name)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| {
                        // Strip "Bearer " prefix if present
                        if let Some(token) = s.strip_prefix("Bearer ") {
                            token.to_string()
                        } else {
                            s.to_string()
                        }
                    })
            }
            TokenExtractor::Query(name) => req.uri().query().and_then(|query| {
                url::form_urlencoded::parse(query.as_bytes())
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.into_owned())
            }),
            TokenExtractor::Protocol => req
                .headers()
                .get("Sec-WebSocket-Protocol")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        }
    }
}

/// Trait for validating authentication tokens
///
/// Implement this trait to provide custom token validation logic.
#[async_trait::async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validate a token and return the claims if valid
    async fn validate(&self, token: &str) -> Result<Claims, AuthError>;
}

/// Configuration for WebSocket authentication
#[derive(Clone)]
pub struct WsAuthConfig {
    /// Token extractor configuration
    pub extractor: TokenExtractor,
    /// Token validator
    pub validator: Arc<dyn TokenValidator>,
    /// Whether authentication is required (if false, missing tokens are allowed)
    pub required: bool,
}

impl WsAuthConfig {
    /// Create a new authentication configuration with a validator
    pub fn new<V: TokenValidator + 'static>(validator: V) -> Self {
        Self {
            extractor: TokenExtractor::default(),
            validator: Arc::new(validator),
            required: true,
        }
    }

    /// Set the token extractor
    pub fn extractor(mut self, extractor: TokenExtractor) -> Self {
        self.extractor = extractor;
        self
    }

    /// Set whether authentication is required
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Extract and validate a token from a request
    pub async fn authenticate<B>(
        &self,
        req: &http::Request<B>,
    ) -> Result<Option<Claims>, AuthError> {
        match self.extractor.extract(req) {
            Some(token) => {
                let claims = self.validator.validate(&token).await?;
                Ok(Some(claims))
            }
            None if self.required => Err(AuthError::TokenMissing),
            None => Ok(None),
        }
    }
}

impl std::fmt::Debug for WsAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsAuthConfig")
            .field("extractor", &self.extractor)
            .field("required", &self.required)
            .finish()
    }
}

/// A simple token validator that accepts any non-empty token
///
/// This is useful for testing or when token validation is handled elsewhere.
pub struct AcceptAllValidator;

#[async_trait::async_trait]
impl TokenValidator for AcceptAllValidator {
    async fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        if token.is_empty() {
            return Err(AuthError::invalid_format("Token cannot be empty"));
        }
        Ok(Claims::new(token))
    }
}

/// A token validator that rejects all tokens
///
/// This is useful for testing authentication failure scenarios.
pub struct RejectAllValidator;

#[async_trait::async_trait]
impl TokenValidator for RejectAllValidator {
    async fn validate(&self, _token: &str) -> Result<Claims, AuthError> {
        Err(AuthError::validation_failed("All tokens rejected"))
    }
}

/// A token validator that validates against a static list of valid tokens
pub struct StaticTokenValidator {
    tokens: HashMap<String, Claims>,
}

impl StaticTokenValidator {
    /// Create a new static token validator
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Add a valid token with associated claims
    pub fn add_token(mut self, token: impl Into<String>, claims: Claims) -> Self {
        self.tokens.insert(token.into(), claims);
        self
    }
}

impl Default for StaticTokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TokenValidator for StaticTokenValidator {
    async fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        self.tokens
            .get(token)
            .cloned()
            .ok_or_else(|| AuthError::validation_failed("Invalid token"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Request;

    #[test]
    fn test_token_extractor_header() {
        let extractor = TokenExtractor::header("Authorization");

        let req = Request::builder()
            .header("Authorization", "Bearer test-token")
            .body(())
            .unwrap();

        assert_eq!(extractor.extract(&req), Some("test-token".to_string()));
    }

    #[test]
    fn test_token_extractor_header_no_bearer() {
        let extractor = TokenExtractor::header("X-API-Key");

        let req = Request::builder()
            .header("X-API-Key", "my-api-key")
            .body(())
            .unwrap();

        assert_eq!(extractor.extract(&req), Some("my-api-key".to_string()));
    }

    #[test]
    fn test_token_extractor_query() {
        let extractor = TokenExtractor::query("token");

        let req = Request::builder()
            .uri("ws://localhost/ws?token=query-token&other=value")
            .body(())
            .unwrap();

        assert_eq!(extractor.extract(&req), Some("query-token".to_string()));
    }

    #[test]
    fn test_token_extractor_protocol() {
        let extractor = TokenExtractor::protocol();

        let req = Request::builder()
            .header("Sec-WebSocket-Protocol", "my-protocol-token")
            .body(())
            .unwrap();

        assert_eq!(
            extractor.extract(&req),
            Some("my-protocol-token".to_string())
        );
    }

    #[test]
    fn test_token_extractor_missing() {
        let extractor = TokenExtractor::header("Authorization");

        let req = Request::builder().body(()).unwrap();

        assert_eq!(extractor.extract(&req), None);
    }

    #[tokio::test]
    async fn test_accept_all_validator() {
        let validator = AcceptAllValidator;

        let result = validator.validate("any-token").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().subject(), "any-token");
    }

    #[tokio::test]
    async fn test_accept_all_validator_empty() {
        let validator = AcceptAllValidator;

        let result = validator.validate("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reject_all_validator() {
        let validator = RejectAllValidator;

        let result = validator.validate("any-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_static_token_validator() {
        let validator =
            StaticTokenValidator::new().add_token("valid-token", Claims::new("user-123"));

        let result = validator.validate("valid-token").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().subject(), "user-123");

        let result = validator.validate("invalid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ws_auth_config_required() {
        let config = WsAuthConfig::new(AcceptAllValidator)
            .extractor(TokenExtractor::header("Authorization"))
            .required(true);

        let req = Request::builder().body(()).unwrap();

        let result = config.authenticate(&req).await;
        assert!(matches!(result, Err(AuthError::TokenMissing)));
    }

    #[tokio::test]
    async fn test_ws_auth_config_optional() {
        let config = WsAuthConfig::new(AcceptAllValidator)
            .extractor(TokenExtractor::header("Authorization"))
            .required(false);

        let req = Request::builder().body(()).unwrap();

        let result = config.authenticate(&req).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ws_auth_config_with_token() {
        let config = WsAuthConfig::new(AcceptAllValidator)
            .extractor(TokenExtractor::header("Authorization"));

        let req = Request::builder()
            .header("Authorization", "Bearer my-token")
            .body(())
            .unwrap();

        let result = config.authenticate(&req).await;
        assert!(result.is_ok());
        let claims = result.unwrap().unwrap();
        assert_eq!(claims.subject(), "my-token");
    }

    #[test]
    fn test_claims_extra() {
        let mut claims = Claims::new("user-123");
        claims.insert("role", "admin");
        claims.insert("tenant", "acme");

        assert_eq!(claims.subject(), "user-123");
        assert_eq!(claims.get("role"), Some("admin"));
        assert_eq!(claims.get("tenant"), Some("acme"));
        assert_eq!(claims.get("missing"), None);
    }

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::TokenMissing;
        assert_eq!(err.to_string(), "Authentication token missing");

        let err = AuthError::validation_failed("custom error");
        assert_eq!(err.to_string(), "Token validation failed: custom error");
    }

    #[test]
    fn test_token_extractor_default() {
        let extractor = TokenExtractor::default();
        match extractor {
            TokenExtractor::Header(name) => assert_eq!(name, "Authorization"),
            _ => panic!("Expected Header extractor"),
        }
    }
}

/// Property-based tests for WebSocket authentication
///
/// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
/// **Validates: Requirements 4.1, 4.3**
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating random tokens
    fn token_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9._-]{1,100}").unwrap()
    }

    /// Strategy for generating random header names
    fn header_name_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[A-Za-z][A-Za-z0-9-]{0,30}").unwrap()
    }

    /// Strategy for generating random query parameter names
    fn query_param_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9_]{0,20}").unwrap()
    }

    /// Strategy for generating token extractors
    fn extractor_strategy() -> impl Strategy<Value = TokenExtractor> {
        prop_oneof![
            header_name_strategy().prop_map(TokenExtractor::Header),
            query_param_strategy().prop_map(TokenExtractor::Query),
            Just(TokenExtractor::Protocol),
        ]
    }

    proptest! {
        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any WebSocket connection attempt with required authentication,
        /// if no token is provided, authentication SHALL fail with TokenMissing error.
        #[test]
        fn prop_auth_required_rejects_missing_token(
            extractor in extractor_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = WsAuthConfig::new(AcceptAllValidator)
                    .extractor(extractor)
                    .required(true);

                // Request without any token
                let req = http::Request::builder()
                    .uri("ws://localhost/ws")
                    .body(())
                    .unwrap();

                let result = config.authenticate(&req).await;
                prop_assert!(matches!(result, Err(AuthError::TokenMissing)));
                Ok(())
            })?;
        }

        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any WebSocket connection attempt with a valid token,
        /// authentication SHALL succeed and return claims.
        #[test]
        fn prop_auth_accepts_valid_token_in_header(
            token in token_strategy(),
            header_name in header_name_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = WsAuthConfig::new(AcceptAllValidator)
                    .extractor(TokenExtractor::Header(header_name.clone()))
                    .required(true);

                let req = http::Request::builder()
                    .uri("ws://localhost/ws")
                    .header(&header_name, format!("Bearer {}", token))
                    .body(())
                    .unwrap();

                let result = config.authenticate(&req).await;
                prop_assert!(result.is_ok());
                let claims = result.unwrap();
                prop_assert!(claims.is_some());
                let claims = claims.unwrap();
                prop_assert_eq!(claims.subject(), &token);
                Ok(())
            })?;
        }

        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any WebSocket connection attempt with a valid token in query,
        /// authentication SHALL succeed and return claims.
        #[test]
        fn prop_auth_accepts_valid_token_in_query(
            token in token_strategy(),
            param_name in query_param_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = WsAuthConfig::new(AcceptAllValidator)
                    .extractor(TokenExtractor::Query(param_name.clone()))
                    .required(true);

                let uri = format!("ws://localhost/ws?{}={}", param_name, token);
                let req = http::Request::builder()
                    .uri(&uri)
                    .body(())
                    .unwrap();

                let result = config.authenticate(&req).await;
                prop_assert!(result.is_ok());
                let claims = result.unwrap();
                prop_assert!(claims.is_some());
                let claims = claims.unwrap();
                prop_assert_eq!(claims.subject(), &token);
                Ok(())
            })?;
        }

        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any WebSocket connection attempt with an invalid token,
        /// authentication SHALL fail with validation error.
        #[test]
        fn prop_auth_rejects_invalid_token(
            token in token_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = WsAuthConfig::new(RejectAllValidator)
                    .extractor(TokenExtractor::Header("Authorization".to_string()))
                    .required(true);

                let req = http::Request::builder()
                    .uri("ws://localhost/ws")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(())
                    .unwrap();

                let result = config.authenticate(&req).await;
                prop_assert!(result.is_err());
                prop_assert!(matches!(result, Err(AuthError::ValidationFailed(_))));
                Ok(())
            })?;
        }

        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any WebSocket connection with optional auth and no token,
        /// authentication SHALL succeed with None claims.
        #[test]
        fn prop_optional_auth_allows_missing_token(
            extractor in extractor_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = WsAuthConfig::new(AcceptAllValidator)
                    .extractor(extractor)
                    .required(false);

                let req = http::Request::builder()
                    .uri("ws://localhost/ws")
                    .body(())
                    .unwrap();

                let result = config.authenticate(&req).await;
                prop_assert!(result.is_ok());
                prop_assert!(result.unwrap().is_none());
                Ok(())
            })?;
        }

        /// **Feature: v1-features-roadmap, Property 10: WebSocket authentication enforcement**
        /// **Validates: Requirements 4.1, 4.3**
        ///
        /// For any static token validator with known valid tokens,
        /// only those exact tokens SHALL be accepted.
        #[test]
        fn prop_static_validator_only_accepts_known_tokens(
            valid_token in token_strategy(),
            test_token in token_strategy(),
            user_id in "[a-z]{3,10}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let validator = StaticTokenValidator::new()
                    .add_token(valid_token.clone(), Claims::new(user_id.clone()));

                let result = validator.validate(&test_token).await;

                if test_token == valid_token {
                    prop_assert!(result.is_ok());
                    let claims = result.unwrap();
                    prop_assert_eq!(claims.subject(), &user_id);
                } else {
                    prop_assert!(result.is_err());
                }
                Ok(())
            })?;
        }
    }
}
