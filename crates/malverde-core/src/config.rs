use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{MalverdeError, Result};

/// Modo de aprobación de herramientas (paridad con el sistema de permisos de Hermes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    /// Nunca pregunta: todas las herramientas se ejecutan.
    #[default]
    AutoApprove,
    /// Pregunta en la terminal antes de cada herramienta sensible.
    Ask,
    /// Lista de permitidos + preguntar para el resto.
    AllowList,
    /// No ejecuta herramientas sensibles.
    ReadOnly,
}

impl PermissionMode {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "auto_approve" | "auto" | "yolo" => Ok(Self::AutoApprove),
            "ask" => Ok(Self::Ask),
            "allow_list" | "allowlist" => Ok(Self::AllowList),
            "read_only" | "readonly" => Ok(Self::ReadOnly),
            other => Err(MalverdeError::config(format!(
                "modo de permisos inválido '{other}' (auto_approve|ask|allow_list|read_only)"
            ))),
        }
    }
}

/// Proveedores LLM preconfigurados (endpoints compatibles con OpenAI).
pub const PROVIDER_PRESETS: &[(&str, &str, &str)] = &[
    ("openai", "https://api.openai.com/v1", "OPENAI_API_KEY"),
    ("openrouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY"),
    ("nous", "https://inference-api.nousresearch.com/v1", "NOUS_API_KEY"),
    ("groq", "https://api.groq.com/openai/v1", "GROQ_API_KEY"),
    ("ollama", "http://localhost:11434/v1", ""),
    ("lmstudio", "http://localhost:1234/v1", ""),
];

/// Configuración global de Malverde.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub model: String,
    pub provider: String,
    pub base_url: String,
    /// Nombre de la variable de entorno que contiene la API key.
    pub api_key_env: String,
    pub max_iterations: usize,
    pub temperature: f32,
    pub max_tokens: u32,
    pub permission_mode: PermissionMode,
    pub allow_list: Vec<String>,
    pub disabled_tools: Vec<String>,
    pub context_max_messages: usize,
    pub max_tool_output_chars: usize,
    pub data_dir: PathBuf,
    pub working_dir: PathBuf,
    pub skills_dirs: Vec<PathBuf>,
    pub shell_timeout_secs: u64,
    pub identity_name: String,
    pub identity_prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = default_data_dir();
        Self {
            model: "gpt-4o-mini".to_string(),
            provider: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            max_iterations: 90,
            temperature: 0.7,
            max_tokens: 4096,
            permission_mode: PermissionMode::AutoApprove,
            allow_list: vec![
                "shell".to_string(),
                "read_file".to_string(),
                "write_file".to_string(),
                "edit_file".to_string(),
                "list_dir".to_string(),
                "search".to_string(),
                "fetch".to_string(),
                "now".to_string(),
                "remember".to_string(),
                "recall".to_string(),
                "forget".to_string(),
                "spawn_agent".to_string(),
            ],
            disabled_tools: vec![],
            context_max_messages: 120,
            max_tool_output_chars: 20_000,
            data_dir,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            skills_dirs: vec![],
            shell_timeout_secs: 120,
            identity_name: "Malverde".to_string(),
            identity_prompt: String::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path).map_err(MalverdeError::io)?;
        let mut cfg: Config = serde_json::from_str(&raw).map_err(|e| MalverdeError::serialization(e.to_string()))?;
        cfg.apply_env_overrides()?;
        cfg.skills_dirs.push(default_skills_dir());
        cfg.skills_dirs.push(cfg.data_dir.join("skills"));
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(MalverdeError::io)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| MalverdeError::serialization(e.to_string()))?;
        std::fs::write(path, json).map_err(MalverdeError::io)
    }

    pub fn default_path() -> PathBuf {
        default_data_dir().join("config.json")
    }

    fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(model) = std::env::var("MALVERDE_MODEL") {
            self.model = model;
        }
        if let Ok(provider) = std::env::var("MALVERDE_PROVIDER") {
            self.set_provider(&provider)?;
        }
        if let Ok(base) = std::env::var("MALVERDE_BASE_URL") {
            self.base_url = base;
        }
        if let Ok(mode) = std::env::var("MALVERDE_PERMISSION_MODE") {
            self.permission_mode = PermissionMode::parse(&mode)?;
        }
        if let Ok(max) = std::env::var("MALVERDE_MAX_ITERATIONS") {
            self.max_iterations = max.parse().map_err(|_| MalverdeError::config("MALVERDE_MAX_ITERATIONS debe ser un número"))?;
        }
        Ok(())
    }

    /// Configura provider + base_url + api_key_env a partir de un preset o URL arbitraria.
    pub fn set_provider(&mut self, provider: &str) -> Result<()> {
        for (name, url, env) in PROVIDER_PRESETS {
            if *name == provider {
                self.provider = name.to_string();
                self.base_url = url.to_string();
                self.api_key_env = env.to_string();
                return Ok(());
            }
        }
        if provider.starts_with("http://") || provider.starts_with("https://") {
            self.provider = "custom".to_string();
            self.base_url = provider.trim_end_matches('/').to_string();
            self.api_key_env = "MALVERDE_API_KEY".to_string();
            return Ok(());
        }
        Err(MalverdeError::config(format!("proveedor desconocido '{provider}'")))
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("malverde.db")
    }

    pub fn traces_dir(&self) -> PathBuf {
        self.data_dir.join("traces")
    }

    pub fn cron_path(&self) -> PathBuf {
        self.data_dir.join("cron.json")
    }

    pub fn api_key(&self) -> Option<String> {
        if self.api_key_env.is_empty() {
            return None;
        }
        std::env::var(&self.api_key_env).ok().filter(|k| !k.is_empty())
    }
}

/// Directorio por defecto: ~/.malverde (respeta MALVERDE_HOME).
pub fn default_data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("MALVERDE_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".malverde")
}

/// Directorio de skills embebidas en el repo (./skills) si existe.
pub fn default_skills_dir() -> PathBuf {
    PathBuf::from("skills")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.max_iterations, 90);
        assert!(cfg.max_iterations > 0);
        assert_eq!(cfg.identity_name, "Malverde");
    }

    #[test]
    fn provider_presets_resolve() {
        let mut cfg = Config::default();
        cfg.set_provider("ollama").unwrap();
        assert_eq!(cfg.base_url, "http://localhost:11434/v1");
        cfg.set_provider("https://mi-servidor/v1").unwrap();
        assert_eq!(cfg.provider, "custom");
        assert!(cfg.set_provider("no-existe").is_err());
    }

    #[test]
    fn permission_modes_parse() {
        assert_eq!(PermissionMode::parse("ask").unwrap(), PermissionMode::Ask);
        assert!(PermissionMode::parse("bogus").is_err());
    }
}
