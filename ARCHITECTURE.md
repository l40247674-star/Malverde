# Arquitectura · Malverde v0.1.0

> Cómo está construido el harness y por qué. Misma filosofía que Hermes Agent:
> **el LLM es un componente reemplazable; la ingeniería está en las capas
> alrededor** (instrucciones, restricciones, feedback, memoria, orquestación).

## 1. Principios

1. **Loop explícito y síncrono**: `llm → tool_calls → resultados → llm → …` hasta
   respuesta final o presupuesto agotado. Depurable paso a paso.
2. **Registro ≠ exposición**: las herramientas se registran en un `ToolRegistry`,
   pero solo las *habilitadas* se envían al modelo como schemas.
3. **Seguridad por capas**: motor de permisos independiente, con callback de
   aprobación humana inyectable.
4. **Todo persiste**: sesiones, mensajes y hechos en SQLite (FTS5); trazas JSONL.
5. **Model-agnostic**: solo se asume una API de chat estilo OpenAI.

## 2. Crates y dependencias

```
                    ┌───────────────────┐
                    │  malverde-cli /   │  (binarios)
                    │  malverde-tui     │
                    └────────┬──────────┘
                             │
        ┌────────────────────┼─────────────────────┐
        ▼                    ▼                     ▼
┌───────────────┐   ┌────────────────┐   ┌────────────────┐
│ malverde-agent│   │ malverde-      │   │ malverde-      │
│  (EL LOOP)    │   │  scheduler     │   │  gateway       │
└───┬───────┬───┘   └───────┬────────┘   └───────┬────────┘
    │       │               └──────────┬─────────┘
    ▼       ▼                          ▼
┌────────────┐ ┌───────────┐ ┌──────────────┐
│ malverde-  │ │ malverde- │ │  malverde-   │
│  provider  │ │  tools    │ │  memory      │
└─────┬──────┘ └────┬──────┘ └──────┬───────┘
      │             │               │
      └─────────────┼───────────────┘
                    ▼
             ┌──────────────┐
             │ malverde-core│  (Config, tipos, errores)
             └──────────────┘
```

| Crate | Responsabilidad |
|---|---|
| `malverde-core` | `Config` (con overrides por env), tipos wire (`Message`, `ToolCall`, `Role`, `Usage`), errores unificados, logging. |
| `malverde-provider` | Trait `LlmProvider`, provider HTTP compatible OpenAI (ureq), `MockProvider` determinista para tests/offline. |
| `malverde-tools` | Trait `Tool`, `ToolRegistry` (registro/exposición/enable/disable), `PermissionManager` (4 modos + aprobador), herramientas integradas. |
| `malverde-memory` | `MemoryStore` (SQLite): sesiones, mensajes, hechos; FTS5 con triggers; búsqueda cross-sesión. |
| `malverde-skills` | `SkillRegistry`: parse de `SKILL.md` con frontmatter, `create`, `improve` (auto-mejora), índice para el prompt. |
| `malverde-agent` | `Agent` (el loop), `AgentConfig`, fábrica de subagentes, observadores, system prompt, trazas JSONL. |
| `malverde-scheduler` | Parser cron de 5 campos (regla Vixie dom/dow), `CronStore` JSON, daemon tokio. |
| `malverde-gateway` | Trait `Platform`, `LoopbackWebhook` (tiny_http en 127.0.0.1, `POST /message`). |
| `malverde-cli` | Binario `malverde`: chat, run y todos los subcomandos. |
| `malverde-tui` | Chat TUI (ratatui) con el agente en hilo worker. |

## 3. El loop agéntico (`crates/malverde-agent/src/agent.rs`)

