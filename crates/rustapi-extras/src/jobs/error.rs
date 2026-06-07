use std::fmt;

/// Errors that can occur during job processing.
#[derive(Debug)]
pub enum JobError {
    /// Failed to serialize or deserialize job data.
    SerializationError(serde_json::Error),
    /// The storage backend encountered an error.
    BackendError(String),
    /// The requested job was not found.
    NotFound(String),
    /// A worker failed to execute the job.
    WorkerError(String),
    /// Configuration is invalid or missing.
    ConfigError(String),
    /// No handler is registered for the given job type.
    UnknownJobType(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationError(e) => write!(f, "Job serialization error: {}", e),
            Self::BackendError(msg) => write!(f, "Backend error: {}", msg),
            Self::NotFound(msg) => write!(f, "Job not found: {}", msg),
            Self::WorkerError(msg) => write!(f, "Worker error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::UnknownJobType(msg) => write!(f, "Unknown job type: {}", msg),
        }
    }
}

impl std::error::Error for JobError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SerializationError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for JobError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e)
    }
}

/// Specialized `Result` type for job operations.
pub type Result<T> = std::result::Result<T, JobError>;
