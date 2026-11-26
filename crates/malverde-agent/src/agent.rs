use std::sync::{Arc, Mutex};

use malverde_core::{Message, Role, SessionId, StopReason, ToolCall, Usage};
use malverde_memory::MemoryStore;
use malverde_provider::LlmProvider;
use malverde_tools::{PermissionManager, ToolContext, ToolRegistry};

use crate::ext_tools::SubagentFactory;
use crate::observer::{AgentEvent, AgentObserver};
use crate::prompt::build_system_prompt;

/// Configuración de una ejecución del agente.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub temperature: f32,
    pub max_tokens: u32,
    pub context_max_messages: usize,
    pub max_tool_output_chars: usize,
    pub shell_timeout_secs: u64,
    pub working_dir: std::path::PathBuf,
    pub identity_name: String,
    pub identity_prompt: String,
    pub save_traces: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 90,
            temperature: 0.7,
            max_tokens: 4096,
            context_max_messages: 120,
            max_tool_output_chars: 20_000,
            shell_timeout_secs: 120,
            working_dir: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            identity_name: "Malverde".to_string(),
            identity_prompt: String::new(),
            save_traces: true,
        }
    }
}

impl AgentConfig {
    pub fn from_config(cfg: &malverde_core::Config) -> Self {
        Self {
            max_iterations: cfg.max_iterations,
            temperature: cfg.temperature,
            max_tokens: cfg.max_tokens,
            context_max_messages: cfg.context_max_messages,
            max_tool_output_chars: cfg.max_tool_output_chars,
            shell_timeout_secs: cfg.shell_timeout_secs,
            working_dir: cfg.working_dir.clone(),
            identity_name: cfg.identity_name.clone(),
            identity_prompt: cfg.identity_prompt.clone(),
            save_traces: true,
        }
    }
}

