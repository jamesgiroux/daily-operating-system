#!/usr/bin/env bash
# .claude/hooks/post-edit-check.sh — PostToolUse advisory lint.
#
# After an Edit/Write/MultiEdit, runs a fast, file-scoped lint and surfaces
# any failure as stderr (exit 1, advisory — does not block, but Claude sees it).
#
# Scope: deliberately fast checks only.
#   - .ts / .tsx → eslint --quiet on the single file (~1s warm)
#   - .rs        → cargo clippy is too slow per-edit; we skip and rely on
#                  .githooks/pre-commit + LSP. Surface only a one-line nudge.
#   - everything else → no-op
#
# Why advisory:
#   - Pre-commit already gates the heavy checks
#   - In-loop signal helps catch obvious errors before the next edit compounds them
#   - Blocking on every lint slip would be intolerable

set -euo pipefail

input="$(cat)"

file_path="$(printf '%s' "$input" | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    ti = data.get("tool_input", {})
    p = ti.get("file_path") or ti.get("path") or ""
    print(p)
except Exception:
    print("")
' 2>/dev/null || true)"

[ -z "$file_path" ] && exit 0

REPO_ROOT="$(git -C "$(dirname "$file_path")" rev-parse --show-toplevel 2>/dev/null || true)"
[ -z "$REPO_ROOT" ] && exit 0

cd "$REPO_ROOT"

# Only check files under src/ — frontend lint scope.
case "$file_path" in
  "$REPO_ROOT"/src/*.ts|"$REPO_ROOT"/src/*.tsx|"$REPO_ROOT"/src/**/*.ts|"$REPO_ROOT"/src/**/*.tsx)
    if command -v pnpm >/dev/null 2>&1 && [ -d node_modules/eslint ]; then
      out="$(pnpm exec eslint --quiet --no-warn-ignored "$file_path" 2>&1 || true)"
      if [ -n "$out" ] && printf '%s' "$out" | grep -qE 'error|problem'; then
        echo "post-edit-check: eslint surfaced issues in $file_path" >&2
        printf '%s\n' "$out" >&2
        exit 1
      fi
    fi
    ;;
  "$REPO_ROOT"/src-tauri/src/*.rs|"$REPO_ROOT"/src-tauri/src/**/*.rs)
    # Clippy per-edit is too slow. One-line nudge if the file added obvious lint bait
    # (unwrap, todo!, dbg!). Cheap grep, no compiler invocation.
    if grep -nE '\b(unwrap_or_else\s*\(\s*\|_\|\s*panic|^\s*todo!\(|^\s*dbg!\()' "$file_path" >/dev/null 2>&1; then
      echo "post-edit-check: $file_path contains todo!/dbg!/panic-style code — pre-commit clippy will fail" >&2
      exit 1
    fi
    ;;
esac

exit 0
