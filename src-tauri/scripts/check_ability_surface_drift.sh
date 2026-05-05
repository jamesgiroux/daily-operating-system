#!/usr/bin/env bash
set -euo pipefail

# DOS-217 Phase 2 surface drift guard.
#
# Allowed Tauri command names are the pre-existing hand-written commands
# registered in src-tauri/src/lib.rs, minus the single registry bridge command
# in commands/abilities.rs. New capability-level Tauri commands must be
# implemented as abilities and exposed through invoke_ability.
#
# Allowed MCP #[tool] handlers are the Phase 2 static mechanical reads below.
# Registry-backed abilities, get_provenance, and request_confirmation are
# manually routed and must not be added as new hand-written #[tool] handlers.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_ALLOWLIST_FILE="$SCRIPT_DIR/ability_surface_allowlist.txt"

if [[ -n "${DOS217_SURFACE_DRIFT_ROOT_OVERRIDE:-}" ]]; then
  ROOT_DIR="$DOS217_SURFACE_DRIFT_ROOT_OVERRIDE"
elif [[ -d "src-tauri/src" ]]; then
  ROOT_DIR="$(pwd)"
else
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

if [[ ! -f "$TAURI_ALLOWLIST_FILE" ]]; then
  echo "Ability surface drift: missing Tauri command allowlist: $TAURI_ALLOWLIST_FILE"
  exit 1
fi

normalized_tauri_allowlist="$(mktemp)"
trap 'rm -f "$normalized_tauri_allowlist"' EXIT

if ! awk '
  /^[[:space:]]*($|#)/ {
    next
  }
  {
    if ($0 !~ /^[A-Za-z_][A-Za-z0-9_]*$/) {
      printf("Ability surface drift: invalid Tauri command allowlist entry at %s:%d: %s\n", FILENAME, NR, $0)
      invalid = 1
      next
    }
    print
  }
  END {
    exit invalid ? 10 : 0
  }
' "$TAURI_ALLOWLIST_FILE" > "$normalized_tauri_allowlist"; then
  exit 1
fi

if [[ ! -s "$normalized_tauri_allowlist" ]]; then
  echo "Ability surface drift: Tauri command allowlist is empty: $TAURI_ALLOWLIST_FILE"
  exit 1
fi

duplicate_tauri_commands="$(sort "$normalized_tauri_allowlist" | uniq -d)"
if [[ -n "$duplicate_tauri_commands" ]]; then
  echo "Ability surface drift: duplicate Tauri command allowlist entries:"
  echo "$duplicate_tauri_commands"
  exit 1
fi

is_allowed_tauri_command() {
  grep -qxF "$1" "$normalized_tauri_allowlist"
}

extract_tauri_commands() {
  local file_path="$1"
  local rel_path="$2"

  awk -v rel="$rel_path" '
    /^[[:space:]]*#\[tauri::command([^]]*)?\]/ {
      pending = NR
      next
    }

    pending {
      trimmed = $0
      sub(/^[[:space:]]+/, "", trimmed)

      if (trimmed ~ /^#\[/ || trimmed ~ /^$/ || trimmed ~ /^\/\//) {
        next
      }

      if (trimmed ~ /^(pub(\([^)]*\))?[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/) {
        name = trimmed
        sub(/^(pub(\([^)]*\))?[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+/, "", name)
        sub(/\(.*/, "", name)
        printf("%s\t%d\t%s\n", rel, NR, name)
      }

      pending = 0
    }
  ' "$file_path"
}

violations=0
commands_dir="$ROOT_DIR/src-tauri/src/commands"

if [[ -d "$commands_dir" ]]; then
  while IFS= read -r file_path; do
    rel_path="${file_path#$ROOT_DIR/}"

    while IFS=$'\t' read -r command_rel_path command_line command_name; do
      if [[ "$command_rel_path" == "src-tauri/src/commands/abilities.rs" && "$command_name" == "invoke_ability" ]]; then
        continue
      fi

      if ! is_allowed_tauri_command "$command_name"; then
        printf 'Ability surface drift: %s:%s new hand-written Tauri command `%s` is not allowlisted\n' \
          "$command_rel_path" "$command_line" "$command_name"
        violations=1
      fi
    done < <(extract_tauri_commands "$file_path" "$rel_path")
  done < <(find "$commands_dir" -maxdepth 1 -type f -name '*.rs' | sort)
fi

mcp_main="$ROOT_DIR/src-tauri/src/mcp/main.rs"
if [[ -f "$mcp_main" ]]; then
  if ! awk -v rel="src-tauri/src/mcp/main.rs" '
    function allowed_tool(name) {
      return name == "get_briefing" ||
        name == "query_entity" ||
        name == "list_entities" ||
        name == "search_meetings" ||
        name == "search_content"
    }

    {
      trimmed = $0
      sub(/^[[:space:]]+/, "", trimmed)

      if (trimmed ~ /^#\[tool/) {
        pending = NR
        pending_line = trimmed
        next
      }

      if (pending && trimmed ~ /^impl[[:space:]]+DailyOsMcp/) {
        if (pending_line != "#[tool(tool_box)]") {
          printf("Ability surface drift: %s:%d unexpected #[tool] before impl DailyOsMcp\n", rel, pending)
          found = 1
        }
        pending = 0
        next
      }

      if (pending && trimmed ~ /^(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/) {
        line = trimmed
        sub(/^(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+/, "", line)
        sub(/\(.*/, "", line)
        if (!allowed_tool(line)) {
          printf("Ability surface drift: %s:%d new hand-written MCP #[tool] handler `%s` is not allowlisted\n", rel, NR, line)
          found = 1
        }
        pending = 0
      }
    }

    END { exit found ? 10 : 0 }
  ' "$mcp_main"; then
    violations=1
  fi
fi

if [[ "$violations" -ne 0 ]]; then
  cat <<'EOF'
Ability surface drift detected.

Add capability-level operations as W3 registry abilities and expose them through
the DOS-217 bridge. Update the MCP tool allowlist only for explicit Phase 2
static mechanical-read exceptions.
EOF
  exit 1
fi

echo "Ability surface drift check passed."
