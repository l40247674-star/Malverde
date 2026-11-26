# Uso diario

## Chat interactivo

```bash
malverde chat                  # usa la config persistente
malverde chat --verbose        # muestra la salida completa de herramientas
malverde chat --session mi-proyecto
```

Comandos slash dentro del chat:

| Comando | Efecto |
|---|---|
| `/help` | ayuda |
| `/new` | nueva sesión (contexto limpio) |
| `/tools` | lista herramientas habilitadas |
| `/memory <consulta>` | busca en la memoria persistente |
| `/exit` | salir |

Con `permission_mode: ask` o `allow_list`, cada herramienta sensible pide
aprobación: `[s]í / [t]oda la sesión / [n]o`.

## Tareas one-shot

```bash
malverde run "clona el repo X y resume su README"
malverde run "arregla el test roto de crates/foo" --session fix-42 --verbose
malverde run "dime la hora" --json      # salida estructurada para scripts
```

Salida `--json`:

```json
{
  "session": "c476c1e1-…",
  "final_text": "…",
  "iterations": 2,
  "tool_calls": 1,
  "stop_reason": "completed",
  "usage": { "prompt_tokens": 20, "completion_tokens": 15, "total_tokens": 35 }
}
```

## Sesiones

```bash
malverde sessions list
malverde sessions show c476c1e1        # acepta prefijo
malverde sessions search "docker"      # FTS5 en TODO el historial
malverde sessions delete c476c1e1
```

## Memoria persistente

El agente puede recordar por sí mismo (herramientas `remember`/`recall`/`forget`),
y tú puedes gestionarla directamente:

```bash
malverde memory remember "Despliegues solo los jueves"
malverde memory search "despliegues"
malverde memory list
malverde memory forget <id>
```

## Skills

```bash
malverde skills list
malverde skills create reporte "Genera reportes semanales"
malverde skills path
```

Ver [SKILLS.md](SKILLS.md).

## TUI

```bash
malverde tui
```

Atajos: `Enter` enviar, `Esc`/`Ctrl+C` salir, `/new` y `/tools` dentro de la TUI.

## Trazas y depuración

Cada ejecución escribe `.malverde-traces/run-<sesión>-<timestamp>.jsonl` con los
eventos `llm_response` y `tool_result` (prompt, arguments, salida, motivo de
fin). Útil para replay y para medir tasa de éxito / errores de herramienta.

```bash
RUST_LOG=debug malverde run "…"        # logs detallados por stderr
```
