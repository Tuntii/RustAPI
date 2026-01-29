//! JWT authentication middleware and extractors.
//!
//! This module provides JWT token validation middleware and the `AuthUser<T>`
//! extractor for accessing decoded claims in handlers.
//!
//! # Example
//!
//! ```ignore
//! use rustapi_extras::jwt::{JwtLayer, AuthUser};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Claims {
//!     sub: String,
//!     exp: u64,
//! }
//!
//! async fn protected(AuthUser(claims): AuthUser<Claims>) -> String {
//!     format!("Hello, {}", claims.sub)
//! }
//! ```

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use jsonwebtoken::{decode, DecodingKey, Validation};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{ApiError, FromRequestParts, Request, Response, ResponseBody, Result};
use rustapi_openapi::{Operation, OperationModifier};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// JWT validation configuration.
#[derive(Debug, Clone)]
pub struct JwtValidation {
    /// Leeway in seconds for expiration validation.
    pub leeway: u64,
    /// Whether to validate the expiration claim.
    pub validate_exp: bool,
    /// Allowed algorithms for token validation.
    pub algorithms: Vec<jsonwebtoken::Algorithm>,
}

impl Default for JwtValidation {
    fn default() -> Self {
        Self {
            leeway: 0,
            validate_exp: true,
            algorithms: vec![jsonwebtoken::Algorithm::HS256],
        }
    }
}

impl JwtValidation {
    /// Convert to jsonwebtoken's Validation struct
    fn to_jsonwebtoken_validation(&self) -> Validation {
        let mut validation = Validation::new(
            self.algorithms
                .first()
                .copied()
                .unwrap_or(jsonwebtoken::Algorithm::HS256),
        );
        validation.leeway = self.leeway;
        validation.validate_exp = self.validate_exp;
        validation.set_required_spec_claims::<&str>(&[]);
        validation
    }
}

/// JWT authentication middleware layer.
///
/// Validates incoming `Authorization: Bearer <token>` headers and makes
/// decoded claims available via the `AuthUser<T>` extractor.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::jwt::{JwtLayer, AuthUser};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Claims {
///     sub: String,
///     exp: u64,
/// }
///
/// let app = RustApi::new()
///     .layer(JwtLayer::<Claims>::new("my-secret-key")
///         .skip_paths(vec!["/health", "/docs", "/auth/login"]))
///     .route("/protected", get(protected_handler));
/// ```
#[derive(Clone)]
pub struct JwtLayer<T> {
    secret: Arc<String>,
    validation: JwtValidation,
    skip_paths: Arc<Vec<String>>,
    _claims: PhantomData<T>,
}

impl<T: DeserializeOwned + Clone + Send + Sync + 'static> JwtLayer<T> {
    /// Create a new JWT layer with the given secret.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: Arc::new(secret.into()),
            validation: JwtValidation::default(),
            skip_paths: Arc::new(Vec::new()),
            _claims: PhantomData,
        }
    }

    /// Configure custom validation options.
    pub fn with_validation(mut self, validation: JwtValidation) -> Self {
        self.validation = validation;
        self
    }

    /// Skip JWT validation for specific paths.
    ///
    /// Paths that start with any of the provided prefixes will bypass JWT validation.
    /// This is useful for public endpoints like health checks, documentation, and login.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let layer = JwtLayer::<Claims>::new("secret")
    ///     .skip_paths(vec!["/health", "/docs", "/auth/login"]);
    /// ```
    pub fn skip_paths(mut self, paths: Vec<&str>) -> Self {
        self.skip_paths = Arc::new(paths.into_iter().map(String::from).collect());
        self
    }

    /// Get the configured secret.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Get the validation configuration.
    pub fn validation(&self) -> &JwtValidation {
        &self.validation
    }

    /// Validate a JWT token and return the decoded claims.
    pub fn validate_token(&self, token: &str) -> std::result::Result<T, JwtError> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = self.validation.to_jsonwebtoken_validation();

        match decode::<T>(token, &decoding_key, &validation) {
            Ok(token_data) => Ok(token_data.claims),
            Err(err) => Err(JwtError::from(err)),
        }
    }
}

