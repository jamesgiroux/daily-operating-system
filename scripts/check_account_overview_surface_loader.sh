#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"

targets=("src-tauri/abilities-runtime/src/abilities/account_overview.rs")
if [[ -d "src-tauri/tests" ]]; then
  while IFS= read -r file; do
    targets+=("$file")
  done < <(find src-tauri/tests -maxdepth 2     \( -name 'dos568*.rs' -o -path 'src-tauri/tests/dos568_support/*.rs' \)     -type f | sort)
fi

violations=0
check_pattern() {
  local label=$1
  local pattern=$2
  local output
  if output=$(rg -n "$pattern" "${targets[@]}" 2>/dev/null); then
    printf '%s
' "$output"
    printf 'account_overview surface loader gate: forbidden %s usage
' "$label" >&2
    violations=1
  fi
}

check_pattern 'load_claims_active' 'load_claims_active\s*\('
check_pattern 'load_claims_active_by_source_ref' 'load_claims_active_by_source_ref\s*\('
check_pattern 'load_entity_context_claims_active' 'load_entity_context_claims_active\s*\('
check_pattern 'load_claims_where' 'load_claims_where\s*\('
check_pattern 'load_claim_by_id' 'load_claim_by_id\s*\('
check_pattern 'direct intelligence_claims table access' 'intelligence_claims'

if (( violations )); then
  cat >&2 <<'EOF'
account_overview.rs and DOS-568 fixtures must use surface-aware claim readers.
Use EntityContextClaimReadHandle or load_entity_context_claims_active_for_surface so surface dismissals and projection boundaries are preserved.
EOF
  exit 1
fi
