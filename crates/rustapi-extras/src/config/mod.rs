//! Configuration management with environment variable support.
//!
//! This module provides configuration loading from `.env` files and
//! typed configuration extraction via the `Config<T>` extractor.
//!
//! # Example
//!
//! ```ignore
//! use rustapi_extras::config::{Config, Environment, load_dotenv};
//! use serde::Deserialize;
//!
//! // Load .env file at startup
//! load_dotenv();
//!
//! #[derive(Deserialize)]
//! struct AppConfig {
//!     database_url: String,
//!     port: u16,
//! }
//!
//! // Load config from environment variables
//! let config = Config::<AppConfig>::from_env().expect("Failed to load config");
//! println!("Database URL: {}", config.0.database_url);
//! ```

use serde::de::DeserializeOwned;
use std::fmt;

/// Error type for configuration loading failures.
#[derive(Debug)]
pub enum ConfigError {
    /// Environment variable deserialization failed.
    EnvyError(envy::Error),
    /// A required environment variable is missing.
    MissingVar(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::EnvyError(e) => write!(f, "Configuration error: {}", e),
            ConfigError::MissingVar(var) => {
                write!(f, "Missing required environment variable: {}", var)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::EnvyError(e) => Some(e),
            ConfigError::MissingVar(_) => None,
        }
    }
}

impl From<envy::Error> for ConfigError {
    fn from(err: envy::Error) -> Self {
        ConfigError::EnvyError(err)
    }
}

/// Environment profile for the application.
///
/// Detected from the `RUSTAPI_ENV` environment variable.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::Environment;
///
/// let env = Environment::current();
/// if env.is_production() {
///     // Apply production settings
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    /// Development environment with verbose errors and debug logging.
    Development,
    /// Production environment with error masking and optimized settings.
    Production,
    /// Custom environment name for specialized deployments.
    Custom(String),
}

impl Environment {
    /// Detect the current environment from `RUSTAPI_ENV`.
    ///
    /// Returns:
    /// - `Production` if `RUSTAPI_ENV` is "production" or "prod"
    /// - `Development` if `RUSTAPI_ENV` is "development", "dev", or not set
    /// - `Custom(name)` for any other value
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rustapi_extras::config::Environment;
    ///
    /// // Set RUSTAPI_ENV=production for production mode
    /// let env = Environment::current();
    /// assert!(env.is_production());
    /// ```
    pub fn current() -> Self {
        match std::env::var("RUSTAPI_ENV").as_deref() {
            Ok("production") | Ok("prod") => Self::Production,
            Ok("development") | Ok("dev") => Self::Development,
            Ok(other) => Self::Custom(other.to_string()),
            Err(_) => Self::Development,
        }
    }

