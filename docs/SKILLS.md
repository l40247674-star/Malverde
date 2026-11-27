# Skills

Una skill es un paquete de instrucciones reutilizables en un archivo `SKILL.md`,
compatible con el estándar abierto skills-type (portable entre harnesses).

## Formato

```
~/.malverde/skills/
└── reporte/
    └── SKILL.md
```

```markdown
---
name: reporte
description: Genera el reporte semanal del repo
---

# Reporte semanal

1. Ejecuta `git log --since="7 days ago" --oneline`.
2. Agrupa los cambios por área.
3. Guarda el resultado en `reportes/<fecha>.md`.
```

## Carga

Malverde lee skills de: `./skills` (repo), `~/.malverde/skills` (usuario) y
`config.skills_dirs`. El índice (nombre + descripción) se inyecta en el system
prompt de cada ejecución.

## CLI

```bash
malverde skills list
malverde skills create <nombre> <descripción>   # esqueleto listo para editar
malverde skills path
```

## Creación autónoma y auto-mejora

El agente puede crear skills cuando detecta repetición (varios turnos con el
mismo patrón de herramientas) o tras una corrección del usuario que enseñe algo
general. `SkillRegistry::improve` añade notas de mejora timestampadas al final
del `SKILL.md`, de modo que la skill se afina con el uso.
