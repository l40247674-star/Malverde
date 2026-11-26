use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use malverde_core::{MalverdeError, Result};

/// Contexto de ejecución entregado a cada herramienta.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub max_output_chars: usize,
    pub shell_timeout_secs: u64,
}

impl ToolContext {
    pub fn new(working_dir: PathBuf, max_output_chars: usize, shell_timeout_secs: u64) -> Self {
        Self { working_dir, max_output_chars, shell_timeout_secs }
    }
}

/// Trait que toda herramienta de Malverde debe implementar.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    /// Si es true, el motor de permisos puede exigir aprobación.
    fn sensitive(&self) -> bool {
        true
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String>;
}

pub fn get_str<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| MalverdeError::tool(format!("argumento '{key}' (string) requerido")))
}

pub fn truncate_output(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}\n…[salida truncada a {max} caracteres]")
    }
}

fn resolve_path(ctx: &ToolContext, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        ctx.working_dir.join(p)
    }
}

// ---------------------------------------------------------------------------
// shell
// ---------------------------------------------------------------------------

pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }
    fn description(&self) -> &str {
        "Ejecuta un comando en la shell del sistema (sh -c en Unix, cmd /C en Windows) y devuelve stdout+stderr."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Comando a ejecutar" },
                "timeout_secs": { "type": "integer", "description": "Timeout opcional en segundos" }
            },
            "required": ["command"]
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let command = get_str(args, "command")?;
        let timeout = args
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.shell_timeout_secs)
            .max(1);

        #[cfg(target_family = "unix")]
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&ctx.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(MalverdeError::io)?;

        #[cfg(target_family = "windows")]
        let mut child = Command::new("cmd")
            .args(["/C", command])
            .current_dir(&ctx.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(MalverdeError::io)?;

        let start = Instant::now();
        let deadline = Duration::from_secs(timeout);
        let status = loop {
            match child.try_wait().map_err(MalverdeError::io)? {
                Some(status) => break status,
                None => {
                    if start.elapsed() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(MalverdeError::tool(format!("timeout de {timeout}s alcanzado")));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };

        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_string(&mut stdout);
        }
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut stderr);
        }

        let mut out = String::new();
        if !stdout.trim().is_empty() {
            out.push_str(&stdout);
        }
        if !stderr.trim().is_empty() {
            out.push_str("\n[stderr] ");
            out.push_str(&stderr);
        }
        if !status.success() {
            out.push_str(&format!("\n[exit code: {}]", status.code().unwrap_or(-1)));
        }
        if out.trim().is_empty() {
            out.push_str("(sin salida)");
        }
        Ok(truncate_output(&out, ctx.max_output_chars))
    }
}

// ---------------------------------------------------------------------------
// read_file / write_file / edit_file / list_dir
// ---------------------------------------------------------------------------

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Lee un archivo de texto y devuelve su contenido."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Ruta del archivo (relativa al working dir o absoluta)" },
                "max_chars": { "type": "integer", "description": "Límite opcional de caracteres" }
            },
            "required": ["path"]
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let path = resolve_path(ctx, get_str(args, "path")?);
        let content = std::fs::read_to_string(&path).map_err(MalverdeError::io)?;
        let max = args.get("max_chars").and_then(|v| v.as_u64()).unwrap_or(ctx.max_output_chars as u64) as usize;
        Ok(truncate_output(&content, max))
    }
}

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    fn description(&self) -> &str {
        "Escribe (o sobrescribe) un archivo con el contenido indicado."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Ruta del archivo" },
                "content": { "type": "string", "description": "Contenido a escribir" }
            },
            "required": ["path", "content"]
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let path = resolve_path(ctx, get_str(args, "path")?);
        let content = get_str(args, "content")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(MalverdeError::io)?;
        }
        std::fs::write(&path, content).map_err(MalverdeError::io)?;
        Ok(format!("escrito {} ({} bytes)", path.display(), content.len()))
    }
}

pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }
    fn description(&self) -> &str {
        "Reemplaza texto exacto dentro de un archivo (old -> new). Falla si old no aparece exactamente una vez."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Ruta del archivo" },
                "old": { "type": "string", "description": "Texto exacto a reemplazar" },
                "new": { "type": "string", "description": "Texto de reemplazo" }
            },
            "required": ["path", "old", "new"]
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let path = resolve_path(ctx, get_str(args, "path")?);
        let old = get_str(args, "old")?;
        let new = get_str(args, "new")?;
        let content = std::fs::read_to_string(&path).map_err(MalverdeError::io)?;
        let occurrences = content.matches(old).count();
        if occurrences != 1 {
            return Err(MalverdeError::tool(format!(
                "'old' aparece {occurrences} veces (se requiere exactamente 1); usa más contexto"
            )));
        }
        let updated = content.replacen(old, new, 1);
        std::fs::write(&path, &updated).map_err(MalverdeError::io)?;
        Ok(format!("editado {}", path.display()))
    }
}

pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }
    fn description(&self) -> &str {
        "Lista el contenido de un directorio con tamaño y tipo de cada entrada."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directorio a listar (por defecto el working dir)" }
            }
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let raw = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let dir = resolve_path(ctx, raw);
        let entries = std::fs::read_dir(&dir).map_err(MalverdeError::io)?;
        let mut lines = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let desc = match entry.metadata() {
                Ok(m) if m.is_dir() => format!("{name}/"),
                Ok(m) => format!("{name} ({} bytes)", m.len()),
                Err(_) => name,
            };
            lines.push(desc);
        }
        lines.sort();
        if lines.is_empty() {
            return Ok("(directorio vacío)".to_string());
        }
        Ok(truncate_output(&lines.join("\n"), ctx.max_output_chars))
    }
}

