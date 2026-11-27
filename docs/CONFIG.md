# Configuración

Archivo: `~/.malverde/config.json` (respeta `MALVERDE_HOME`). Se edita con
`malverde config set <clave> <valor>` o a mano (JSON).

## Claves

| Clave | Defecto | Descripción |
|---|---|---|
| `model` | `gpt-4o-mini` | Modelo para el endpoint elegido. |
| `provider` | `openai` | `openai` `openrouter` `nous` `groq` `ollama` `lmstudio` `custom` `mock`. |
| `base_url` | según proveedor | Endpoint compatible OpenAI (terminado en `/v1`). |
| `api_key_env` | `OPENAI_API_KEY` | Variable de entorno con la clave. Vacío = sin clave (locales). |
| `max_iterations` | `90` | Presupuesto de turnos del LLM por ejecución. |
| `temperature` | `0.7` | Temperatura de muestreo. |
| `max_tokens` | `4096` | Límite de tokens de respuesta. |
| `permission_mode` | `auto_approve` | `auto_approve` \| `ask` \| `allow_list` \| `read_only`. |
| `allow_list` | herramientas básicas | Herramientas permitidas sin preguntar en `allow_list`. |
| `disabled_tools` | `[]` | Herramientas ocultas al modelo y no ejecutables. |
| `context_max_messages` | `120` | Ventana deslizante de contexto. |
| `max_tool_output_chars` | `20000` | Truncado de salidas de herramientas. |
| `data_dir` | `~/.malverde` | Dónde viven DB, skills, cron y trazas. |
| `working_dir` | `.` | Directorio de trabajo de las herramientas. |
| `skills_dirs` | `[]` | Directorios extra de skills (se suman a `./skills` y `~/.malverde/skills`). |
| `shell_timeout_secs` | `120` | Timeout del tool `shell`. |
| `identity_name` | `Malverde` | Nombre del agente en el system prompt. |
| `identity_prompt` | `""` | Instrucciones permanentes del usuario. |

## Variables de entorno (override por invocación)

| Variable | Efecto |
|---|---|
| `MALVERDE_HOME` | cambia el directorio de datos |
| `MALVERDE_MODEL` | sobreescribe `model` |
| `MALVERDE_PROVIDER` | sobreescribe el proveedor |
| `MALVERDE_BASE_URL` | sobreescribe `base_url` |
| `MALVERDE_PERMISSION_MODE` | sobreescribe `permission_mode` |
| `MALVERDE_MAX_ITERATIONS` | sobreescribe `max_iterations` |
| `RUST_LOG` | nivel de logs (`debug`, `info`…) |

## Proveedores preconfigurados

```
openai      https://api.openai.com/v1               OPENAI_API_KEY
openrouter  https://openrouter.ai/api/v1            OPENROUTER_API_KEY
nous        https://inference-api.nousresearch.com  NOUS_API_KEY
groq        https://api.groq.com/openai/v1          GROQ_API_KEY
ollama      http://localhost:11434/v1               (sin clave)
lmstudio    http://localhost:1234/v1                (sin clave)
mock        respuestas simuladas                    (sin red)
```

```bash
malverde model set openrouter meta-llama/llama-3.3-70b-instruct
malverde model set http://mi-vllm:8000/v1 mi-modelo     # URL arbitraria
```
