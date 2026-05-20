#!/usr/bin/env bash
#
# DOS-340: Receipt vs operational audit boundary — CI lint.
#
# Operational audit tables (e.g. sensitivity_reveal_audit, audit_log,
# provenance_audit_storage, maintenance_audit) carry tamper-evident
# internal trace data — raw source ids, prompt hashes, correlation ids,
# raw model i/o, private snippets. Reading those rows from any
# receipt / activity-log / lint surface bypasses the boundary contract
# defined in services::claim_receipt::boundary.
#
# Allowed readers:
#   * commands/audit_management*.rs  — audit-management surface only
#   * services/source_asof_backfill, services/claims_backfill — migration paths
#   * services/claim_receipt/privacy.rs — privacy module MAY WRITE assertions
#       (per AC-341.11), but never SELECT raw rows from receipt path
#   * the migration files that declare audit tables
#
# Forbidden readers:
#   * services/claim_receipt/* (anywhere except privacy assertions)
#   * services/entity_intelligence/*
#   * any commands/activity*, commands/lint*
#   * all W2-W5 block render entry points (PHP land — out of scope for this
#     shell lint; covered by separate render gate)
#
# Pattern modeled on check_claim_writer_allowlist.sh.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

# Names of audit tables (operational, non-receipt). Extending this list is
# expected as new audit storage lands; the companion completeness check
# (check_audit_denylist_completeness.sh) ensures every column gets a
# classification.
audit_tables='sensitivity_reveal_audit|audit_log|provenance_audit_storage|maintenance_audit|legacy_user_note_migration_audit'

# Files that are ALLOWED to read these tables.
allowed_basename_regex='commands/audit_management[^:]*\.rs'
allowed_basename_regex="${allowed_basename_regex}|services/source_asof_backfill\.rs"
allowed_basename_regex="${allowed_basename_regex}|services/claims_backfill\.rs"
allowed_basename_regex="${allowed_basename_regex}|migrations/[^:]+\.(sql|rs)"
# Tests for audit-management surface itself. DOS-412 / DOS-411 tests
# assert audit-row behaviour directly and are NOT receipt-rendering
# paths.
allowed_basename_regex="${allowed_basename_regex}|tests/dos411_user_note_migration_test\.rs"
allowed_basename_regex="${allowed_basename_regex}|tests/dos412_[^:]+_test\.rs"
# Negative-fixture for THIS lint (proves the lint catches a disclosure
# attempt). Marker: file path under tests/audit_disclosure_negative/.
allowed_basename_regex="${allowed_basename_regex}|tests/audit_disclosure_negative/"

# Read pattern: SELECT FROM <audit table>. Also covers the rusqlite
# prepare() form `prepare("SELECT ... FROM audit_log ...")`.
pattern="SELECT[[:space:]].*FROM[[:space:]]+(${audit_tables})\b"

matches="$(
  grep -rEni --include='*.rs' --include='*.sql' -- "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_basename_regex)" \
    | grep -v 'audit-disclosure-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "DOS-340: direct SELECT against operational audit table outside the allowlist." >&2
  echo "Route through commands/audit_management*.rs only. Receipts/activity-log/lint" >&2
  echo "paths surface claim-substrate state, not raw audit rows." >&2
  echo >&2
  echo "Per-line override comment: '// audit-disclosure-allowed: <reason>'" >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

# Stronger guard for the claim_receipt module: forbid SELECT against an
# audit table inside services/claim_receipt/*. AC-341.11.
# Doc comments (lines starting with `//`, `//!`, or `///`) are scanned
# for prose references and excluded from the SELECT match — privacy.rs
# documentation refers to maintenance_audit by name as part of the
# boundary contract.
claim_receipt_matches="$(
  grep -rEni --include='*.rs' -- "SELECT[[:space:]].*FROM[[:space:]]+(${audit_tables})\b" \
    "${roots[0]}/services/claim_receipt" 2>/dev/null \
    | grep -v 'audit-disclosure-allowed:' \
    || true
)"

if [[ -n "$claim_receipt_matches" ]]; then
  echo "DOS-340 / AC-341.11: services::claim_receipt::* MUST NOT SELECT" >&2
  echo "from operational audit tables — privacy.rs may WRITE assertions but" >&2
  echo "never surface raw rows." >&2
  echo >&2
  echo "$claim_receipt_matches" >&2
  exit 1
fi

echo "DOS-340: receipt-vs-audit disclosure allowlist clean."
