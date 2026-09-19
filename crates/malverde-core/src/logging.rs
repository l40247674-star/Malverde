use std::sync::Arc;
use tracing::{Level, Subscriber};
use tracing_subscriber::{fmt, EnvFilter};

use super::config::Config;
use super::error::MalverdeError;
use super::types::LogLevel;

pub fn init_logger(config: &Config) -> Result<(), MalverdeError> {
    let level = match config.log_level {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(level.into())
        .add_directive("malverde=debug".parse().unwrap());

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_target(config.debug_mode)
        .with_ansi(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| MalverdeError::ConfigError(e.to_string()))?;

    Ok(())
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            LogLevel::Error => tracing::error!($($arg)*),
            LogLevel::Warn => tracing::warn!($($arg)*),
            LogLevel::Info => tracing::info!($($arg)*),
            LogLevel::Debug => tracing::debug!($($arg)*),
            LogLevel::Trace => tracing::trace!($($arg)*),
        }
    };
}

#[derive(Debug, Clone)]
pub struct LoggingContext {
    pub module: String,
    pub operation: String,
}

impl LoggingContext {
    pub fn new(module: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            operation: operation.into(),
        }
    }
    pub fn log(&self, level: LogLevel, message: impl AsRef<str>) {
        let full_message = format!(
            "[{}] [{}] {}",
            self.module, self.operation, message.as_ref()
        );
        match level {
            LogLevel::Error => tracing::error!("{}", full_message),
            LogLevel::Warn => tracing::warn!("{}", full_message),
            LogLevel::Info => tracing::info!("{}", full_message),
            LogLevel::Debug => tracing::debug!("{}", full_message),
            LogLevel::Trace => tracing::trace!("{}", full_message),
        }
    }
}