    /// Check if running in production mode.
    ///
    /// In production mode:
    /// - Error details are masked
    /// - Debug logging is disabled
    /// - Performance optimizations are enabled
    pub fn is_production(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Check if running in development mode.
    ///
    /// In development mode:
    /// - Verbose error messages are shown
    /// - Debug logging is enabled
    /// - Hot reloading may be available
    pub fn is_development(&self) -> bool {
        matches!(self, Self::Development)
    }

    /// Get the environment name as a string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
            Self::Custom(name) => name,
        }
    }

    /// Check if error details should be shown.
    ///
    /// In development mode, verbose error messages are shown.
    /// In production mode, error details are masked for security.
    pub fn show_error_details(&self) -> bool {
        !self.is_production()
    }

    /// Check if debug logging should be enabled.
    ///
    /// Returns true in development mode, false in production.
    pub fn enable_debug_logging(&self) -> bool {
        self.is_development()
    }

    /// Get the default log level for this environment.
    ///
    /// - Development: "debug"
    /// - Production: "info"
    /// - Custom: "info"
    pub fn default_log_level(&self) -> &'static str {
        match self {
            Self::Development => "debug",
            Self::Production => "info",
            Self::Custom(_) => "info",
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::current()
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration extractor that deserializes environment variables.
///
/// Uses the `envy` crate to deserialize environment variables into
/// a typed configuration struct. Field names are converted to
/// SCREAMING_SNAKE_CASE for environment variable lookup.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::Config;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct DatabaseConfig {
///     database_url: String,  // Reads from DATABASE_URL
///     pool_size: u32,        // Reads from POOL_SIZE
/// }
///
/// let config = Config::<DatabaseConfig>::from_env()?;
/// println!("URL: {}", config.0.database_url);
/// ```
#[derive(Debug, Clone)]
pub struct Config<T>(pub T);

impl<T: DeserializeOwned> Config<T> {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing
    /// or if deserialization fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rustapi_extras::config::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct AppConfig {
    ///     port: u16,
    ///     host: String,
    /// }
    ///
    /// // Set PORT=8080 and HOST=localhost
    /// let config = Config::<AppConfig>::from_env()?;
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::from_env::<T>().map(Config).map_err(ConfigError::from)
    }

    /// Load configuration with a prefix.
    ///
    /// Only environment variables starting with the given prefix
    /// (followed by underscore) will be considered.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rustapi_extras::config::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct DbConfig {
    ///     url: String,   // Reads from DB_URL
    ///     pool: u32,     // Reads from DB_POOL
    /// }
    ///
    /// let config = Config::<DbConfig>::from_env_prefixed("DB")?;
    /// ```
    pub fn from_env_prefixed(prefix: &str) -> Result<Self, ConfigError> {
        envy::prefixed(format!("{}_", prefix))
            .from_env::<T>()
            .map(Config)
            .map_err(ConfigError::from)
    }

    /// Get the inner configuration value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Config<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Config<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Load environment variables from a `.env` file.
///
/// This function should be called early in application startup,
/// typically before creating the `RustApi` instance.
///
/// The function will:
/// - Look for a `.env` file in the current directory
/// - Load variables from the file into the environment
/// - NOT override existing environment variables
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::load_dotenv;
///
/// fn main() {
///     // Load .env file first
///     load_dotenv();
///
///     // Now environment variables from .env are available
///     let api_key = std::env::var("API_KEY").expect("API_KEY not set");
/// }
/// ```
///
/// # Notes
///
/// - If the `.env` file doesn't exist, this function silently succeeds
/// - Existing environment variables take precedence over `.env` values
/// - The file is expected to be in `KEY=value` format
pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

/// Load environment variables from a specific file path.
///
/// Unlike `load_dotenv()`, this function allows specifying a custom
/// path for the environment file.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::load_dotenv_from;
///
/// // Load from a custom path
/// load_dotenv_from(".env.local");
/// ```
pub fn load_dotenv_from<P: AsRef<std::path::Path>>(path: P) {
    let _ = dotenvy::from_path(path);
}

/// Check if a required environment variable is set.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::require_env;
///
/// // Panics if DATABASE_URL is not set
/// let db_url = require_env("DATABASE_URL");
/// ```
///
/// # Panics
///
/// Panics with a descriptive message if the variable is not set.
pub fn require_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!(
            "Required environment variable '{}' is not set. \
             Please set it in your .env file or environment.",
            name
        )
    })
}

