//! Audit store trait

use super::event::AuditEvent;
use super::query::AuditQueryBuilder;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

/// Result type for audit operations.
pub type AuditResult<T> = Result<T, AuditError>;

/// Errors that can occur during audit operations.
#[derive(Debug)]
pub enum AuditError {
    /// Failed to write audit event.
    WriteError(String),
    /// Failed to read audit events.
    ReadError(String),
    /// Storage is full.
    StorageFull,
    /// Event not found.
    NotFound(String),
    /// Serialization error.
    SerializationError(String),
    /// IO error.
    IoError(String),
    /// Configuration error.
    ConfigError(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WriteError(msg) => write!(f, "Failed to write audit event: {}", msg),
            Self::ReadError(msg) => write!(f, "Failed to read audit events: {}", msg),
            Self::StorageFull => write!(f, "Audit storage is full"),
            Self::NotFound(msg) => write!(f, "Audit event not found: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for AuditError {}

/// Trait for audit event storage backends.
pub trait AuditStore: Send + Sync {
    /// Log an audit event.
    fn log(&self, event: AuditEvent) -> AuditResult<()>;

    /// Log an audit event asynchronously.
    fn log_async(
        &self,
        event: AuditEvent,
    ) -> Pin<Box<dyn Future<Output = AuditResult<()>> + Send + '_>> {
        Box::pin(async move { self.log(event) })
    }

    /// Get an event by ID.
    fn get(&self, id: &str) -> AuditResult<Option<AuditEvent>>;

    /// Create a query builder.
    fn query(&self) -> AuditQueryBuilder<'_>
    where
        Self: Sized,
    {
        AuditQueryBuilder::new(self)
    }

    /// Execute a query and return matching events.
    fn execute_query(&self, query: &super::query::AuditQuery) -> AuditResult<Vec<AuditEvent>>;

    /// Count events matching the query.
    fn count(&self, query: &super::query::AuditQuery) -> AuditResult<usize>;

    /// Get the total number of stored events.
    fn total_count(&self) -> AuditResult<usize>;

    /// Clear all events (use with caution - for testing).
    fn clear(&self) -> AuditResult<()>;

    /// Flush any buffered events to storage.
    fn flush(&self) -> AuditResult<()>;
}
