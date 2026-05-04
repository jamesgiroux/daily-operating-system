#!/usr/bin/env bash
set -euo pipefail

# DOS-217 Phase 2 surface drift guard.
#
# Allowed Tauri command files are the pre-existing hand-written command
# modules plus the single registry bridge command in commands/abilities.rs.
# New capability-level Tauri commands must be implemented as abilities and
# exposed through invoke_ability.
#
# Allowed MCP #[tool] handlers are the Phase 2 static mechanical reads below.
# Registry-backed abilities, get_provenance, and request_confirmation are
# manually routed and must not be added as new hand-written #[tool] handlers.

if [[ -n "${DOS217_SURFACE_DRIFT_ROOT_OVERRIDE:-}" ]]; then
  ROOT_DIR="$DOS217_SURFACE_DRIFT_ROOT_OVERRIDE"
elif [[ -d "src-tauri/src" ]]; then
  ROOT_DIR="$(pwd)"
else
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

is_allowed_tauri_command_file() {
  case "$1" in
    src-tauri/src/commands/abilities.rs | \
    src-tauri/src/commands/accounts_content_chat.rs | \
    src-tauri/src/commands/actions_calendar.rs | \
    src-tauri/src/commands/app_support.rs | \
    src-tauri/src/commands/core.rs | \
    src-tauri/src/commands/integrations.rs | \
    src-tauri/src/commands/people_entities.rs | \
    src-tauri/src/commands/planning_reports.rs | \
    src-tauri/src/commands/projects_data.rs | \
    src-tauri/src/commands/success_plans.rs | \
    src-tauri/src/commands/workspace.rs)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

violations=0
commands_dir="$ROOT_DIR/src-tauri/src/commands"

if [[ -d "$commands_dir" ]]; then
  while IFS= read -r file_path; do
    rel_path="${file_path#$ROOT_DIR/}"
    if ! grep -qE '^[[:space:]]*#\[tauri::command\]' "$file_path"; then
      continue
    fi

    if ! is_allowed_tauri_command_file "$rel_path"; then
      echo "Ability surface drift: new hand-written Tauri command file is not allowlisted: $rel_path"
      violations=1
      continue
    fi

    if [[ "$rel_path" == "src-tauri/src/commands/abilities.rs" ]]; then
      if ! awk -v rel="$rel_path" '
        /^[[:space:]]*#\[tauri::command\]/ {
          pending = NR
          next
        }
        pending && /^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/ {
          line = $0
          sub(/^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+/, "", line)
          sub(/\(.*/, "", line)
          if (line != "invoke_ability") {
            printf("Ability surface drift: %s:%d new bridge command `%s` is not allowed\n", rel, NR, line)
            found = 1
          }
          pending = 0
        }
        END { exit found ? 10 : 0 }
      ' "$file_path"; then
        violations=1
      fi
    fi
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
the DOS-217 bridge. Update this allowlist only for explicit Phase 2 static
mechanical-read exceptions.
EOF
  exit 1
fi

echo "Ability surface drift check passed."
