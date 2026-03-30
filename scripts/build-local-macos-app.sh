#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"
VERSION="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)}"
DATE="$(date +"%Y%m%d.%H%M%S")"
DIST_DIR="$ROOT/dist"
APP_NAME="MacXLR.app"
APP_DIR="$DIST_DIR/$APP_NAME"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"

echo "Building MacXLR release binaries..."
cargo build --manifest-path="$ROOT/Cargo.toml" --release

echo "Creating app bundle at $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

for binary in goxlr-client goxlr-daemon goxlr-launcher goxlr-defaults goxlr-initialiser; do
    cp "$ROOT/target/release/$binary" "$MACOS_DIR/"
done

cp "$ROOT/daemon/resources/icon.icns" "$RESOURCES_DIR/"
sed -e "s/{{VERSION}}/$VERSION/g" \
    -e "s/{{DATE}}/$DATE/g" \
    "$ROOT/ci/macos/Info.plist.template.xml" > "$APP_DIR/Contents/Info.plist"

echo "App bundle ready:"
echo "  $APP_DIR"
echo
echo "Launch it with:"
echo "  open \"$APP_DIR\""