// ---------------------------------------------------------------------------
// search (grep recursivo por substring)
// ---------------------------------------------------------------------------

pub struct SearchTool;

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }
    fn description(&self) -> &str {
        "Busca un texto dentro de los archivos de un directorio (recursivo) y devuelve archivo:línea: coincidencia."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Texto a buscar" },
                "path": { "type": "string", "description": "Directorio raíz de la búsqueda" },
                "case_sensitive": { "type": "boolean", "description": "Distinguir mayúsculas (por defecto false)" }
            },
            "required": ["query"]
        })
    }
    fn execute(&self, args: &serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let query = get_str(args, "query")?;
        let root = resolve_path(ctx, args.get("path").and_then(|v| v.as_str()).unwrap_or("."));
        let case_sensitive = args.get("case_sensitive").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut hits = Vec::new();
        search_recursive(&root, query, case_sensitive, &mut hits, 0);
        if hits.is_empty() {
            return Ok(format!("sin coincidencias para '{query}'"));
        }
        let out = hits.join("\n");
        Ok(truncate_output(&out, ctx.max_output_chars))
    }
}

const MAX_DEPTH: usize = 12;
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

fn search_recursive(dir: &Path, query: &str, case_sensitive: bool, hits: &mut Vec<String>, depth: usize) {
    if depth > MAX_DEPTH || hits.len() >= 200 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if path.is_dir() {
            search_recursive(&path, query, case_sensitive, hits, depth + 1);
        } else if entry.metadata().map(|m| m.len() <= MAX_FILE_SIZE).unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    let matched = if case_sensitive {
                        line.contains(query)
                    } else {
                        line.to_lowercase().contains(&query.to_lowercase())
                    };
                    if matched {
                        hits.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                        if hits.len() >= 200 {
                            return;
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// fetch (HTTP GET)
// ---------------------------------------------------------------------------

pub struct FetchTool;

impl Tool for FetchTool {
    fn name(&self) -> &str {
        "fetch"
    }
    fn description(&self) -> &str {
        "Realiza una petición HTTP GET a una URL http(s) y devuelve el cuerpo (texto)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL a descargar" },
                "max_chars": { "type": "integer", "description": "Límite de caracteres de la respuesta" }
            },
            "required": ["url"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let url = get_str(args, "url")?;
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(MalverdeError::tool("solo se permiten URLs http(s)"));
        }
        let resp = ureq::get(url)
            .timeout(Duration::from_secs(30))
            .set("User-Agent", "Malverde/0.1")
            .call()
            .map_err(|e| MalverdeError::tool(format!("petición falló: {e}")))?;
        let body = resp.into_string().map_err(MalverdeError::io)?;
        let max = args.get("max_chars").and_then(|v| v.as_u64()).unwrap_or(20_000) as usize;
        Ok(truncate_output(&body, max))
    }
}

// ---------------------------------------------------------------------------
// now (fecha/hora, no sensible)
// ---------------------------------------------------------------------------

pub struct NowTool;

impl Tool for NowTool {
    fn name(&self) -> &str {
        "now"
    }
    fn description(&self) -> &str {
        "Devuelve la fecha y hora actual en formato ISO-8601 UTC."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn sensitive(&self) -> bool {
        false
    }
    fn execute(&self, _args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        Ok(chrono::Utc::now().to_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(tmp: &Path) -> ToolContext {
        ToolContext::new(tmp.to_path_buf(), 5_000, 10)
    }

    #[test]
    fn now_tool_reports_utc() {
        let out = NowTool.execute(&serde_json::json!({}), &ctx(Path::new("."))).unwrap();
        assert!(out.contains('T'));
    }

    #[test]
    fn write_read_edit_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("malverde-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let c = ctx(&tmp);

        let out = WriteFileTool
            .execute(&serde_json::json!({"path": "a.txt", "content": "hola mundo"}), &c)
            .unwrap();
        assert!(out.contains("a.txt"));

        let content = ReadFileTool.execute(&serde_json::json!({"path": "a.txt"}), &c).unwrap();
        assert_eq!(content, "hola mundo");

        EditFileTool
            .execute(&serde_json::json!({"path": "a.txt", "old": "mundo", "new": "malverde"}), &c)
            .unwrap();
        let content = ReadFileTool.execute(&serde_json::json!({"path": "a.txt"}), &c).unwrap();
        assert_eq!(content, "hola malverde");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn shell_runs_command() {
        let c = ctx(Path::new("."));
        let out = ShellTool.execute(&serde_json::json!({"command": "echo hola-malverde"}), &c).unwrap();
        assert!(out.contains("hola-malverde"));
    }

    #[test]
    fn search_finds_substring() {
        let tmp = std::env::temp_dir().join(format!("malverde-search-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("x.txt"), "una linea\nagus busca AQUI\n").unwrap();
        let c = ctx(&tmp);
        let out = SearchTool.execute(&serde_json::json!({"query": "busca aqui"}), &c).unwrap();
        assert!(out.contains("x.txt"));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
