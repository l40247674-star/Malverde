# Guía de Instalación – Malverde Core Framework v0.1.0

> **Instrucciones detalladas para instalar el framework en Termux (Android) o sistemas Linux/Unix.**

---

## 📌 **Requisitos**

### **Termux (Android)**
- **Termux** (versión reciente).
- **Paquetes necesarios:**
  ```bash
  pkg update && pkg upgrade
  pkg install rust git curl wget tar gzip
  ```

### **Linux/Unix (PC)**
- **Rust** (versión 1.70+).
- **Git**.
- **SQLite 3** (para almacenamiento).

---

## 🚀 **Instalación en Termux (recomendado)**

### **Paso 1: Clona el repositorio**
```bash
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde
```

### **Paso 2: Ejecuta el script de instalación**
```bash
chmod +x install.sh
./install.sh
```

### **Paso 3: Verifica la instalación**
```bash
malverde doctor
```

---

## 🖥 **Instalación en Linux/Unix (PC)**

### **Paso 1: Instala Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### **Paso 2: Clona el repositorio**
```bash
git clone https://github.com/l40247674-star/Malverde.git
cd Malverde
```

### **Paso 3: Compila el proyecto**
```bash
cargo build --release
```

### **Paso 4: Instala el binario (opcional)**
```bash
sudo cp target/release/malverde /usr/local/bin/
```

### **Paso 5: Verifica la instalación**
```bash
malverde doctor
```

---

## ⚙ **Configuración Inicial**

### **Inicializar el workspace**
```bash
malverde workspace init
```

---

## 🎯 **Próximos Pasos**

- **[ARCHITECTURE.md](ARCHITECTURE.md)** – Aprende cómo funciona el framework.
- **Ejecuta `malverde tui`** para probar la interfaz de terminal.