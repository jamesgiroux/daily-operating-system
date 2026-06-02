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
# enforces a BOUNDED, ENUMERATED manifest: the mcp_v2 handler directory plus the
# specific service-adapter modules a handler is known to reach. New self-opens
# in these files must move to McpHandlerContext, or carry a per-line
# `mcp-self-open-allowed:` rationale for a legitimate fallback the context
# does not yet cover.
#
# The behavioral test (handler_context single-owned-connection assertions) is
# the load-bearing check; this gate catches the obvious regression.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  root="src-tauri/src"
else
  root="src"
fi

# Bounded manifest: handler dir + enumerated handler-reachable adapters.
# Keep this list in sync as handlers gain/lose service-adapter dependencies.
manifest=(
  "${root}/services/mcp_v2/handlers"
  "${root}/services/mcp_v2/audit.rs"
  "${root}/services/context.rs"
  "${root}/services/workspace_ingestion/workspace_intake_impl.rs"
)

pattern='(ActionDb::open(_readonly|_encrypted)?|LocalKeychain::new)[[:space:]]*\('

matches=""
for entry in "${manifest[@]}"; do
  while IFS=: read -r file line _match; do
    [[ -z "${file:-}" || -z "${line:-}" ]] && continue
    if sed -n "${line}p" "$file" | grep -q 'mcp-self-open-allowed:'; then
      continue
    fi
    matches+="${file}:${line}:${_match}"$'\n'
  done < <(
    grep -rEn --include='*.rs' "$pattern" "$entry" 2>/dev/null \
      | grep -vE ':\s*(//|//!|///|\*)' \
      || true
  )
done

if [[ -n "$matches" ]]; then
  echo "MCP handler self-open forbidden: route DB access through McpHandlerContext." >&2
  echo "The sidecar owns one connection per process; per-handler opens race the app writer's WAL." >&2
  echo >&2
  echo "Scanned (bounded manifest):" >&2
  for entry in "${manifest[@]}"; do
    echo "  - ${entry}" >&2
  done
  echo >&2
  echo "Fix: take the connection from McpHandlerContext::with_conn, or add a narrow" >&2
  echo "'mcp-self-open-allowed:' rationale for a legitimate uncovered fallback." >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

echo "MCP handler self-open lint clean."
