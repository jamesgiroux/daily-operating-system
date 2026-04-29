#!/usr/bin/env bash
# DOS-311: enforce universal write fence — production callers of
# `write_intelligence_json` should route through
# `crate::intelligence::write_fence::fenced_write_intelligence_json`.
#
# This lint is intentionally narrow for W1 ship: it warns about NEW direct
# `write_intelligence_json(` call sites added outside the allowlist, but
# does NOT yet require migrating the W0 post-commit warn-log paths in
# `services/intelligence.rs` to the fence (those are the natural fence
# integration points; W3 cleanup migrates them alongside DOS-7's cutover).
#
# Allowlist:
#   - intelligence/write_fence.rs   — the fence module's own internal use
#   - intelligence/io.rs            — the canonical implementation
#   - tests, fixtures, examples
#   - services/intelligence.rs      — W0 post-commit cache writes (transitional;
#                                     migrated in W3 alongside DOS-7)
#   - intel_queue.rs                — the queue worker's final write
#                                     (transitional; migrated in W3)
#
# After W3, the allowlist for `services/intelligence.rs` and `intel_queue.rs`
# is removed.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PATTERN='\bwrite_intelligence_json[[:space:]]*\('

violations=0
while IFS= read -r line; do
  # Path-based allowlist
  case "$line" in
    "$ROOT_DIR/src-tauri/src/intelligence/write_fence.rs"*) continue ;;
    "$ROOT_DIR/src-tauri/src/intelligence/io.rs"*) continue ;;
    "$ROOT_DIR/src-tauri/tests/"*) continue ;;
    "$ROOT_DIR/.docs/"*) continue ;;
    "$ROOT_DIR/scripts/"*) continue ;;
  esac

  # Inline marker exemption: a `// fence-exempt: <reason>` comment within 3
  # lines above the call deliberately bypasses the fence (test cleanup,
  # imports, etc.). Extract <file>:<lineno>:<text> from grep -rEn output.
  file_part="${line%%:*}"
  rest="${line#*:}"
  lineno="${rest%%:*}"
  if [ -n "$lineno" ] && [ -f "$file_part" ]; then
    start=$((lineno - 3))
    [ "$start" -lt 1 ] && start=1
    if sed -n "${start},${lineno}p" "$file_part" 2>/dev/null \
        | grep -q "fence-exempt:"; then
      continue
    fi
  fi

  echo "$line"
  violations=$((violations + 1))
done < <(grep -rEn "$PATTERN" \
  "$ROOT_DIR/src-tauri/src/" \
  2>/dev/null || true)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: ${violations} direct write_intelligence_json call(s) outside fence allowlist."
  echo "Use intelligence::write_fence::fenced_write_intelligence_json (DOS-311)."
  echo "If intentionally adding to a transitional allowlist (e.g., post-W3 cleanup),"
  echo "update scripts/check_write_fence_usage.sh."
  exit 1
fi
