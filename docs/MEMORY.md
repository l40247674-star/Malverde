# Memoria y sesiones

## Qué se guarda

Todo en `~/.malverde/malverde.db` (SQLite con FTS5):

- **Sesiones**: cada conversación con título automático (primer mensaje) y modelo.
- **Mensajes**: entradas del usuario y respuestas finales del asistente.
- **Hechos**: lo que el agente decide recordar explícitamente.

## Recall cross-sesión

La herramienta `recall` busca simultáneamente en los hechos y en todo el
historial de sesiones con FTS5 (búsqueda de texto completo con ranking):

```text
agente usa recall {"query": "puerto del servidor"}
## Hechos
- [a1b2…] el servidor corre en el puerto 8080

## Historial de sesiones
- [9f0e…] user: subí el servidor al puerto 8080
```

## Nudges

El system prompt lista los últimos 20 hechos y anima al agente a guardar
aprendizajes generales. Esto crea el "ciclo cerrado de aprendizaje": usar →
recordar → recordar en futuras sesiones.

## Gestión manual

```bash
malverde memory remember "tu hecho aquí"
malverde memory search "texto"
malverde memory list
malverde memory forget <id>
```

## Privacidad

Toda la memoria es local (SQLite). Para borrar todo: borra `~/.malverde/malverde.db`.
