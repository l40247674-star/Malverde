use thiserror::Error;

#[derive(Debug, Error)]
pub enum MalverdeError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Security error: {0}")]
    SecurityError(String),
    #[error("Plugin error: {0}")]
    PluginError(String),
    #[error("Job error: {0}")]
    JobError(String),
    #[error("Event error: {0}")]
    EventError(String),
    #[error("Memory error: {0}")]
    MemoryError(String),
    #[error("Learning error: {0}")]
    LearningError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Filesystem error: {0}")]
    FilesystemError(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl MalverdeError {
    pub fn storage(error: impl Into<String>) -> Self {
        MalverdeError::StorageError(error.into())
    }
    pub fn serialization(error: impl Into<String>) -> Self {
        MalverdeError::SerializationError(error.into())
    }
    pub fn config(error: impl Into<String>) -> Self {
        MalverdeError::ConfigError(error.into())
    }
    pub fn validation(error: impl Into<String>) -> Self {
        MalverdeError::ValidationError(error.into())
    }
    pub fn security(error: impl Into<String>) -> Self {
        MalverdeError::SecurityError(error.into())
    }
    pub fn plugin(error: impl Into<String>) -> Self {
        MalverdeError::PluginError(error.into())
    }
    pub fn job(error: impl Into<String>) -> Self {
        MalverdeError::JobError(error.into())
    }
    pub fn event(error: impl Into<String>) -> Self {
        MalverdeError::EventError(error.into())
    }
    pub fn memory(error: impl Into<String>) -> Self {
        MalverdeError::MemoryError(error.into())
    }
    pub fn learning(error: impl Into<String>) -> Self {
        MalverdeError::LearningError(error.into())
    }
    pub fn network(error: impl Into<String>) -> Self {
        MalverdeError::NetworkError(error.into())
    }
    pub fn filesystem(error: impl Into<String>) -> Self {
        MalverdeError::FilesystemError(error.into())
    }
}

pub type MalverdeResult<T> = Result<T, MalverdeError>;