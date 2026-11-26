use std::path::{Path, PathBuf};

use malverde_core::{MalverdeError, Result};

/// Una skill: un paquete de instrucciones reutilizables (estándar tipo SKILL.md).
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub path: Option<PathBuf>,
}

/// Registro de skills cargadas desde uno o varios directorios.
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    pub skills: Vec<Skill>,
}

impl SkillRegistry {
    pub fn load(dirs: &[PathBuf]) -> Self {
        let mut reg = Self::default();
        for dir in dirs {
            reg.load_dir(dir);
        }
        reg
    }

    pub fn load_dir(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let skill_file = if path.is_dir() { path.join("SKILL.md") } else { path.clone() };
            if skill_file.is_file() && skill_file.file_name().map(|f| f == "SKILL.md").unwrap_or(false) {
                match parse_skill_file(&skill_file) {
                    Ok(skill) => self.skills.push(skill),
                    Err(e) => tracing::warn!("skill inválida en {}: {e}", skill_file.display()),
                }
            }
        }
    }

    pub fn find(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn names(&self) -> Vec<String> {
        self.skills.iter().map(|s| s.name.clone()).collect()
    }

    /// Bloque de texto que se inyecta en el system prompt con el índice de skills.
    pub fn render_index(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        let mut out = String::from("\n## Skills disponibles\n");
        out.push_str("Cuando una tarea encaje con una skill, sigue sus instrucciones:\n\n");
        for s in &self.skills {
            out.push_str(&format!("- **{}**: {}\n", s.name, s.description));
        }
        out.push_str("\nPara ver el contenido completo de una skill, pide al usuario que consulte el archivo o usa los datos ya inyectados arriba.\n");
        out
    }

    /// Contenido completo de una skill (índice + cuerpo), para inyectar en el prompt.
    pub fn render_full(&self, name: &str) -> Option<String> {
        self.find(name).map(|s| {
            format!("\n### Skill: {}\n{}\n", s.name, s.instructions)
        })
    }

    /// Crea una nueva skill en disco (creación autónoma de skills).
    pub fn create(base_dir: &Path, name: &str, description: &str, instructions: &str) -> Result<PathBuf> {
        validate_name(name)?;
        let dir = base_dir.join(name);
        std::fs::create_dir_all(&dir).map_err(MalverdeError::io)?;
        let path = dir.join("SKILL.md");
        let content = format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n\n{instructions}\n"
        );
        std::fs::write(&path, content).map_err(MalverdeError::io)?;
        Ok(path)
    }

    /// Auto-mejora: añade/aparea una nota de mejora al final de una skill existente.
    pub fn improve(&self, name: &str, note: &str) -> Result<PathBuf> {
        let skill = self
            .find(name)
            .ok_or_else(|| MalverdeError::skill(format!("skill '{name}' no encontrada")))?;
        let path = skill
            .path
            .clone()
            .ok_or_else(|| MalverdeError::skill("la skill no proviene de disco"))?;
        let mut content = std::fs::read_to_string(&path).map_err(MalverdeError::io)?;
        content.push_str(&format!("\n<!-- mejora {} -->\n{}\n", chrono::Utc::now().to_rfc3339(), note));
        std::fs::write(&path, content).map_err(MalverdeError::io)?;
        Ok(path)
    }
}

fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && name.len() <= 64;
    if ok {
        Ok(())
    } else {
        Err(MalverdeError::skill("nombre de skill inválido (usa letras, números, '-' y '_'; máx 64)"))
    }
}

/// Parser de SKILL.md con frontmatter `--- name: … description: … ---`.
fn parse_skill_file(path: &Path) -> Result<Skill> {
    let raw = std::fs::read_to_string(path).map_err(MalverdeError::io)?;
    let mut name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut description = String::new();
    let mut body = raw.as_str();

    if let Some(rest) = raw.strip_prefix("---\n").or_else(|| raw.strip_prefix("---\r\n")) {
        if let Some(end) = rest.find("\n---") {
            let front = &rest[..end];
            body = rest[end + 4..].trim_start_matches(['\n', '\r']).trim_start();
            for line in front.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().trim_matches('"');
                    if k.trim() == "name" && !v.is_empty() {
                        name = v.to_string();
                    }
                    if k.trim() == "description" {
                        description = v.to_string();
                    }
                }
            }
        }
    }

    if name.is_empty() {
        return Err(MalverdeError::skill(format!("{}: falta 'name'", path.display())));
    }
    validate_name(&name)?;
    if description.is_empty() {
        description = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("skill").chars().take(120).collect();
    }
    Ok(Skill { name, description, instructions: body.to_string(), path: Some(path.to_path_buf()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_skill() {
        let tmp = std::env::temp_dir().join(format!("malverde-skill-{}", std::process::id()));
        let dir = tmp.join("saludar");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            "---\nname: saludar\ndescription: Saluda con estilo\n---\n\nDi hola y despídete con humor.\n",
        )
        .unwrap();

        let reg = SkillRegistry::load(&[tmp.clone()]);
        assert_eq!(reg.names(), vec!["saludar"]);
        assert!(reg.render_index().contains("saludar"));
        assert!(reg.render_full("saludar").unwrap().contains("humor"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn creates_new_skill() {
        let tmp = std::env::temp_dir().join(format!("malverde-skill-new-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = SkillRegistry::create(&tmp, "resumir", "Resume textos", "Resume en 3 puntos.").unwrap();
        assert!(path.exists());
        let reg = SkillRegistry::load(&[tmp.clone()]);
        assert!(reg.find("resumir").is_some());
        assert!(SkillRegistry::create(&tmp, "nombre inválido!", "x", "y").is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
