#!/usr/bin/env bash
#
# DOS-7 D4-2 lint: assertion columns on intelligence_claims are immutable
# after insert. Per plan §"Memory substrate amendments — D. Immutability
# allowlist (formalized)":
#
# UPDATE-FORBIDDEN columns (assertion identity — never change after insert):
#   text, claim_type, subject_ref, source_asof, created_at
#
# UPDATE-ALLOWED columns (lifecycle, trust, threading — owned by services):
#   claim_state, surfacing_state, demotion_reason, reactivated_at,
#   superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
#   retraction_reason, expires_at, item_hash (legacy backfill only)
#
# This lint scans any `UPDATE intelligence_claims SET <col>` where <col> is
# in the forbidden list. The services/claims.rs writer is exempt because
# the assertion columns there are only set via INSERT, never UPDATE.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

# Match UPDATE intelligence_claims followed by SET <forbidden_column>.
# Multi-line UPDATEs are common; we run a 3-line context grep first then
# filter for forbidden columns.
forbidden_pattern='UPDATE[[:space:]]+intelligence_claims[[:space:]]+SET[[:space:]]+("?)(text|claim_type|subject_ref|source_asof|created_at)("?)[[:space:]]*='

# Also catch the multi-line case where UPDATE is on one line and SET <col> is on the next.
# A simple grep -A 2 followed by per-block matching is robust enough for the audit.

matches="$(
  grep -rEni --include='*.rs' --include='*.sql' -A 2 "UPDATE[[:space:]]+intelligence_claims" "${roots[@]}" 2>/dev/null \
    | grep -E "SET[[:space:]]+(text|claim_type|subject_ref|source_asof|created_at)[[:space:]]*=" \
    | grep -v 'dos7-allowed:' \
    || true
)"

# Single-line UPDATE pattern
single_line="$(
  grep -rEni --include='*.rs' --include='*.sql' "$forbidden_pattern" "${roots[@]}" 2>/dev/null \
    | grep -v 'dos7-allowed:' \
    || true
)"

combined="$(printf '%s\n%s\n' "$matches" "$single_line" | sort -u | sed '/^$/d' || true)"

if [[ -n "$combined" ]]; then
  echo "UPDATE on assertion-identity columns of intelligence_claims is forbidden."
  echo "Forbidden: text, claim_type, subject_ref, source_asof, created_at"
  echo "These are immutable after insert per plan amendment D."
  echo "Allowed lifecycle columns: claim_state, surfacing_state, superseded_by,"
  echo "  trust_score, trust_computed_at, trust_version, retraction_reason,"
  echo "  expires_at, demotion_reason, reactivated_at, thread_id."
  echo
  echo "$combined"
  exit 1
fi

echo "All UPDATE statements against intelligence_claims target lifecycle columns only."
