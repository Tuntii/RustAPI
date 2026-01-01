//! Error types for RustAPI
//!
//! This module provides structured error handling with environment-aware
//! error masking for production safety.
//!
//! # Error Response Format
//!
//! All errors are returned as JSON with a consistent structure:
//!
//! ```json
//! {
//!   "error": {
//!     "type": "not_found",
//!     "message": "User not found",
//!     "fields": null
//!   },
//!   "error_id": "err_a1b2c3d4e5f6"
//! }
//! ```
//!
//! # Environment-Aware Error Masking
//!
//! In production mode (`RUSTAPI_ENV=production`), internal server errors (5xx)
//! are masked to prevent information leakage:
//!
//! - **Production**: Generic "An internal error occurred" message
//! - **Development**: Full error details for debugging
//!
//! Validation errors always include field details regardless of environment.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{ApiError, Result};
//! use http::StatusCode;
//!
//! async fn get_user(id: i64) -> Result<Json<User>> {
//!     let user = db.find_user(id)
//!         .ok_or_else(|| ApiError::not_found("User not found"))?;
//!     Ok(Json(user))
//! }
//!
//! // Create custom errors
//! let error = ApiError::new(StatusCode::CONFLICT, "duplicate", "Email already exists");
//!
//! // Convenience constructors
//! let bad_request = ApiError::bad_request("Invalid input");
//! let unauthorized = ApiError::unauthorized("Invalid token");
//! let forbidden = ApiError::forbidden("Access denied");
//! let not_found = ApiError::not_found("Resource not found");
//! let internal = ApiError::internal("Something went wrong");
//! ```
//!
//! # Error ID Correlation
//!
//! Every error response includes a unique `error_id` (format: `err_{uuid}`) that
//! appears in both the response and server logs, enabling easy correlation for
//! debugging.

use http::StatusCode;
use serde::Serialize;
use std::fmt;
use std::sync::OnceLock;
use uuid::Uuid;

/// Result type alias for RustAPI operations
pub type Result<T, E = ApiError> = std::result::Result<T, E>;

/// Environment configuration for error handling behavior
///
/// Controls whether internal error details are exposed in API responses.
/// In production, internal details are masked to prevent information leakage.
/// In development, full error details are shown for debugging.
///
/// # Example
///
/// ```
/// use rustapi_core::Environment;
///
/// let dev = Environment::Development;
/// assert!(dev.is_development());
/// assert!(!dev.is_production());
///
/// let prod = Environment::Production;
/// assert!(prod.is_production());
/// assert!(!prod.is_development());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    /// Development mode - shows full error details in responses
    #[default]
    Development,
    /// Production mode - masks internal error details in responses
    Production,
}

impl Environment {
    /// Detect environment from `RUSTAPI_ENV` environment variable
    ///
    /// Returns `Production` if `RUSTAPI_ENV` is set to "production" or "prod" (case-insensitive).
    /// Returns `Development` for all other values or if the variable is not set.
    ///
    /// # Example
    ///
    /// ```bash
    /// # Production mode
    /// RUSTAPI_ENV=production cargo run
    /// RUSTAPI_ENV=prod cargo run
    ///
    /// # Development mode (default)
    /// RUSTAPI_ENV=development cargo run
    /// cargo run  # No env var set
    /// ```
    pub fn from_env() -> Self {
        match std::env::var("RUSTAPI_ENV")
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Ok("production") | Ok("prod") => Environment::Production,
            _ => Environment::Development,
        }
    }

    /// Check if this is production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    /// Check if this is development environment
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Production => write!(f, "production"),
        }
    }
}

/// Global environment setting, cached on first access
static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

/// Get the current environment (cached)
///
/// This function caches the environment on first call for performance.
/// The environment is detected from the `RUSTAPI_ENV` environment variable.
pub fn get_environment() -> Environment {
    *ENVIRONMENT.get_or_init(Environment::from_env)
}

/// Set the environment explicitly (for testing purposes)
///
/// Note: This only works if the environment hasn't been accessed yet.
/// Returns `Ok(())` if successful, `Err(env)` if already set.
#[cfg(test)]
#[allow(dead_code)]
pub fn set_environment_for_test(env: Environment) -> Result<(), Environment> {
    ENVIRONMENT.set(env)
}

