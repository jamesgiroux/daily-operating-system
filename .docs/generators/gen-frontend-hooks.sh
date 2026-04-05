#!/bin/bash
# Generates .docs/architecture/FRONTEND-HOOKS.md from React hook source files.
# Extracts Tauri invoke() calls, event listeners, and consumer components.
# macOS-compatible (no grep -P).

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOKS_DIR="$ROOT/src/hooks"
OUT="$ROOT/.docs/architecture/FRONTEND-HOOKS.md"

TOTAL_FILES=$(ls "$HOOKS_DIR"/*.ts "$HOOKS_DIR"/*.tsx 2>/dev/null | wc -l | tr -d ' ')
TOTAL_LINES=$(cat "$HOOKS_DIR"/*.ts "$HOOKS_DIR"/*.tsx 2>/dev/null | wc -l | tr -d ' ')

{
  echo "# Frontend Hooks Reference"
  echo ""
  echo "> Registry of all React hooks in \`src/hooks/\`."
  echo "> **Auto-generated:** $(date +%Y-%m-%d) by \`.docs/generators/gen-frontend-hooks.sh\`"
  echo ""
  echo "**$TOTAL_FILES** hook files | **$TOTAL_LINES** total lines"
  echo ""
  echo "---"
  echo ""
  echo "## Hook Registry"
  echo ""
  echo "| Hook | File | Lines | Tauri Commands | Events Listened |"
  echo "|------|------|-------|---------------|-----------------|"

  for file in $(ls "$HOOKS_DIR"/*.ts "$HOOKS_DIR"/*.tsx 2>/dev/null | sort); do
    fname=$(basename "$file")
    lines=$(wc -l < "$file" | tr -d ' ')

    # Extract hook name — try function export, then const export, then filename
    hook_name=$(grep -oE 'export (default )?function use[A-Za-z]+' "$file" 2>/dev/null | head -1 | sed -E 's/export (default )?function //' || true)
    if [ -z "$hook_name" ]; then
      hook_name=$(grep -oE 'export const use[A-Za-z]+' "$file" 2>/dev/null | head -1 | sed 's/export const //' || true)
    fi
    if [ -z "$hook_name" ]; then
      hook_name=$(basename "$file" | sed 's/\.tsx\?$//')
    fi

    # Extract Tauri invoke commands (macOS-compatible)
    commands=$(grep -oE 'invoke[^(]*\("[a-z_]+"' "$file" 2>/dev/null | sed -E 's/invoke[^(]*\("//' | sed 's/"$//' | sort -u | tr '\n' ', ' | sed 's/,$//' | sed 's/,/, /g' || true)
    [ -z "$commands" ] && commands="—"

    # Extract Tauri event listeners
    events=$(grep -oE '(listen|useTauriEvent)\(.*"[a-z][-a-z]+"' "$file" 2>/dev/null | grep -oE '"[a-z][-a-z]+"' | tr -d '"' | sort -u | tr '\n' ', ' | sed 's/,$//' | sed 's/,/, /g' || true)
    [ -z "$events" ] && events="—"

    echo "| \`$hook_name\` | \`$fname\` | $lines | $commands | $events |"
  done

  echo ""
  echo "---"
  echo ""
  echo "## Command Usage Summary"
  echo ""
  echo "All Tauri commands invoked from hooks:"
  echo ""

  grep -rhoE 'invoke[^(]*\("[a-z_]+"' "$HOOKS_DIR/" 2>/dev/null | sed -E 's/invoke[^(]*\("//' | sed 's/"$//' | sort | uniq -c | sort -rn | while read -r count cmd; do
    echo "- \`$cmd\` ($count hooks)"
  done || true

  echo ""
  echo "## Event Listener Summary"
  echo ""
  echo "All Tauri events listened to from hooks:"
  echo ""

  grep -rhoE '(listen|useTauriEvent)\(.*"[a-z][-a-z]+"' "$HOOKS_DIR/" 2>/dev/null | grep -oE '"[a-z][-a-z]+"' | tr -d '"' | sort | uniq -c | sort -rn | while read -r count evt; do
    echo "- \`$evt\` ($count hooks)"
  done || true

  echo ""
} > "$OUT"

echo "Generated FRONTEND-HOOKS.md: $TOTAL_FILES hooks"
