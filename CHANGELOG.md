# Changelog

## 0.1.0 — Malverde Agent Harness

Reescritura completa del proyecto: pasa de "core framework" genérico a un
**harness agéntico** con paridad funcional respecto a Hermes Agent.

### Añadido
- Loop agéntico (`malverde-agent`): planificar → actuar → observar, con
  presupuesto de iteraciones, gestión de contexto y trazas JSONL.
- Proveedores LLM model-agnostic (`malverde-provider`): cualquier endpoint
  compatible OpenAI + `MockProvider` para tests/offline.
- Registro de herramientas con motor de permisos de 4 modos y aprobación humana
  (`malverde-tools`): shell, read/write/edit_file, list_dir, search, fetch, now.
- Herramientas de memoria (remember/recall/forget) y `spawn_agent` para
  subagentes aislados.
- Memoria persistente SQLite + FTS5 con recall cross-sesión (`malverde-memory`).
- Skills SKILL.md creables y auto-mejorables (`malverde-skills`).
- Cron integrado con daemon (`malverde-scheduler`).
- Gateway webhook SOLO en loopback (`malverde-gateway`).
- CLI completa: chat, run, model, tools, config, sessions, memory, skills, cron,
  gateway, tui, doctor, setup.
- TUI de chat (ratatui).
- CI (build + tests), documentación completa en `docs/` y ejemplo de skill.

### Eliminado
- Crates del antiguo esqueleto (bus, events, job, audit, storage, knowledge…)
  reemplazados por la nueva arquitectura del harness.