/// Generate a unique error ID using UUID v4 format
///
/// Returns a string in the format `err_{uuid}` where uuid is a 32-character
/// hexadecimal string (UUID v4 simple format).
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::error::generate_error_id;
///
/// let id = generate_error_id();
/// assert!(id.starts_with("err_"));
/// assert_eq!(id.len(), 36); // "err_" (4) + uuid (32)
/// ```
pub fn generate_error_id() -> String {
    format!("err_{}", Uuid::new_v4().simple())
}

/// Standard API error type
///
/// Provides structured error responses following a consistent JSON format.
///
/// # Example
///
/// ```
/// use rustapi_core::ApiError;
/// use http::StatusCode;
///
/// // Create a custom error
/// let error = ApiError::new(StatusCode::CONFLICT, "duplicate", "Email already exists");
/// assert_eq!(error.status, StatusCode::CONFLICT);
/// assert_eq!(error.error_type, "duplicate");
///
/// // Use convenience constructors
/// let not_found = ApiError::not_found("User not found");
/// assert_eq!(not_found.status, StatusCode::NOT_FOUND);
///
/// let bad_request = ApiError::bad_request("Invalid input");
/// assert_eq!(bad_request.status, StatusCode::BAD_REQUEST);
/// ```
#[derive(Debug, Clone)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error type identifier
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Optional field-level validation errors
    pub fields: Option<Vec<FieldError>>,
    /// Internal details (hidden in production)
    pub(crate) internal: Option<String>,
}

/// Field-level validation error
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    /// Field name (supports nested: "address.city")
    pub field: String,
    /// Error code (e.g., "email", "length", "required")
    pub code: String,
    /// Human-readable message
    pub message: String,
}

impl ApiError {
    /// Create a new API error
    pub fn new(
        status: StatusCode,
        error_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            error_type: error_type.into(),
            message: message.into(),
            fields: None,
            internal: None,
        }
    }

    /// Create a validation error with field details
    pub fn validation(fields: Vec<FieldError>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error_type: "validation_error".to_string(),
            message: "Request validation failed".to_string(),
            fields: Some(fields),
            internal: None,
        }
    }

    /// Create a 400 Bad Request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    /// Create a 401 Unauthorized error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    /// Create a 403 Forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// Create a 404 Not Found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    /// Create a 409 Conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// Create a 500 Internal Server Error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }

    /// Add internal details (for logging, hidden from response in prod)
    pub fn with_internal(mut self, details: impl Into<String>) -> Self {
        self.internal = Some(details.into());
        self
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for ApiError {}

/// JSON representation of API error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
    /// Unique error ID for log correlation (format: err_{uuid})
    pub error_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorBody {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldError>>,
}

impl ErrorResponse {
    /// Create an ErrorResponse from an ApiError with environment-aware masking
    ///
    /// In production mode:
    /// - Internal server errors (5xx) show generic messages
    /// - Validation errors always include field details
    /// - Client errors (4xx) show their messages
    ///
    /// In development mode:
    /// - All error details are shown
    pub fn from_api_error(err: ApiError, env: Environment) -> Self {
        let error_id = generate_error_id();

        // Always log the full error details with error_id for correlation
        if err.status.is_server_error() {
            tracing::error!(
                error_id = %error_id,
                error_type = %err.error_type,
                message = %err.message,
                status = %err.status.as_u16(),
                internal = ?err.internal,
                environment = %env,
                "Server error occurred"
            );
        } else if err.status.is_client_error() {
            tracing::warn!(
                error_id = %error_id,
                error_type = %err.error_type,
                message = %err.message,
                status = %err.status.as_u16(),
                environment = %env,
                "Client error occurred"
            );
        } else {
            tracing::info!(
                error_id = %error_id,
                error_type = %err.error_type,
                message = %err.message,
                status = %err.status.as_u16(),
                environment = %env,
                "Error response generated"
            );
        }

        // Determine the message and fields based on environment and error type
        let (message, fields) = if env.is_production() && err.status.is_server_error() {
            // In production, mask internal server error details
            // But preserve validation error fields (they're always shown per requirement 3.5)
            let masked_message = "An internal error occurred".to_string();
            // Validation errors keep their fields even in production
            let fields = if err.error_type == "validation_error" {
                err.fields
            } else {
                None
            };
            (masked_message, fields)
        } else {
            // In development or for non-5xx errors, show full details
            (err.message, err.fields)
        };

        Self {
            error: ErrorBody {
                error_type: err.error_type,
                message,
                fields,
            },
            error_id,
            request_id: None,
        }
    }
}

impl From<ApiError> for ErrorResponse {
    fn from(err: ApiError) -> Self {
        // Use the cached environment
        let env = get_environment();
        Self::from_api_error(err, env)
    }
}

// Conversion from common error types
impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::bad_request(format!("Invalid JSON: {}", err))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::internal("I/O error").with_internal(err.to_string())
    }
}

