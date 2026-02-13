//! SQLx database integration for RustAPI
//!
//! This module provides error conversion from SQLx errors to RustAPI's `ApiError` type,
//! and a pool builder for easy database connection pool configuration.
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
//! ## Pool Builder Example
//!
//! ```rust,ignore
//! use rustapi_extras::sqlx::{SqlxPoolBuilder, PoolError};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), PoolError> {
//!     let pool = SqlxPoolBuilder::new("postgres://user:pass@localhost/db")
//!         .max_connections(10)
//!         .min_connections(2)
//!         .connect_timeout(Duration::from_secs(5))
//!         .idle_timeout(Duration::from_secs(300))
//!         .max_lifetime(Duration::from_secs(3600))
//!         .build()
//!         .await?;
//!
//!     // Use pool...
//!     Ok(())
//! }
//! ```
//!
//! ## Error Conversion Example
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

#[cfg(any(
    feature = "sqlx-postgres",
    feature = "sqlx-mysql",
    feature = "sqlx-sqlite"
))]
use rustapi_core::health::{HealthCheck, HealthCheckBuilder, HealthStatus};
use rustapi_core::ApiError;
#[cfg(any(
    feature = "sqlx-postgres",
    feature = "sqlx-mysql",
    feature = "sqlx-sqlite"
))]
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Error type for pool operations
#[derive(Debug, Error)]
pub enum PoolError {
    /// Configuration error
    #[error("Pool configuration error: {0}")]
    Configuration(String),

    /// Connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// SQLx error
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Configuration for SQLx connection pool
///
/// This struct holds all configuration options for the pool builder.
/// It can be serialized/deserialized for configuration file support.
#[derive(Debug, Clone)]
pub struct SqlxPoolConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Timeout for acquiring a connection
    pub connect_timeout: Duration,
    /// Maximum idle time before a connection is closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
}

impl Default for SqlxPoolConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl SqlxPoolConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), PoolError> {
        if self.url.is_empty() {
            return Err(PoolError::Configuration(
                "Database URL cannot be empty".to_string(),
            ));
        }
        if self.max_connections == 0 {
            return Err(PoolError::Configuration(
                "max_connections must be greater than 0".to_string(),
            ));
        }
        if self.min_connections > self.max_connections {
            return Err(PoolError::Configuration(
                "min_connections cannot exceed max_connections".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for SQLx connection pools
///
/// Provides a fluent API for configuring database connection pools with
/// sensible defaults and health check integration.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_extras::sqlx::SqlxPoolBuilder;
/// use std::time::Duration;
///
/// let pool = SqlxPoolBuilder::new("postgres://localhost/mydb")
///     .max_connections(20)
///     .min_connections(5)
///     .connect_timeout(Duration::from_secs(10))
///     .build()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct SqlxPoolBuilder {
    config: SqlxPoolConfig,
}

impl SqlxPoolBuilder {
    /// Create a new pool builder with the given database URL
    ///
    /// # Arguments
    ///
    /// * `url` - Database connection URL (e.g., "postgres://user:pass@localhost/db")
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config: SqlxPoolConfig {
                url: url.into(),
                ..Default::default()
            },
        }
    }

    /// Set the maximum number of connections in the pool
    ///
    /// Default: 10
    pub fn max_connections(mut self, n: u32) -> Self {
        self.config.max_connections = n;
        self
    }

    /// Set the minimum number of connections to maintain
    ///
    /// Default: 1
    pub fn min_connections(mut self, n: u32) -> Self {
        self.config.min_connections = n;
        self
    }

    /// Set the timeout for acquiring a connection
    ///
    /// Default: 30 seconds
    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.config.connect_timeout = d;
        self
    }

    /// Set the maximum idle time before a connection is closed
    ///
    /// Default: 600 seconds (10 minutes)
    pub fn idle_timeout(mut self, d: Duration) -> Self {
        self.config.idle_timeout = d;
        self
    }

