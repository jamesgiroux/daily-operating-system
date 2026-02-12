#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TAURI_MANIFEST="$ROOT_DIR/src-tauri/Cargo.toml"
BIN_PATH="$ROOT_DIR/src-tauri/target/release/dailyos"

cargo build --manifest-path "$TAURI_MANIFEST" --release

if [[ ! -f "$BIN_PATH" ]]; then
  echo "binary-not-found: $BIN_PATH" >&2
  exit 1
fi

SIZE_BYTES="$(stat -f%z "$BIN_PATH")"
echo "binary=$BIN_PATH"
echo "size_bytes=$SIZE_BYTES"
echo "size_mib=$(awk "BEGIN { printf \"%.2f\", $SIZE_BYTES/1024/1024 }")"
