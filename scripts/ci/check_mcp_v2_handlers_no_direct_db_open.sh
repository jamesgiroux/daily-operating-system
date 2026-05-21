#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HANDLER_DIR="$ROOT_DIR/src-tauri/src/services/mcp_v2/handlers"

if [[ ! -d "$HANDLER_DIR" ]]; then
  echo "MCP v2 handler directory not found: $HANDLER_DIR" >&2
  exit 1
fi

if rg -n 'ActionDb::open|LocalKeychain::new' "$HANDLER_DIR" -g '*.rs'; then
  cat >&2 <<'EOF'
MCP v2 handlers must receive DB and service access through McpHandlerContext.
Direct database opens and keychain provider construction are not allowed here.
EOF
  exit 1
fi

echo "MCP v2 handler DB-open boundary check passed."
