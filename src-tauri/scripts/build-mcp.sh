#!/usr/bin/env bash
# Build the DailyOS MCP sidecars and place them where Tauri's externalBin expects them.
# Usage:
#   bash src-tauri/scripts/build-mcp.sh                  # auto-detect target triple
#   bash src-tauri/scripts/build-mcp.sh aarch64-apple-darwin  # explicit triple (CI)
#   bash src-tauri/scripts/build-mcp.sh --stub           # create stubs only (fast, for postinstall)

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
SERVER_NAME="dailyos-mcp"
LAUNCHER_NAME="dailyos-mcp-launcher"
LAUNCHER_BIN_TARGET="dailyos-mcp-launcher-bin"
GUARD_EPOCH="dailyos-mcp-runtime-guard:v1"

# Resolve target triple: use arg if provided, otherwise detect from rustc
if [ -n "$ARG" ]; then
  TARGET_TRIPLE="$ARG"
else
  TARGET_TRIPLE=$(rustc -vV | awk '/^host:/ { print $2 }')
fi

# Create stubs so Tauri's build.rs passes externalBin validation during cargo build.
# Real binaries overwrite these stubs after compilation.
mkdir -p "$BINARIES_DIR"
SERVER_PATH="$BINARIES_DIR/$SERVER_NAME-$TARGET_TRIPLE"
LAUNCHER_PATH="$BINARIES_DIR/$LAUNCHER_NAME-$TARGET_TRIPLE"
PROVENANCE_PATH="$BINARIES_DIR/$SERVER_NAME-bundle-$TARGET_TRIPLE.provenance.json"

for STUB_PATH in "$SERVER_PATH" "$LAUNCHER_PATH"; do
  if [ ! -e "$STUB_PATH" ]; then
    touch "$STUB_PATH"
  fi
done

resolve_build_sha() {
  if [ -n "${DAILYOS_BUILD_SHA:-}" ]; then
    printf '%s' "$DAILYOS_BUILD_SHA"
  elif [ -n "${GITHUB_SHA:-}" ]; then
    printf '%s' "$GITHUB_SHA"
  else
    git -C "$TAURI_DIR" rev-parse HEAD 2>/dev/null || printf '%s' "unknown"
  fi
}

sha256_file() {
  shasum -a 256 "$1" | awk '{ print $1 }'
}

write_provenance() {
  local stub="$1"
  local build_sha="$2"
  local server_sha="$3"
  local launcher_sha="$4"
  local generated_at
  generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  cat > "$PROVENANCE_PATH" <<EOF
{
  "schemaVersion": 1,
  "guardEpoch": "$GUARD_EPOCH",
  "targetTriple": "$TARGET_TRIPLE",
  "appBuildSha": "$build_sha",
  "generatedAt": "$generated_at",
  "stub": $stub,
  "sidecars": [
    {
      "name": "$SERVER_NAME",
      "filename": "$SERVER_NAME-$TARGET_TRIPLE",
      "buildSha": "$build_sha",
      "sha256": "$server_sha",
      "stub": $stub
    },
    {
      "name": "$LAUNCHER_NAME",
      "filename": "$LAUNCHER_NAME-$TARGET_TRIPLE",
      "buildSha": "$build_sha",
      "sha256": "$launcher_sha",
      "stub": $stub
    }
  ]
}
EOF
}

if [ "$STUB_ONLY" = true ]; then
  BUILD_SHA="$(resolve_build_sha)"
  write_provenance true "$BUILD_SHA" "stub" "stub"
  echo "Stubs ready: $SERVER_PATH $LAUNCHER_PATH"
  echo "Stub provenance ready: $PROVENANCE_PATH"
  exit 0
fi

echo "Building MCP sidecars for target: $TARGET_TRIPLE"

cargo build \
  --manifest-path "$TAURI_DIR/Cargo.toml" \
  --release \
  --features mcp \
  --bin dailyos-mcp \
  --bin "$LAUNCHER_BIN_TARGET" \
  --target "$TARGET_TRIPLE"

cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/$SERVER_NAME" \
   "$SERVER_PATH"
cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/$LAUNCHER_BIN_TARGET" \
   "$LAUNCHER_PATH"
chmod +x "$SERVER_PATH" "$LAUNCHER_PATH"

BUILD_SHA="$(resolve_build_sha)"
SERVER_SHA="$(sha256_file "$SERVER_PATH")"
LAUNCHER_SHA="$(sha256_file "$LAUNCHER_PATH")"
write_provenance false "$BUILD_SHA" "$SERVER_SHA" "$LAUNCHER_SHA"

echo "MCP sidecar ready: $SERVER_PATH"
echo "MCP launcher ready: $LAUNCHER_PATH"
echo "MCP provenance ready: $PROVENANCE_PATH"
