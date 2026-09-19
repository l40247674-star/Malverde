# Malverde Core Framework v0.1.0

> **Un framework modular en Rust para construir sistemas de conocimiento local, aprendizaje autónomo y automatización.**

---

## 🚀 **Características**

✅ **12 crates modulares** (núcleo, eventos, almacenamiento, jobs, CLI, TUI, aprendizaje, seguridad, etc.).
✅ **Almacenamiento con SQLite + FTS5** (búsqueda de texto completo).
✅ **Sistema de eventos asíncronos** (Event Bus).
✅ **Sistema de jobs con checkpoints y recovery** (para tareas largas).
✅ **CLI con comandos útiles** (`malverde doctor`, `malverde workspace audit`, etc.).
✅ **TUI (Interfaz de Terminal)** para interacción visual.
✅ **Motor de aprendizaje local** (sin dependencias externas de IA).
✅ **Validación de seguridad** (datasets, secretos, permisos).
✅ **Soporte para Termux/ARM64** (script `install.sh` incluido).
✅ **Documentación completa** (ARCHITECTURE.md, INSTALL.md, etc.).

---

## 📥 **Instalación**

### **Opción 1: Usando `install.sh` (recomendado para Termux/ARM64)**
```bash
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde
chmod +x install.sh
./install.sh
```

### **Opción 2: Compilación manual**
```bash
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde
cargo build --release
```

---

## 🛠 **Uso Básico**

### **Verificar el entorno**
```bash
malverde doctor
```

### **Auditar el workspace**
```bash
malverde workspace audit
```

### **Iniciar la TUI**
```bash
malverde tui
```

---

## 📂 **Estructura del Proyecto**

```
Malverde/
├── Cargo.toml
├── README.md
├── INSTALL.md
├── ARCHITECTURE.md
├── install.sh
└── crates/
    ├── malverde-core/
    ├── malverde-bus/
    ├── malverde-storage/
    ├── malverde-job/
    ├── malverde-cli/
    ├── malverde-tui/
    ├── malverde-learning/
    ├── malverde-security/
    ├── malverde-knowledge/
    ├── malverde-memory/
    ├── malverde-events/
    └── malverde-plugins/
```

---

## 📚 **Documentación**

- **[ARCHITECTURE.md](ARCHITECTURE.md)** – Arquitectura detallada.
- **[INSTALL.md](INSTALL.md)** – Guía de instalación.

---

## 🤝 **Contribuciones**

Abre un Pull Request o reporta un Issue.

---

## 📜 **Licencia**

MIT.