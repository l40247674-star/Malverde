# Malverde · Harness agéntico local

> **Malverde** es un harness de agentes LLM completo, escrito en Rust desde cero.
> Convierte cualquier modelo (OpenAI, OpenRouter, Nous, Groq, Ollama, LM Studio o
> cualquier endpoint compatible con OpenAI) en un agente personal con herramientas,
> memoria persistente, skills, tareas programadas, subagentes y gateway de mensajería.

```
tú> revisa el repo y dime qué falla
→ shell git status --short
✓ shell
→ search "TODO"
✓ search
malverde> Encontré 3 TODOs sin asignar…
```

---

## ✨ Capacidades (paridad con Hermes Agent)

| Capacidad | Estado |
|---|---|
| Loop agéntico (planificar → actuar → observar → repetir) | ✅ |
| Presupuesto de iteraciones y gestión de contexto | ✅ |
| Model-agnostic: cualquier endpoint compatible OpenAI | ✅ |
| Registro de herramientas separado de su exposición al modelo | ✅ |
| Sistema de permisos (auto_approve / ask / allow_list / read_only) | ✅ |
| Herramientas integradas: shell, archivos, búsqueda, fetch, hora | ✅ |
| Herramientas de memoria: remember / recall / forget | ✅ |
| Subagentes aislados (spawn_agent) | ✅ |
| Sesiones persistentes en SQLite + FTS5 (recall cross-sesión) | ✅ |
| Skills estándar SKILL.md, creables y auto-mejorables | ✅ |
| Cron integrado con daemon (`malverde cron`) | ✅ |
| Gateway de mensajería (webhook loopback + CLI) | ✅ |
| CLI completa + TUI de chat | ✅ |
| Trazas JSONL exportables por ejecución | ✅ |
| Doctor / setup wizard | ✅ |

La tabla completa punto por punto está en [docs/PARITY.md](docs/PARITY.md).

---

## 🚀 Inicio rápido

```bash
# 1. Compilar
cargo build --release

# 2. Configurar (wizard) o usar mock sin red
./target/release/malverde setup
./target/release/malverde model set mock          # pruebas sin API key

# 3. Probar
malverde run "usa la herramienta now para darme la hora" --json
malverde chat
```

Guía completa en [INSTALL.md](INSTALL.md) y [docs/USAGE.md](docs/USAGE.md).

---

## 🖥️ Comandos principales

```
malverde chat              # REPL interactivo (con aprobación de permisos)
malverde run "<tarea>"     # one-shot, termina al responder
malverde model list|set    # proveedor y modelo
malverde tools list|enable|disable
malverde config get|set|list
malverde sessions list|show|search|delete
malverde memory remember|list|search|forget
malverde skills list|create|path
malverde cron add|list|remove|enable|disable|run
malverde gateway serve     # webhook SOLO en 127.0.0.1
malverde tui               # chat visual
malverde doctor            # diagnóstico
malverde setup             # wizard inicial
```

---

## 📦 Estructura

```
Malverde/
├── crates/
│   ├── malverde-core/        # config, tipos, errores, logging
│   ├── malverde-provider/    # trait LlmProvider + OpenAI-compatible + mock
│   ├── malverde-tools/       # registry + permisos + herramientas integradas
│   ├── malverde-memory/      # SQLite + FTS5: sesiones, mensajes, hechos
│   ├── malverde-skills/      # SKILL.md: cargar, crear, mejorar
│   ├── malverde-agent/       # EL LOOP: presupuesto, contexto, subagentes, trazas
│   ├── malverde-scheduler/   # cron + jobs + daemon
│   ├── malverde-gateway/     # plataformas de mensajería (webhook loopback)
│   ├── malverde-cli/         # binario `malverde`
│   └── malverde-tui/         # TUI de chat
├── docs/                     # documentación completa
├── examples/                 # config de ejemplo, skills, scripts
├── skills/                   # skills embebidas del repo
└── .github/workflows/ci.yml  # CI: build + tests
```

---

## 🔒 Seguridad

- El gateway webhook **solo escucha en 127.0.0.1**; nunca expone el agente a la red.
- Modo `read_only` bloquea herramientas sensibles; `ask` exige aprobación humana;
  `allow_list` combina lista de permitidos con preguntas.
- Las API keys nunca se escriben en disco: se leen de variables de entorno.
- Confirmación recomendada antes de comandos destructivos (regla en el system prompt).

## 📚 Documentación

- [INSTALL.md](INSTALL.md) · instalación (Linux, macOS, Termux/ARM64, Windows)
- [ARCHITECTURE.md](ARCHITECTURE.md) · arquitectura y flujo de datos
- [docs/USAGE.md](docs/USAGE.md) · uso diario con ejemplos
- [docs/CONFIG.md](docs/CONFIG.md) · todas las claves de configuración
- [docs/TOOLS.md](docs/TOOLS.md) · herramientas y cómo añadir las tuyas
- [docs/SKILLS.md](docs/SKILLS.md) · formato SKILL.md y skills autónomas
- [docs/MEMORY.md](docs/MEMORY.md) · sesiones, hechos y recall
- [docs/CRON.md](docs/CRON.md) · automatizaciones programadas
- [docs/GATEWAY.md](docs/GATEWAY.md) · integración por webhook loopback
- [docs/PARITY.md](docs/PARITY.md) · Malverde vs Hermes Agent
- [CHANGELOG.md](CHANGELOG.md)

## 📜 Licencia

MIT.
