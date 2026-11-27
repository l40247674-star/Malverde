use thiserror::Error;

/// Error unificado del harness Malverde.
#[derive(Debug, Error)]
pub enum MalverdeError {
    #[error("error de configuración: {0}")]
    Config(String),
    #[error("error de almacenamiento: {0}")]
    Storage(String),
    #[error("error del proveedor LLM: {0}")]
    Provider(String),
    #[error("error de herramienta: {0}")]
    Tool(String),
    #[error("error del agente: {0}")]
    Agent(String),
    #[error("permiso denegado: {0}")]
    Permission(String),
    #[error("error de skills: {0}")]
    Skill(String),
    #[error("error del planificador: {0}")]
    Scheduler(String),
    #[error("error del gateway: {0}")]
    Gateway(String),
    #[error("error de E/S: {0}")]
    Io(String),
    #[error("error de serialización: {0}")]
    Serialization(String),
    #[error("validación fallida: {0}")]
    Validation(String),
    #[error("error desconocido: {0}")]
    Unknown(String),
}

impl MalverdeError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }
    pub fn provider(msg: impl Into<String>) -> Self {
        Self::Provider(msg.into())
    }
    pub fn tool(msg: impl Into<String>) -> Self {
        Self::Tool(msg.into())
    }
    pub fn agent(msg: impl Into<String>) -> Self {
        Self::Agent(msg.into())
    }
    pub fn permission(msg: impl Into<String>) -> Self {
        Self::Permission(msg.into())
    }
    pub fn skill(msg: impl Into<String>) -> Self {
        Self::Skill(msg.into())
    }
    pub fn scheduler(msg: impl Into<String>) -> Self {
        Self::Scheduler(msg.into())
    }
    pub fn gateway(msg: impl Into<String>) -> Self {
        Self::Gateway(msg.into())
    }
    pub fn io(err: impl std::fmt::Display) -> Self {
        Self::Io(err.to_string())
    }
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, MalverdeError>;
