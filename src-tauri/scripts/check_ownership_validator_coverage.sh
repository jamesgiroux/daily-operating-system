#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() {
  printf 'ownership-validator-coverage: %s\n' "$1" >&2
  exit 1
}

[[ -f src/abilities/provenance/ownership.rs ]] \
  || fail "missing src/abilities/provenance/ownership.rs"

rg -q 'pub fn validate_subject_ownership' src/abilities/provenance/ownership.rs \
  || fail "validator API validate_subject_ownership is not exported"

rg -q 'validate_serialized_subject_ownership' src/commands/abilities.rs \
  || fail "Tauri invoke_ability does not run the ownership validator"

rg -q 'validate_serialized_subject_ownership' src/mcp/main.rs \
  || fail "MCP ability tool route does not run the ownership validator"

rg -q 'ability_name != "get_entity_context"' src/mcp/main.rs \
  || fail "MCP get_entity_context exception is not explicit"

rg -q 'BridgeSurfaceError::Ownership' src/commands/abilities.rs src/mcp/main.rs \
  || fail "validator failures are not returned as structured ownership errors"

if rg -n '\.invoke_ability\(session_id, &ability_name' src/mcp/main.rs >/dev/null; then
  rg -q 'validate_serialized_subject_ownership' src/mcp/main.rs \
    || fail "MCP bridge invocation bypasses ownership validation"
fi

printf 'ownership-validator-coverage: ok\n'
