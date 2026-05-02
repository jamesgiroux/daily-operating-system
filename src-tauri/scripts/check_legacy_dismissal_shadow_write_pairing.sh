#!/usr/bin/env bash
#
# DOS-7 L2 cycle-1 fix #5: every runtime write to a legacy dismissal
# table must be paired with a shadow_write_tombstone_claim call so the
# claim substrate stays in parity with legacy storage. This is the
# load-bearing invariant that lets commit_claim PRE-GATE block the AI
# from re-surfacing a user-dismissed item — without the pairing, the
# claim substrate misses the dismissal entirely and resurrection
# slips through.
#
# Scope:
#   suppression_tombstones, account_stakeholder_roles.dismissed_at,
#   email_dismissals, meeting_entity_dismissals, linking_dismissals,
#   nudge_dismissals, briefing_callouts.dismissed_at, triage_snoozes
#
# Pairing rule: each match must have a `shadow_write_tombstone_claim`
# call within 50 lines (above OR below) in the same file. Allowlist
# files are excluded outright (backfill code, migrations, schema
# fixtures).

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

# Files that legitimately write legacy dismissal tables WITHOUT needing
# a shadow_write pair: backfill code (mechanism owners), migrations
# (schema setup + DOS-258 transitional dual-write), DB modules (raw
# SQL methods called from services that pair the shadow-write
# externally), and tests/fixtures.
allowed_basename_regex='services/claims_backfill\.rs|services/claims\.rs|migrations/|migrations\.rs|db/intelligence_feedback\.rs|db/feedback\.rs|db/signals\.rs|db/projects\.rs|db/accounts\.rs|db/entity_linking\.rs|db/mod_tests\.rs|tests/dos7_d3a1_backfill_test\.rs|tests/dos7_d3a2_backfill_test\.rs|tests/dos7_d1_schema_test\.rs|tests/dos7_d4_lint_test\.rs|tests/dos7_d5_ghost_resurrection_test\.rs|tests/dos311_fixtures/|intel_queue\.rs|demo\.rs|signals/rules\.rs|devtools/'

# Pattern for legacy-dismissal-table writes at the service/command
# layer. Match raw SQL only, not function-call call-sites — call-sites
# in commands/ delegate to services, which pair internally.
#
# We deliberately do NOT match `INSERT INTO suppression_tombstones_quarantine`
# (DOS-308 audit trail) or generic `account_stakeholder_roles`
# inserts without `dismissed_at` (those create roles, not dismissals).
write_pattern='INSERT[[:space:]]+INTO[[:space:]]+(email_dismissals|meeting_entity_dismissals|linking_dismissals|nudge_dismissals|triage_snoozes)|UPDATE[[:space:]]+briefing_callouts[[:space:]]+SET[[:space:]]+dismissed_at|INSERT[[:space:]]+OR[[:space:]]+IGNORE[[:space:]]+INTO[[:space:]]+linking_dismissals'

# Files containing matches, after allowlist exclusion.
candidate_files="$(
  grep -rEli --include='*.rs' "$write_pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "$allowed_basename_regex" \
    || true
)"

if [[ -z "$candidate_files" ]]; then
  echo "All runtime legacy dismissal writes are paired with shadow_write_tombstone_claim (none in non-allowlisted files)."
  exit 0
fi

violations=""
for file in $candidate_files; do
  # For each matching write line in this file, check for a
  # shadow_write_tombstone_claim within ±50 lines.
  while read -r match_line; do
    [[ -z "$match_line" ]] && continue
    # Strip leading "lineno:" prefix.
    lineno="${match_line%%:*}"
    [[ -z "$lineno" ]] && continue
    start=$((lineno - 50))
    [[ $start -lt 1 ]] && start=1
    end=$((lineno + 50))
    has_shadow="$(sed -n "${start},${end}p" "$file" | grep -c 'shadow_write_tombstone_claim' || true)"
    if [[ "$has_shadow" -eq 0 ]]; then
      violations+="${file}:${lineno}: legacy dismissal write without shadow_write_tombstone_claim within ±50 lines"$'\n'
    fi
  done < <(grep -nE "$write_pattern" "$file" 2>/dev/null || true)
done

if [[ -n "$violations" ]]; then
  echo "Legacy dismissal write(s) without paired shadow_write_tombstone_claim:"
  echo
  echo "$violations"
  echo
  echo "Each runtime write to a legacy dismissal table must be paired with"
  echo "a shadow_write_tombstone_claim call within 50 lines so the claim"
  echo "substrate stays in parity with legacy storage. Without the pair,"
  echo "commit_claim PRE-GATE misses the dismissal and the AI can"
  echo "re-surface the item on the next enrichment."
  echo
  echo "If a write is intentionally not a dismissal (e.g. role insert"
  echo "with NULL dismissed_at), add the file to allowed_basename_regex"
  echo "in this script with a comment justifying the exception."
  exit 1
fi

echo "All runtime legacy dismissal writes are paired with shadow_write_tombstone_claim."
