use clap::{Parser, Subcommand};
use malverde_core::{Config, MalverdeError, PermissionMode, Result, SessionId, PROVIDER_PRESETS};
use malverde_scheduler::{CronStore, Scheduler};
use std::io::Write;

mod doctor;
mod runner;

#[derive(Debug, Parser)]
#[command(name = "malverde")]
#[command(version = "0.1.0")]
#[command(about = "Malverde: harness agéntico local con herramientas, memoria, skills, cron y gateway")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Chat interactivo con el agente
    Chat {
        #[arg(long)] session: Option<String>,
        #[arg(long)] model: Option<String>,
        #[arg(long, short)] verbose: bool,
    },
    /// Ejecuta una tarea puntual y termina
    Run {
        task: String,
        #[arg(long)] session: Option<String>,
        #[arg(long)] model: Option<String>,
        #[arg(long)] json: bool,
        #[arg(long, short)] verbose: bool,
    },
    /// Gestión del modelo/proveedor
    Model {
        #[command(subcommand)]
        cmd: ModelCommands,
    },
    /// Gestión de herramientas
    Tools {
        #[command(subcommand)]
        cmd: ToolsCommands,
    },
    /// Configuración persistente
    Config {
        #[command(subcommand)]
        cmd: ConfigCommands,
    },
    /// Asistente de configuración inicial
    Setup,
    /// Diagnóstico del entorno
    Doctor,
    /// Gestión de sesiones
    Sessions {
        #[command(subcommand)]
        cmd: SessionsCommands,
    },
    /// Memoria persistente del agente
    Memory {
        #[command(subcommand)]
        cmd: MemoryCommands,
    },
    /// Sistema de skills
    Skills {
        #[command(subcommand)]
        cmd: SkillsCommands,
    },
    /// Tareas programadas (cron)
    Cron {
        #[command(subcommand)]
        cmd: CronCommands,
    },
    /// Gateway de mensajería (webhook en loopback)
    Gateway {
        #[command(subcommand)]
        cmd: GatewayCommands,
    },
    /// Interfaz de terminal (TUI)
    Tui {
        #[arg(long)] session: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ModelCommands {
    /// Lista proveedores preconfigurados y el modelo actual
    List,
    /// Configura proveedor y modelo: `model set <proveedor|url> [modelo]`
    Set { provider: String, model: Option<String> },
}

#[derive(Debug, Subcommand)]
pub enum ToolsCommands {
    /// Lista herramientas y su estado
    List,
    /// Habilita una herramienta
    Enable { name: String },
    /// Deshabilita una herramienta
    Disable { name: String },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Muestra toda la configuración
    List,
    /// Lee una clave
    Get { key: String },
    /// Escribe una clave
    Set { key: String, value: String },
    /// Ruta del archivo de configuración
    Path,
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommands {
    List,
    Show { id: String },
    Search { query: String },
    Delete { id: String },
}

#[derive(Debug, Subcommand)]
pub enum MemoryCommands {
    Remember { fact: String },
    List,
    Search { query: String },
    Forget { id: String },
}

#[derive(Debug, Subcommand)]
pub enum SkillsCommands {
    List,
    Create { name: String, description: String },
    Path,
}

#[derive(Debug, Subcommand)]
pub enum CronCommands {
    /// Añade un trabajo: `cron add <nombre> "<cron>" --prompt "<prompt>"`
    Add {
        name: String,
        cron: String,
        #[arg(long)] prompt: String,
    },
    List,
    Remove { id: String },
    Enable { id: String },
    Disable { id: String },
    /// Ejecuta el daemon del planificador
    Run,
}

#[derive(Debug, Subcommand)]
pub enum GatewayCommands {
    /// Sirve el webhook del agente SOLO en 127.0.0.1
    Serve {
        #[arg(long, default_value = "8787")] port: u16,
    },
    Status,
}

fn main() {
    malverde_core::init_logging();
    let cli = Cli::parse();
    let code = match dispatch(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[error] {e}");
            1
        }
    };
    std::process::exit(code);
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Doctor => doctor::run(),
        Commands::Setup => setup_wizard(),
        Commands::Config { cmd } => config_cmd(cmd),
        Commands::Model { cmd } => model_cmd(cmd),
        Commands::Tools { cmd } => tools_cmd(cmd),
        Commands::Sessions { cmd } => sessions_cmd(cmd),
        Commands::Memory { cmd } => memory_cmd(cmd),
        Commands::Skills { cmd } => skills_cmd(cmd),
        Commands::Cron { cmd } => cron_cmd(cmd),
        Commands::Gateway { cmd } => gateway_cmd(cmd),
        Commands::Chat { session, model, verbose } => {
            let mut config = load_config()?;
            if let Some(m) = model {
                config.model = m;
            }
            let agent = runner::build_agent(&config)?;
            runner::enable_interactive_approvals(&agent);
            let session = session.map(SessionId::from_raw).unwrap_or_default();
            runner::chat(&agent, session, verbose)
        }
        Commands::Run { task, session, model, json, verbose } => {
            let mut config = load_config()?;
            if let Some(m) = model {
                config.model = m;
            }
            let agent = runner::build_agent(&config)?;
            let session = session.map(SessionId::from_raw).unwrap_or_default();
            runner::run_once(&agent, &session, &task, verbose, json)
        }
        Commands::Tui { session } => {
            let config = load_config()?;
            let agent = runner::build_agent(&config)?;
            let session = session.map(SessionId::from_raw).unwrap_or_default();
            malverde_tui::ChatTui::new(agent, session).run().map_err(MalverdeError::io)
        }
    }
}

