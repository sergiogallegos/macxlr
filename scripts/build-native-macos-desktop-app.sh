#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"
DESKTOP_DIR="$ROOT/desktop-ui"
TAURI_DIR="$DESKTOP_DIR/src-tauri"
BIN_DIR="$TAURI_DIR/binaries"
DIST_DIR="$ROOT/dist"
APP_NAME="MacXLR Desktop.app"
APP_SRC="$TAURI_DIR/target/release/bundle/macos/$APP_NAME"
APP_DST="$DIST_DIR/$APP_NAME"
HOST_TUPLE="$(rustc --print host-tuple)"
SIDECAR_NAME="goxlr-daemon-$HOST_TUPLE"

mkdir -p "$BIN_DIR" "$DIST_DIR"

echo "Building goxlr-daemon release binary..."
cargo build -p goxlr-daemon --release --manifest-path="$ROOT/Cargo.toml"

echo "Preparing bundled daemon sidecar..."
cp "$ROOT/target/release/goxlr-daemon" "$BIN_DIR/$SIDECAR_NAME"
chmod +x "$BIN_DIR/$SIDECAR_NAME"

cd "$DESKTOP_DIR"

if [ ! -d node_modules ]; then
  echo "Installing desktop UI dependencies..."
  npm install
fi

echo "Building MacXLR Desktop.app..."
npm run build

echo "Copying app bundle to $APP_DST"
rm -rf "$APP_DST"
cp -R "$APP_SRC" "$APP_DST"

DMG_SRC="$(find "$TAURI_DIR/target/release/bundle/dmg" -maxdepth 1 -name 'MacXLR Desktop_*.dmg' 2>/dev/null | head -n1 || true)"
if [ -n "${DMG_SRC:-}" ] && [ -f "$DMG_SRC" ]; then
  DMG_DST="$DIST_DIR/$(basename "$DMG_SRC")"
  echo "Copying DMG to $DMG_DST"
  cp "$DMG_SRC" "$DMG_DST"
fi

echo
echo "Desktop app ready:"
echo "  $APP_DST"
if [ -n "${DMG_DST:-}" ]; then
  echo "  $DMG_DST"
fi
echo
echo "You can now drag it into /Applications and launch it like a normal macOS app."
