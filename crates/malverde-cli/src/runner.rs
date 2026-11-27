use std::io::Write;
use std::sync::Arc;

use malverde_agent::{default_subagent_factory, Agent, AgentConfig, AgentObserver};
use malverde_core::{Config, MalverdeError, Result, SessionId};
use malverde_memory::MemoryStore;
use malverde_provider::{LlmProvider, MockProvider, OpenAiCompatibleProvider};
use malverde_tools::{ApprovalAnswer, PermissionManager, ToolRegistry};

/// Construye el agente completo a partir de la configuración.
pub fn build_agent(config: &Config) -> Result<Arc<Agent>> {
    let provider = build_provider(config)?;
    let memory = Arc::new(MemoryStore::open(config.db_path())?);
    let skills = malverde_skills::SkillRegistry::load(&config.skills_dirs);
    let mut registry = ToolRegistry::with_builtins();
    for tool in &config.disabled_tools {
        registry.disable(tool);
    }
    let agent_config = AgentConfig::from_config(config);
    let mut agent = Agent::new(
        provider.clone(),
        registry,
        Arc::clone(&memory),
        skills.clone(),
        agent_config,
        PermissionManager::from_config(config),
    );
    let factory = default_subagent_factory(provider, memory, skills, AgentConfig::from_config(config));
    agent = agent.with_subagent_factory(factory);
    agent.install_ext_tools();
    Ok(Arc::new(agent))
}

/// Habilita el flujo de aprobación humana (chat interactivo).
pub fn enable_interactive_approvals(agent: &Arc<Agent>) {
    let mut pm = agent.permissions_handle().lock().unwrap();
    pm.set_approver(Box::new(|tool, reason| {
        print!("\n[permiso] {reason}\n¿Permitir '{tool}'? [s]í / [t]oda la sesión / [n]o: ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return ApprovalAnswer::No;
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "s" | "si" | "sí" | "y" | "yes" => ApprovalAnswer::Yes,
            "t" | "todo" | "a" | "always" => ApprovalAnswer::YesAlways,
            _ => ApprovalAnswer::No,
        }
    }));
}

pub fn build_provider(config: &Config) -> Result<Arc<dyn LlmProvider>> {
    if config.provider == "mock" {
        return Ok(Arc::new(MockProvider::new(&config.model)));
    }
    match config.api_key() {
        Some(key) => Ok(Arc::new(OpenAiCompatibleProvider::new(
            config.base_url.clone(),
            Some(key),
            config.model.clone(),
        ))),
        None if config.api_key_env.is_empty() => Ok(Arc::new(OpenAiCompatibleProvider::new(
            config.base_url.clone(),
            None,
            config.model.clone(),
        ))),
        None => Err(MalverdeError::config(format!(
            "exporta {} antes de usar el proveedor '{}', o usa `malverde model set mock` para probar sin red",
            config.api_key_env, config.provider
        ))),
    }
}

/// REPL de chat interactivo con comandos slash.
pub fn chat(agent: &Arc<Agent>, session: SessionId, verbose: bool) -> Result<()> {
    let observer = malverde_agent::PrintObserver { verbose };
    println!("Malverde chat · modelo {} · sesión {}", agent.model(), session);
    println!("Comandos: /help /new /tools /memory <consulta> /exit\n");
    let mut session = session;
    loop {
        print!("tú> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if let Some(cmd) = input.strip_prefix('/') {
            let mut parts = cmd.splitn(2, ' ');
            let (cmd, rest) = (parts.next().unwrap_or(""), parts.next().unwrap_or("").trim());
            match cmd {
                "exit" | "quit" | "q" => break,
                "help" => println!("/new  /tools  /memory <consulta>  /exit"),
                "new" => {
                    session = SessionId::new();
                    println!("[nueva sesión {}]", session);
                }
                "tools" => {
                    for name in agent.registry.enabled_names() {
                        println!("  • {name}");
                    }
                }
                "memory" => {
                    let hits = agent.memory.search_facts(rest, 10)?;
                    if hits.is_empty() {
                        println!("(sin resultados)");
                    }
                    for h in hits {
                        println!("  [{}] {}", h.id, h.fact);
                    }
                }
                other => println!("comando desconocido /{other} (prueba /help)"),
            }
            continue;
        }
        match agent.run(&session, input, &observer) {
            Ok(res) => println!("\nmalverde> {}\n", res.final_text),
            Err(e) => println!("\n[error] {e}\n"),
        }
    }
    Ok(())
}

/// Etiqueta legible de un modo de permisos.
pub fn permission_mode_label(mode: malverde_core::PermissionMode) -> &'static str {
    match mode {
        malverde_core::PermissionMode::AutoApprove => "auto_approve",
        malverde_core::PermissionMode::Ask => "ask",
        malverde_core::PermissionMode::AllowList => "allow_list",
        malverde_core::PermissionMode::ReadOnly => "read_only",
    }
}

/// Convierte un valor de texto al tipo de la clave actual en la configuración.
pub fn coerce_value(current: Option<&serde_json::Value>, raw: &str) -> Result<serde_json::Value> {
    let as_number = raw.parse::<f64>().ok();
    let as_bool = matches!(raw, "true" | "false").then(|| raw == "true");
    Ok(match current {
        Some(serde_json::Value::Number(_)) => {
            as_number.map(|n| serde_json::json!(n)).unwrap_or_else(|| serde_json::json!(raw))
        }
        Some(serde_json::Value::Bool(_)) => {
            as_bool.map(|b| serde_json::json!(b)).unwrap_or_else(|| serde_json::json!(raw))
        }
        Some(serde_json::Value::Array(_)) => {
            serde_json::json!(raw.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
        }
        _ => serde_json::json!(raw),
    })
}

/// Ejecución one-shot de una tarea.
pub fn run_once(agent: &Arc<Agent>, session: &SessionId, task: &str, verbose: bool, as_json: bool) -> Result<()> {
    let observer: &dyn AgentObserver = &malverde_agent::PrintObserver { verbose };
    let result = agent.run(session, task, observer)?;
    if as_json {
        let json = serde_json::json!({
            "session": session.0,
            "final_text": result.final_text,
            "iterations": result.iterations,
            "tool_calls": result.tool_calls,
            "stop_reason": result.stop_reason,
            "usage": result.usage,
        });
        println!("{}", serde_json::to_string_pretty(&json).map_err(|e| MalverdeError::serialization(e.to_string()))?);
    } else {
        println!("{}", result.final_text);
        println!(
            "\n[iteraciones: {} · llamadas a herramientas: {} · tokens: {}]",
            result.iterations, result.tool_calls, result.usage.total_tokens
        );
    }
    Ok(())
}