pub fn load_config() -> Result<Config> {
    let cfg = Config::load()?;
    std::fs::create_dir_all(&cfg.data_dir).map_err(MalverdeError::io)?;
    Ok(cfg)
}

fn setup_wizard() -> Result<()> {
    let mut config = load_config()?;
    println!("Asistente de configuración de Malverde\n");
    println!("Proveedores disponibles:");
    for (i, (name, url, env)) in PROVIDER_PRESETS.iter().enumerate() {
        println!("  {i}. {name:<10} {url}  (env: {env})");
    }
    println!("  {}. mock (sin red, para pruebas)", PROVIDER_PRESETS.len());
    print!("\nElige proveedor [0-{}]: ", PROVIDER_PRESETS.len());
    std::io::stdout().flush().ok();

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(MalverdeError::io)?;
    let idx: usize = line.trim().parse().unwrap_or(0);
    if idx == PROVIDER_PRESETS.len() {
        config.provider = "mock".into();
        config.api_key_env = String::new();
        config.model = "mock-1".into();
    } else if let Some((name, url, env)) = PROVIDER_PRESETS.get(idx) {
        config.provider = name.to_string();
        config.base_url = url.to_string();
        config.api_key_env = env.to_string();
    } else {
        return Err(MalverdeError::config("opción inválida"));
    }

    print!("Modelo [{}]: ", config.model);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(MalverdeError::io)?;
    let model = line.trim();
    if !model.is_empty() {
        config.model = model.to_string();
    }

    print!(
        "Modo de permisos [auto_approve|ask|allow_list|read_only] ({}): ",
        runner::permission_mode_label(config.permission_mode)
    );
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(MalverdeError::io)?;
    let mode = line.trim();
    if !mode.is_empty() {
        config.permission_mode = PermissionMode::parse(mode)?;
    }

    config.save()?;
    println!("\nConfiguración guardada en {}", Config::default_path().display());
    println!("Prueba con: malverde run \"dime la hora con la herramienta now\" --json");
    Ok(())
}

fn config_cmd(cmd: ConfigCommands) -> Result<()> {
    let mut config = load_config()?;
    match cmd {
        ConfigCommands::Path => println!("{}", Config::default_path().display()),
        ConfigCommands::List => {
            let json = serde_json::to_string_pretty(&config).map_err(|e| MalverdeError::serialization(e.to_string()))?;
            println!("{json}");
        }
        ConfigCommands::Get { key } => {
            let value = config_to_value(&config)?
                .get(&key)
                .cloned()
                .ok_or_else(|| MalverdeError::config(format!("clave '{key}' inexistente")))?;
            println!("{value}");
        }
        ConfigCommands::Set { key, value } => {
            let mut v = config_to_value(&config)?;
            let coerced = runner::coerce_value(v.get(&key), &value)?;
            v[&key] = coerced;
            config = serde_json::from_value(v).map_err(|e| MalverdeError::serialization(e.to_string()))?;
            config.save()?;
            println!("{} = {}", key, value);
        }
    }
    Ok(())
}