impl<T: DeserializeOwned + Clone + Send + Sync + 'static> MiddlewareLayer for JwtLayer<T> {
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let secret = self.secret.clone();
        let validation = self.validation.clone();
        let skip_paths = self.skip_paths.clone();

        Box::pin(async move {
            // Check if this path should skip JWT validation
            let path = req.uri().path();
            if skip_paths.iter().any(|skip| path.starts_with(skip)) {
                return next(req).await;
            }

            // Extract the Authorization header
            let auth_header = req.headers().get(http::header::AUTHORIZATION);

            let token = match auth_header {
                Some(header_value) => {
                    match header_value.to_str() {
                        Ok(header_str) => {
                            // Check for "Bearer " prefix
                            if let Some(token) = header_str.strip_prefix("Bearer ") {
                                token.to_string()
                            } else if let Some(token) = header_str.strip_prefix("bearer ") {
                                token.to_string()
                            } else {
                                return create_unauthorized_response(
                                    "Invalid Authorization header format",
                                );
                            }
                        }
                        Err(_) => {
                            return create_unauthorized_response(
                                "Invalid Authorization header encoding",
                            );
                        }
                    }
                }
                None => {
                    return create_unauthorized_response("Missing Authorization header");
                }
            };

            // Validate the token
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            let jwt_validation = validation.to_jsonwebtoken_validation();

            match decode::<T>(&token, &decoding_key, &jwt_validation) {
                Ok(token_data) => {
                    // Store the validated claims in request extensions
                    req.extensions_mut()
                        .insert(ValidatedClaims(token_data.claims));

                    // Continue to the next handler
                    next(req).await
                }
                Err(err) => {
                    let message = match err.kind() {
                        jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token has expired",
                        jsonwebtoken::errors::ErrorKind::InvalidToken => "Invalid token",
                        jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                            "Invalid token signature"
                        }
                        jsonwebtoken::errors::ErrorKind::InvalidAlgorithm => {
                            "Invalid token algorithm"
                        }
                        _ => "Invalid or expired token",
                    };
                    create_unauthorized_response(message)
                }
            }
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Internal wrapper for validated claims stored in request extensions
#[derive(Clone)]
pub struct ValidatedClaims<T>(pub T);

/// Create a 401 Unauthorized JSON response
fn create_unauthorized_response(message: &str) -> Response {
    let error_body = serde_json::json!({
        "error": {
            "type": "unauthorized",
            "message": message
        }
    });

    let body = serde_json::to_vec(&error_body).unwrap_or_default();

    http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(ResponseBody::Full(Full::new(Bytes::from(body))))
        .unwrap()
}

/// JWT-related errors
#[derive(Debug, Clone)]
pub enum JwtError {
    /// Token has expired
    Expired,
    /// Token is invalid (malformed, bad signature, etc.)
    Invalid(String),
    /// Token is missing
    Missing,
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::Expired => write!(f, "Token has expired"),
            JwtError::Invalid(msg) => write!(f, "Invalid token: {}", msg),
            JwtError::Missing => write!(f, "Missing token"),
        }
    }
}

impl std::error::Error for JwtError {}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
            _ => JwtError::Invalid(err.to_string()),
        }
    }
}

/// Extractor for authenticated user claims from a validated JWT token.
///
/// This extractor retrieves the decoded claims from a JWT token that was
/// validated by the `JwtLayer` middleware.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::jwt::AuthUser;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone)]
/// struct Claims {
///     sub: String,
///     exp: u64,
/// }
///
/// async fn protected(AuthUser(claims): AuthUser<Claims>) -> String {
///     format!("Hello, {}", claims.sub)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser<T>(pub T);

impl<T: Clone + Send + Sync + 'static> FromRequestParts for AuthUser<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.extensions()
            .get::<ValidatedClaims<T>>()
            .map(|claims| AuthUser(claims.0.clone()))
            .ok_or_else(|| {
                ApiError::unauthorized(
                    "No authenticated user. Did you forget to add JwtLayer middleware?",
                )
            })
    }
}

