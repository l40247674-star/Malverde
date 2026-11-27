//! Malverde Tools
//!
//! Registro de herramientas con motor de permisos y herramientas integradas:
//! shell, lectura/escritura/edición de archivos, listado, búsqueda y fetch HTTP.

pub mod builtin;
pub mod permissions;
pub mod registry;

pub use builtin::{
    EditFileTool, FetchTool, ListDirTool, NowTool, ReadFileTool, SearchTool, ShellTool, Tool, ToolContext,
    WriteFileTool,
};
pub use permissions::{ApprovalAnswer, Decision, PermissionManager};
pub use registry::ToolRegistry;
