#!/bin/bash
# Generates .docs/architecture/COMMAND-REFERENCE.md from Rust source files.
# Scans all #[tauri::command] functions in commands/ directory.

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CMD_DIR="$ROOT/src-tauri/src/commands"
CMD_RS="$ROOT/src-tauri/src/commands.rs"
LIB_RS="$ROOT/src-tauri/src/lib.rs"
OUT="$ROOT/.docs/architecture/COMMAND-REFERENCE.md"

# Count registered commands in lib.rs
REGISTERED=$(grep -c 'commands::' "$LIB_RS" 2>/dev/null || echo "?")

# Collect all command sources
SOURCES=()
[ -f "$CMD_RS" ] && SOURCES+=("$CMD_RS")
if [ -d "$CMD_DIR" ]; then
  for f in "$CMD_DIR"/*.rs; do
    [ -f "$f" ] && SOURCES+=("$f")
  done
fi

# Temporary file for parsed commands
TMPFILE=$(mktemp)
trap "rm -f $TMPFILE" EXIT

# Parse all #[tauri::command] functions using awk
for src in "${SOURCES[@]}"; do
  module=$(basename "$src" .rs)
  if [ "$module" = "commands" ]; then
    module="commands (root)"
  else
    module="commands/$module"
  fi

  awk -v mod="$module" '
  /^#\[tauri::command\]/ { found = 1; next }
  found && /^pub / {
    line = $0
    sub(/^pub /, "", line)
    async_flag = "—"
    if (line ~ /^async fn/) {
      async_flag = "yes"
      sub(/^async fn /, "", line)
    } else {
      sub(/^fn /, "", line)
    }
    # Get function name
    split(line, parts, /[(<]/)
    fname = parts[1]
    gsub(/[[:space:]]/, "", fname)

    # Collect full signature for params
    sig = $0
    while (sig !~ /\{/ && sig !~ /->/) {
      if (getline <= 0) break
      sig = sig " " $0
    }

    # Extract user-facing params (skip State, AppHandle, Window, Sender)
    n = split(sig, chars, "")
    depth = 0; raw_params = ""; in_params = 0
    for (i = 1; i <= n; i++) {
      if (chars[i] == "(") { depth++; if (depth == 1) { in_params = 1; continue } }
      if (chars[i] == ")") { depth--; if (depth == 0) break }
      if (in_params && depth == 1) raw_params = raw_params chars[i]
    }

    np = split(raw_params, plist, ",")
    user_params = ""
    for (p = 1; p <= np; p++) {
      param = plist[p]
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", param)
      if (param ~ /[Ss]tate</ || param ~ /[Ss]ender/ || param ~ /[Ww]indow/ || param ~ /AppHandle/ || param ~ /Arc</ || param == "") continue
      split(param, pname, ":")
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", pname[1])
      if (user_params != "") user_params = user_params ", "
      user_params = user_params pname[1]
    }
    if (user_params == "") user_params = "—"

    printf "%s|%s|%s|%s\n", mod, fname, async_flag, user_params
    found = 0
    next
  }
  found && !/^[[:space:]]*$/ && !/^\/\// { found = 0 }
  ' "$src" >> "$TMPFILE"
done

# Generate the markdown
{
  echo "# Command Reference"
  echo ""
  echo "Complete inventory of all Tauri IPC commands (\`#[tauri::command]\` functions)."
  echo ""
  echo "**Auto-generated:** $(date +%Y-%m-%d) by \`.docs/generators/gen-command-reference.sh\`"
  echo "**Registered in lib.rs:** ~${REGISTERED} commands"
  echo "**Source files:** ${#SOURCES[@]}"
  echo ""
  echo "---"
  echo ""

  CURRENT_MOD=""
  sort -t'|' -k1,1 -k2,2 "$TMPFILE" | while IFS='|' read -r mod fname async_flag params; do
    if [ "$mod" != "$CURRENT_MOD" ]; then
      if [ -n "$CURRENT_MOD" ]; then
        echo ""
      fi
      CURRENT_MOD="$mod"
      echo "## \`$mod\`"
      echo ""
      echo "| Command | Async | Parameters |"
      echo "|---------|-------|------------|"
    fi
    echo "| \`$fname\` | $async_flag | $params |"
  done

  echo ""
} > "$OUT"

TOTAL=$(wc -l < "$TMPFILE" | tr -d ' ')
echo "Generated COMMAND-REFERENCE.md: $TOTAL commands across ${#SOURCES[@]} source files"