```
run(session, input):
  1. persistir input del usuario (MemoryStore)
  2. construir system prompt:
       identidad + fecha + reglas + instrucciones del usuario
       + índice de skills + hechos de memoria (últimos 20)
  3. loop (máx. max_iterations = 90):
       a. ventana de contexto: system + últimos context_max_messages
       b. provider.complete(mensajes, schemas de tools habilitadas)
       c. si no hay tool_calls → respuesta final; persistir; END
       d. por cada tool_call:
            - PermissionManager.evaluate(tool, sensitive)
            - si NeedsApproval y hay aprobador humano → preguntar
            - ejecutar → resultado (o [error])
            - append tool_result al historial
       e. si history > context_max_messages → recortar a la mitad
```

Decisiones clave:

- **Presupuesto**: `max_iterations` con hard-cap; el agente devuelve
  `stop_reason = max_iterations` en vez de colgarse.
- **Contexto**: ventana deslizante por recorte (`context_max_messages`, por
  defecto 120). Los mensajes `tool` no se recargan del disco: su detalle vive en
  las trazas, y así el wire format siempre es válido al reanudar una sesión.
- **Trazas**: cada corrida escribe `.malverde-traces/run-<sesión>-<ts>.jsonl` con
  un evento por respuesta del LLM y por resultado de herramienta (replay/eval).
- **Subagentes**: `default_subagent_factory` crea un `Agent` hijo *sin*
  `spawn_agent` (sin recursión infinita), presupuesto corto (15 iteraciones),
  sesión propia; solo el texto final vuelve al padre.

## 4. Permisos (`crates/malverde-tools/src/permissions.rs`)

```
evaluate(tool, sensitive) →
  AutoApprove → Allowed
  ReadOnly    → Denied si sensitive
  AllowList   → Allowed si tool ∈ allow_list o aprobada en sesión
  Ask         → NeedsApproval salvo aprobada en sesión

NeedsApproval + aprobador humano →
  Yes        → ejecutar una vez
  YesAlways  → aprobar para la sesión
  No         → error de permiso → el modelo recibe "[error]" y se re-planifica
```

`malverde chat` instala un aprobador por terminal; `cron` y `gateway` corren sin
aprobador (fallan cerrado, nunca abierto).

## 5. Persistencia (`malverde-memory`)

```
sessions(id, title, model, created_at, updated_at)
messages(id, session_id, role, content, tool_name, tool_call_id, created_at)
memory(id, fact, tags, created_at, updated_at)
messages_fts / memory_fts  (FTS5 + triggers de sincronización)
```

- `search_messages(query)` → recall cross-sesión (herramienta `recall`).
- `remember/forget/list/search` → hechos curados por el agente; el system prompt
  lista los hechos recientes y anima a guardar aprendizajes ("nudges").

## 6. Skills (`malverde-skills`)

- Formato: `<dir>/<nombre>/SKILL.md` con frontmatter `name:` y `description:`.
- Se cargan de `./skills`, `~/.malverde/skills` y `config.skills_dirs`.
- `SkillRegistry::create` permite creación autónoma; `improve` añade notas de
  mejora (auto-mejora durante el uso).

## 7. Orquestación

- **`malverde-scheduler`**: cron estándar de 5 campos (`* , - /`), regla clásica
  dom/dow "o", daemon tokio con tick de 30 s y `last_run` en JSON.
- **`malverde-gateway`**: `Platform` como frontera. `LoopbackWebhook` expone
  `POST /message` exclusivamente en 127.0.0.1; cada `chat_id` mapea a la sesión
  estable `webhook-<chat_id>` (continuidad cross-plataforma).

## 8. Flujo de datos de una ejecución

```
usuario → CLI chat/run → Agent.run
  ├─→ MemoryStore (persistir user/assistant)
  ├─→ PromptBuilder (identity + skills + memory)
  ├─→ LlmProvider (HTTP OpenAI-compatible | mock)
  ├─→ ToolRegistry + PermissionManager → Tool.execute
  ├─→ Observer (CLI imprime progreso)
  └─→ trazas JSONL (.malverde-traces/)
```
