//! # Malverde Job System
//!
//! Provides job management capabilities for the Malverde Core Framework.
//! Handles job execution, state management, progress tracking, and checkpointing.

pub mod error;
pub mod job;
pub mod context;
pub mod manager;

pub use error::*;
pub use job::*;
pub use context::*;
pub use manager::*;
