# Instalación · Malverde

## Requisitos

- **Rust 1.75+** (`rustup`) — en el sandbox de desarrollo se usa 1.100.
- Git y una terminal estándar (Linux, macOS, Windows/WSL, Termux).
- Opcional: una API key de tu proveedor (OpenAI / OpenRouter / Nous / Groq) o un
  servidor local (Ollama, LM Studio) para no necesitar clave.

## Compilar

```bash
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde
cargo build --release
```

El binario queda en `target/release/malverde`.

## Instalar (opcional)

```bash
cp target/release/malverde ~/.local/bin/   # o /usr/local/bin
```

En Termux/ARM64 se puede usar `./install.sh`, que compila y enlaza el binario.

## Configuración inicial

### Opción A · wizard

```bash
malverde setup
```

### Opción B · manual

```bash
export MALVERDE_PROVIDER=openai        # openai|openrouter|nous|groq|ollama|lmstudio
export MALVERDE_MODEL=gpt-4o-mini
export OPENAI_API_KEY=sk-…             # la clave SIEMPRE por variable de entorno
malverde doctor                        # verificación
```

La configuración persistente vive en `~/.malverde/config.json` (respeta
`MALVERDE_HOME`). Documentación completa: [docs/CONFIG.md](docs/CONFIG.md).

## Verificación sin red

Malverde incluye un proveedor `mock` para probar el harness completo sin API key:

```bash
malverde model set mock
malverde run "usa la herramienta now para darme la hora" --json
```

## Termux (Android/ARM64)

```bash
pkg update && pkg install rust git
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde && chmod +x install.sh && ./install.sh
malverde doctor
```

## Datos que crea Malverde

| Ruta | Contenido |
|---|---|
| `~/.malverde/config.json` | configuración persistente |
| `~/.malverde/malverde.db` | SQLite: sesiones, mensajes, memoria (FTS5) |
| `~/.malverde/skills/` | skills creadas por el agente o el usuario |
| `~/.malverde/cron.json` | trabajos programados |
| `./.malverde-traces/` | trazas JSONL de las ejecuciones |
