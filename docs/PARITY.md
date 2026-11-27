# Paridad Malverde ↔ Hermes Agent

Mapa funcional entre Malverde (este repo) y las capacidades documentadas de
[Hermes Agent (NousResearch)](https://github.com/nousresearch/hermes-agent).

| Capacidad de Hermes Agent | Malverde | Notas |
|---|---|---|
| Loop agéntico planificar/actuar/observar | ✅ `malverde-agent` | Síncrono, con observadores inyectables. |
| Presupuesto de iteraciones (hard cap) | ✅ `max_iterations` | `stop_reason: max_iterations` en vez de colgarse. |
| Model-agnostic (OpenAI/OpenRouter/Nous/locales) | ✅ `malverde-provider` | Presets + URL arbitraria + `MockProvider`. |
| Tool registry separado de la exposición | ✅ `malverde-tools::ToolRegistry` | `disable/enable` sin borrar. |
| Sistema de permisos + aprobación humana | ✅ `PermissionManager` | 4 modos + aprobador interactivo (sí/siempre/no). |
| Herramientas: shell, archivos, búsqueda, web | ✅ herramientas integradas | shell, read/write/edit_file, list_dir, search, fetch, now. |
| Memoria persistente curada por el agente + nudges | ✅ `remember/recall/forget` | Hechos listados en el system prompt (nudge). |
| FTS5 + búsqueda cross-sesión | ✅ `malverde-memory` | FTS5 sobre mensajes y hechos; `recall` combina ambos. |
| Skills estándar (SKILL.md) portables | ✅ `malverde-skills` | Frontmatter name/description; dirs múltiples. |
| Creación autónoma de skills + auto-mejora | ✅ `SkillRegistry::create/improve` | Notas de mejora timestampadas. |
| Subagentes aislados | ✅ `spawn_agent` | Hijo sin recursión, presupuesto corto, solo resumen final. |
| Sesiones persistentes + reanudación | ✅ `malverde-memory` | `--session <id>` en chat/run/tui. |
| Cron / automatizaciones desatendidas | ✅ `malverde-scheduler` | Cron 5 campos, daemon tokio, `last_run`. |
| Gateway de mensajería multiplataforma | ✅ `malverde-gateway` | CLI + webhook loopback; trait `Platform` extensible. |
| TUI completa | ✅ `malverde-tui` | Chat con worker en hilo aparte. |
| CLI con `model/tools/config/setup/doctor` | ✅ `malverde-cli` | Mismos verbos de configuración. |
| Trazas / export de trayectorias | ✅ `.malverde-traces/*.jsonl` | Un JSONL por ejecución (replay/evals). |
| `doctor` diagnóstico | ✅ `malverde doctor` | Config, DB, API key, herramientas, skills, proveedor. |
| MCP (Model Context Protocol) | ⛔ no incluido | La frontera `Tool` permite añadir un puente MCP como herramienta. |
| Cliente de escritorio (Desktop) | ⛔ no incluido | La TUI + gateway loopback cubren la interacción local. |
| Apps de mensajería cloud (Telegram/Discord/…) | ⚠️ interfaz lista | Implementa `Platform` con el SDK correspondiente; el webhook loopback es el pegamento local. |

Fuera de alcance deliberado en v0.1.0 (documentado para transparencia): MCP,
desktop y conectores cloud específicos. Todo lo demás tiene equivalente
funcional implementado en este repo, desde cero, en Rust.
