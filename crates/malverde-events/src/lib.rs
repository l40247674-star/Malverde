//! # Malverde Event System
//!
//! Provides event-driven architecture for the Malverde Core Framework.
//! Includes event types, event bus, publishers, subscribers, and correlation.

pub mod event;
pub mod bus;
pub mod correlation;

pub use event::*;
pub use bus::*;
pub use correlation::*;
