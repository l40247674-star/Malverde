use malverde_core::{MalverdeError, MalverdeResult};
use thiserror::Error;

/// Job-specific error types
#[derive(Debug, Error)]
pub enum JobError {
    /// Job not found
    #[error("Job not found: {0}")]
    NotFound(String),

    /// Job already exists
    #[error("Job already exists: {0}")]
    AlreadyExists(String),

    /// Job is in invalid state for operation
    #[error("Job in invalid state: {0}")]
    InvalidState(String),

    /// Job execution failed
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    /// Job timeout
    #[error("Job timeout after {0}ms")]
    Timeout(u64),

    /// Checkpoint error
    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    /// Recovery error
    #[error("Recovery error: {0}")]
    RecoveryError(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(#[from] Box<dyn std::error::Error + Send + Sync>),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Result type for job operations
pub type JobResult<T> = Result<T, JobError>;

impl From<JobError> for MalverdeError {
    fn from(err: JobError) -> Self {
        MalverdeError::JobError(err.to_string())
    }
}

impl From<MalverdeError> for JobError {
    fn from(err: MalverdeError) -> Self {
        JobError::ExecutionFailed(err.to_string())
    }
}
