#!/usr/bin/env bash
#
# Claims-substrate lint: LegacyUnattributed may only be written by the
# DOS cutover/backfill path that explicitly downgrades legacy rows whose
# source timestamp cannot be attributed.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  candidate_roots=(
    "src-tauri/src/services"
    "src-tauri/src/db"
    "src-tauri/src/commands"
    "src-tauri/src/migrations"
    "src-tauri/tests"
  )
else
  candidate_roots=("src" "db" "commands" "migrations" "tests")
fi

roots=()
for root in "${candidate_roots[@]}"; do
  if [[ -d "$root" ]]; then
    roots+=("$root")
  fi
done

if [[ "${#roots[@]}" -eq 0 ]]; then
  echo "No source roots found for LegacyUnattributed writer lint."
  exit 0
fi

allowed_basename_regex='services/source_asof_backfill\.rs|services/claims_backfill\.rs|migrations/[^:]+\.sql'
direct_pattern="data_source[[:space:]]*=[[:space:]]*'legacy_unattributed'|DataSource::LegacyUnattributed|'legacy_unattributed'|\"legacy_unattributed\""
assignment_pattern='data_source[[:space:]]*='

candidate_files="$(
  {
    grep -rEli --include='*.rs' --include='*.sql' "$direct_pattern" "${roots[@]}" 2>/dev/null
    grep -rEli --include='*.rs' --include='*.sql' "$assignment_pattern" "${roots[@]}" 2>/dev/null
  } \
    | sort -u \
    | grep -Ev "($allowed_basename_regex)" \
    || true
)"

matches=""
for file in $candidate_files; do
  direct_hits="$(grep -nE "$direct_pattern" "$file" 2>/dev/null || true)"
  if [[ -n "$direct_hits" ]]; then
    matches+="$direct_hits"$'\n'
  fi

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    end=$((lineno + 3))
    if sed -n "${lineno},${end}p" "$file" | grep -qE 'legacy_unattributed'; then
      matches+="${file}:${lineno}: data_source assignment writes legacy_unattributed within 3-line window"$'\n'
    fi
  done < <(grep -nE "$assignment_pattern" "$file" 2>/dev/null | cut -d: -f1 || true)
done

matches="$(printf '%s\n' "$matches" | sort -u | sed '/^$/d')"

if [[ -n "$matches" ]]; then
  echo "LegacyUnattributed writers are restricted to the cutover/backfill allowlist."
  echo
  echo "Allowed files:"
  echo "  - src-tauri/src/services/source_asof_backfill.rs"
  echo "  - src-tauri/src/services/claims_backfill.rs"
  echo "  - src-tauri/src/migrations/*.sql"
  echo
  echo "$matches"
  exit 1
fi

echo "OK: no LegacyUnattributed writers outside allowlist."