fn config_to_value(config: &Config) -> Result<serde_json::Value> {
    serde_json::to_value(config).map_err(|e| MalverdeError::serialization(e.to_string()))
}

fn model_cmd(cmd: ModelCommands) -> Result<()> {
    let mut config = load_config()?;
    match cmd {
        ModelCommands::List => {
            println!("modelo actual: {} (proveedor: {})", config.model, config.provider);
            println!("\nproveedores preconfigurados:");
            for (name, url, env) in PROVIDER_PRESETS {
                let env_note = if env.is_empty() { "sin API key" } else { env };
                println!("  {name:<10} {url}  [{env_note}]");
            }
            println!("  mock       (sin red, determinista)");
            println!("\nuso: malverde model set <proveedor|url> [modelo]");
        }
        ModelCommands::Set { provider, model } => {
            if provider == "mock" {
                config.provider = "mock".into();
                config.api_key_env = String::new();
            } else {
                config.set_provider(&provider)?;
            }
            if let Some(m) = model {
                config.model = m;
            }
            config.save()?;
            println!("proveedor: {} · modelo: {} · base_url: {}", config.provider, config.model, config.base_url);
        }
    }
    Ok(())
}

fn tools_cmd(cmd: ToolsCommands) -> Result<()> {
    let mut config = load_config()?;
    match cmd {
        ToolsCommands::List => {
            let registry = malverde_tools::ToolRegistry::with_builtins();
            println!("{:<12} estado", "herramienta");
            for name in registry.all_names() {
                let disabled = config.disabled_tools.iter().any(|d| d == &name);
                println!("{:<12} {}", name, if disabled { "off" } else { "on" });
            }
        }
        ToolsCommands::Enable { name } => {
            config.disabled_tools.retain(|d| d != &name);
            config.save()?;
            println!("herramienta '{name}' habilitada");
        }
        ToolsCommands::Disable { name } => {
            if !config.disabled_tools.contains(&name) {
                config.disabled_tools.push(name.clone());
                config.save()?;
            }
            println!("herramienta '{name}' deshabilitada");
        }
    }
    Ok(())
}

fn sessions_cmd(cmd: SessionsCommands) -> Result<()> {
    let config = load_config()?;
    let store = malverde_memory::MemoryStore::open(config.db_path())?;
    match cmd {
        SessionsCommands::List => {
            for s in store.list_sessions(50)? {
                println!(
                    "{}  {}  [{}] {}",
                    s.id.0.get(..8).unwrap_or(&s.id.0),
                    s.updated_at.format("%Y-%m-%d %H:%M"),
                    s.message_count,
                    s.title
                );
            }
        }
        SessionsCommands::Show { id } => {
            let sid = resolve_session(&store, &id)?;
            for m in store.session_messages(&sid, 500)? {
                let content = m.content.unwrap_or_default();
                let snippet: String = content.chars().take(400).collect();
                println!("[{}] {}", m.role.as_str(), snippet);
            }
        }
        SessionsCommands::Search { query } => {
            for (session, role, content) in store.search_messages(&query, 25)? {
                let snippet: String = content.chars().take(200).collect();
                println!("[{session}] {role}: {snippet}");
            }
        }
        SessionsCommands::Delete { id } => {
            let sid = resolve_session(&store, &id)?;
            if store.delete_session(&sid)? {
                println!("sesión {id} borrada");
            } else {
                println!("sesión {id} no encontrada");
            }
        }
    }
    Ok(())
}

fn resolve_session(store: &malverde_memory::MemoryStore, id: &str) -> Result<SessionId> {
    let sessions = store.list_sessions(200)?;
    if let Some(full) = sessions.iter().find(|s| s.id.0 == id) {
        return Ok(full.id.clone());
    }
    if let Some(full) = sessions.iter().find(|s| s.id.0.starts_with(id)) {
        return Ok(full.id.clone());
    }
    Ok(SessionId::from_raw(id))
}