    /// Set the maximum lifetime of a connection
    ///
    /// Default: 1800 seconds (30 minutes)
    pub fn max_lifetime(mut self, d: Duration) -> Self {
        self.config.max_lifetime = d;
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &SqlxPoolConfig {
        &self.config
    }

    /// Build a PostgreSQL connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "sqlx-postgres")]
    pub async fn build_postgres(self) -> Result<sqlx::PgPool, PoolError> {
        self.config.validate()?;

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(self.config.connect_timeout)
            .idle_timeout(Some(self.config.idle_timeout))
            .max_lifetime(Some(self.config.max_lifetime))
            .connect(&self.config.url)
            .await?;

        Ok(pool)
    }

    /// Build a MySQL connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "sqlx-mysql")]
    pub async fn build_mysql(self) -> Result<sqlx::MySqlPool, PoolError> {
        self.config.validate()?;

        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(self.config.connect_timeout)
            .idle_timeout(Some(self.config.idle_timeout))
            .max_lifetime(Some(self.config.max_lifetime))
            .connect(&self.config.url)
            .await?;

        Ok(pool)
    }

    /// Build a SQLite connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "sqlx-sqlite")]
    pub async fn build_sqlite(self) -> Result<sqlx::SqlitePool, PoolError> {
        self.config.validate()?;

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(self.config.connect_timeout)
            .idle_timeout(Some(self.config.idle_timeout))
            .max_lifetime(Some(self.config.max_lifetime))
            .connect(&self.config.url)
            .await?;

        Ok(pool)
    }

    /// Create a health check for a PostgreSQL pool
    ///
    /// The health check will execute a simple query to verify connectivity.
    #[cfg(feature = "sqlx-postgres")]
    pub fn health_check_postgres(pool: Arc<sqlx::PgPool>) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("postgres", move || {
                let pool = pool.clone();
                async move {
                    match sqlx::query("SELECT 1").execute(pool.as_ref()).await {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }

    /// Create a health check for a MySQL pool
    ///
    /// The health check will execute a simple query to verify connectivity.
    #[cfg(feature = "sqlx-mysql")]
    pub fn health_check_mysql(pool: Arc<sqlx::MySqlPool>) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("mysql", move || {
                let pool = pool.clone();
                async move {
                    match sqlx::query("SELECT 1").execute(pool.as_ref()).await {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }

    /// Create a health check for a SQLite pool
    ///
    /// The health check will execute a simple query to verify connectivity.
    #[cfg(feature = "sqlx-sqlite")]
    pub fn health_check_sqlite(pool: Arc<sqlx::SqlitePool>) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("sqlite", move || {
                let pool = pool.clone();
                async move {
                    match sqlx::query("SELECT 1").execute(pool.as_ref()).await {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }
}

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

    // Unit tests for SqlxPoolBuilder
    #[test]
    fn test_builder_default_values() {
        let builder = SqlxPoolBuilder::new("postgres://localhost/test");
        let config = builder.config();

        assert_eq!(config.url, "postgres://localhost/test");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
    }

    #[test]
    fn test_builder_custom_values() {
        let builder = SqlxPoolBuilder::new("postgres://localhost/test")
            .max_connections(20)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(900));

        let config = builder.config();

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.max_lifetime, Duration::from_secs(900));
    }

    #[test]
    fn test_config_validation_empty_url() {
        let config = SqlxPoolConfig::default();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolError::Configuration(_)));
    }

    #[test]
    fn test_config_validation_zero_max_connections() {
        let config = SqlxPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_min_exceeds_max() {
        let config = SqlxPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 5,
            min_connections: 10,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = SqlxPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 10,
            min_connections: 2,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
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

    // **Feature: v1-features-roadmap, Property 8: Pool configuration respect**
    //
    // *For any* pool configuration with connection limits, the pool SHALL never
    // exceed the configured maximum connections.
    //
    // **Validates: Requirements 3.4**
    //
    // Note: This property test validates that the configuration is correctly
    // stored and validated. Actual pool behavior testing requires integration
    // tests with a real database.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_pool_configuration_respects_limits(
            max_conn in 1u32..100,
            min_conn_factor in 0.0f64..1.0,
            connect_timeout_secs in 1u64..120,
            idle_timeout_secs in 60u64..3600,
            max_lifetime_secs in 300u64..7200,
        ) {
            // Calculate min_connections as a fraction of max to ensure min <= max
            let min_conn = ((max_conn as f64) * min_conn_factor).floor() as u32;

            let builder = SqlxPoolBuilder::new("postgres://localhost/test")
                .max_connections(max_conn)
                .min_connections(min_conn)
                .connect_timeout(Duration::from_secs(connect_timeout_secs))
                .idle_timeout(Duration::from_secs(idle_timeout_secs))
                .max_lifetime(Duration::from_secs(max_lifetime_secs));

            let config = builder.config();

            // Verify all configuration values are correctly stored
            prop_assert_eq!(config.max_connections, max_conn);
            prop_assert_eq!(config.min_connections, min_conn);
            prop_assert_eq!(config.connect_timeout, Duration::from_secs(connect_timeout_secs));
            prop_assert_eq!(config.idle_timeout, Duration::from_secs(idle_timeout_secs));
            prop_assert_eq!(config.max_lifetime, Duration::from_secs(max_lifetime_secs));

            // Verify configuration validates successfully
            prop_assert!(config.validate().is_ok());

            // Verify invariant: min_connections <= max_connections
            prop_assert!(config.min_connections <= config.max_connections);
        }
    }

    // Property test for configuration validation
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_invalid_config_is_rejected(
            max_conn in 1u32..50,
            min_conn_excess in 1u32..50,
        ) {
            // Create config where min > max (invalid)
            let config = SqlxPoolConfig {
                url: "postgres://localhost/test".to_string(),
                max_connections: max_conn,
                min_connections: max_conn + min_conn_excess,
                ..Default::default()
            };

            // Should fail validation
            prop_assert!(config.validate().is_err());
        }
    }
}
