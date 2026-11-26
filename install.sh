#!/usr/bin/env bash
# Instalador de Malverde (Linux/macOS/Termux).
set -euo pipefail

echo "==> Malverde: instalando dependencias del sistema"
if command -v pkg >/dev/null 2>&1; then
  pkg update -y && pkg install -y rust git
elif command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update && sudo apt-get install -y build-essential curl git
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Instalando rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  . "$HOME/.cargo/env"
fi

echo "==> Compilando (esto puede tardar varios minutos)"
cargo build --release

BIN="$PWD/target/release/malverde"
DEST="$HOME/.local/bin"
mkdir -p "$DEST"
cp "$BIN" "$DEST/malverde"
case ":$PATH:" in
  *":$DEST:"*) ;;
  *) echo "==> Añade $DEST a tu PATH: export PATH=\"$DEST:\$PATH\"" ;;
esac

echo "==> Instalación completada"
"$DEST/malverde" doctor || true
echo "==> Siguiente paso: malverde setup"
