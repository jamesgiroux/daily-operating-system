#!/usr/bin/env bash
# Build the dailyos-mcp sidecar binary and place it where Tauri's externalBin expects it.
# Usage:
#   bash src-tauri/scripts/build-mcp.sh                  # auto-detect target triple
#   bash src-tauri/scripts/build-mcp.sh aarch64-apple-darwin  # explicit triple (CI)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TAURI_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$TAURI_DIR/binaries"

# Resolve target triple: use arg if provided, otherwise detect from rustc
if [ -n "${1:-}" ]; then
  TARGET_TRIPLE="$1"
else
  TARGET_TRIPLE=$(rustc -vV | awk '/^host:/ { print $2 }')
fi

echo "Building dailyos-mcp for target: $TARGET_TRIPLE"

cargo build \
  --manifest-path "$TAURI_DIR/Cargo.toml" \
  --release \
  --features mcp \
  --bin dailyos-mcp \
  --target "$TARGET_TRIPLE"

mkdir -p "$BINARIES_DIR"
cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/dailyos-mcp" \
   "$BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"
chmod +x "$BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"

echo "Sidecar ready: $BINARIES_DIR/dailyos-mcp-$TARGET_TRIPLE"
