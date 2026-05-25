#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${DAILYOS_ROOT_OVERRIDE:-}" ]]; then
  ROOT_DIR="$DAILYOS_ROOT_OVERRIDE"
elif [[ -d "src-tauri/src" ]]; then
  ROOT_DIR="$(pwd)"
else
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

REGISTRY="$ROOT_DIR/src-tauri/src/lib.rs"

if [[ ! -f "$REGISTRY" ]]; then
  echo "Tauri registry guard: missing $REGISTRY"
  exit 1
fi

violations="$(
  awk '
    /commands::dev_(explore|test|spike|tmp|temp)[A-Za-z0-9_]*/ {
      print FILENAME ":" NR ": temporary dev command registered: " $0
    }
    /temporary dev|temporary exploration|validation spike|spike exploration/ {
      print FILENAME ":" NR ": temporary spike marker in command registry: " $0
    }
  ' "$REGISTRY"
)"

if [[ -n "$violations" ]]; then
  echo "$violations"
  exit 1
fi

echo "Tauri registry guard: no temporary dev commands registered"