// Implement OperationModifier for AuthUser to enable use in handlers
impl<T> OperationModifier for AuthUser<T> {
    fn update_operation(op: &mut Operation) {
        // Add 401 Unauthorized response to OpenAPI spec
        use rustapi_openapi::schema::Reference;
        use rustapi_openapi::{MediaType, ResponseSpec, SchemaRef};
        use std::collections::HashMap;

        op.responses.insert(
            "401".to_string(),
            ResponseSpec {
                description: "Unauthorized - Invalid or missing JWT token".to_string(),
                content: {
                    let mut map = HashMap::new();
                    map.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: SchemaRef::Ref(Reference {
                                ref_path: "#/components/schemas/ErrorSchema".to_string(),
                            }),
                        },
                    );
                    Some(map)
                },
            },
        );
    }
}

/// Helper function to create a JWT token (useful for testing)
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::jwt::create_token;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Claims {
///     sub: String,
///     exp: u64,
/// }
///
/// let claims = Claims {
///     sub: "user123".to_string(),
///     exp: 9999999999,
/// };
///
/// let token = create_token(&claims, "my-secret").unwrap();
/// ```
pub fn create_token<T: Serialize>(
    claims: &T,
    secret: &str,
) -> std::result::Result<String, JwtError> {
    let encoding_key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
    let header = jsonwebtoken::Header::default();

    jsonwebtoken::encode(&header, claims, &encoding_key)
        .map_err(|e| JwtError::Invalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Method, StatusCode};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use rustapi_core::middleware::LayerStack;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Test claims structure
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestClaims {
        sub: String,
        exp: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_field: Option<String>,
    }

    /// Create a test request with optional Authorization header
    fn create_test_request(auth_header: Option<&str>) -> Request {
        let uri: http::Uri = "/test".parse().unwrap();
        let mut builder = http::Request::builder().method(Method::GET).uri(uri);

        if let Some(auth) = auth_header {
            builder = builder.header(http::header::AUTHORIZATION, auth);
        }

        let req = builder.body(()).unwrap();
        Request::from_http_request(req, Bytes::new())
    }

    /// Get current timestamp plus offset in seconds
    fn future_timestamp(offset_secs: u64) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + offset_secs
    }

    /// Get a past timestamp
    fn past_timestamp(offset_secs: u64) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(offset_secs)
    }

    /// Strategy for generating valid subject strings
    fn subject_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s)
    }

    /// Strategy for generating valid secret keys
    fn secret_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9!@#$%^&*]{16,64}".prop_map(|s| s)
    }

    /// Strategy for generating optional custom fields
    fn custom_field_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![Just(None), "[a-zA-Z0-9 ]{1,100}".prop_map(Some),]
    }

    /// Helper to setup stack with JWT layer
    fn setup_stack<T: DeserializeOwned + Clone + Send + Sync + 'static>(
        secret: &str,
    ) -> LayerStack {
        let mut stack = LayerStack::new();
        stack.push(Box::new(JwtLayer::<T>::new(secret)));
        stack
    }

    /// Helper to create a dummy handler
    fn dummy_handler() -> rustapi_core::middleware::BoxedNext {
        Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(StatusCode::OK)
                    .body(ResponseBody::Full(Full::new(Bytes::from("success"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        })
    }

    // **Feature: phase3-batteries-included, Property 5: JWT validation correctness**
    //
    // For any JWT token signed with secret S, when JwtLayer is configured with secret S,
    // the token SHALL be accepted; when configured with a different secret S', the token
    // SHALL be rejected with 401.
    //
    // **Validates: Requirements 2.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_jwt_validation_correctness(
            subject in subject_strategy(),
            correct_secret in secret_strategy(),
            wrong_secret in secret_strategy(),
            custom_field in custom_field_strategy(),
        ) {
            // Ensure secrets are different
            prop_assume!(correct_secret != wrong_secret);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                // Create claims with future expiration
                let claims = TestClaims {
                    sub: subject.clone(),
                    exp: future_timestamp(3600), // 1 hour from now
                    custom_field,
                };

                // Create a valid token with the correct secret
                let token = create_token(&claims, &correct_secret)
                    .expect("Failed to create token");

                // Test 1: Token should be accepted with correct secret
                {
                    let stack = setup_stack::<TestClaims>(&correct_secret);
                    let handler = dummy_handler();

                    let request = create_test_request(Some(&format!("Bearer {}", token)));
                    let response = stack.execute(request, handler).await;

                    prop_assert_eq!(
                        response.status(),
                        StatusCode::OK,
                        "Token signed with correct secret should be accepted"
                    );
                }

                // Test 2: Token should be rejected with wrong secret
                {
                    let stack = setup_stack::<TestClaims>(&wrong_secret);
                    let handler = dummy_handler();

                    let request = create_test_request(Some(&format!("Bearer {}", token)));
                    let response = stack.execute(request, handler).await;

                    prop_assert_eq!(
                        response.status(),
                        StatusCode::UNAUTHORIZED,
                        "Token signed with wrong secret should be rejected with 401"
                    );
                }

                Ok(())
            });
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 6: JWT claims round-trip**
    //
    // For any valid JWT token containing claims C of type T, the `AuthUser<T>` extractor
    // SHALL return claims equal to C.
    //
    // **Validates: Requirements 2.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_jwt_claims_round_trip(
            subject in subject_strategy(),
            secret in secret_strategy(),
            custom_field in custom_field_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                // Create claims with future expiration
                let original_claims = TestClaims {
                    sub: subject.clone(),
                    exp: future_timestamp(3600), // 1 hour from now
                    custom_field: custom_field.clone(),
                };

                // Create a valid token
                let token = create_token(&original_claims, &secret)
                    .expect("Failed to create token");

                // Set up middleware stack
                let stack = setup_stack::<TestClaims>(&secret);

                // Track extracted claims
                let extracted_claims = Arc::new(std::sync::Mutex::new(None::<TestClaims>));
                let extracted_claims_clone = extracted_claims.clone();

                let handler: rustapi_core::middleware::BoxedNext = Arc::new(move |req: Request| {
                    let extracted = extracted_claims_clone.clone();
                    Box::pin(async move {
                        // Extract claims using AuthUser
                        if let Ok(AuthUser(claims)) = AuthUser::<TestClaims>::from_request_parts(&req) {
                            *extracted.lock().unwrap() = Some(claims);
                        }

                        http::Response::builder()
                            .status(StatusCode::OK)
                            .body(ResponseBody::Full(Full::new(Bytes::from("success"))))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                let request = create_test_request(Some(&format!("Bearer {}", token)));
                let response = stack.execute(request, handler).await;

                prop_assert_eq!(response.status(), StatusCode::OK);

                // Verify extracted claims match original
                let extracted = extracted_claims.lock().unwrap();
                prop_assert!(extracted.is_some(), "Claims should have been extracted");

                let extracted = extracted.as_ref().unwrap();
                prop_assert_eq!(
                    &extracted.sub, &original_claims.sub,
                    "Subject should match"
                );
                prop_assert_eq!(
                    extracted.exp, original_claims.exp,
                    "Expiration should match"
                );
                prop_assert_eq!(
                    &extracted.custom_field, &original_claims.custom_field,
                    "Custom field should match"
                );

                Ok(())
            });
            result?;
        }
    }

    // **Feature: phase3-batteries-included, Property 7: Invalid JWT rejection**
    //
    // For any malformed, tampered, or expired JWT token, the System SHALL return a 401
    // Unauthorized response with a JSON error body containing type "unauthorized".
    //
    // **Validates: Requirements 2.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_invalid_jwt_rejection(
            subject in subject_strategy(),
            secret in secret_strategy(),
            invalid_token_type in 0u8..5u8,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: std::result::Result<(), TestCaseError> = rt.block_on(async {
                let stack = setup_stack::<TestClaims>(&secret);

                // Generate different types of invalid tokens
                let invalid_token = match invalid_token_type {
                    0 => {
                        // Expired token
                        let claims = TestClaims {
                            sub: subject.clone(),
                            exp: past_timestamp(3600), // 1 hour ago
                            custom_field: None,
                        };
                        create_token(&claims, &secret).expect("Failed to create token")
                    }
                    1 => {
                        // Malformed token (random string)
                        "not.a.valid.jwt.token".to_string()
                    }
                    2 => {
                        // Tampered token (valid structure but modified)
                        let claims = TestClaims {
                            sub: subject.clone(),
                            exp: future_timestamp(3600),
                            custom_field: None,
                        };
                        let mut token = create_token(&claims, &secret).expect("Failed to create token");
                        // Tamper with the signature by changing last character
                        let len = token.len();
                        if len > 0 {
                            let last_char = token.chars().last().unwrap();
                            let new_char = if last_char == 'a' { 'b' } else { 'a' };
                            token.pop();
                            token.push(new_char);
                        }
                        token
                    }
                    3 => {
                        // Empty token
                        "".to_string()
                    }
                    _ => {
                        // Token with wrong number of parts
                        "header.payload".to_string()
                    }
                };

                let handler = dummy_handler();

                let request = create_test_request(Some(&format!("Bearer {}", invalid_token)));
                let response = stack.execute(request, handler).await;

                // Should return 401 Unauthorized
                prop_assert_eq!(
                    response.status(),
                    StatusCode::UNAUTHORIZED,
                    "Invalid token should be rejected with 401"
                );

                // Verify response body contains error type "unauthorized"
                let body_bytes = {
                    use http_body_util::BodyExt;
                    let body = response.into_body();
                    body.collect().await.unwrap().to_bytes()
                };
                let body_str = String::from_utf8_lossy(&body_bytes);

                prop_assert!(
                    body_str.contains("\"type\":\"unauthorized\"") || body_str.contains("\"type\": \"unauthorized\""),
                    "Response body should contain error type 'unauthorized', got: {}",
                    body_str
                );

                Ok(())
            });
            result?;
        }
    }

    // Additional unit tests for edge cases

    #[tokio::test]
    async fn test_missing_authorization_header() {
        let stack = setup_stack::<TestClaims>("secret");
        let handler = dummy_handler();

        let request = create_test_request(None);
        let response = stack.execute(request, handler).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_authorization_format() {
        let stack = setup_stack::<TestClaims>("secret");
        let handler = dummy_handler();

        // Test with "Basic" auth instead of "Bearer"
        let request = create_test_request(Some("Basic dXNlcjpwYXNz"));
        let response = stack.execute(request, handler).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_auth_user_extractor_without_middleware() {
        let request = create_test_request(None);
        let result = AuthUser::<TestClaims>::from_request_parts(&request);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_create_token_helper() {
        let claims = TestClaims {
            sub: "user123".to_string(),
            exp: future_timestamp(3600),
            custom_field: Some("test".to_string()),
        };

        let token = create_token(&claims, "my-secret").unwrap();

        // Token should have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_jwt_layer_validate_token() {
        let secret = "test-secret-key";
        let layer = JwtLayer::<TestClaims>::new(secret);

        let claims = TestClaims {
            sub: "user123".to_string(),
            exp: future_timestamp(3600),
            custom_field: None,
        };

        let token = create_token(&claims, secret).unwrap();
        let result = layer.validate_token(&token);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.exp, claims.exp);
    }

    #[test]
    fn test_jwt_layer_validate_token_wrong_secret() {
        let layer = JwtLayer::<TestClaims>::new("correct-secret");

        let claims = TestClaims {
            sub: "user123".to_string(),
            exp: future_timestamp(3600),
            custom_field: None,
        };

        let token = create_token(&claims, "wrong-secret").unwrap();
        let result = layer.validate_token(&token);

        assert!(result.is_err());
    }
}
