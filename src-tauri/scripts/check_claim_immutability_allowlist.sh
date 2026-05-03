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

# L2 cycle-19 fix: catch forbidden columns ANYWHERE in the SET clause,
# not just immediately after `SET`. The previous regex
# `SET[[:space:]]+(forbidden)` matched only the FIRST SET target,
# so a multi-column UPDATE like
#   `UPDATE intelligence_claims SET dedup_key = ?, subject_ref = ?`
# bypassed the gate because `dedup_key` was matched first and
# `subject_ref` slipped past as a non-leading column.
#
# New approach: get a window of context lines after each
# `UPDATE intelligence_claims` (covers multi-line UPDATEs), then
# scan each line in the window for `<forbidden_col>[[:space:]]*=`
# regardless of position. The dos7-allowed marker still exempts
# legitimate canonicalization rewrites (rekey path).
#
# 7 lines covers the longest UPDATE in the codebase plus margin.
matches="$(
  grep -rEni --include='*.rs' --include='*.sql' -A 7 \
    "UPDATE[[:space:]]+intelligence_claims" "${roots[@]}" 2>/dev/null \
    | grep -E "\b(text|claim_type|subject_ref|source_asof|created_at)[[:space:]]*=" \
    | grep -Ev "(text|claim_type|subject_ref|source_asof|created_at)[[:space:]]*=[[:space:]]*=" \
    | grep -v 'dos7-allowed:' \
    || true
)"

combined="$(printf '%s\n' "$matches" | sort -u | sed '/^$/d' || true)"

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
