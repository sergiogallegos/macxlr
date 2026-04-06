#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"

cd "$ROOT"
cargo build -p goxlr-daemon

cd "$ROOT/desktop-ui"
if [ ! -d node_modules ]; then
  npm install
fi
npm run dev
