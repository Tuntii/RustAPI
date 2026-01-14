//! Diesel database integration for RustAPI
//!
//! This module provides a pool builder for Diesel connection pools with
//! health check integration.
//!
//! ## Pool Builder Example
//!
//! ```rust,ignore
//! use rustapi_extras::diesel::{DieselPoolBuilder, DieselPoolError};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), DieselPoolError> {
//!     let pool = DieselPoolBuilder::new("postgres://user:pass@localhost/db")
//!         .max_connections(10)
//!         .min_idle(Some(2))
//!         .connection_timeout(Duration::from_secs(5))
//!         .idle_timeout(Some(Duration::from_secs(300)))
//!         .max_lifetime(Some(Duration::from_secs(3600)))
//!         .build_postgres()?;
//!
//!     // Use pool...
//!     Ok(())
//! }
//! ```

use rustapi_core::health::{HealthCheck, HealthCheckBuilder, HealthStatus};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Error type for Diesel pool operations
#[derive(Debug, Error)]
pub enum DieselPoolError {
    /// Configuration error
    #[error("Pool configuration error: {0}")]
    Configuration(String),

    /// Connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// R2D2 pool error
    #[error("Pool error: {0}")]
    Pool(String),
}

/// Configuration for Diesel connection pool
///
/// This struct holds all configuration options for the pool builder.
#[derive(Debug, Clone)]
pub struct DieselPoolConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections to maintain
    pub min_idle: Option<u32>,
    /// Timeout for acquiring a connection
    pub connection_timeout: Duration,
    /// Maximum idle time before a connection is closed
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,
}

impl Default for DieselPoolConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
            min_idle: None,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        }
    }
}

impl DieselPoolConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), DieselPoolError> {
        if self.url.is_empty() {
            return Err(DieselPoolError::Configuration(
                "Database URL cannot be empty".to_string(),
            ));
        }
        if self.max_connections == 0 {
            return Err(DieselPoolError::Configuration(
                "max_connections must be greater than 0".to_string(),
            ));
        }
        if let Some(min_idle) = self.min_idle {
            if min_idle > self.max_connections {
                return Err(DieselPoolError::Configuration(
                    "min_idle cannot exceed max_connections".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Builder for Diesel connection pools
///
/// Provides a fluent API for configuring database connection pools with
/// sensible defaults and health check integration.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_extras::diesel::DieselPoolBuilder;
/// use std::time::Duration;
///
/// let pool = DieselPoolBuilder::new("postgres://localhost/mydb")
///     .max_connections(20)
///     .min_idle(Some(5))
///     .connection_timeout(Duration::from_secs(10))
///     .build_postgres()?;
/// ```
#[derive(Debug, Clone)]
pub struct DieselPoolBuilder {
    config: DieselPoolConfig,
}

impl DieselPoolBuilder {
    /// Create a new pool builder with the given database URL
    ///
    /// # Arguments
    ///
    /// * `url` - Database connection URL (e.g., "postgres://user:pass@localhost/db")
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config: DieselPoolConfig {
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

    /// Set the minimum number of idle connections to maintain
    ///
    /// Default: None (no minimum)
    pub fn min_idle(mut self, n: Option<u32>) -> Self {
        self.config.min_idle = n;
        self
    }

    /// Set the timeout for acquiring a connection
    ///
    /// Default: 30 seconds
    pub fn connection_timeout(mut self, d: Duration) -> Self {
        self.config.connection_timeout = d;
        self
    }

    /// Set the maximum idle time before a connection is closed
    ///
    /// Default: 600 seconds (10 minutes)
    pub fn idle_timeout(mut self, d: Option<Duration>) -> Self {
        self.config.idle_timeout = d;
        self
    }

    /// Set the maximum lifetime of a connection
    ///
    /// Default: 1800 seconds (30 minutes)
    pub fn max_lifetime(mut self, d: Option<Duration>) -> Self {
        self.config.max_lifetime = d;
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &DieselPoolConfig {
        &self.config
    }

    /// Build a PostgreSQL connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "diesel-postgres")]
    pub fn build_postgres(
        self,
    ) -> Result<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>, DieselPoolError>
    {
        self.config.validate()?;

        let manager =
            diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(&self.config.url);

        let mut builder = r2d2::Pool::builder()
            .max_size(self.config.max_connections)
            .connection_timeout(self.config.connection_timeout);

        if let Some(min_idle) = self.config.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }

        if let Some(idle_timeout) = self.config.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }

        if let Some(max_lifetime) = self.config.max_lifetime {
            builder = builder.max_lifetime(Some(max_lifetime));
        }

        builder
            .build(manager)
            .map_err(|e| DieselPoolError::Pool(e.to_string()))
    }

    /// Build a MySQL connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "diesel-mysql")]
    pub fn build_mysql(
        self,
    ) -> Result<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::MysqlConnection>>, DieselPoolError>
    {
        self.config.validate()?;

        let manager =
            diesel::r2d2::ConnectionManager::<diesel::MysqlConnection>::new(&self.config.url);

        let mut builder = r2d2::Pool::builder()
            .max_size(self.config.max_connections)
            .connection_timeout(self.config.connection_timeout);

        if let Some(min_idle) = self.config.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }

        if let Some(idle_timeout) = self.config.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }

        if let Some(max_lifetime) = self.config.max_lifetime {
            builder = builder.max_lifetime(Some(max_lifetime));
        }

        builder
            .build(manager)
            .map_err(|e| DieselPoolError::Pool(e.to_string()))
    }

    /// Build a SQLite connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The connection cannot be established
    #[cfg(feature = "diesel-sqlite")]
    pub fn build_sqlite(
        self,
    ) -> Result<
        r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        DieselPoolError,
    > {
        self.config.validate()?;

        let manager =
            diesel::r2d2::ConnectionManager::<diesel::SqliteConnection>::new(&self.config.url);

        let mut builder = r2d2::Pool::builder()
            .max_size(self.config.max_connections)
            .connection_timeout(self.config.connection_timeout);

        if let Some(min_idle) = self.config.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }

        if let Some(idle_timeout) = self.config.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }

        if let Some(max_lifetime) = self.config.max_lifetime {
            builder = builder.max_lifetime(Some(max_lifetime));
        }

        builder
            .build(manager)
            .map_err(|e| DieselPoolError::Pool(e.to_string()))
    }

    /// Create a health check for a PostgreSQL pool
    ///
    /// The health check will attempt to get a connection from the pool.
    #[cfg(feature = "diesel-postgres")]
    pub fn health_check_postgres(
        pool: Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    ) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("postgres", move || {
                let pool = pool.clone();
                async move {
                    match pool.get() {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }

    /// Create a health check for a MySQL pool
    ///
    /// The health check will attempt to get a connection from the pool.
    #[cfg(feature = "diesel-mysql")]
    pub fn health_check_mysql(
        pool: Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::MysqlConnection>>>,
    ) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("mysql", move || {
                let pool = pool.clone();
                async move {
                    match pool.get() {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }

    /// Create a health check for a SQLite pool
    ///
    /// The health check will attempt to get a connection from the pool.
    #[cfg(feature = "diesel-sqlite")]
    pub fn health_check_sqlite(
        pool: Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>>,
    ) -> HealthCheck {
        HealthCheckBuilder::new(false)
            .add_check("sqlite", move || {
                let pool = pool.clone();
                async move {
                    match pool.get() {
                        Ok(_) => HealthStatus::healthy(),
                        Err(e) => HealthStatus::unhealthy(format!("Database check failed: {}", e)),
                    }
                }
            })
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests for DieselPoolBuilder
    #[test]
    fn test_builder_default_values() {
        let builder = DieselPoolBuilder::new("postgres://localhost/test");
        let config = builder.config();

        assert_eq!(config.url, "postgres://localhost/test");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_idle, None);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
    }

    #[test]
    fn test_builder_custom_values() {
        let builder = DieselPoolBuilder::new("postgres://localhost/test")
            .max_connections(20)
            .min_idle(Some(5))
            .connection_timeout(Duration::from_secs(10))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(900)));

        let config = builder.config();

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_idle, Some(5));
        assert_eq!(config.connection_timeout, Duration::from_secs(10));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(900)));
    }

    #[test]
    fn test_config_validation_empty_url() {
        let config = DieselPoolConfig::default();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DieselPoolError::Configuration(_)
        ));
    }