/// Get an environment variable with a default value.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::env_or;
///
/// let port = env_or("PORT", "8080");
/// let host = env_or("HOST", "127.0.0.1");
/// ```
pub fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Get an environment variable and parse it to a specific type.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::config::env_parse;
///
/// let port: u16 = env_parse("PORT").unwrap_or(8080);
/// let debug: bool = env_parse("DEBUG").unwrap_or(false);
/// ```
pub fn env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;

    #[test]
    fn test_environment_detection_development() {
        // Clear any existing value
        std::env::remove_var("RUSTAPI_ENV");

        let env = Environment::current();
        assert!(env.is_development());
        assert!(!env.is_production());
        assert_eq!(env.as_str(), "development");
    }

    #[test]
    fn test_environment_detection_production() {
        std::env::set_var("RUSTAPI_ENV", "production");
        let env = Environment::current();
        assert!(env.is_production());
        assert!(!env.is_development());
        assert_eq!(env.as_str(), "production");

        // Also test "prod" shorthand
        std::env::set_var("RUSTAPI_ENV", "prod");
        let env = Environment::current();
        assert!(env.is_production());

        // Clean up
        std::env::remove_var("RUSTAPI_ENV");
    }

    #[test]
    fn test_environment_detection_custom() {
        std::env::set_var("RUSTAPI_ENV", "staging");
        let env = Environment::current();
        assert!(!env.is_production());
        assert!(!env.is_development());
        assert_eq!(env.as_str(), "staging");

        // Clean up
        std::env::remove_var("RUSTAPI_ENV");
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(format!("{}", Environment::Development), "development");
        assert_eq!(format!("{}", Environment::Production), "production");
        assert_eq!(
            format!("{}", Environment::Custom("staging".to_string())),
            "staging"
        );
    }

    #[test]
    fn test_environment_defaults() {
        // Development defaults
        let dev = Environment::Development;
        assert!(dev.show_error_details());
        assert!(dev.enable_debug_logging());
        assert_eq!(dev.default_log_level(), "debug");

        // Production defaults
        let prod = Environment::Production;
        assert!(!prod.show_error_details());
        assert!(!prod.enable_debug_logging());
        assert_eq!(prod.default_log_level(), "info");

        // Custom defaults (same as production for safety)
        let custom = Environment::Custom("staging".to_string());
        assert!(custom.show_error_details()); // Custom is not production
        assert!(!custom.enable_debug_logging()); // Custom is not development
        assert_eq!(custom.default_log_level(), "info");
    }

    #[test]
    fn test_env_or_with_default() {
        // Use a unique var name to avoid conflicts
        let var_name = "RUSTAPI_TEST_ENV_OR_12345";
        std::env::remove_var(var_name);

        let value = env_or(var_name, "default_value");
        assert_eq!(value, "default_value");

        std::env::set_var(var_name, "actual_value");
        let value = env_or(var_name, "default_value");
        assert_eq!(value, "actual_value");

        // Clean up
        std::env::remove_var(var_name);
    }

    #[test]
    fn test_env_parse() {
        let var_name = "RUSTAPI_TEST_PARSE_12345";

        std::env::set_var(var_name, "42");
        let value: Option<u32> = env_parse(var_name);
        assert_eq!(value, Some(42));

        std::env::set_var(var_name, "true");
        let value: Option<bool> = env_parse(var_name);
        assert_eq!(value, Some(true));

        std::env::set_var(var_name, "not_a_number");
        let value: Option<u32> = env_parse(var_name);
        assert_eq!(value, None);

        // Clean up
        std::env::remove_var(var_name);
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestConfig {
        unit_test_string: String,
        unit_test_number: u32,
    }

    #[test]
    fn test_config_from_env() {
        // Set up test environment variables with unique names
        std::env::set_var("UNIT_TEST_STRING", "hello");
        std::env::set_var("UNIT_TEST_NUMBER", "42");

        let config = Config::<TestConfig>::from_env().unwrap();
        assert_eq!(config.unit_test_string, "hello");
        assert_eq!(config.unit_test_number, 42);

        // Clean up
        std::env::remove_var("UNIT_TEST_STRING");
        std::env::remove_var("UNIT_TEST_NUMBER");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MissingVarTestConfig {
        missing_var_test_string: String,
        missing_var_test_number: u32,
    }

    #[test]
    fn test_config_from_env_missing_var() {
        // Ensure the variables don't exist (use unique names to avoid race conditions)
        std::env::remove_var("MISSING_VAR_TEST_STRING");
        std::env::remove_var("MISSING_VAR_TEST_NUMBER");

        let result = Config::<MissingVarTestConfig>::from_env();
        assert!(result.is_err());
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct PrefixedConfig {
        url: String,
        port: u16,
    }

    #[test]
    fn test_config_from_env_prefixed() {
        std::env::set_var("MYAPP_URL", "http://localhost");
        std::env::set_var("MYAPP_PORT", "3000");

        let config = Config::<PrefixedConfig>::from_env_prefixed("MYAPP").unwrap();
        assert_eq!(config.url, "http://localhost");
        assert_eq!(config.port, 3000);

        // Clean up
        std::env::remove_var("MYAPP_URL");
        std::env::remove_var("MYAPP_PORT");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct DerefTestConfig {
        deref_test_string: String,
        deref_test_number: u32,
    }

    #[test]
    fn test_config_deref() {
        std::env::set_var("DEREF_TEST_STRING", "deref_test");
        std::env::set_var("DEREF_TEST_NUMBER", "100");

        let config = Config::<DerefTestConfig>::from_env().unwrap();

        // Test Deref
        assert_eq!(config.deref_test_string, "deref_test");
        assert_eq!(config.deref_test_number, 100);

        // Clean up
        std::env::remove_var("DEREF_TEST_STRING");
        std::env::remove_var("DEREF_TEST_NUMBER");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct InnerTestConfig {
        inner_test_string: String,
        inner_test_number: u32,
    }

    #[test]
    fn test_config_into_inner() {
        std::env::set_var("INNER_TEST_STRING", "inner_test");
        std::env::set_var("INNER_TEST_NUMBER", "200");

        let config = Config::<InnerTestConfig>::from_env().unwrap();
        let inner = config.into_inner();

        assert_eq!(inner.inner_test_string, "inner_test");
        assert_eq!(inner.inner_test_number, 200);

        // Clean up
        std::env::remove_var("INNER_TEST_STRING");
        std::env::remove_var("INNER_TEST_NUMBER");
    }

    // **Feature: phase3-batteries-included, Property 21: Config extractor deserialization**
    //
    // For any set of environment variables E and config struct T with fields matching E,
    // `Config<T>` SHALL deserialize E into T with correct field values.
    //
    // **Validates: Requirements 7.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_config_deserialization(
            string_value in "[a-zA-Z0-9_]{1,50}",
            number_value in 0u32..10000u32,
        ) {
            // Use unique variable names to avoid test interference
            let string_var = format!("PROP_TEST_STR_{}", std::process::id());
            let number_var = format!("PROP_TEST_NUM_{}", std::process::id());

            // Set environment variables
            std::env::set_var(&string_var, &string_value);
            std::env::set_var(&number_var, number_value.to_string());

            // Define a config struct that matches our env vars
            #[derive(Debug, Deserialize, PartialEq)]
            #[allow(dead_code)]
            struct PropTestConfig {
                prop_test_str: String,
                prop_test_num: u32,
            }

            // We need to use a prefix since envy uses field names directly
            // Actually, envy converts field names to SCREAMING_SNAKE_CASE
            // So prop_test_str becomes PROP_TEST_STR

            // But we have process ID in the var name, so we need a different approach
            // Let's use fixed names for the property test

            // Clean up the dynamic vars
            std::env::remove_var(&string_var);
            std::env::remove_var(&number_var);

            // Use fixed variable names for the actual test
            std::env::set_var("PROP_CONFIG_STRING", &string_value);
            std::env::set_var("PROP_CONFIG_NUMBER", number_value.to_string());

            #[derive(Debug, Deserialize, PartialEq)]
            struct PropConfig {
                prop_config_string: String,
                prop_config_number: u32,
            }

            let result = Config::<PropConfig>::from_env();

            // Clean up
            std::env::remove_var("PROP_CONFIG_STRING");
            std::env::remove_var("PROP_CONFIG_NUMBER");

            // Verify the result
            prop_assert!(result.is_ok(), "Config deserialization should succeed");
            let config = result.unwrap();
            prop_assert_eq!(&config.prop_config_string, &string_value, "String value should match");
            prop_assert_eq!(config.prop_config_number, number_value, "Number value should match");
        }
    }

    // Additional property test for optional fields
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_config_optional_fields(
            required_value in "[a-zA-Z0-9]{1,30}",
            optional_present in prop::bool::ANY,
            optional_value in "[a-zA-Z0-9]{1,30}",
        ) {
            #[derive(Debug, Deserialize, PartialEq)]
            struct OptionalConfig {
                required_field: String,
                #[serde(default)]
                optional_field: Option<String>,
            }

            std::env::set_var("REQUIRED_FIELD", &required_value);

            if optional_present {
                std::env::set_var("OPTIONAL_FIELD", &optional_value);
            } else {
                std::env::remove_var("OPTIONAL_FIELD");
            }

            let result = Config::<OptionalConfig>::from_env();

            // Clean up
            std::env::remove_var("REQUIRED_FIELD");
            std::env::remove_var("OPTIONAL_FIELD");

            prop_assert!(result.is_ok(), "Config with optional fields should deserialize");
            let config = result.unwrap();
            prop_assert_eq!(&config.required_field, &required_value);

            if optional_present {
                prop_assert_eq!(&config.optional_field, &Some(optional_value));
            } else {
                prop_assert!(config.optional_field.is_none());
            }
        }
    }
}
