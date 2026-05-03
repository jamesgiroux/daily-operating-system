#!/usr/bin/env bash
#
# Claim projection lint: legacy AI projection targets are written only by
# services/derived_state.rs during the dual-projection window.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  candidate_roots=(
    "src-tauri/src"
  )
else
  candidate_roots=(
    "src"
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

# Durable allowlist for direct writes that are not clean text claims:
# - derived_state.rs owns transitional projection/cache SQL.
# - claims_backfill.rs and migrations are one-time substrate/bootstrap writers.
# - success plan CRUD writes user-authored objectives/milestones, not claim-shaped projections.
# - self_healing/privacy/callouts/glean/hygiene/data_lifecycle write operational quality,
#   cleanup, or scoring metadata that has no registered claim-type mapping.
# - intelligence.json file I/O remains schema-fenced cache work until file projection lands.
# - devtools writes seed/mock data or debug-only repair state.
allowed_basename_regex='services/derived_state\.rs|services/claims_backfill\.rs|migrations/[^:]+\.sql|db/success_plans\.rs|services/success_plans\.rs|self_healing/[^:]+\.rs|privacy\.rs|signals/callouts\.rs|context_provider/glean\.rs|hygiene/mod\.rs|db/data_lifecycle\.rs|intelligence/io\.rs|intelligence/write_fence\.rs|devtools/mod\.rs'
projection_tables='entity_assessment|entity_quality|account_objectives|account_milestones'
registry_backed_entity_assessment_columns='executive_assessment|risks_json|recent_wins_json|current_state_json|stakeholder_insights_json|company_context_json|value_delivered'
account_ai_columns='company_overview|strategic_programs|notes'
sql_write='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE)'
direct_pattern="(${sql_write}[[:space:]]+(${projection_tables})\\b)|(^|[^[:alnum:]_])write_intelligence_json[[:space:]]*\\("
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
  cfg_test_start="$(grep -nE '^[[:space:]]*#\[cfg\(test\)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1 || true)"
  direct_hits="$(
    grep -nHEi "$direct_pattern" "$file" 2>/dev/null \
      | awk -F: -v start="$cfg_test_start" 'start == "" || $2 < start' \
      || true
  )"
  if [[ -n "$direct_hits" ]]; then
    matches+="$direct_hits"$'\n'
  fi

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    if [[ -n "$cfg_test_start" && "$lineno" -ge "$cfg_test_start" ]]; then
      continue
    fi
    end=$((lineno + 7))
    if sed -n "${lineno},${end}p" "$file" \
      | grep -qiE '("|`|\[)?\b(company_overview|strategic_programs|notes)\b("|`|\])?[[:space:]]*='; then
      matches+="${file}:${lineno}: UPDATE accounts mutates legacy AI projection column within 7-line window"$'\n'
    fi
  done < <(grep -nEi "$account_update_pattern" "$file" 2>/dev/null | cut -d: -f1 || true)

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    if [[ -n "$cfg_test_start" && "$lineno" -ge "$cfg_test_start" ]]; then
      continue
    fi
    end=$((lineno + 7))
    if sed -n "${lineno},${end}p" "$file" \
      | grep -qiE '("|`|\[)?\b(company_overview|strategic_programs|notes)\b("|`|\])?[[:space:]]*[,)]'; then
      matches+="${file}:${lineno}: INSERT accounts includes legacy AI projection column within 7-line window"$'\n'
    fi
  done < <(grep -nEi "$account_insert_pattern" "$file" 2>/dev/null | cut -d: -f1 || true)
done

for root in "${roots[@]}"; do
  legacy_snapshot_file="$root/services/derived_state.rs"
  [[ -f "$legacy_snapshot_file" ]] || continue

  legacy_snapshot_hits="$(
    awk '
      /pub fn upsert_entity_intelligence_legacy_snapshot[[:space:]]*\(/ { in_fn = 1 }
      in_fn { print FILENAME ":" FNR ":" $0 }
      in_fn && /pub fn upsert_entity_health_legacy_projection[[:space:]]*\(/ { in_fn = 0 }
    ' "$legacy_snapshot_file" \
      | grep -Ei "\b(${registry_backed_entity_assessment_columns})\b" \
      || true
  )"

  if [[ -n "$legacy_snapshot_hits" ]]; then
    matches+="upsert_entity_intelligence_legacy_snapshot writes registry-backed entity_assessment columns:"$'\n'
    matches+="$legacy_snapshot_hits"$'\n'
  fi
done

matches="$(printf '%s\n' "$matches" | sort -u | sed '/^$/d')"

if [[ -n "$matches" ]]; then
  echo "Direct writes to legacy AI projection targets are restricted to derived_state."
  echo
  echo "Allowed files:"
  echo "  - src-tauri/src/services/derived_state.rs"
  echo "  - src-tauri/src/services/claims_backfill.rs"
  echo "  - src-tauri/src/migrations/*.sql"
  echo "  - documented non-claim metadata writers in the script allowlist"
  echo
  echo "$matches"
  exit 1
fi

echo "Legacy AI projection target writers are restricted to derived_state."
