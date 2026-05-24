#!/usr/bin/env bash
# .claude/hooks/pre-edit-guard.sh — PreToolUse blocklist guard.
#
# Reads Claude Code hook JSON from stdin, extracts the target file_path,
# and blocks edits to invariant files (lock files, .env, the three sync'd
# version files when edited individually). Exit code 2 blocks the tool call
# and surfaces stderr to Claude.
#
# Why this exists:
#   - pnpm-lock.yaml / Cargo.lock should change via the package manager, not by hand
#   - .env / pii-blocklist files leak secrets / PII
#   - tauri.conf.json + Cargo.toml + package.json are CLAUDE.md three-file invariant —
#     editing one in isolation drifts; use /version-bump or edit all three in one turn
#
# Bypass: Claude can justify and try again (the block is informational, not absolute).

set -euo pipefail

# Hook input arrives on stdin as JSON. Read it once.
input="$(cat)"

# Try to extract file_path; works for Edit, Write, MultiEdit shapes.
# Falls through quietly if the tool isn't an edit/write.
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

# Normalize: strip repo prefix so patterns work on relative paths too.
rel="${file_path#*/dailyos-repo/}"
base="$(basename "$file_path")"

block() {
  echo "pre-edit-guard: BLOCKED edit to $rel" >&2
  echo "  reason: $1" >&2
  echo "  override: justify why this edit is correct, then re-attempt." >&2
  exit 2
}

# 1. Lock files — package manager owns these.
case "$base" in
  pnpm-lock.yaml|package-lock.json|yarn.lock)
    block "lock files change via the package manager (pnpm install / add / update), not by hand"
    ;;
  Cargo.lock)
    block "Cargo.lock changes via cargo (add/update/build), not by hand"
    ;;
esac

# 2. .env files — secret leak surface.
case "$base" in
  .env|.env.*|*.env)
    block ".env files may contain secrets; edit via env-var injection or document the change in a non-env file"
    ;;
esac

# 3. PII blocklist — repo policy says edit via the canonical file at ~/Documents/.claude/pii-blocklist.txt,
#    not the symlink target in the worktree (per feature_claude_tracking_permanent_fix.md).
case "$rel" in
  .claude/pii-blocklist.txt|.claude/pii-blocklist)
    block "PII blocklist source-of-truth is ~/Documents/.claude/pii-blocklist.txt; worktree copy is a symlink — edit the source"
    ;;
esac

# 4. Three sync'd version files — block solo edits, encourage /version-bump.
#    Only blocks if the edit looks like a version bump (touches the "version" line).
#    Detection: peek at tool_input.new_string / old_string for a version pattern.
case "$rel" in
  src-tauri/tauri.conf.json|src-tauri/Cargo.toml|package.json)
    touches_version="$(printf '%s' "$input" | python3 -c '
import json, sys, re
try:
    data = json.load(sys.stdin)
    ti = data.get("tool_input", {})
    blob = (ti.get("new_string", "") or "") + (ti.get("old_string", "") or "") + (ti.get("content", "") or "")
    print("yes" if re.search(r"\"?version\"?\s*[:=]\s*\"?\d+\.\d+\.\d+", blob) else "no")
except Exception:
    print("no")
' 2>/dev/null || echo "no")"
    if [ "$touches_version" = "yes" ]; then
      block "version bump detected in a single file — use /version-bump <ver> to keep tauri.conf.json + Cargo.toml + package.json in sync"
    fi
    ;;
esac

exit 0
