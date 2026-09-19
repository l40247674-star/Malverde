#!/bin/bash
set -e

echo "=== Malverde Core Framework v0.1.0 - Install Script ==="
echo ""

if [ -d "$PREFIX" ] && [ -f "$PREFIX/etc/termux-version" ]; then
    echo "Termux detected. Installing dependencies..."
    pkg update -y > /dev/null 2>&1
    pkg upgrade -y > /dev/null 2>&1
    pkg install -y rust git curl wget tar gzip sqlite > /dev/null 2>&1
    echo "Dependencies installed in Termux."
else
    echo "Linux/Unix detected. Checking Rust..."
    if ! command -v rustc &> /dev/null; then
        echo "Rust not found. Installing with rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo "Rust installed."
    else
        echo "Rust is already installed."
    fi
    
    if ! command -v git &> /dev/null; then
        echo "ERROR: Git not found. Install it manually."
        exit 1
    fi
    
    if ! command -v sqlite3 &> /dev/null; then
        echo "ERROR: SQLite3 not found. Install it manually."
        exit 1
    fi
fi

echo "Building Malverde..."
cargo build --release
echo "Build successful."

echo "Installing binary..."
BINARY_PATH="target/release/malverde"
if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$BINARY_PATH" "$INSTALL_DIR/malverde"
chmod +x "$INSTALL_DIR/malverde"

if ! grep -q "$HOME/.local/bin" "$HOME/.bashrc" 2>/dev/null; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
fi

echo "Binary installed to $INSTALL_DIR/malverde"
echo ""
echo "=== Installation Complete! ==="
echo "Run: malverde doctor"