#!/usr/bin/env bash
#
# Claim projection lint: legacy AI projection tables are written only by
# services/derived_state.rs during the dual-projection window.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  candidate_roots=(
    "src-tauri/src"
    "src-tauri/tests"
  )
else
  candidate_roots=(
    "src"
    "tests"
  )
fi

roots=()
for root in "${candidate_roots[@]}"; do
  if [[ -d "$root" ]]; then
    roots+=("$root")
  fi
done

if [[ "${#roots[@]}" -eq 0 ]]; then
  echo "No source roots found for legacy projection writer lint."
  exit 0
fi

allowed_basename_regex='services/derived_state\.rs|services/claims_backfill\.rs|migrations/[^:]+\.sql'
pattern='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE)[[:space:]]+(entity_assessment|entity_quality)\b'

matches="$(
  grep -rEni --include='*.rs' --include='*.sql' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_basename_regex)" \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct writes to legacy AI projection tables are restricted to derived_state."
  echo
  echo "Allowed files:"
  echo "  - src-tauri/src/services/derived_state.rs"
  echo "  - src-tauri/src/services/claims_backfill.rs"
  echo "  - src-tauri/src/migrations/*.sql"
  echo
  echo "$matches"
  exit 1
fi

echo "Legacy AI projection table writers are restricted to derived_state."
