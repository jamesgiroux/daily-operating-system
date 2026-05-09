#!/usr/bin/env bash
# Guard ADR-0106 prompt fingerprinting boundary.
#
# Low-level canonical hash calls and direct PromptFingerprint construction must
# stay in intelligence/prompt_fingerprint.rs (or the provenance type definition).
# Abilities should call prompt_fingerprint_from_completion instead.

set -euo pipefail

if [ "${1:-}" != "" ]; then
  ROOT_DIR="$(cd "$1" && pwd)"
else
  ROOT_DIR="$(git rev-parse --show-toplevel)"
fi

violations=0

is_allowed_file() {
  case "$1" in
    "$ROOT_DIR/src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs") return 0 ;;
    "$ROOT_DIR/src-tauri/src/intelligence/prompt_fingerprint.rs") return 0 ;;
    "$ROOT_DIR/src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs") return 0 ;;
    "$ROOT_DIR/src-tauri/abilities-runtime/src/abilities/provenance/render.rs") return 0 ;;
    *) return 1 ;;
  esac
}

check_line() {
  local line="$1"
  local file="${line%%:*}"

  case "$line" in
    *"-> PromptFingerprint {"*) return 0 ;;
  esac

  if is_allowed_file "$file"; then
    return 0
  fi

  echo "$line"
  violations=$((violations + 1))
}

while IFS= read -r line; do
  check_line "$line"
done < <(
  grep -RInE '\bcanonical_prompt_hash[[:space:]]*\(|\bCanonicalPromptRequest[[:space:]]*\{|\bPromptFingerprint[[:space:]]*\{' \
    "$ROOT_DIR/src-tauri/src" \
    "$ROOT_DIR/src-tauri/abilities-runtime/src" 2>/dev/null || true
)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: $violations direct prompt fingerprint call(s) outside the provider boundary."
  echo "Use intelligence::prompt_fingerprint::prompt_fingerprint_from_completion."
  exit 1
fi

echo "prompt fingerprint boundary: ok"
