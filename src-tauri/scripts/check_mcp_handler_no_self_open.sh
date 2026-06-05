#!/usr/bin/env bash
#
# MCP handler self-open lint.
#
# The MCP v2 sidecar runs out-of-process with no in-process DbService, so any
# ActionDb::open / open_readonly / LocalKeychain::new reached from a tool
# handler is an independent connection that races the app writer's WAL
# cross-process (the db-lock-storm class). Handlers must instead take DB access
# from the request-scoped McpHandlerContext (one owned connection per process).
#
# A static grep cannot prove Rust call-graph reachability in-crate, so this gate
# enforces bounded manifests:
#   - registered handler/write-path files forbid DB self-opens unless the file
#     and fallback rationale are explicitly allowlisted below;
#   - live reader adapters remain enumerated separately so read-only porosity is
#     visible without pretending unrelated writer adapters are handler-reachable.
#
# The behavioral test (handler_context single-owned-connection assertions) is
# the load-bearing check; this gate catches the obvious regression.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  root="src-tauri/src"
else
  root="src"
fi

registered_write_manifest=(
  "${root}/services/mcp_v2/handlers"
  "${root}/services/mcp_v2/audit.rs"
  "${root}/services/workspace_ingestion/workspace_intake_impl.rs"
)

live_reader_manifest=(
  "${root}/services/context.rs"
)

registered_write_pattern='(ActionDb::open(_readonly|_encrypted)?|LocalKeychain::new|local_db_keyed_audit_tag)[[:space:]]*\('
live_reader_pattern='(ActionDb::open_readonly|LocalKeychain::new)[[:space:]]*\('

allowed_registered_fallback() {
  local file="$1"
  local line="$2"
  local line_text="$3"
  local marker_text="$line_text"
  local previous_line=$((line - 1))
  local next_line=$((line + 1))

  if (( previous_line > 0 )); then
    marker_text="$(sed -n "${previous_line}p" "$file") ${marker_text}"
  fi
  marker_text="${marker_text} $(sed -n "${next_line}p" "$file")"

  case "$file" in
    */services/mcp_v2/handlers/tool_account_status.rs)
      [[ "$line_text" == *"ActionDb::open_readonly"* \
        && "$marker_text" == *"mcp-self-open-allowed: ctx-less fallback"* ]]
      ;;
    */services/mcp_v2/audit.rs)
      [[ "$line_text" == *"ActionDb::open("* \
        && "$marker_text" == *"mcp-self-open-allowed: ctx-less + no-DbService fallback"* ]]
      ;;
    */services/workspace_ingestion/workspace_intake_impl.rs)
      [[ ( "$line_text" == *"ActionDb::open("* \
        || "$line_text" == *"local_db_keyed_audit_tag("* ) \
        && "$marker_text" == *"mcp-self-open-allowed: non-MCP workspace intake fallback"* ]]
      ;;
    *)
      return 1
      ;;
  esac
}

allowed_live_reader_fallback() {
  local file="$1"
  local line_text="$2"

  case "$file" in
    */services/context.rs)
      [[ "$line_text" == *"mcp-self-open-allowed: live adapter fallback"* \
        || "$line_text" == *"mcp-self-open-allowed: manifest-adapter fallback outside MCP request context."* ]]
      ;;
    *)
      return 1
      ;;
  esac
}

matches=""
for entry in "${registered_write_manifest[@]}"; do
  while IFS=: read -r file line _match; do
    [[ -z "${file:-}" || -z "${line:-}" ]] && continue
    line_text="$(sed -n "${line}p" "$file")"
    if allowed_registered_fallback "$file" "$line" "$line_text"; then
      continue
    fi
    matches+="${file}:${line}:${_match}"$'\n'
  done < <(
    grep -rEn --include='*.rs' "$registered_write_pattern" "$entry" 2>/dev/null \
      | grep -vE ':\s*(//|//!|///|\*)' \
      || true
  )
done

for entry in "${live_reader_manifest[@]}"; do
  while IFS=: read -r file line _match; do
    [[ -z "${file:-}" || -z "${line:-}" ]] && continue
    line_text="$(sed -n "${line}p" "$file")"
    if allowed_live_reader_fallback "$file" "$line_text"; then
      continue
    fi
    matches+="${file}:${line}:${_match}"$'\n'
  done < <(
    grep -rEn --include='*.rs' "$live_reader_pattern" "$entry" 2>/dev/null \
      | grep -vE ':\s*(//|//!|///|\*)' \
      || true
  )
done

if [[ -n "$matches" ]]; then
  echo "MCP handler self-open forbidden: route DB access through McpHandlerContext." >&2
  echo "The sidecar owns one connection per process; per-handler opens race the app writer's WAL." >&2
  echo >&2
  echo "Scanned registered write-path manifest:" >&2
  for entry in "${registered_write_manifest[@]}"; do
    echo "  - ${entry}" >&2
  done
  echo "Scanned live-reader manifest:" >&2
  for entry in "${live_reader_manifest[@]}"; do
    echo "  - ${entry}" >&2
  done
  echo >&2
  echo "Fix: take the connection from McpHandlerContext, or add the exact" >&2
  echo "fallback to allowed_registered_fallback / allowed_live_reader_fallback." >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

echo "MCP handler self-open lint clean."