/// Resultado de una ejecución completa del agente.
#[derive(Debug, Clone)]
pub struct AgentRunResult {
    pub final_text: String,
    pub iterations: usize,
    pub tool_calls: usize,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

/// El agente: posee el modelo, el registro de herramientas, la memoria y el presupuesto.
pub struct Agent {
    provider: Arc<dyn LlmProvider>,
    pub registry: ToolRegistry,
    pub memory: Arc<MemoryStore>,
    pub skills: malverde_skills::SkillRegistry,
    pub config: AgentConfig,
    permissions: Mutex<PermissionManager>,
    subagent_factory: Option<SubagentFactory>,
}

impl Agent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        registry: ToolRegistry,
        memory: Arc<MemoryStore>,
        skills: malverde_skills::SkillRegistry,
        config: AgentConfig,
        permissions: PermissionManager,
    ) -> Self {
        Self {
            provider,
            registry,
            memory,
            skills,
            config,
            permissions: Mutex::new(permissions),
            subagent_factory: None,
        }
    }

    /// Instala la fábrica de subagentes (usada por la herramienta spawn_agent).
    pub fn with_subagent_factory(mut self, factory: SubagentFactory) -> Self {
        self.subagent_factory = Some(factory);
        self
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Acceso al motor de permisos (para instalar aprobadores interactivos).
    pub fn permissions_handle(&self) -> &Mutex<PermissionManager> {
        &self.permissions
    }

    pub fn model(&self) -> &str {
        self.provider.model()
    }

    /// Añade herramientas de memoria y de subagentes al registro.
    pub fn install_ext_tools(&mut self) {
        let mem = Arc::clone(&self.memory);
        self.registry.register(Arc::new(crate::ext_tools::MemoryRememberTool::new(Arc::clone(&mem))));
        self.registry.register(Arc::new(crate::ext_tools::MemoryRecallTool::new(mem)));
        let mem2 = Arc::clone(&self.memory);
        self.registry.register(Arc::new(crate::ext_tools::MemoryForgetTool::new(mem2)));
        if let Some(factory) = self.subagent_factory.clone() {
            self.registry.register(Arc::new(crate::ext_tools::SpawnAgentTool::new(factory)));
        }
    }

    /// Ejecuta un turno completo: input del usuario → loop de herramientas → respuesta final.
    pub fn run(
        &self,
        session_id: &SessionId,
        user_input: &str,
        observer: &dyn AgentObserver,
    ) -> malverde_core::Result<AgentRunResult> {
        self.memory.create_session(session_id, self.provider.model())?;
        self.memory.append_message(session_id, &Message::user(user_input))?;

        let mut history = self.history(session_id)?;
        let system = build_system_prompt(
            &self.config.identity_name,
            &self.config.identity_prompt,
            &self.skills,
            &self.memory,
        )?;

        let mut iterations: usize = 0;
        let mut tool_calls_total: usize = 0;
        let mut usage_total = Usage::default();
        let mut trace: Vec<malverde_core::TraceEvent> = vec![];
        let tool_ctx = ToolContext::new(
            self.config.working_dir.clone(),
            self.config.max_tool_output_chars,
            self.config.shell_timeout_secs,
        );

        loop {
            if iterations >= self.config.max_iterations {
                let text =
                    "[presupuesto agotado] Se alcanzó max_iterations sin una respuesta final.".to_string();
                self.memory.append_message(session_id, &Message::assistant(&text))?;
                observer.on_event(&AgentEvent::RunEnd { reason: StopReason::MaxIterations });
                self.persist_trace(session_id, &trace);
                return Ok(AgentRunResult {
                    final_text: text,
                    iterations,
                    tool_calls: tool_calls_total,
                    stop_reason: StopReason::MaxIterations,
                    usage: usage_total,
                });
            }
            iterations += 1;
            observer.on_event(&AgentEvent::TurnStart { iteration: iterations });

            let messages = self.window(&system, &history);
            let completion = match self.provider.complete(
                &messages,
                &self.registry.schemas(),
                self.config.temperature,
                self.config.max_tokens,
            ) {
                Ok(c) => c,
                Err(e) => {
                    observer.on_event(&AgentEvent::RunEnd { reason: StopReason::ProviderError });
                    self.persist_trace(session_id, &trace);
                    return Err(e);
                }
            };
            usage_total.prompt_tokens += completion.usage.prompt_tokens;
            usage_total.completion_tokens += completion.usage.completion_tokens;
            usage_total.total_tokens += completion.usage.total_tokens;

            trace.push(malverde_core::TraceEvent {
                ts: chrono::Utc::now(),
                kind: "llm_response".into(),
                payload: serde_json::json!({
                    "iteration": iterations,
                    "content": completion.content,
                    "tool_calls": completion.tool_calls,
                    "usage": completion.usage,
                    "finish_reason": completion.finish_reason,
                }),
            });

            if !completion.has_tool_calls() {
                let text = completion.content.unwrap_or_default();
                history.push(Message::assistant(&text));
                self.memory.append_message(session_id, &Message::assistant(&text))?;
                observer.on_event(&AgentEvent::AssistantMessage { content: text.clone() });
                observer.on_event(&AgentEvent::RunEnd { reason: StopReason::Completed });
                self.persist_trace(session_id, &trace);
                return Ok(AgentRunResult {
                    final_text: text,
                    iterations,
                    tool_calls: tool_calls_total,
                    stop_reason: StopReason::Completed,
                    usage: usage_total,
                });
            }

            // Registrar el turno del asistente con sus llamadas y ejecutarlas en orden.
            history.push(Message::assistant_with_tool_calls(
                completion.content.clone(),
                completion.tool_calls.clone(),
            ));
            for call in &completion.tool_calls {
                tool_calls_total += 1;
                observer.on_event(&AgentEvent::ToolCallStart { call: call.clone() });
                let outcome = self.execute_tool(call, &tool_ctx);
                let output = match &outcome {
                    Ok(text) => text.clone(),
                    Err(e) => format!("[error] {e}"),
                };
                observer.on_event(&AgentEvent::ToolCallEnd {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    ok: outcome.is_ok(),
                    output: output.clone(),
                });
                trace.push(malverde_core::TraceEvent {
                    ts: chrono::Utc::now(),
                    kind: "tool_result".into(),
                    payload: serde_json::json!({
                        "id": call.id,
                        "name": call.name,
                        "ok": outcome.is_ok(),
                        "output": output,
                    }),
                });
                history.push(Message::tool_result(&call.id, &call.name, output));
            }

            // Compacta el contexto si creció demasiado (gestión de contexto por recorte).
            if history.len() > self.config.context_max_messages {
                let keep = self.config.context_max_messages / 2;
                history = history.split_off(history.len() - keep);
            }
        }
    }

    fn execute_tool(&self, call: &ToolCall, ctx: &ToolContext) -> malverde_core::Result<String> {
        let mut permissions = self.permissions.lock().unwrap();
        self.registry.execute(&call.name, &call.arguments, ctx, &mut permissions)
    }

    fn history(&self, session_id: &SessionId) -> malverde_core::Result<Vec<Message>> {
        // Los mensajes de herramienta no se recargan (requieren su turno asistente
        // emparejado); el detalle completo queda en las trazas JSONL.
        self.memory
            .session_messages(session_id, 10_000)
            .map(|msgs| msgs.into_iter().filter(|m| m.role != Role::Tool).collect())
    }

    fn window<'a>(&self, system: &str, history: &'a [Message]) -> Vec<Message> {
        let mut messages = vec![Message::system(system)];
        let start = history.len().saturating_sub(self.config.context_max_messages);
        messages.extend_from_slice(&history[start..]);
        messages
    }

    fn persist_trace(&self, session_id: &SessionId, trace: &[malverde_core::TraceEvent]) {
        if !self.config.save_traces || trace.is_empty() {
            return;
        }
        let dir = std::path::PathBuf::from(".malverde-traces");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S%3f");
        let path = dir.join(format!("run-{}-{stamp}.jsonl", session_id.0.get(..8).unwrap_or("run")));
        let mut body = String::new();
        for ev in trace {
            if let Ok(line) = serde_json::to_string(ev) {
                body.push_str(&line);
                body.push('\n');
            }
        }
        let _ = std::fs::write(path, body);
    }
}

