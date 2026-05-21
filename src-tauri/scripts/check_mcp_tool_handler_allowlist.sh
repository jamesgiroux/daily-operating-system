#!/usr/bin/env bash
#
# MCP v2 handler lint: tool handlers do not touch SQLite directly.
#
# Forbids direct conn.execute, db.upsert_*, and db.query_* calls inside
# services/mcp_v2/handlers/*.rs. Handler-side persistence must route
# through approved ability invocation wrappers so actor policy, schema
# validation, audit, and provenance stay on the bridge path.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  handler_root="src-tauri/src/services/mcp_v2/handlers"
else
  handler_root="src/services/mcp_v2/handlers"
fi

if [[ ! -d "$handler_root" ]]; then
  echo "MCP v2 handler allowlist: handler directory not found; nothing to check."
  exit 0
fi

pattern='(conn\.execute[[:space:]]*\(|db\.(upsert_[A-Za-z0-9_]+|query_[A-Za-z0-9_]+)[[:space:]]*\()'

# Allowed only when the direct DB-looking token is part of a documented
# ability invocation wrapper. Keep this narrow: ordinary handler bodies
# should call the bridge/ability layer, not ActionDb or rusqlite directly.
approved_ability_invocation_pattern='(invoke_registry_json(_for_actor)?|invoke_by_name_json|invoke_mcp_ability_tool|ability_bridge\.invoke_ability|mcp-v2-ability-invocation-approved:)'

matches="$(
  grep -Eni --include='*.rs' "$pattern" "$handler_root"/*.rs 2>/dev/null \
    | grep -Ev "$approved_ability_invocation_pattern" \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct SQLite/ActionDb calls inside MCP v2 tool handlers are forbidden."
  echo "Route handler effects through an approved ability invocation wrapper so"
  echo "actor policy, schema validation, audit, and provenance are preserved."
  echo
  echo "Forbidden patterns:"
  echo "  - conn.execute(...)"
  echo "  - db.upsert_*(...)"
  echo "  - db.query_*(...)"
  echo
  echo "Approved wrapper patterns:"
  echo "  - invoke_registry_json(...) / invoke_registry_json_for_actor(...)"
  echo "  - registry.invoke_by_name_json(...)"
  echo "  - invoke_mcp_ability_tool(...)"
  echo "  - ability_bridge.invoke_ability(...)"
  echo
  echo "$matches"
  exit 1
fi

echo "MCP v2 tool handlers avoid direct SQLite/ActionDb calls."
