use thiserror::Error;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Unknown job type: {0}")]
    UnknownJobType(String),
}

pub type Result<T> = std::result::Result<T, JobError>;
