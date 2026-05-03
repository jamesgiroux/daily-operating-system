#!/usr/bin/env bash
#
# Claims-substrate lint: claims are written ONLY through services/claims.rs.
#
# Forbids INSERT/UPDATE statements against intelligence_claims,
# claim_corroborations, claim_contradictions outside the
# services/claims.rs writer service. The backfill migrations
# (130_dos_7_claims_backfill_a1.sql, 131_dos_7_claims_backfill_a2.sql)
# are explicit one-time exceptions — they bypass the runtime gate
# precisely because they're the cutover mechanism.
#
# Claims plan acceptance: all writes go through services/claims.rs; CI
# enumerates write sites and rejects direct INSERT INTO intelligence_claims
# elsewhere.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

# Allowed-by-design files (full or trailing path match). Backfill
# migrations are the cutover mechanism; the claims service is the
# canonical writer; claims_backfill wraps cutover orchestration and
# JSON-blob backfill writes that cannot be expressed as live proposals.
# Tests intentionally seed legacy tables and re-run backfill SQL through
# execute_batch. db/intelligence_feedback.rs is read-only in production;
# its claim INSERTs live in test parity fixtures. db/data_lifecycle.rs
# seeds Email-subject claims in cascade tests and performs lifecycle-only
# UPDATEs during hard-delete purges. services/source_asof_backfill.rs
# lifts knowable source timestamps into legacy rows before freshness
# scoring reads the substrate.
allowed_basename_regex='services/claims\.rs|services/claims_backfill\.rs|services/source_asof_backfill\.rs|services/meetings\.rs|migrations/130_dos_7_claims_backfill_a1\.sql|migrations/131_dos_7_claims_backfill_a2\.sql|migrations/129_dos_7_claims_schema\.sql|migrations/133_dos_7_withdraw_unsupported_m5_kinds\.sql|migrations/136_dos_299_source_asof_quarantine\.sql|db/intelligence_feedback\.rs|db/data_lifecycle\.rs|tests/dos7_d3a1_backfill_test\.rs|tests/dos7_d3a2_backfill_test\.rs|tests/dos7_d1_schema_test\.rs|tests/dos7_d5_ghost_resurrection_test\.rs|tests/dos7_d4_lint_test\.rs|tests/dos311_fixtures/'

pattern='(INSERT([[:space:]]+OR[[:space:]]+(IGNORE|REPLACE))?[[:space:]]+INTO|REPLACE[[:space:]]+INTO|UPDATE)[[:space:]]+(intelligence_claims|claim_corroborations|claim_contradictions)\b'

matches="$(
  grep -rEni --include='*.rs' --include='*.sql' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_basename_regex)" \
    | grep -v 'dos7-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct INSERT/UPDATE against claim tables outside the allowlist is forbidden."
  echo "Route writes through services/claims.rs::commit_claim, ::record_corroboration,"
  echo "or ::reconcile_contradiction. Backfill migrations are the only documented exception."
  echo
  echo "Allowed files:"
  echo "  - src-tauri/src/services/claims.rs"
  echo "  - src-tauri/src/services/claims_backfill.rs"
  echo "  - src-tauri/src/services/source_asof_backfill.rs"
  echo "  - src-tauri/src/migrations/{129,130,131}_dos_7_*.sql"
  echo "  - src-tauri/tests/dos7_d{1,3a1,3a2}_*.rs (test seeding via execute_batch)"
  echo
  echo "$matches"
  exit 1
fi

echo "All writes to claim tables route through the services/claims.rs allowlist."
