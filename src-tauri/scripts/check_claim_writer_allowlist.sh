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

# Allowed-by-design files (full or trailing path match). The backfill
# migrations are the cutover mechanism; the claims service is the
# canonical writer; the claims_backfill module wraps cutover orchestration
# + JSON-blob backfill writes. Tests intentionally seed legacy tables and
# re-run backfill SQL via execute_batch — they're scoped to dos7_d3a*.
# db/intelligence_feedback.rs is exempt because the only INSERTs in that
# file live inside `#[cfg(test)] mod tests` to seed the D5-2 parity tests
# (see fn seed_pair); production reads from the file are read-only.
# db/data_lifecycle.rs is exempt for the L2 cycle-4 fix #2 cascade:
# when emails are hard-deleted (purge_aged_emails or DataSource::Google
# full purge), the corresponding Email-subject claim rows must be
# transitioned from active/tombstoned/dormant to `withdrawn` so the
# substrate doesn't carry stale suppression for a re-imported email.
# Only claim_state + retraction_reason are touched (both in the
# UPDATE-allowed list per check_claim_immutability_allowlist.sh).
# services/source_asof_backfill.rs is exempt because the cutover must
# lift knowable source timestamps into legacy rows before trust freshness
# scoring reads the substrate.
allowed_basename_regex='services/claims\.rs|services/claims_backfill\.rs|services/source_asof_backfill\.rs|services/meetings\.rs|migrations/130_dos_7_claims_backfill_a1\.sql|migrations/131_dos_7_claims_backfill_a2\.sql|migrations/129_dos_7_claims_schema\.sql|migrations/133_dos_7_withdraw_unsupported_m5_kinds\.sql|migrations/136_dos_299_source_asof_quarantine\.sql|db/intelligence_feedback\.rs|db/data_lifecycle\.rs|tests/dos7_d3a1_backfill_test\.rs|tests/dos7_d3a2_backfill_test\.rs|tests/dos7_d1_schema_test\.rs|tests/dos7_d5_ghost_resurrection_test\.rs|tests/dos7_d4_lint_test\.rs|tests/dos311_fixtures/'

pattern='(INSERT[[:space:]]+INTO|UPDATE)[[:space:]]+(intelligence_claims|claim_corroborations|claim_contradictions)\b'

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
