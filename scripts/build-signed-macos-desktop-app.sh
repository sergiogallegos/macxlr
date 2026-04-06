#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1 && pwd)"
ENV_FILE="${MACXLR_SIGNING_ENV_FILE:-$ROOT/.env.macos-desktop-signing}"

if [ -f "$ENV_FILE" ]; then
  echo "Loading signing environment from $ENV_FILE"
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  echo "APPLE_SIGNING_IDENTITY is required for a signed desktop build." >&2
  exit 1
fi

HAS_API_CREDS=0
if [ -n "${APPLE_API_ISSUER:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
  HAS_API_CREDS=1
fi

HAS_APPLE_ID_CREDS=0
if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
  HAS_APPLE_ID_CREDS=1
fi

if [ "$HAS_API_CREDS" -ne 1 ] && [ "$HAS_APPLE_ID_CREDS" -ne 1 ]; then
  cat >&2 <<'EOF'
Signed desktop builds intended for distribution also need notarization credentials.

Provide one of:
  1. APPLE_API_ISSUER + APPLE_API_KEY + APPLE_API_KEY_PATH
  2. APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID

See .env.macos-desktop-signing.example for the expected variables.
EOF
  exit 1
fi

echo "Building signed and notarization-ready MacXLR Desktop.app..."
"$ROOT/scripts/build-native-macos-desktop-app.sh"
