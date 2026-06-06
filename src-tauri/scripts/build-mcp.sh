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
MCP_GUARD_SOURCE_PATHS=(
  "build.rs"
  "Cargo.lock"
  "Cargo.toml"
  "scripts/build-mcp.sh"
  "src/db/core.rs"
  "src/mcp/main.rs"
  "src/mcp/launcher.rs"
  "src/mcp_launcher_contract.rs"
  "src/mcp_runtime_guard_constants.rs"
  "src/services/integrations.rs"
)

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
GENERATIONS_DIR="$BINARIES_DIR/.dailyos-mcp-generations/$TARGET_TRIPLE"
CURRENT_LINK_NAME=".dailyos-mcp-current-$TARGET_TRIPLE"
CURRENT_LINK="$BINARIES_DIR/$CURRENT_LINK_NAME"
SERVER_LINK_TARGET="$CURRENT_LINK_NAME/$SERVER_NAME-$TARGET_TRIPLE"
LAUNCHER_LINK_TARGET="$CURRENT_LINK_NAME/$LAUNCHER_NAME-$TARGET_TRIPLE"
PROVENANCE_LINK_TARGET="$CURRENT_LINK_NAME/$SERVER_NAME-bundle-$TARGET_TRIPLE.provenance.json"
BUILD_LOCK_DIR="$BINARIES_DIR/.build-mcp-$TARGET_TRIPLE.lock"
BUILD_LOCK_ACQUIRED=false
GENERATION_DIR=""
CURRENT_TMP_LINK=""
STABLE_TMP_LINK=""
TMP_SERVER_PATH=""
TMP_LAUNCHER_PATH=""
TMP_PROVENANCE_PATH=""

cleanup() {
  if [ -n "$STABLE_TMP_LINK" ]; then
    rm -f "$STABLE_TMP_LINK"
  fi
  if [ -n "$CURRENT_TMP_LINK" ]; then
    rm -f "$CURRENT_TMP_LINK"
  fi
  if [ -n "$TMP_SERVER_PATH" ]; then
    rm -f "$TMP_SERVER_PATH"
  fi
  if [ -n "$TMP_LAUNCHER_PATH" ]; then
    rm -f "$TMP_LAUNCHER_PATH"
  fi
  if [ -n "$TMP_PROVENANCE_PATH" ]; then
    rm -f "$TMP_PROVENANCE_PATH"
  fi
  if [ "$BUILD_LOCK_ACQUIRED" = true ]; then
    rm -f "$BUILD_LOCK_DIR/pid"
    rmdir "$BUILD_LOCK_DIR" 2>/dev/null || true
  fi
  if [ -n "$GENERATION_DIR" ]; then
    rm -rf "$GENERATION_DIR"
  fi
}
trap cleanup EXIT

for STUB_PATH in "$SERVER_PATH" "$LAUNCHER_PATH"; do
  if [ ! -e "$STUB_PATH" ] && [ -L "$STUB_PATH" ]; then
    rm -f "$STUB_PATH"
  fi
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
    local head
    if ! head="$(git -C "$TAURI_DIR" rev-parse HEAD 2>/dev/null)"; then
      printf '%s' "unknown"
      return
    fi
    local digest
    digest="$(dirty_mcp_guard_digest || true)"
    if [ -n "$digest" ]; then
      printf '%s+dirty.%s' "$head" "${digest:0:12}"
    else
      printf '%s' "$head"
    fi
  fi
}

dirty_mcp_guard_digest() {
  local diff_path
  diff_path="$(mktemp "${TMPDIR:-/tmp}/dailyos-mcp-guard-diff.XXXXXX")"
  git -C "$TAURI_DIR" diff --binary HEAD -- "${MCP_GUARD_SOURCE_PATHS[@]}" > "$diff_path"
  local untracked_path
  while IFS= read -r untracked_path; do
    if [ -f "$TAURI_DIR/$untracked_path" ]; then
      {
        printf '\n-- DAILYOS UNTRACKED MCP GUARD SOURCE --\n'
        printf '%s\n' "$untracked_path"
        cat "$TAURI_DIR/$untracked_path"
        printf '\n'
      } >> "$diff_path"
    fi
  done < <(git -C "$TAURI_DIR" ls-files --others --exclude-standard -- "${MCP_GUARD_SOURCE_PATHS[@]}" | LC_ALL=C sort)
  if [ ! -s "$diff_path" ]; then
    rm -f "$diff_path"
    return 0
  fi
  git -C "$TAURI_DIR" hash-object --stdin < "$diff_path"
  rm -f "$diff_path"
}

sha256_file() {
  shasum -a 256 "$1" | awk '{ print $1 }'
}

