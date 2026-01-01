//! SQLx database integration for RustAPI
//!
//! This module provides error conversion from SQLx errors to RustAPI's `ApiError` type,
//! enabling seamless database integration with appropriate HTTP status codes.
//!
//! ## Error Mapping
//!
//! | SQLx Error Type | HTTP Status | Error Type |
//! |-----------------|-------------|------------|
//! | Connection/Pool exhausted | 503 | service_unavailable |
//! | Row not found | 404 | not_found |
//! | Unique constraint violation | 409 | conflict |
//! | Other database errors | 500 | internal_error |
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_extras::sqlx::SqlxErrorExt;
//! use sqlx::PgPool;
//!
//! async fn get_user(pool: &PgPool, id: i64) -> Result<User, ApiError> {
//!     sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
//!         .fetch_one(pool)
//!         .await
//!         .map_err(|e| e.into_api_error())
//! }
//! ```

use rustapi_core::ApiError;

/// Extension trait for converting SQLx errors to ApiError
pub trait SqlxErrorExt {
    /// Convert this SQLx error into an appropriate ApiError
    fn into_api_error(self) -> ApiError;
}

impl SqlxErrorExt for sqlx::Error {
    fn into_api_error(self) -> ApiError {
        convert_sqlx_error(self)
    }
}

