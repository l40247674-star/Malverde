use malverde_memory::MemoryStore;
use malverde_skills::SkillRegistry;

/// Construye el system prompt con identidad, fecha, skills y memoria reciente.
pub fn build_system_prompt(
    identity_name: &str,
    identity_extra: &str,
    skills: &SkillRegistry,
    memory: &MemoryStore,
) -> malverde_core::Result<String> {
    let mut prompt = format!(
        "Eres {name}, un agente personal de propósito general que se ejecuta en la terminal del usuario.\n\
         Fecha actual: {now} (UTC).\n\n\
         Reglas de operación:\n\
         - Usa las herramientas cuando aporten valor; no las inventes.\n\
         - Antes de ejecutar comandos destructivos, confirma con el usuario.\n\
         - Sé conciso y directo en tus respuestas finales.\n",
        name = identity_name,
        now = chrono::Utc::now().to_rfc3339(),
    );

    if !identity_extra.is_empty() {
        prompt.push_str("\n## Instrucciones del usuario\n\n");
        prompt.push_str(identity_extra);
        prompt.push('\n');
    }

    let idx = skills.render_index();
    if !idx.is_empty() {
        prompt.push_str(&idx);
    }

    if let Ok(facts) = memory.recent_facts(20) {
        if !facts.is_empty() {
            prompt.push_str("\n## Memoria persistente (hechos recordados)\n");
            for f in facts {
                prompt.push_str(&format!("- {}\n", f.fact));
            }
            prompt.push_str("\nPuedes recordar más con la herramienta `recall`, guardar con `remember` y borrar con `forget`.\n");
        }
    }

    Ok(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_sections() {
        let memory = malverde_memory::MemoryStore::open_in_memory().unwrap();
        memory.remember("prueba de hecho", &[]).unwrap();
        let skills = SkillRegistry::default();
        let prompt = build_system_prompt("Malverde", "Sé breve.", &skills, &memory).unwrap();
        assert!(prompt.contains("Eres Malverde"));
        assert!(prompt.contains("Sé breve."));
        assert!(prompt.contains("prueba de hecho"));
    }
}
