//! Malverde Agent
//!
//! El loop agéntico del harness: llama al modelo, ejecuta las herramientas que
//! pide, observa los resultados y repite hasta terminar o agotar el presupuesto.

pub mod agent;
pub mod ext_tools;
pub mod observer;
pub mod prompt;

pub use agent::{default_subagent_factory, Agent, AgentConfig, AgentRunResult};
pub use ext_tools::{MemoryForgetTool, MemoryRecallTool, MemoryRememberTool, SpawnAgentTool, SubagentFactory};
pub use observer::{AgentEvent, AgentObserver, PrintObserver, SilentObserver};
pub use prompt::build_system_prompt;