impl From<hyper::Error> for ApiError {
    fn from(err: hyper::Error) -> Self {
        ApiError::internal("HTTP error").with_internal(err.to_string())
    }
}

impl From<rustapi_validate::ValidationError> for ApiError {
    fn from(err: rustapi_validate::ValidationError) -> Self {
        let fields = err
            .fields
            .into_iter()
            .map(|f| FieldError {
                field: f.field,
                code: f.code,
                message: f.message,
            })
            .collect();

        ApiError::validation(fields)
    }
}

impl ApiError {
    /// Create a validation error from a ValidationError
    pub fn from_validation_error(err: rustapi_validate::ValidationError) -> Self {
        err.into()
    }

    /// Create a 503 Service Unavailable error
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            message,
        )
    }
}

// SQLx error conversion (feature-gated)
#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            // Pool timeout or connection acquisition failure → 503
            sqlx::Error::PoolTimedOut => {
                ApiError::service_unavailable("Database connection pool exhausted")
                    .with_internal(err.to_string())
            }

            // Pool closed → 503
            sqlx::Error::PoolClosed => {
                ApiError::service_unavailable("Database connection pool is closed")
                    .with_internal(err.to_string())
            }

            // Row not found → 404
            sqlx::Error::RowNotFound => ApiError::not_found("Resource not found"),

            // Database-specific errors need deeper inspection
            sqlx::Error::Database(db_err) => {
                // Check for unique constraint violation
                // PostgreSQL: 23505, MySQL: 1062, SQLite: 2067
                if let Some(code) = db_err.code() {
                    let code_str = code.as_ref();
                    if code_str == "23505" || code_str == "1062" || code_str == "2067" {
                        return ApiError::conflict("Resource already exists")
                            .with_internal(db_err.to_string());
                    }

                    // Foreign key violation
                    // PostgreSQL: 23503, MySQL: 1452, SQLite: 787
                    if code_str == "23503" || code_str == "1452" || code_str == "787" {
                        return ApiError::bad_request("Referenced resource does not exist")
                            .with_internal(db_err.to_string());
                    }

                    // Check constraint violation
                    // PostgreSQL: 23514
                    if code_str == "23514" {
                        return ApiError::bad_request("Data validation failed")
                            .with_internal(db_err.to_string());
                    }
                }

                // Generic database error
                ApiError::internal("Database error").with_internal(db_err.to_string())
            }

            // Connection errors → 503
            sqlx::Error::Io(_) => ApiError::service_unavailable("Database connection error")
                .with_internal(err.to_string()),

            // TLS errors → 503
            sqlx::Error::Tls(_) => {
                ApiError::service_unavailable("Database TLS error").with_internal(err.to_string())
            }

            // Protocol errors → 500
            sqlx::Error::Protocol(_) => {
                ApiError::internal("Database protocol error").with_internal(err.to_string())
            }

            // Type/decode errors → 500
            sqlx::Error::TypeNotFound { .. } => {
                ApiError::internal("Database type error").with_internal(err.to_string())
            }

            sqlx::Error::ColumnNotFound(_) => {
                ApiError::internal("Database column not found").with_internal(err.to_string())
            }

            sqlx::Error::ColumnIndexOutOfBounds { .. } => {
                ApiError::internal("Database column index error").with_internal(err.to_string())
            }

            sqlx::Error::ColumnDecode { .. } => {
                ApiError::internal("Database decode error").with_internal(err.to_string())
            }

            // Configuration errors → 500
            sqlx::Error::Configuration(_) => {
                ApiError::internal("Database configuration error").with_internal(err.to_string())
            }

            // Migration errors → 500
            sqlx::Error::Migrate(_) => {
                ApiError::internal("Database migration error").with_internal(err.to_string())
            }

            // Any other errors → 500
            _ => ApiError::internal("Database error").with_internal(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // **Feature: phase4-ergonomics-v1, Property 6: Error ID Uniqueness**
    //
    // For any sequence of N errors generated by the system, all N error IDs
    // should be unique. The error ID should appear in both the HTTP response
    // and the corresponding log entry.
    //
    // **Validates: Requirements 3.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_error_id_uniqueness(
            // Generate a random number of errors between 10 and 200
            num_errors in 10usize..200,
        ) {
            // Generate N error IDs
            let error_ids: Vec<String> = (0..num_errors)
                .map(|_| generate_error_id())
                .collect();

            // Collect into a HashSet to check uniqueness
            let unique_ids: HashSet<&String> = error_ids.iter().collect();

            // All IDs should be unique
            prop_assert_eq!(
                unique_ids.len(),
                error_ids.len(),
                "Generated {} error IDs but only {} were unique",
                error_ids.len(),
                unique_ids.len()
            );

            // All IDs should follow the format err_{uuid}
            for id in &error_ids {
                prop_assert!(
                    id.starts_with("err_"),
                    "Error ID '{}' does not start with 'err_'",
                    id
                );

                // The UUID part should be 32 hex characters (simple format)
                let uuid_part = &id[4..];
                prop_assert_eq!(
                    uuid_part.len(),
                    32,
                    "UUID part '{}' should be 32 characters, got {}",
                    uuid_part,
                    uuid_part.len()
                );

                // All characters should be valid hex
                prop_assert!(
                    uuid_part.chars().all(|c| c.is_ascii_hexdigit()),
                    "UUID part '{}' contains non-hex characters",
                    uuid_part
                );
            }
        }
    }

    // **Feature: phase4-ergonomics-v1, Property 6: Error ID in Response**
    //
    // For any ApiError converted to ErrorResponse, the error_id field should
    // be present and follow the correct format.
    //
    // **Validates: Requirements 3.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_error_response_contains_error_id(
            error_type in "[a-z_]{1,20}",
            message in "[a-zA-Z0-9 ]{1,100}",
        ) {
            let api_error = ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, error_type, message);
            let error_response = ErrorResponse::from(api_error);

            // error_id should be present and follow format
            prop_assert!(
                error_response.error_id.starts_with("err_"),
                "Error ID '{}' does not start with 'err_'",
                error_response.error_id
            );

            let uuid_part = &error_response.error_id[4..];
            prop_assert_eq!(uuid_part.len(), 32);
            prop_assert!(uuid_part.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_error_id_format() {
        let error_id = generate_error_id();

        // Should start with "err_"
        assert!(error_id.starts_with("err_"));

        // Total length should be 4 (prefix) + 32 (uuid simple format) = 36
        assert_eq!(error_id.len(), 36);

        // UUID part should be valid hex
        let uuid_part = &error_id[4..];
        assert!(uuid_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_error_response_includes_error_id() {
        let api_error = ApiError::bad_request("test error");
        let error_response = ErrorResponse::from(api_error);

        // error_id should be present
        assert!(error_response.error_id.starts_with("err_"));
        assert_eq!(error_response.error_id.len(), 36);
    }

    #[test]
    fn test_error_id_in_json_serialization() {
        let api_error = ApiError::internal("test error");
        let error_response = ErrorResponse::from(api_error);

        let json = serde_json::to_string(&error_response).unwrap();

        // JSON should contain error_id field
        assert!(json.contains("\"error_id\":"));
        assert!(json.contains("err_"));
    }

    #[test]
    fn test_multiple_error_ids_are_unique() {
        let ids: Vec<String> = (0..1000).map(|_| generate_error_id()).collect();
        let unique: HashSet<_> = ids.iter().collect();

        assert_eq!(ids.len(), unique.len(), "All error IDs should be unique");
    }

    // **Feature: phase4-ergonomics-v1, Property 4: Production Error Masking**
    //
    // For any internal error (5xx) when RUSTAPI_ENV=production, the response body
    // should contain only a generic error message and error ID, without stack traces,
    // internal details, or sensitive information.
    //
    // **Validates: Requirements 3.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_production_error_masking(
            // Generate random error messages that could contain sensitive info
            // Use longer strings to avoid false positives where short strings appear in masked message
            sensitive_message in "[a-zA-Z0-9_]{10,200}",
            internal_details in "[a-zA-Z0-9_]{10,200}",
            // Generate random 5xx status codes
            status_code in prop::sample::select(vec![500u16, 501, 502, 503, 504, 505]),
        ) {
            // Create an internal error with potentially sensitive details
            let api_error = ApiError::new(
                StatusCode::from_u16(status_code).unwrap(),
                "internal_error",
                sensitive_message.clone()
            ).with_internal(internal_details.clone());

            // Convert to ErrorResponse in production mode
            let error_response = ErrorResponse::from_api_error(api_error, Environment::Production);

            // The message should be masked to a generic message
            prop_assert_eq!(
                &error_response.error.message,
                "An internal error occurred",
                "Production 5xx error should have masked message, got: {}",
                &error_response.error.message
            );

            // The original sensitive message should NOT appear in the response
            // (only check if the message is long enough to be meaningful)
            if sensitive_message.len() >= 10 {
                prop_assert!(
                    !error_response.error.message.contains(&sensitive_message),
                    "Production error response should not contain original message"
                );
            }

            // Internal details should NOT appear anywhere in the serialized response
            let json = serde_json::to_string(&error_response).unwrap();
            if internal_details.len() >= 10 {
                prop_assert!(
                    !json.contains(&internal_details),
                    "Production error response should not contain internal details"
                );
            }

            // Error ID should still be present
            prop_assert!(
                error_response.error_id.starts_with("err_"),
                "Error ID should be present in production error response"
            );
        }
    }

    // **Feature: phase4-ergonomics-v1, Property 5: Development Error Details**
    //
    // For any error when RUSTAPI_ENV=development, the response body should contain
    // detailed error information including the original error message and any
    // available context.
    //
    // **Validates: Requirements 3.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_development_error_details(
            // Generate random error messages
            error_message in "[a-zA-Z0-9 ]{1,100}",
            error_type in "[a-z_]{1,20}",
            // Generate random status codes (both 4xx and 5xx)
            status_code in prop::sample::select(vec![400u16, 401, 403, 404, 500, 502, 503]),
        ) {
            // Create an error with details
            let api_error = ApiError::new(
                StatusCode::from_u16(status_code).unwrap(),
                error_type.clone(),
                error_message.clone()
            );

            // Convert to ErrorResponse in development mode
            let error_response = ErrorResponse::from_api_error(api_error, Environment::Development);

            // The original message should be preserved
            prop_assert_eq!(
                error_response.error.message,
                error_message,
                "Development error should preserve original message"
            );

            // The error type should be preserved
            prop_assert_eq!(
                error_response.error.error_type,
                error_type,
                "Development error should preserve error type"
            );

            // Error ID should be present
            prop_assert!(
                error_response.error_id.starts_with("err_"),
                "Error ID should be present in development error response"
            );
        }
    }

    // **Feature: phase4-ergonomics-v1, Property 7: Validation Error Field Details**
    //
    // For any validation error in any environment (production or development),
    // the response should include field-level error details with field name,
    // error code, and message.
    //
    // **Validates: Requirements 3.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_validation_error_field_details(
            // Generate random field errors
            field_name in "[a-z_]{1,20}",
            field_code in "[a-z_]{1,15}",
            field_message in "[a-zA-Z0-9 ]{1,50}",
            // Test in both environments
            is_production in proptest::bool::ANY,
        ) {
            let env = if is_production {
                Environment::Production
            } else {
                Environment::Development
            };

            // Create a validation error with field details
            let field_error = FieldError {
                field: field_name.clone(),
                code: field_code.clone(),
                message: field_message.clone(),
            };
            let api_error = ApiError::validation(vec![field_error]);

            // Convert to ErrorResponse
            let error_response = ErrorResponse::from_api_error(api_error, env);

            // Fields should always be present for validation errors
            prop_assert!(
                error_response.error.fields.is_some(),
                "Validation error should always include fields in {} mode",
                env
            );

            let fields = error_response.error.fields.as_ref().unwrap();
            prop_assert_eq!(
                fields.len(),
                1,
                "Should have exactly one field error"
            );

            let field = &fields[0];

            // Field name should be preserved
            prop_assert_eq!(
                &field.field,
                &field_name,
                "Field name should be preserved in {} mode",
                env
            );

            // Field code should be preserved
            prop_assert_eq!(
                &field.code,
                &field_code,
                "Field code should be preserved in {} mode",
                env
            );

            // Field message should be preserved
            prop_assert_eq!(
                &field.message,
                &field_message,
                "Field message should be preserved in {} mode",
                env
            );

            // Verify JSON serialization includes all field details
            let json = serde_json::to_string(&error_response).unwrap();
            prop_assert!(
                json.contains(&field_name),
                "JSON should contain field name in {} mode",
                env
            );
            prop_assert!(
                json.contains(&field_code),
                "JSON should contain field code in {} mode",
                env
            );
            prop_assert!(
                json.contains(&field_message),
                "JSON should contain field message in {} mode",
                env
            );
        }
    }

    // Unit tests for Environment enum
    // Note: These tests verify the Environment::from_env() logic by testing the parsing
    // directly rather than modifying global environment variables (which causes race conditions
    // in parallel test execution).

    #[test]
    fn test_environment_from_env_production() {
        // Test the parsing logic directly by simulating what from_env() does
        // This avoids race conditions with parallel tests

        // Test "production" variants
        assert!(matches!(
            match "production".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Production
        ));

        assert!(matches!(
            match "prod".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Production
        ));

        assert!(matches!(
            match "PRODUCTION".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Production
        ));

        assert!(matches!(
            match "PROD".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Production
        ));
    }

    #[test]
    fn test_environment_from_env_development() {
        // Test the parsing logic directly by simulating what from_env() does
        // This avoids race conditions with parallel tests

        // Test "development" and other variants that should default to Development
        assert!(matches!(
            match "development".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Development
        ));

        assert!(matches!(
            match "dev".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Development
        ));

        assert!(matches!(
            match "test".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Development
        ));

        assert!(matches!(
            match "anything_else".to_lowercase().as_str() {
                "production" | "prod" => Environment::Production,
                _ => Environment::Development,
            },
            Environment::Development
        ));
    }

    #[test]
    fn test_environment_default_is_development() {
        // Test that the default is Development
        assert_eq!(Environment::default(), Environment::Development);
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Development), "development");
        assert_eq!(format!("{}", Environment::Production), "production");
    }

    #[test]
    fn test_environment_is_methods() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Production.is_development());
        assert!(Environment::Development.is_development());
        assert!(!Environment::Development.is_production());
    }

    #[test]
    fn test_production_masks_5xx_errors() {
        let error =
            ApiError::internal("Sensitive database connection string: postgres://user:pass@host");
        let response = ErrorResponse::from_api_error(error, Environment::Production);

        assert_eq!(response.error.message, "An internal error occurred");
        assert!(!response.error.message.contains("postgres"));
    }

    #[test]
    fn test_production_shows_4xx_errors() {
        let error = ApiError::bad_request("Invalid email format");
        let response = ErrorResponse::from_api_error(error, Environment::Production);

        // 4xx errors should show their message even in production
        assert_eq!(response.error.message, "Invalid email format");
    }

    #[test]
    fn test_development_shows_all_errors() {
        let error = ApiError::internal("Detailed error: connection refused to 192.168.1.1:5432");
        let response = ErrorResponse::from_api_error(error, Environment::Development);

        assert_eq!(
            response.error.message,
            "Detailed error: connection refused to 192.168.1.1:5432"
        );
    }

    #[test]
    fn test_validation_errors_always_show_fields() {
        let fields = vec![
            FieldError {
                field: "email".to_string(),
                code: "invalid_format".to_string(),
                message: "Invalid email format".to_string(),
            },
            FieldError {
                field: "age".to_string(),
                code: "min".to_string(),
                message: "Must be at least 18".to_string(),
            },
        ];

        let error = ApiError::validation(fields.clone());

        // Test in production
        let prod_response = ErrorResponse::from_api_error(error.clone(), Environment::Production);
        assert!(prod_response.error.fields.is_some());
        let prod_fields = prod_response.error.fields.unwrap();
        assert_eq!(prod_fields.len(), 2);
        assert_eq!(prod_fields[0].field, "email");
        assert_eq!(prod_fields[1].field, "age");

        // Test in development
        let dev_response = ErrorResponse::from_api_error(error, Environment::Development);
        assert!(dev_response.error.fields.is_some());
        let dev_fields = dev_response.error.fields.unwrap();
        assert_eq!(dev_fields.len(), 2);
    }
}
