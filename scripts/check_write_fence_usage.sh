#!/usr/bin/env bash
# enforce universal write fence — production callers of
# `write_intelligence_json` should route through
# `crate::intelligence::write_fence::fenced_write_intelligence_json`.
#
# This lint is intentionally narrow for W1 ship: it warns about NEW direct
# `write_intelligence_json(` call sites added outside the allowlist, but
# does NOT yet require migrating the W0 post-commit warn-log paths in
# `services/intelligence.rs` to the fence (those are the natural fence
# integration points; the claims cleanup migrates them alongside the schema
# cutover).
#
# Allowlist:
#   - intelligence/write_fence.rs   — the fence module's own internal use
#   - intelligence/io.rs            — the canonical implementation
#   - tests, fixtures, examples
#   - services/intelligence.rs      — W0 post-commit cache writes (transitional;
#                                     migrated alongside the claims schema cutover)
#   - intel_queue.rs                — the queue worker's final write
#                                     (transitional; migrated alongside the fence)
#
# After the fence migration, the allowlist for `services/intelligence.rs` and
# `intel_queue.rs` is removed.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PATTERN='\bwrite_intelligence_json[[:space:]]*\('

violations=0
check_line() {
  local line="$1"
  # Path-based allowlist
  case "$line" in
    "$ROOT_DIR/src-tauri/src/intelligence/write_fence.rs"*) return 0 ;;
    "$ROOT_DIR/src-tauri/src/intelligence/io.rs"*) return 0 ;;
    "$ROOT_DIR/src-tauri/tests/"*) return 0 ;;
    "$ROOT_DIR/.docs/"*) return 0 ;;
    "$ROOT_DIR/scripts/"*) return 0 ;;
  esac

  # Inline marker exemption: a `// fence-exempt: <reason>` comment within 3
  # lines above the call deliberately bypasses the fence (test cleanup,
  # imports, etc.).
  local file_part="${line%%:*}"
  local rest="${line#*:}"
  local lineno="${rest%%:*}"
  if [ -n "$lineno" ] && [ -f "$file_part" ]; then
    local start=$((lineno - 3))
    [ "$start" -lt 1 ] && start=1
    if sed -n "${start},${lineno}p" "$file_part" 2>/dev/null \
        | grep -q "fence-exempt:"; then
      return 0
    fi
  fi

  echo "$line"
  violations=$((violations + 1))
}

# Direct write_intelligence_json calls (the function-level guard).
while IFS= read -r line; do
  check_line "$line"
done < <(grep -rEn "$PATTERN" "$ROOT_DIR/src-tauri/src/" 2>/dev/null || true)

# also catch any atomic_write_str call whose path
# argument references intelligence.json. The W1 audit found no such
# bypass today; this guard prevents future regressions where a caller
# constructs an intelligence.json path manually and writes it raw.
ATOMIC_PATTERN='atomic_write_str[[:space:]]*\([^)]*intelligence(\.json|/io|_json|"[[:space:]]*[,)])'
while IFS= read -r line; do
  check_line "$line"
done < <(grep -rEn "$ATOMIC_PATTERN" "$ROOT_DIR/src-tauri/src/" 2>/dev/null || true)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: ${violations} direct write_intelligence_json call(s) outside fence allowlist."
  echo "Use intelligence::write_fence::fenced_write_intelligence_json."
  echo "If intentionally adding to a transitional allowlist,"
  echo "update scripts/check_write_fence_usage.sh."
  exit 1
fi
