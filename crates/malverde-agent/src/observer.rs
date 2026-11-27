use malverde_core::{StopReason, ToolCall};

/// Eventos observables durante una ejecución del agente.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    TurnStart { iteration: usize },
    AssistantMessage { content: String },
    ToolCallStart { call: ToolCall },
    ToolCallEnd { id: String, name: String, ok: bool, output: String },
    RunEnd { reason: StopReason },
}

/// Observador del loop agéntico: la CLI/TUI lo usa para pintar el progreso.
pub trait AgentObserver: Send + Sync {
    fn on_event(&self, event: &AgentEvent) {
        let _ = event;
    }
}

/// Observador por defecto que imprime el progreso en la terminal.
pub struct PrintObserver {
    pub verbose: bool,
}

/// Observador silencioso (tests, cron, gateway).
pub struct SilentObserver;

impl AgentObserver for SilentObserver {}

impl AgentObserver for PrintObserver {
    fn on_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::ToolCallStart { call } => {
                println!("→ {} {}", call.name, call.arguments.to_string());
            }
            AgentEvent::ToolCallEnd { name, ok, output, .. } => {
                let mark = if *ok { "✓" } else { "✗" };
                if self.verbose {
                    println!("{mark} {name}: {output}");
                } else {
                    println!("{mark} {name}");
                }
            }
            AgentEvent::RunEnd { reason } => {
                if *reason != malverde_core::StopReason::Completed {
                    println!("[fin: {reason:?}]");
                }
            }
            _ => {}
        }
    }
}
