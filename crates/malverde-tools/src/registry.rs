use std::collections::HashMap;
use std::sync::Arc;

use malverde_core::Result;
use malverde_provider::ToolSchema;

use crate::builtin::{EditFileTool, FetchTool, ListDirTool, NowTool, ReadFileTool, SearchTool, ShellTool, Tool, ToolContext, WriteFileTool};
use crate::permissions::{Decision, PermissionManager};

/// Registro central de herramientas del harness.
///
/// Separa **registro** (qué existe) de **exposición** (qué se le enseña al modelo),
/// igual que hace Hermes: puedes deshabilitar herramientas sin borrarlas.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    disabled: Vec<String>,
    order: Vec<String>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new(), disabled: vec![], order: vec![] }
    }

    /// Registro con las herramientas integradas de fábrica.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(ShellTool));
        reg.register(Arc::new(ReadFileTool));
        reg.register(Arc::new(WriteFileTool));
        reg.register(Arc::new(EditFileTool));
        reg.register(Arc::new(ListDirTool));
        reg.register(Arc::new(SearchTool));
        reg.register(Arc::new(FetchTool));
        reg.register(Arc::new(NowTool));
        reg
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        if !self.order.contains(&tool.name().to_string()) {
            self.order.push(tool.name().to_string());
        }
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub fn disable(&mut self, name: &str) -> bool {
        if self.tools.contains_key(name) && !self.disabled.iter().any(|d| d == name) {
            self.disabled.push(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn enable(&mut self, name: &str) -> bool {
        let before = self.disabled.len();
        self.disabled.retain(|d| d != name);
        self.disabled.len() != before
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.tools.contains_key(name) && !self.disabled.iter().any(|d| d == name)
    }

    pub fn enabled_names(&self) -> Vec<String> {
        self.order.iter().filter(|n| self.is_enabled(n)).cloned().collect()
    }

    pub fn all_names(&self) -> Vec<String> {
        self.order.clone()
    }

    /// Schemas listos para enviar al modelo (solo herramientas habilitadas).
    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.enabled_names()
            .iter()
            .filter_map(|n| self.tools.get(n))
            .map(|t| ToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }

    /// Ejecuta una herramienta pasando por el motor de permisos.
    pub fn execute(
        &self,
        name: &str,
        args: &serde_json::Value,
        ctx: &ToolContext,
        permissions: &mut PermissionManager,
    ) -> Result<String> {
        let tool = self
            .get(name)
            .ok_or_else(|| malverde_core::MalverdeError::tool(format!("herramienta desconocida '{name}'")))?;
        if !self.is_enabled(name) {
            return Err(malverde_core::MalverdeError::tool(format!("herramienta '{name}' está deshabilitada")));
        }
        match permissions.evaluate(name, tool.sensitive()) {
            Decision::Allowed => tool.execute(args, ctx),
            Decision::Denied { reason } => Err(malverde_core::MalverdeError::Permission(reason)),
            Decision::NeedsApproval { reason } => {
                if permissions.request_approval(name, &reason) {
                    tool.execute(args, ctx)
                } else {
                    Err(malverde_core::MalverdeError::Permission(format!("{reason} (denegado por el usuario)")))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::PermissionMode;

    #[test]
    fn builtin_registry_lists_tools() {
        let reg = ToolRegistry::with_builtins();
        assert!(reg.contains("shell"));
        assert!(reg.contains("now"));
        assert_eq!(reg.enabled_names().len(), 8);
    }

    #[test]
    fn disable_and_enable() {
        let mut reg = ToolRegistry::with_builtins();
        assert!(reg.disable("fetch"));
        assert!(!reg.is_enabled("fetch"));
        let schemas = reg.schemas();
        assert!(schemas.iter().all(|s| s.name != "fetch"));
        assert!(reg.enable("fetch"));
        assert!(reg.is_enabled("fetch"));
    }

    #[test]
    fn execute_respects_read_only() {
        let reg = ToolRegistry::with_builtins();
        let mut pm = PermissionManager::new(PermissionMode::ReadOnly, &[]);
        let ctx = ToolContext::new(std::env::temp_dir(), 1_000, 5);
        let out = reg.execute("now", &serde_json::json!({}), &ctx, &mut pm).unwrap();
        assert!(out.contains('T'));
        assert!(reg.execute("shell", &serde_json::json!({"command": "ls"}), &ctx, &mut pm).is_err());
    }
}
