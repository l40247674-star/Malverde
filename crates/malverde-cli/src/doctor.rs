use malverde_core::{Config, MalverdeError, Result};

pub fn run() -> Result<()> {
    println!("Malverde doctor\n");
    let mut failures = 0usize;

    let config = match Config::load() {
        Ok(c) => {
            println!("[OK]   configuración cargada de {}", Config::default_path().display());
            c
        }
        Err(e) => {
            println!("[FALLO] configuración: {e}");
            return Err(e);
        }
    };

    let dir = &config.data_dir;
    if dir.exists() && test_write(dir) {
        println!("[OK]   data_dir escribible: {}", dir.display());
    } else if std::fs::create_dir_all(dir).is_ok() && test_write(dir) {
        println!("[OK]   data_dir creado: {}", dir.display());
    } else {
        failures += 1;
        println!("[FALLO] data_dir no escribible: {}", dir.display());
    }

    match malverde_memory::MemoryStore::open(config.db_path()) {
        Ok(store) => {
            let sessions = store.list_sessions(1).map(|s| s.len()).unwrap_or(0);
            println!("[OK]   base de datos abierta ({} sesión(es))", sessions);
        }
        Err(e) => {
            failures += 1;
            println!("[FALLO] base de datos: {e}");
        }
    }

    if config.provider == "mock" {
        println!("[AVISO] proveedor 'mock': respuestas simuladas, sin red");
    } else if config.api_key_env.is_empty() {
        println!("[OK]   proveedor '{}' no requiere API key", config.provider);
    } else if config.api_key().is_some() {
        println!("[OK]   API key presente en {}", config.api_key_env);
    } else {
        failures += 1;
        println!("[FALLO] {} no definida (exporta la variable o usa `malverde model set mock`)", config.api_key_env);
    }

    let registry = malverde_tools::ToolRegistry::with_builtins();
    let all = registry.all_names();
    let enabled = registry.enabled_names();
    let disabled: Vec<&String> = all.iter().filter(|n| !enabled.contains(*n)).collect();
    println!("[OK]   herramientas: {} habilitadas, {} deshabilitadas", enabled.len(), disabled.len());

    for dir in &config.skills_dirs {
        if dir.exists() {
            println!("[OK]   directorio de skills: {}", dir.display());
        } else {
            println!("[AVISO] directorio de skills no existe: {}", dir.display());
        }
    }

    match crate::runner::build_provider(&config) {
        Ok(_) => println!("[OK]   proveedor LLM construido: {}", config.provider),
        Err(e) => {
            failures += 1;
            println!("[FALLO] proveedor LLM: {e}");
        }
    }

    println!();
    if failures == 0 {
        println!("Todo en orden. Prueba: malverde run \"hola\" --json");
        Ok(())
    } else {
        Err(MalverdeError::validation(format!("doctor detectó {failures} problema(s)")))
    }
}

fn test_write(dir: &std::path::Path) -> bool {
    let probe = dir.join(".doctor-probe");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}
