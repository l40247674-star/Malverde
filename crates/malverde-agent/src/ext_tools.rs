use std::sync::Arc;

use malverde_core::Result;
use malverde_memory::MemoryStore;
use malverde_tools::{Tool, ToolContext};

/// Fábrica de subagentes: recibe una tarea y devuelve el resumen final del hijo.
pub type SubagentFactory = Arc<dyn Fn(&str) -> Result<String> + Send + Sync>;

// ---------------------------------------------------------------------------
// remember / recall / forget
// ---------------------------------------------------------------------------

pub struct MemoryRememberTool {
    memory: Arc<MemoryStore>,
}

impl MemoryRememberTool {
    pub fn new(memory: Arc<MemoryStore>) -> Self {
        Self { memory }
    }
}

impl Tool for MemoryRememberTool {
    fn name(&self) -> &str {
        "remember"
    }
    fn description(&self) -> &str {
        "Guarda un hecho permanente en la memoria del agente (visible en futuras sesiones)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "fact": { "type": "string", "description": "Hecho a recordar" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Etiquetas opcionales" }
            },
            "required": ["fact"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let fact = malverde_tools::builtin::get_str(args, "fact")?;
        let tags: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let saved = self.memory.remember(fact, &tags)?;
        Ok(format!("recordado con id {}", saved.id))
    }
}

pub struct MemoryRecallTool {
    memory: Arc<MemoryStore>,
}

impl MemoryRecallTool {
    pub fn new(memory: Arc<MemoryStore>) -> Self {
        Self { memory }
    }
}

impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "recall"
    }
    fn description(&self) -> &str {
        "Busca en la memoria persistente (hechos) y en el historial de sesiones anteriores."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Consulta de búsqueda" },
                "limit": { "type": "integer", "description": "Máximo de resultados (por defecto 10)" }
            },
            "required": ["query"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let query = malverde_tools::builtin::get_str(args, "query")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let mut out = String::from("## Hechos\n");
        for f in self.memory.search_facts(query, limit)? {
            out.push_str(&format!("- [{}] {}\n", f.id, f.fact));
        }
        out.push_str("\n## Historial de sesiones\n");
        for (session, role, content) in self.memory.search_messages(query, limit)? {
            let snippet: String = content.chars().take(200).collect();
            out.push_str(&format!("- [{session}] {role}: {snippet}\n"));
        }
        Ok(out)
    }
}

/// Herramienta para olvidar un hecho por id.
pub struct MemoryForgetTool {
    memory: Arc<MemoryStore>,
}

impl MemoryForgetTool {
    pub fn new(memory: Arc<MemoryStore>) -> Self {
        Self { memory }
    }
}

impl Tool for MemoryForgetTool {
    fn name(&self) -> &str {
        "forget"
    }
    fn description(&self) -> &str {
        "Borra un hecho de la memoria persistente por su id."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Id del hecho a borrar" }
            },
            "required": ["id"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let id = malverde_tools::builtin::get_str(args, "id")?;
        if self.memory.forget(id)? {
            Ok(format!("hecho {id} borrado"))
        } else {
            Ok(format!("no existe ningún hecho con id {id}"))
        }
    }
}

// ---------------------------------------------------------------------------
// spawn_agent
// ---------------------------------------------------------------------------

/// Herramienta que lanza un subagente aislado y devuelve solo su resumen final.
pub struct SpawnAgentTool {
    factory: SubagentFactory,
}

impl SpawnAgentTool {
    pub fn new(factory: SubagentFactory) -> Self {
        Self { factory }
    }
}

impl Tool for SpawnAgentTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }
    fn description(&self) -> &str {
        "Lanza un subagente aislado con su propio contexto y herramientas para trabajar en paralelo. \
         Solo su resumen final entra en esta conversación."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "Tarea autónoma para el subagente" }
            },
            "required": ["task"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let task = malverde_tools::builtin::get_str(args, "task")?;
        (self.factory)(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::SessionId;
    use std::path::PathBuf;

    #[test]
    fn remember_recall_forget_flow() {
        let memory = Arc::new(MemoryStore::open_in_memory().unwrap());
        let remember = MemoryRememberTool::new(Arc::clone(&memory));
        let recall = MemoryRecallTool::new(Arc::clone(&memory));
        let forget = MemoryForgetTool::new(memory);
        let ctx = ToolContext::new(PathBuf::from("."), 1_000, 5);

        let out = remember
            .execute(&serde_json::json!({"fact": "Malverde es un harness", "tags": ["meta"]}), &ctx)
            .unwrap();
        assert!(out.contains("recordado"));

        let out = recall.execute(&serde_json::json!({"query": "Malverde"}), &ctx).unwrap();
        assert!(out.contains("harness"));

        let id = out
            .lines()
            .find(|l| l.starts_with("- ["))
            .and_then(|l| l.split(']').next())
            .map(|s| s.trim_start_matches("- [").to_string())
            .unwrap();
        let out = forget.execute(&serde_json::json!({"id": id}), &ctx).unwrap();
        assert!(out.contains("borrado"));
    }

    #[test]
    fn spawn_agent_delegates() {
        let tool = SpawnAgentTool::new(Arc::new(|task: &str| Ok(format!("subagente completó: {task}"))));
        let ctx = ToolContext::new(PathBuf::from("."), 1_000, 5);
        let out = tool
            .execute(&serde_json::json!({"task": "revisar el código"}), &ctx)
            .unwrap();
        assert!(out.contains("revisar el código"));
        let _ = SessionId::new();
    }
}