/// Convert a SQLx error to an appropriate ApiError
///
/// This function maps SQLx error types to HTTP status codes:
/// - Connection errors and pool exhaustion → 503 Service Unavailable
/// - Row not found → 404 Not Found
/// - Unique constraint violations → 409 Conflict
/// - Other errors → 500 Internal Server Error
pub fn convert_sqlx_error(err: sqlx::Error) -> ApiError {
    match &err {
        // Pool timeout or connection acquisition failure → 503
        sqlx::Error::PoolTimedOut => ApiError::new(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Database connection pool exhausted",
        )
        .with_internal(err.to_string()),

        // Pool closed → 503
        sqlx::Error::PoolClosed => ApiError::new(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Database connection pool is closed",
        )
        .with_internal(err.to_string()),

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
        sqlx::Error::Io(_) => ApiError::new(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Database connection error",
        )
        .with_internal(err.to_string()),

        // TLS errors → 503
        sqlx::Error::Tls(_) => ApiError::new(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Database TLS error",
        )
        .with_internal(err.to_string()),

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

/// Implement From<sqlx::Error> for ApiError
///
/// Note: This implementation is provided in rustapi-core with the `sqlx` feature flag.
/// The extension trait `SqlxErrorExt` is provided here for convenience when you need
/// explicit conversion control.
#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use proptest::prelude::*;

    #[test]
    fn test_pool_timeout_returns_503() {
        let err = sqlx::Error::PoolTimedOut;
        let api_err = convert_sqlx_error(err);
        assert_eq!(api_err.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(api_err.error_type, "service_unavailable");
    }

    #[test]
    fn test_pool_closed_returns_503() {
        let err = sqlx::Error::PoolClosed;
        let api_err = convert_sqlx_error(err);
        assert_eq!(api_err.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(api_err.error_type, "service_unavailable");
    }

    #[test]
    fn test_row_not_found_returns_404() {
        let err = sqlx::Error::RowNotFound;
        let api_err = convert_sqlx_error(err);
        assert_eq!(api_err.status, StatusCode::NOT_FOUND);
        assert_eq!(api_err.error_type, "not_found");
    }

    /// Enum representing the different categories of SQLx errors for property testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SqlxErrorCategory {
        /// Pool timeout - should return 503
        PoolTimeout,
        /// Pool closed - should return 503
        PoolClosed,
        /// Row not found - should return 404
        RowNotFound,
        /// Protocol error - should return 500
        Protocol,
        /// Column not found - should return 500
        ColumnNotFound,
    }

    impl SqlxErrorCategory {
        fn expected_status(&self) -> StatusCode {
            match self {
                SqlxErrorCategory::PoolTimeout => StatusCode::SERVICE_UNAVAILABLE,
                SqlxErrorCategory::PoolClosed => StatusCode::SERVICE_UNAVAILABLE,
                SqlxErrorCategory::RowNotFound => StatusCode::NOT_FOUND,
                SqlxErrorCategory::Protocol => StatusCode::INTERNAL_SERVER_ERROR,
                SqlxErrorCategory::ColumnNotFound => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }

        fn expected_error_type(&self) -> &'static str {
            match self {
                SqlxErrorCategory::PoolTimeout => "service_unavailable",
                SqlxErrorCategory::PoolClosed => "service_unavailable",
                SqlxErrorCategory::RowNotFound => "not_found",
                SqlxErrorCategory::Protocol => "internal_error",
                SqlxErrorCategory::ColumnNotFound => "internal_error",
            }
        }

        fn create_error(&self) -> sqlx::Error {
            match self {
                SqlxErrorCategory::PoolTimeout => sqlx::Error::PoolTimedOut,
                SqlxErrorCategory::PoolClosed => sqlx::Error::PoolClosed,
                SqlxErrorCategory::RowNotFound => sqlx::Error::RowNotFound,
                SqlxErrorCategory::Protocol => {
                    sqlx::Error::Protocol("test protocol error".to_string())
                }
                SqlxErrorCategory::ColumnNotFound => {
                    sqlx::Error::ColumnNotFound("test_column".to_string())
                }
            }
        }
    }

    /// Strategy to generate SQLx error categories
    fn sqlx_error_category_strategy() -> impl Strategy<Value = SqlxErrorCategory> {
        prop_oneof![
            Just(SqlxErrorCategory::PoolTimeout),
            Just(SqlxErrorCategory::PoolClosed),
            Just(SqlxErrorCategory::RowNotFound),
            Just(SqlxErrorCategory::Protocol),
            Just(SqlxErrorCategory::ColumnNotFound),
        ]
    }

    // **Feature: phase3-batteries-included, Property 22: SQLx error conversion**
    //
    // *For any* SQLx error type, conversion to ApiError SHALL produce an appropriate
    // HTTP status code (e.g., connection error → 503, query error → 500).
    //
    // **Validates: Requirements 8.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_sqlx_error_conversion_produces_appropriate_status(
            category in sqlx_error_category_strategy()
        ) {
            let sqlx_err = category.create_error();
            let api_err = convert_sqlx_error(sqlx_err);

            // Verify the status code matches the expected category
            prop_assert_eq!(
                api_err.status,
                category.expected_status(),
                "SQLx error category {:?} should produce status {:?}, got {:?}",
                category,
                category.expected_status(),
                api_err.status
            );

            // Verify error type is set appropriately
            prop_assert_eq!(
                api_err.error_type.as_str(),
                category.expected_error_type(),
                "SQLx error category {:?} should have error_type {:?}",
                category,
                category.expected_error_type()
            );
        }
    }

    // Additional property test for connection-related errors
    //
    // **Feature: phase3-batteries-included, Property 22: SQLx error conversion**
    // **Validates: Requirements 8.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_connection_errors_return_503(
            category in prop_oneof![
                Just(SqlxErrorCategory::PoolTimeout),
                Just(SqlxErrorCategory::PoolClosed),
            ]
        ) {
            let sqlx_err = category.create_error();
            let api_err = convert_sqlx_error(sqlx_err);

            // All connection-related errors should return 503
            prop_assert_eq!(
                api_err.status,
                StatusCode::SERVICE_UNAVAILABLE,
                "Connection errors should return 503 Service Unavailable"
            );

            // Error type should be service_unavailable
            prop_assert_eq!(
                api_err.error_type.as_str(),
                "service_unavailable",
                "Connection errors should have error_type 'service_unavailable'"
            );
        }
    }
}
