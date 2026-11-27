# Gateway

El gateway conecta plataformas de mensajería con el agente. Malverde incluye:

- **CLI** (por defecto): stdin/stdout.
- **Webhook loopback**: HTTP en `127.0.0.1` para integrar con otros procesos
  locales (bots, scripts, otro software).

## Webhook loopback

```bash
malverde gateway serve --port 8787
```

Solo acepta conexiones de 127.0.0.1 (nunca expone el agente a la red):

```text
GET  /health   → {"status": "ok"}
POST /message  → {"chat_id": "canal-1", "text": "resume el repo"}
                ← {"reply": "…"}
```

Ejemplo con curl (en otra terminal):

```bash
curl http://127.0.0.1:8787/health
curl -X POST http://127.0.0.1:8787/message \
  -H 'Content-Type: application/json' \
  -d '{"chat_id": "canal-1", "text": "resume el repo"}'
```

Cada `chat_id` tiene una sesión estable (`webhook-<chat_id>`), así que la
conversación es continua por canal.

## Añadir una plataforma

Implementa el trait `Platform` (y el transporte que necesites) en
`malverde-gateway/src/platform.rs`; la sesión estable por chat viene de fábrica:

```rust
impl Platform for MiBot {
    fn name(&self) -> &str { "mibot" }
}
```
