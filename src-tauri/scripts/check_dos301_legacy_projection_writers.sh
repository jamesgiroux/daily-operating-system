#!/usr/bin/env bash
#
# Claim projection lint: legacy AI projection targets are written only by
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
projection_tables='entity_assessment|entity_quality|account_objectives|account_milestones'
account_ai_columns='company_overview|strategic_programs|notes'
sql_write='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE)'
direct_pattern="(${sql_write}[[:space:]]+(${projection_tables})\\b)|(write_intelligence_json[[:space:]]*\\()"
account_update_pattern='UPDATE[[:space:]]+accounts\b'
account_insert_pattern='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO)[[:space:]]+accounts\b'

candidate_files="$(
  {
    grep -rEli --include='*.rs' --include='*.sql' "$direct_pattern" "${roots[@]}" 2>/dev/null
    grep -rEli --include='*.rs' --include='*.sql' "$account_update_pattern" "${roots[@]}" 2>/dev/null
    grep -rEli --include='*.rs' --include='*.sql' "$account_insert_pattern" "${roots[@]}" 2>/dev/null
  } \
    | sort -u \
    | grep -Ev "($allowed_basename_regex)" \
    || true
)"

matches=""
for file in $candidate_files; do
  direct_hits="$(grep -nEi "$direct_pattern" "$file" 2>/dev/null || true)"
  if [[ -n "$direct_hits" ]]; then
    matches+="$direct_hits"$'\n'
  fi

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    end=$((lineno + 7))
    if sed -n "${lineno},${end}p" "$file" \
      | grep -qiE '("|`|\[)?\b(company_overview|strategic_programs|notes)\b("|`|\])?[[:space:]]*='; then
      matches+="${file}:${lineno}: UPDATE accounts mutates legacy AI projection column within 7-line window"$'\n'
    fi
  done < <(grep -nEi "$account_update_pattern" "$file" 2>/dev/null | cut -d: -f1 || true)

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    end=$((lineno + 7))
    if sed -n "${lineno},${end}p" "$file" \
      | grep -qiE '("|`|\[)?\b(company_overview|strategic_programs|notes)\b("|`|\])?[[:space:]]*[,)]'; then
      matches+="${file}:${lineno}: INSERT accounts includes legacy AI projection column within 7-line window"$'\n'
    fi
  done < <(grep -nEi "$account_insert_pattern" "$file" 2>/dev/null | cut -d: -f1 || true)
done

matches="$(printf '%s\n' "$matches" | sort -u | sed '/^$/d')"

if [[ -n "$matches" ]]; then
  echo "Direct writes to legacy AI projection targets are restricted to derived_state."
  echo
  echo "Allowed files:"
  echo "  - src-tauri/src/services/derived_state.rs"
  echo "  - src-tauri/src/services/claims_backfill.rs"
  echo "  - src-tauri/src/migrations/*.sql"
  echo
  echo "$matches"
  exit 1
fi

echo "Legacy AI projection target writers are restricted to derived_state."