/// Fábrica de subagentes por defecto: hijo aislado, sin spawn_agent, presupuesto corto.
pub fn default_subagent_factory(
    provider: Arc<dyn LlmProvider>,
    memory: Arc<MemoryStore>,
    skills: malverde_skills::SkillRegistry,
    base_config: AgentConfig,
) -> crate::ext_tools::SubagentFactory {
    Arc::new(move |task: &str| {
        let config = AgentConfig {
            max_iterations: 15.min(base_config.max_iterations),
            save_traces: false,
            ..base_config.clone()
        };
        let mut child = Agent::new(
            Arc::clone(&provider),
            ToolRegistry::with_builtins(),
            Arc::clone(&memory),
            skills.clone(),
            config,
            PermissionManager::new(malverde_core::PermissionMode::AutoApprove, &[]),
        );
        child.install_ext_tools();
        let sid = SessionId::from_raw(format!(
            "sub-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S%3f")
        ));
        child.run(&sid, task, &crate::observer::SilentObserver).map(|r| r.final_text)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_provider::{Completion, MockProvider, ToolSchema};

    struct SilentObserver;
    impl AgentObserver for SilentObserver {}

    fn make_agent() -> Agent {
        let provider: Arc<dyn LlmProvider> = Arc::new(MockProvider::new("mock-1"));
        let memory = Arc::new(MemoryStore::open_in_memory().unwrap());
        let registry = ToolRegistry::with_builtins();
        let skills = malverde_skills::SkillRegistry::default();
        let mut agent = Agent::new(
            provider,
            registry,
            memory,
            skills,
            AgentConfig { max_iterations: 5, save_traces: false, ..Default::default() },
            PermissionManager::new(malverde_core::PermissionMode::AutoApprove, &[]),
        );
        agent.install_ext_tools();
        agent
    }

    #[test]
    fn simple_completion_roundtrip() {
        let agent = make_agent();
        let sid = SessionId::new();
        let res = agent.run(&sid, "hola", &SilentObserver).unwrap();
        assert_eq!(res.stop_reason, StopReason::Completed);
        assert_eq!(res.iterations, 1);
        assert!(res.final_text.contains("hola"));
        let msgs = agent.memory.session_messages(&sid, 10).unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn tool_loop_completes_within_budget() {
        let agent = make_agent();
        let sid = SessionId::new();
        let res = agent
            .run(&sid, "usa la herramienta now para darme la hora", &SilentObserver)
            .unwrap();
        assert_eq!(res.stop_reason, StopReason::Completed);
        assert_eq!(res.tool_calls, 1);
        assert!(res.final_text.contains("Resultado recibido"));
    }

    #[test]
    fn budget_exhaustion_stops_cleanly() {
        let provider: Arc<dyn LlmProvider> = Arc::new(AlwaysToolMock);
        let memory = Arc::new(MemoryStore::open_in_memory().unwrap());
        let mut agent = Agent::new(
            provider,
            ToolRegistry::with_builtins(),
            memory,
            malverde_skills::SkillRegistry::default(),
            AgentConfig { max_iterations: 3, save_traces: false, ..Default::default() },
            PermissionManager::new(malverde_core::PermissionMode::AutoApprove, &[]),
        );
        agent.install_ext_tools();
        let res = agent.run(&SessionId::new(), "loop infinito", &SilentObserver).unwrap();
        assert_eq!(res.stop_reason, StopReason::MaxIterations);
        assert_eq!(res.iterations, 3);
    }

    /// Mock que siempre pide la herramienta now (para probar el presupuesto).
    struct AlwaysToolMock;
    impl LlmProvider for AlwaysToolMock {
        fn name(&self) -> &str {
            "always-tool"
        }
        fn model(&self) -> &str {
            "mock"
        }
        fn complete(
            &self,
            _messages: &[Message],
            _tools: &[ToolSchema],
            _temperature: f32,
            _max_tokens: u32,
        ) -> malverde_core::Result<Completion> {
            Ok(Completion {
                content: None,
                tool_calls: vec![ToolCall { id: "x".into(), name: "now".into(), arguments: serde_json::json!({}) }],
                usage: Usage::default(),
                finish_reason: "tool_calls".into(),
            })
        }
    }
}