fsync_file() {
  python3 - "$1" <<'PY'
import os
import sys

fd = os.open(sys.argv[1], os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY
}

fsync_dir() {
  python3 - "$1" <<'PY'
import errno
import os
import sys

fd = os.open(sys.argv[1], os.O_RDONLY)
try:
    os.fsync(fd)
except OSError as exc:
    expected = (errno.EINVAL, getattr(errno, "ENOTSUP", 95), getattr(errno, "EOPNOTSUPP", 95))
    if exc.errno not in expected:
        raise
finally:
    os.close(fd)
PY
}

replace_path() {
  python3 - "$1" "$2" <<'PY'
import os
import sys

os.replace(sys.argv[1], sys.argv[2])
PY
}

acquire_build_lock() {
  local deadline=$((SECONDS + 60))
  while ! mkdir "$BUILD_LOCK_DIR" 2>/dev/null; do
    if build_lock_is_stale; then
      rm -rf "$BUILD_LOCK_DIR"
      continue
    fi
    if [ "$SECONDS" -ge "$deadline" ]; then
      echo "Timed out waiting for MCP build lock: $BUILD_LOCK_DIR" >&2
      return 1
    fi
    sleep 0.1
  done
  BUILD_LOCK_ACQUIRED=true
  printf '%s\n' "$$" > "$BUILD_LOCK_DIR/pid"
  fsync_dir "$BUILD_LOCK_DIR"
}

release_build_lock() {
  if [ "$BUILD_LOCK_ACQUIRED" = true ]; then
    rm -f "$BUILD_LOCK_DIR/pid"
    rmdir "$BUILD_LOCK_DIR" 2>/dev/null || true
    BUILD_LOCK_ACQUIRED=false
  fi
}

build_lock_is_stale() {
  local pid=""
  if [ -f "$BUILD_LOCK_DIR/pid" ]; then
    pid="$(cat "$BUILD_LOCK_DIR/pid" 2>/dev/null || true)"
  fi
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    return 1
  fi

  local mtime=""
  mtime="$(stat -f %m "$BUILD_LOCK_DIR" 2>/dev/null || stat -c %Y "$BUILD_LOCK_DIR" 2>/dev/null || true)"
  if [ -z "$mtime" ]; then
    return 1
  fi
  local now
  now="$(date +%s)"
  [ $((now - mtime)) -ge 60 ]
}

write_provenance() {
  local output_path="$1"
  local stub="$2"
  local build_sha="$3"
  local server_sha="$4"
  local launcher_sha="$5"
  local generated_at
  generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  cat > "$output_path" <<EOF
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

provenance_matches_current_artifacts() {
  python3 - "$PROVENANCE_PATH" "$SERVER_PATH" "$LAUNCHER_PATH" "$SERVER_NAME" "$LAUNCHER_NAME" <<'PY'
import hashlib
import json
import os
import sys

provenance_path, server_path, launcher_path, server_name, launcher_name = sys.argv[1:]

try:
    with open(provenance_path, "r", encoding="utf-8") as handle:
        provenance = json.load(handle)
    if provenance.get("stub") is True:
        sys.exit(0)
    sidecars = {sidecar.get("name"): sidecar for sidecar in provenance.get("sidecars", [])}
    for sidecar_name, artifact_path in ((server_name, server_path), (launcher_name, launcher_path)):
        expected_sha = sidecars.get(sidecar_name, {}).get("sha256")
        if not expected_sha or not os.path.isfile(artifact_path):
            sys.exit(1)
        digest = hashlib.sha256()
        with open(artifact_path, "rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        if digest.hexdigest() != expected_sha:
            sys.exit(1)
except Exception:
    sys.exit(1)
PY
}

create_generation_dir() {
  local label="$1"
  local generation_name
  generation_name="$(date -u +"%Y%m%dT%H%M%SZ")-$label-$$"
  mkdir -p "$GENERATIONS_DIR"
  GENERATION_DIR="$GENERATIONS_DIR/$generation_name"
  mkdir "$GENERATION_DIR"
  fsync_dir "$GENERATIONS_DIR"
}

install_stable_symlink() {
  local link_path="$1"
  local target="$2"
  if [ -L "$link_path" ] && [ "$(readlink "$link_path")" = "$target" ]; then
    return
  fi

  STABLE_TMP_LINK="$link_path.linktmp.$$"
  rm -f "$STABLE_TMP_LINK"
  ln -s "$target" "$STABLE_TMP_LINK"
  mv -f "$STABLE_TMP_LINK" "$link_path"
  STABLE_TMP_LINK=""
}

publish_generation() {
  local generation_dir="$1"
  local generation_name
  generation_name="$(basename "$generation_dir")"
  CURRENT_TMP_LINK="$CURRENT_LINK.tmp.$$"
  rm -f "$CURRENT_TMP_LINK"
  ln -s ".dailyos-mcp-generations/$TARGET_TRIPLE/$generation_name" "$CURRENT_TMP_LINK"

  acquire_build_lock
  install_stable_symlink "$SERVER_PATH" "$SERVER_LINK_TARGET"
  install_stable_symlink "$LAUNCHER_PATH" "$LAUNCHER_LINK_TARGET"
  install_stable_symlink "$PROVENANCE_PATH" "$PROVENANCE_LINK_TARGET"
  replace_path "$CURRENT_TMP_LINK" "$CURRENT_LINK"
  CURRENT_TMP_LINK=""
  GENERATION_DIR=""
  fsync_dir "$BINARIES_DIR"
  release_build_lock
}

create_stub_generation() {
  local build_sha="$1"
  create_generation_dir "stub"
  TMP_SERVER_PATH="$GENERATION_DIR/$SERVER_NAME-$TARGET_TRIPLE"
  TMP_LAUNCHER_PATH="$GENERATION_DIR/$LAUNCHER_NAME-$TARGET_TRIPLE"
  TMP_PROVENANCE_PATH="$GENERATION_DIR/$SERVER_NAME-bundle-$TARGET_TRIPLE.provenance.json"
  touch "$TMP_SERVER_PATH" "$TMP_LAUNCHER_PATH"
  write_provenance "$TMP_PROVENANCE_PATH" true "$build_sha" "stub" "stub"
  fsync_file "$TMP_SERVER_PATH"
  fsync_file "$TMP_LAUNCHER_PATH"
  fsync_file "$TMP_PROVENANCE_PATH"
  fsync_dir "$GENERATION_DIR"
  TMP_SERVER_PATH=""
  TMP_LAUNCHER_PATH=""
  TMP_PROVENANCE_PATH=""
  publish_generation "$GENERATION_DIR"
}

if [ "$STUB_ONLY" = true ]; then
  BUILD_SHA="$(resolve_build_sha)"
  if [ -e "$PROVENANCE_PATH" ] && provenance_matches_current_artifacts; then
    echo "Existing provenance preserved: $PROVENANCE_PATH"
  else
    create_stub_generation "$BUILD_SHA"
    echo "Stub provenance ready: $PROVENANCE_PATH"
  fi
  echo "Stubs ready: $SERVER_PATH $LAUNCHER_PATH"
  exit 0
fi

BUILD_SHA="$(resolve_build_sha)"
echo "Building MCP sidecars for target: $TARGET_TRIPLE"

cargo build \
  --manifest-path "$TAURI_DIR/Cargo.toml" \
  --release \
  --features mcp \
  --bin dailyos-mcp \
  --bin "$LAUNCHER_BIN_TARGET" \
  --target "$TARGET_TRIPLE"

create_generation_dir "release"
TMP_SERVER_PATH="$GENERATION_DIR/$SERVER_NAME-$TARGET_TRIPLE"
TMP_LAUNCHER_PATH="$GENERATION_DIR/$LAUNCHER_NAME-$TARGET_TRIPLE"
TMP_PROVENANCE_PATH="$GENERATION_DIR/$SERVER_NAME-bundle-$TARGET_TRIPLE.provenance.json"

cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/$SERVER_NAME" \
   "$TMP_SERVER_PATH"
cp "$TAURI_DIR/target/$TARGET_TRIPLE/release/$LAUNCHER_BIN_TARGET" \
   "$TMP_LAUNCHER_PATH"
chmod +x "$TMP_SERVER_PATH" "$TMP_LAUNCHER_PATH"

SERVER_SHA="$(sha256_file "$TMP_SERVER_PATH")"
LAUNCHER_SHA="$(sha256_file "$TMP_LAUNCHER_PATH")"
write_provenance "$TMP_PROVENANCE_PATH" false "$BUILD_SHA" "$SERVER_SHA" "$LAUNCHER_SHA"

fsync_file "$TMP_SERVER_PATH"
fsync_file "$TMP_LAUNCHER_PATH"
fsync_file "$TMP_PROVENANCE_PATH"
fsync_dir "$GENERATION_DIR"
TMP_SERVER_PATH=""
TMP_LAUNCHER_PATH=""
TMP_PROVENANCE_PATH=""

publish_generation "$GENERATION_DIR"

echo "MCP sidecar ready: $SERVER_PATH"
echo "MCP launcher ready: $LAUNCHER_PATH"
echo "MCP provenance ready: $PROVENANCE_PATH"
