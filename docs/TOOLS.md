# Herramientas

## Integradas de fábrica

| Herramienta | Sensible | Descripción |
|---|---|---|
| `shell` | sí | Ejecuta un comando (`sh -c` / `cmd /C`) con timeout y devuelve stdout+stderr. |
| `read_file` | sí | Lee un archivo de texto. |
| `write_file` | sí | Escribe o sobrescribe un archivo. |
| `edit_file` | sí | Reemplazo exacto `old → new` (falla si `old` no es único). |
| `list_dir` | sí | Lista un directorio con tamaños. |
| `search` | sí | Búsqueda recursiva de texto (excluye `.git`, `target`, `node_modules`). |
| `fetch` | sí | HTTP GET (solo http/https), cuerpo truncado. |
| `now` | no | Fecha/hora UTC actual. |
| `remember` / `recall` / `forget` | sí | Memoria persistente (ver [MEMORY.md](MEMORY.md)). |
| `spawn_agent` | sí | Subagente aislado (ver abajo). |

## Gestión

```bash
malverde tools list
malverde tools disable fetch
malverde tools enable fetch
```

Una herramienta deshabilitada **no se muestra al modelo** ni se ejecuta (el
registro y la exposición están separados, como en Hermes).

## Permisos

Ver [CONFIG.md](CONFIG.md) (`permission_mode`) y la sección 4 de
[ARCHITECTURE.md](../ARCHITECTURE.md). Resumen:

- `auto_approve`: todo se ejecuta (para entornos de confianza).
- `ask`: pregunta antes de cada herramienta sensible.
- `allow_list`: ejecuta solo las de `allow_list`; el resto se pregunta.
- `read_only`: bloquea toda herramienta sensible.

## Añadir una herramienta propia

Implementa el trait `Tool` y regístrala en el `ToolRegistry`:

```rust
use malverde_tools::{Tool, ToolContext};
use malverde_core::Result;

struct SaludoTool;

impl Tool for SaludoTool {
    fn name(&self) -> &str { "saludo" }
    fn description(&self) -> &str { "Devuelve un saludo personalizado." }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "nombre": { "type": "string" } },
            "required": ["nombre"]
        })
    }
    fn execute(&self, args: &serde_json::Value, _ctx: &ToolContext) -> Result<String> {
        let nombre = malverde_tools::builtin::get_str(args, "nombre")?;
        Ok(format!("¡Hola, {nombre}!"))
    }
}

fn main() {
    let mut registry = malverde_tools::ToolRegistry::with_builtins();
    registry.register(std::sync::Arc::new(SaludoTool));
}
```

Pautas: descripciones claras (son lo que lee el modelo), schemas JSON estrictos,
salida truncada con `truncate_output`, y `sensitive() = true` por defecto.

## Subagentes

`spawn_agent` lanza un hijo con sesión propia, herramientas básicas y un
presupuesto corto. Solo el resumen final entra en la conversación padre — útil
para exploraciones largas que ensuciarían el contexto principal.
