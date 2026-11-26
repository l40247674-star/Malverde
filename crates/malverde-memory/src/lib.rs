//! Malverde Memory
//!
//! Memoria persistente del harness: sesiones, mensajes y hechos curados por el
//! agente, con búsqueda de texto completo (FTS5) para recall cross-sesión.

mod store;

pub use store::MemoryStore;