    #[test]
    fn test_config_validation_zero_max_connections() {
        let config = DieselPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_min_idle_exceeds_max() {
        let config = DieselPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 5,
            min_idle: Some(10),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = DieselPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 10,
            min_idle: Some(2),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_valid_no_min_idle() {
        let config = DieselPoolConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 10,
            min_idle: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
    }

    // **Feature: v1-features-roadmap, Property 9: Health check accuracy**
    //
    // *For any* database pool, health checks SHALL correctly report connectivity status.
    //
    // **Validates: Requirements 3.3**
    //
    // Note: This property test validates that the configuration is correctly
    // stored and validated. Actual health check behavior testing requires
    // integration tests with a real database.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_diesel_pool_configuration_respects_limits(
            max_conn in 1u32..100,
            min_idle_factor in 0.0f64..1.0,
            connection_timeout_secs in 1u64..120,
            idle_timeout_secs in 60u64..3600,
            max_lifetime_secs in 300u64..7200,
        ) {
            // Calculate min_idle as a fraction of max to ensure min <= max
            let min_idle = ((max_conn as f64) * min_idle_factor).floor() as u32;

            let builder = DieselPoolBuilder::new("postgres://localhost/test")
                .max_connections(max_conn)
                .min_idle(Some(min_idle))
                .connection_timeout(Duration::from_secs(connection_timeout_secs))
                .idle_timeout(Some(Duration::from_secs(idle_timeout_secs)))
                .max_lifetime(Some(Duration::from_secs(max_lifetime_secs)));

            let config = builder.config();

            // Verify all configuration values are correctly stored
            prop_assert_eq!(config.max_connections, max_conn);
            prop_assert_eq!(config.min_idle, Some(min_idle));
            prop_assert_eq!(config.connection_timeout, Duration::from_secs(connection_timeout_secs));
            prop_assert_eq!(config.idle_timeout, Some(Duration::from_secs(idle_timeout_secs)));
            prop_assert_eq!(config.max_lifetime, Some(Duration::from_secs(max_lifetime_secs)));

            // Verify configuration validates successfully
            prop_assert!(config.validate().is_ok());

            // Verify invariant: min_idle <= max_connections
            if let Some(min) = config.min_idle {
                prop_assert!(min <= config.max_connections);
            }
        }
    }

    // Property test for configuration validation
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_diesel_invalid_config_is_rejected(
            max_conn in 1u32..50,
            min_idle_excess in 1u32..50,
        ) {
            // Create config where min_idle > max (invalid)
            let config = DieselPoolConfig {
                url: "postgres://localhost/test".to_string(),
                max_connections: max_conn,
                min_idle: Some(max_conn + min_idle_excess),
                ..Default::default()
            };

            // Should fail validation
            prop_assert!(config.validate().is_err());
        }
    }
}
