#!/usr/bin/env bash
# DOS-341 AC-341.11 / CSO cycle-1 Finding 11.
#
# OperationalAuditStorage is a non-disclosure tag, not a render target.
# This lint forbids any direct read of `maintenance_audit` table rows from
# `services::claim_receipt::*`. The privacy module is allowed to *write*
# assertions through this audience but never to *surface* raw rows.
#
# Pair with DOS-340 §5.8 audit-management allowlist: any call site touching
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