fn memory_cmd(cmd: MemoryCommands) -> Result<()> {
    let config = load_config()?;
    let store = malverde_memory::MemoryStore::open(config.db_path())?;
    match cmd {
        MemoryCommands::Remember { fact } => {
            let f = store.remember(&fact, &[])?;
            println!("recordado [{}]: {}", f.id, f.fact);
        }
        MemoryCommands::List => {
            for f in store.list_facts(100)? {
                println!("[{}] {}", f.id, f.fact);
            }
        }
        MemoryCommands::Search { query } => {
            for f in store.search_facts(&query, 25)? {
                println!("[{}] {}", f.id, f.fact);
            }
        }
        MemoryCommands::Forget { id } => {
            println!("{}", if store.forget(&id)? { "borrado" } else { "no encontrado" });
        }
    }
    Ok(())
}

fn skills_cmd(cmd: SkillsCommands) -> Result<()> {
    let config = load_config()?;
    match cmd {
        SkillsCommands::List => {
            let reg = malverde_skills::SkillRegistry::load(&config.skills_dirs);
            if reg.skills.is_empty() {
                println!("(sin skills; crea una con `malverde skills create <nombre> <descripción>`)");
            }
            for s in reg.skills {
                println!("{:<16} {}", s.name, s.description);
            }
        }
        SkillsCommands::Create { name, description } => {
            let base = config.data_dir.join("skills");
            let path = malverde_skills::SkillRegistry::create(
                &base,
                &name,
                &description,
                "Escribe aquí las instrucciones de la skill.",
            )?;
            println!("skill creada en {}", path.display());
        }
        SkillsCommands::Path => {
            for d in &config.skills_dirs {
                println!("{}", d.display());
            }
        }
    }
    Ok(())
}

fn cron_cmd(cmd: CronCommands) -> Result<()> {
    let config = load_config()?;
    let store = CronStore::open(config.cron_path());
    match cmd {
        CronCommands::Add { name, cron, prompt } => {
            let job = store.add(&name, &cron, &prompt, "cli")?;
            println!("trabajo '{}' añadido con id {}", job.name, job.id);
        }
        CronCommands::List => {
            for j in store.list()? {
                let next = j.next_run()?.map(|d| d.to_rfc3339()).unwrap_or_else(|| "-".into());
                println!(
                    "[{}] {:<14} {:<14} {:<7} próximo: {}",
                    j.id,
                    j.name,
                    j.cron,
                    if j.enabled { "on" } else { "off" },
                    next
                );
            }
        }
        CronCommands::Remove { id } => {
            println!("{}", if store.remove(&id)? { "trabajo borrado" } else { "no encontrado" });
        }
        CronCommands::Enable { id } => {
            println!("{}", if store.set_enabled(&id, true)? { "habilitado" } else { "no encontrado" });
        }
        CronCommands::Disable { id } => {
            println!("{}", if store.set_enabled(&id, false)? { "deshabilitado" } else { "no encontrado" });
        }
        CronCommands::Run => {
            let agent = runner::build_agent(&config)?;
            let scheduler = Scheduler::new(
                store,
                agent,
                std::sync::Arc::new(malverde_agent::PrintObserver { verbose: false }),
            );
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| MalverdeError::scheduler(e.to_string()))?;
            println!("daemon cron iniciado (Ctrl+C para salir)");
            return runtime.block_on(scheduler.run_forever());
        }
    }
    Ok(())
}

fn gateway_cmd(cmd: GatewayCommands) -> Result<()> {
    let config = load_config()?;
    match cmd {
        GatewayCommands::Serve { port } => {
            let agent = runner::build_agent(&config)?;
            let webhook = malverde_gateway::LoopbackWebhook::new(
                port,
                agent,
                std::sync::Arc::new(malverde_agent::SilentObserver),
            );
            println!("gateway webhook en http://127.0.0.1:{} (solo loopback)", webhook.port());
            webhook.serve()
        }
        GatewayCommands::Status => {
            println!("gateway: webhook loopback disponible con `malverde gateway serve --port 8787`");
            println!("  POST /message {{\"chat_id\": \"…\", \"text\": \"…\"}}");
            Ok(())
        }
    }
}
