#!/usr/bin/env bash
#
# Every runtime write to a legacy dismissal
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
# (schema setup + transitional dual-write), DB modules (raw
# SQL methods called from services that pair the shadow-write
# externally), and tests/fixtures.
allowed_basename_regex='services/claims_backfill\.rs|services/claims\.rs|migrations/|migrations\.rs|db/intelligence_feedback\.rs|db/feedback\.rs|db/signals\.rs|db/projects\.rs|db/accounts\.rs|db/entity_linking\.rs|db/mod_tests\.rs|tests/dos7_d3a1_backfill_test\.rs|tests/dos7_d3a2_backfill_test\.rs|tests/dos7_d1_schema_test\.rs|tests/dos7_d4_lint_test\.rs|tests/dos7_d5_ghost_resurrection_test\.rs|tests/dos309_lint_regex_test\.rs|tests/dos311_fixtures/|intel_queue\.rs|demo\.rs|signals/rules\.rs|devtools/'

# Pattern for legacy-dismissal-table writes at the service/command
# layer. Match raw SQL only, not function-call call-sites — call-sites
# in commands/ delegate to services, which pair internally.
#
# We deliberately do NOT match `INSERT INTO suppression_tombstones_quarantine`
# (audit trail) or generic `account_stakeholder_roles`
# inserts without `dismissed_at` (those create roles, not dismissals).
#
# L2 cycle-20 fix #2: previously matched only
#   `UPDATE briefing_callouts SET dismissed_at = ...`
# which let `UPDATE briefing_callouts SET other_col = ?, dismissed_at = ?`
# slip through (same first-SET-column blind spot the immutability
# lint had pre-cycle-19).
#
# L2 cycle-22 fix #3: cycle-20's `UPDATE … SET [^;]* dismissed_at`
# regex was single-line only, so multi-line UPDATEs (where Rust
# string-literal line-continuation `\\` puts `SET` on one source
# line and `dismissed_at` on the next) bypassed the lint. The
# cycle-22 fix uses a TWO-PHASE detection: first locate UPDATEs
# on legacy dismissal tables (any UPDATE on the table, single
# or multi-line), then for each match scan a 7-line context
# window after the UPDATE for `dismissed_at[[:space:]]*=`. Plus
# helper-call patterns for the in-DB write functions that
# bypass raw SQL (db.upsert_linking_dismissal, etc).

# Phase 1: line patterns whose matches always count as a legacy
# dismissal write (direct INSERTs and helper-call sites).
direct_write_pattern='INSERT[[:space:]]+INTO[[:space:]]+(email_dismissals|meeting_entity_dismissals|linking_dismissals|nudge_dismissals|triage_snoozes)|INSERT[[:space:]]+OR[[:space:]]+IGNORE[[:space:]]+INTO[[:space:]]+linking_dismissals|\bdb\.(upsert_linking_dismissal|dismiss_email_item|record_meeting_entity_dismissal|snooze_triage_item|resolve_triage_item|create_suppression_tombstone)\b'

# Phase 2: UPDATE on a legacy table that may set dismissed_at on
# a later line. For each match, the per-file scan checks the
# next 7 lines for `dismissed_at[[:space:]]*=`.
update_lookahead_pattern='UPDATE[[:space:]]+(briefing_callouts|account_stakeholder_roles)\b'

# Files containing any candidate match, after allowlist exclusion.
candidate_files="$(
  {
    grep -rEli --include='*.rs' "$direct_write_pattern" "${roots[@]}" 2>/dev/null
    grep -rEli --include='*.rs' "$update_lookahead_pattern" "${roots[@]}" 2>/dev/null
  } \
    | sort -u \
    | grep -Ev "$allowed_basename_regex" \
    || true
)"

if [[ -z "$candidate_files" ]]; then
  echo "All runtime legacy dismissal writes are paired with shadow_write_tombstone_claim (none in non-allowlisted files)."
  exit 0
fi

violations=""
for file in $candidate_files; do
  # Direct-write line numbers.
  direct_hits="$(grep -nE "$direct_write_pattern" "$file" 2>/dev/null \
    | cut -d: -f1 || true)"

  # UPDATE lookahead: for each UPDATE <legacy_table> line,
  # check the next 7 lines for `dismissed_at[[:space:]]*=`.
  update_hits=""
  while read -r upd_lineno; do
    [[ -z "$upd_lineno" ]] && continue
    start_check=$upd_lineno
    end_check=$((upd_lineno + 7))
    if sed -n "${start_check},${end_check}p" "$file" \
      | grep -qE '("|`|\[)?\bdismissed_at\b("|`|\])?[[:space:]]*='; then
      update_hits+="${upd_lineno}"$'\n'
    fi
  done < <(grep -nE "$update_lookahead_pattern" "$file" 2>/dev/null \
    | cut -d: -f1 || true)

  all_hits="$(printf '%s\n%s\n' "$direct_hits" "$update_hits" | sort -u | sed '/^$/d')"

  while read -r lineno; do
    [[ -z "$lineno" ]] && continue
    start=$((lineno - 50))
    [[ $start -lt 1 ]] && start=1
    end=$((lineno + 50))
    # Marker-bypass: if a `dos7-allowed:` comment exists within ±10
    # lines of the matching write, treat as authorized (test
    # fixtures in production-file #[cfg(test)] modules use this).
    marker_start=$((lineno - 10))
    [[ $marker_start -lt 1 ]] && marker_start=1
    marker_end=$((lineno + 10))
    has_marker="$(sed -n "${marker_start},${marker_end}p" "$file" | grep -c 'dos7-allowed:' || true)"
    if [[ "$has_marker" -gt 0 ]]; then
      continue
    fi
    has_shadow="$(sed -n "${start},${end}p" "$file" | grep -c 'shadow_write_tombstone_claim' || true)"
    if [[ "$has_shadow" -eq 0 ]]; then
      violations+="${file}:${lineno}: legacy dismissal write without shadow_write_tombstone_claim within ±50 lines"$'\n'
    fi
  done <<< "$all_hits"
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
