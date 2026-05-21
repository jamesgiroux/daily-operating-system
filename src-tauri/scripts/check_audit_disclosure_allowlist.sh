#!/usr/bin/env bash
#
# Receipt vs operational audit boundary — CI lint.
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
# Tests for audit-management surface itself. The audit-row tests
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

# -----------------------------------------------------------------------------
# CSO cycle-1 Finding 11 extension:
# Forbid direct read of maintenance_audit rows from services::claim_receipt::*
# (privacy module writes assertions but never surfaces raw rows)
# -----------------------------------------------------------------------------
# `maintenance_audit` rows must be either (a) audit-management allowlisted in
# that lint OR (b) routed through `build_receipt_for_audience` with a
# non-OperationalAuditStorage audience.
#
# Exit non-zero on violation; print offending file:line.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CLAIM_RECEIPT_DIR="$REPO_ROOT/src-tauri/src/services/claim_receipt"

if [ ! -d "$CLAIM_RECEIPT_DIR" ]; then
  echo "ERROR: $CLAIM_RECEIPT_DIR not found"
  exit 2
fi

# Forbidden patterns: any SQL or rusqlite read of the `maintenance_audit`
# table from inside services::claim_receipt::*.
#
# The match accepts common forms: `FROM maintenance_audit`, `SELECT ... maintenance_audit`,
# `JOIN maintenance_audit`, and bare references in `prepare()`/`query()` calls.
PATTERN='maintenance_audit'

# Allow:
# - documentation/comments (lines beginning with // or in /* */)
# - the explicit non-disclosure tag in audience identifier strings
#   (e.g. "operational_audit_storage")
# The grep -F skips regex; we use grep -nE then filter false positives.

violations=$(grep -rEn "${PATTERN}" "$CLAIM_RECEIPT_DIR" \
  --include='*.rs' \
  2>/dev/null \
  | grep -v 'operational_audit_storage' \
  | grep -v -E ':\s*//' \
  | grep -v -E ':\s*\*' \
  | grep -v 'AC-341.11' \
  | grep -v 'CI lint' \
  | grep -v 'non-disclosure tag' \
  || true)

if [ -n "$violations" ]; then
  echo "VIOLATION: direct read of maintenance_audit from services::claim_receipt::*"
  echo "$violations"
  echo
  echo "Route through build_receipt_for_audience with a non-OperationalAuditStorage"
  echo "audience, OR add an explicit DOS-340 §5.8 audit-management allowlist entry."
  exit 1
fi

echo "OK: no direct maintenance_audit reads from services::claim_receipt::*"
exit 0
