#!/usr/bin/env bash
#
# No second DbService in the MCP sidecar.
#
# The MCP v2 sidecar must own exactly ONE connection (via
# McpHandlerContext / handler_context::open_sidecar_connection), threaded into
# handlers. Installing or constructing a DbService *pool* inside the sidecar
# would create a second writer thread that still races the app writer's WAL
# cross-process — the forbidden fake fix the one-owned-connection design
# exists to prevent. The sidecar reads the global DbService when one exists
# (db_service::try_global, in-app hosting) but must never start its own.
#
# Forbidden in src/mcp and the MCP v2 wiring:
#   - install_global(            (registering a process-global DbService)
#   - DbService::open / open_at  (constructing a DbService pool)
# A legitimate exception carries a per-line `mcp-dbservice-allowed:` rationale.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  root="src-tauri/src"
else
  root="src"
fi

scan=(
  "${root}/mcp"
  "${root}/services/mcp_v2"
)

pattern='(install_global[[:space:]]*\(|DbService::open(_at)?[[:space:]]*\()'

matches="$(
  grep -rEn --include='*.rs' "$pattern" "${scan[@]}" 2>/dev/null \
    | grep -vE ':\s*(//|//!|///|\*)' \
    | grep -v 'mcp-dbservice-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Second DbService forbidden in the MCP sidecar." >&2
  echo "The sidecar owns one connection via McpHandlerContext; a DbService pool" >&2
  echo "starts a second writer thread that races the app writer's WAL." >&2
  echo >&2
  echo "Scanned:" >&2
  for entry in "${scan[@]}"; do
    echo "  - ${entry}" >&2
  done
  echo >&2
  echo "Use handler_context::open_sidecar_connection (one owned connection), or" >&2
  echo "add a narrow 'mcp-dbservice-allowed:' rationale for a legitimate case." >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

echo "No second DbService in MCP sidecar — clean."
