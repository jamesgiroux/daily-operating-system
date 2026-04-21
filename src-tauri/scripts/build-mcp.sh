#!/usr/bin/env bash
# Build the dailyos-mcp sidecar binary and place it where Tauri's externalBin expects it.
# Usage:
#   bash src-tauri/scripts/build-mcp.sh                  # auto-detect target triple
#   bash src-tauri/scripts/build-mcp.sh aarch64-apple-darwin  # explicit triple (CI)
#   bash src-tauri/scripts/build-mcp.sh --stub           # create empty stub only (fast, for postinstall)

set -euo pipefail

STUB_ONLY=false
ARG="${1:-}"
if [ "$ARG" = "--stub" ]; then
  STUB_ONLY=true
  ARG=""
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TAURI_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$TAURI_DIR/binaries"

# Resolve target triple: use arg if provided, otherwise detect from rustc
if [ -n "$ARG" ]; then
  TARGET_TRIPLE="$ARG"
else
  TARGET_TRIPLE=$(rustc -vV | awk '/^host:/ { print $2 }')
fi

# Create stub so Tauri's build.rs passes externalBin validation during cargo build.
# The real binary overwrites this stub after compilation.
mkdir -p "$BINARIES_DIR"
STUB_PATH="$BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"
if [ ! -e "$STUB_PATH" ]; then
  touch "$STUB_PATH"
fi

if [ "$STUB_ONLY" = true ]; then
  echo "Stub ready: $STUB_PATH"
  exit 0
fi

echo "Building dailyos-mcp for target: $TARGET_TRIPLE"

cargo build \
  --manifest-path "$TAURI_DIR/Cargo.toml" \
  --release \
  --features mcp \
  --bin dailyos-mcp \
  --target "$TARGET_TRIPLE"

cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/dailyos-mcp" \
   "$BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"
chmod +x "$BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"

echo "Sidecar ready: $BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"
