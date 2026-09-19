use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use super::error::MalverdeError;
use super::types::LogLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub storage_path: PathBuf,
    pub log_level: LogLevel,
    pub plugins_dir: PathBuf,
    pub data_dir: PathBuf,
    pub debug_mode: bool,
    pub max_threads: usize,
    pub storage_timeout: u64,
    pub enable_fts5: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "Malverde".to_string(),
            version: "0.1.0".to_string(),
            storage_path: PathBuf::from("malverde.db"),
            log_level: LogLevel::Info,
            plugins_dir: PathBuf::from("plugins"),
            data_dir: PathBuf::from("data"),
            debug_mode: false,
            max_threads: 4,
            storage_timeout: 30,
            enable_fts5: true,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, MalverdeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| MalverdeError::FilesystemError(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))
    }
    pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), MalverdeError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| MalverdeError::SerializationError(e.to_string()))?;
        std::fs::write(path, content)
            .map_err(|e| MalverdeError::FilesystemError(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct SharedConfig {
    inner: Arc<Config>,
}

impl SharedConfig {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(config),
        }
    }
    pub fn get(&self) -> Arc<Config> {
        self.inner.clone()
    }
}

impl std::ops::Deref for SharedConfig {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}