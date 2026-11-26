//! Malverde Gateway
//!
//! Conecta plataformas de mensajería con el agente. Incluye un adaptador de
//! webhook que SOLO escucha en loopback (127.0.0.1) por seguridad.

pub mod platform;
pub mod webhook;

pub use platform::{CliPlatform, InboundMessage, Platform};
pub use webhook::LoopbackWebhook;
