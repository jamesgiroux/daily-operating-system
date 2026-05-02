#!/usr/bin/env bash
#
# DOS-7 D4-2 lint: no `DELETE FROM intelligence_claims` anywhere in the
# codebase. Claims are append-only; lifecycle transitions go through
# claim_state column updates via services/claims.rs allowlist.
#
# Plan §6 + Linear DOS-7 acceptance: "No-DELETE invariant test for
# intelligence_claims, claim_corroborations, claim_contradictions."

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src")
else
  roots=("src")
fi

# DELETE statements (case-insensitive) targeting any of the three claim
# tables. Match `DELETE FROM <table>` with optional whitespace.
pattern='DELETE[[:space:]]+FROM[[:space:]]+(intelligence_claims|claim_corroborations|claim_contradictions)\b'

matches="$(
  grep -rEni --include='*.rs' --include='*.sql' --include='*.sh' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -v 'dos7-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "DELETE statements against claim tables are forbidden (claims are append-only)."
  echo "Lifecycle transitions go through claim_state column updates via services/claims.rs."
  echo "Add a 'dos7-allowed:' marker on the line if a specific exception is justified."
  echo
  echo "$matches"
  exit 1
fi

echo "No DELETE statements found against claim tables. (intelligence_claims, claim_corroborations, claim_contradictions)"
