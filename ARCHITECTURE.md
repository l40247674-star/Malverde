# Arquitectura – Malverde Core Framework v0.1.0

> **Descripción detallada de la arquitectura modular del framework.**

---

## 🏗 **Visión General**

Malverde es un **framework modular** diseñado para:
- **Almacenamiento local de conocimiento** (SQLite + FTS5).
- **Procesamiento de eventos asíncronos** (Event Bus).
- **Ejecución de tareas largas** (Job System con checkpoints).
- **Aprendizaje autónomo** (sin dependencias externas de IA).

---

## 📦 **Crates del Framework**

### **1. `malverde-core`**
> **Núcleo del framework.**
- Tipos base (`KnowledgeId`, `MemoryId`, `EventId`, etc.).
- Configuración global (`Config`).
- Manejo de errores (`MalverdeError`).

### **2. `malverde-bus`**
> **Sistema de eventos asíncronos (Event Bus).**
- Publicar/suscribirse a eventos.
- Soporte para handlers asíncronos.

### **3. `malverde-storage`**
> **Almacenamiento en SQLite + FTS5.**
- Tablas para `knowledge`, `memory`, `events`, `jobs`, `checkpoints`, `audit`, `evidence`, `plugins`.
- Migraciones automáticas.
- Búsqueda de texto completo (FTS5).

### **4. `malverde-job`**
> **Sistema de jobs con checkpoints y recovery.**
- Ejecución de tareas largas.
- Guardado de estado (checkpoints).

### **5. `malverde-cli`**
> **Interfaz de línea de comandos (CLI).**
- `malverde doctor` – Verifica el entorno.
- `malverde workspace audit` – Audita el workspace.

### **6. `malverde-tui`**
> **Interfaz de terminal (TUI).**
- Visualización interactiva de datos.

### **7. `malverde-learning`**
> **Motor de aprendizaje local.**
- Procesamiento de texto.
- Extracción de patrones.

### **8. `malverde-security`**
> **Validación de seguridad.**
- Validación de datasets.
- Detección de secretos.

### **9-12. Otros módulos**
- `malverde-knowledge` – Módulo de conocimiento.
- `malverde-memory` – Módulo de memoria.
- `malverde-events` – Módulo de eventos.
- `malverde-plugins` – Sistema de plugins.

---

## 🔄 **Flujo de Datos**

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Malverde Core Framework                          │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐ │
│  │   CLI/TUI    │    │   Plugins   │    │       Event Bus              │ │
│  └──────┬───────┘    └──────┬───────┘    └──────────────┬─────────────┘ │
│         │                  │                        │                  │
│         ▼                  ▼                        ▼                  │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                        Core Module                              │ │
│  └──────────────────────────────┬──────────────────────────────────┘ │
│                                  │                                     │
│         ┌────────────────────────┼─────────────────────────┐        │
│         ▼                        ▼                         ▼         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐   │
│  │   Storage    │    │    Jobs     │    │      Learning        │   │
│  └─────────────┘    └─────────────┘    └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```