# Tareas programadas (cron)

Malverde incluye su propio cron: un prompt del agente se ejecuta de forma
desatendida cuando la expresión coincide.

## Comandos

```bash
# Añadir: nombre, expresión cron de 5 campos y prompt
malverde cron add resumen-diario "0 9 * * *" --prompt "Resume los commits de ayer del repo actual"

malverde cron list
malverde cron disable <id>
malverde cron enable <id>
malverde cron remove <id>

# Daemon (Ctrl+C para salir)
malverde cron run
```

## Formato cron

`min hora día-del-mes mes día-de-la-semana` con soporte para `*`, listas (`,`),
rangos (`-`) y pasos (`*/n`). Semántica clásica Vixie: si restringes tanto
día-del-mes como día-de-la-semana, coincide con cualquiera de los dos.

```
*/15 9-17 * * 1-5   → cada 15 min, de 9 a 17, lunes a viernes
0 9 1 * *           → 09:00 el día 1 de cada mes
30 22 * * 0         → 22:30 los domingos
```

## Detalles

- Los trabajos viven en `~/.malverde/cron.json`.
- El daemon revisa cada 30 s; cada trabajo guarda `last_run` para no duplicar.
- Las ejecuciones usan sesión propia (`cron-<id>`), quedan en la memoria y
  producen trazas JSONL como cualquier otra ejecución.
- Sin aprobador humano: si el modo de permisos lo bloquea, el trabajo falla
  cerrado (nunca ejecuta sin permiso).
