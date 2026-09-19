#!/bin/bash
set -e

echo "=== Malverde Core Framework v0.1.0 - Install Script ==="
echo ""

# Check if in Termux
if [ -d "$PREFIX" ] && [ -f "$PREFIX/etc/termux-version" ]; then
    echo "Detected Termux. Installing dependencies..."
    pkg update -y > /dev/null 2>&1
    pkg upgrade -y > /dev/null 2>&1
    pkg install -y rust git curl wget tar gzip sqlite > /dev/null 2>&1
else
    echo "Linux/Unix detected. Checking Rust..."
    if ! command -v rustc &> /dev/null; then
        echo "Rust not found. Installing with rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    if ! command -v git &> /dev/null; then
        echo "ERROR: Git not found. Install it manually."
        exit 1
    fi
    
    if ! command -v sqlite3 &> /dev/null; then
        echo "WARNING: SQLite3 not found. Trying to install..."
        if command -v apt &> /dev/null; then
            sudo apt update && sudo apt install -y sqlite3 libsqlite3-dev
        elif command -v dnf &> /dev/null; then
            sudo dnf install -y sqlite sqlite-devel
        elif command -v pacman &> /dev/null; then
            sudo pacman -Sy --noconfirm sqlite
        else
            echo "ERROR: Cannot install SQLite3. Install it manually."
            exit 1
        fi
    fi
fi

echo "Building Malverde..."
cargo build --release

echo "Installing binary..."
mkdir -p ~/.local/bin
cp target/release/malverde ~/.local/bin/
chmod +x ~/.local/bin/malverde

echo ""
echo "=== Installation Complete! ==="
echo "Run: malverde doctor"