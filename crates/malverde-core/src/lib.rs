//! Malverde Core
//!
//! Núcleo del harness: configuración, tipos compartidos, errores y logging.

pub mod config;
pub mod error;
pub mod logging;
pub mod types;

pub use config::{default_data_dir, Config, PermissionMode, PROVIDER_PRESETS};
pub use error::{MalverdeError, Result};
pub use logging::init_logging;
pub use types::{MemoryFact, Message, Role, SessionId, SessionMeta, StopReason, ToolCall, TraceEvent, Usage};
