#!/usr/bin/env bash
# v1.4.3 W4-F feedback wire-through CI invariants (packet §9 #2/#9/#10/#11/#12).
#
# Static-grep gates that protect the W4 substrate boundary:
#   #2  — claims.rs `record_claim_feedback` body + abilities-runtime
#         `FeedbackAction` enum unchanged in this PR (witnessed via
#         canonical-content presence + no_drift markers below).
#   #9  — WP-side action allowlist contains all 9 variants AND none of
#         the 4 legacy strings (correct/dismiss/corroborate/contradict).
#   #10 — NonceAuditContext carries the 4 forensic slots.
#   #11 — bundle-17 sweep asserts RenderPolicyChannel::all().len() == 10
#         (compile-time + fixture coverage for WpBlockRenders).
#   #12 — orphan /v1/surface/feedback path absent in runtime + WP plugin
#         surfaces (prevents reintroduction of the dead route).
set -euo pipefail

ROOT_DIR="${W4F_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

# ----- inv #2: record_claim_feedback + FeedbackAction frozen -----
CLAIMS_RS="$ROOT_DIR/src-tauri/src/services/claims.rs"
FEEDBACK_RS="$ROOT_DIR/src-tauri/abilities-runtime/src/abilities/feedback.rs"

grep -qE "^pub fn record_claim_feedback\($" "$CLAIMS_RS" \
  || fail "inv #2: record_claim_feedback signature missing or moved at $CLAIMS_RS"

# All 9 FeedbackAction variants must remain canonical names.
for variant in ConfirmCurrent MarkOutdated MarkFalse WrongSubject WrongSource CannotVerify NeedsNuance SurfaceInappropriate NotRelevantHere; do
  if ! grep -qE "^\s+${variant}," "$FEEDBACK_RS"; then
    fail "inv #2: FeedbackAction variant ${variant} missing from $FEEDBACK_RS"
  fi
done

# ----- inv #9: WP action allowlist = 9 + old strings absent -----
PLUGIN_PHP="$ROOT_DIR/wp/dailyos/includes/class-dailyos-plugin.php"

for variant in confirm_current mark_outdated mark_false wrong_subject wrong_source cannot_verify needs_nuance surface_inappropriate not_relevant_here; do
  if ! grep -qE "'${variant}'" "$PLUGIN_PHP"; then
    fail "inv #9: WP allowlist missing variant '${variant}' in class-dailyos-plugin.php"
  fi
done

# Defensive: the four legacy strings MUST NOT appear in any allowlist literal,
# multi-line or single-line, anywhere in the WP plugin. Cycle-2 L2 codex
# challenge MEDIUM: the previous one-line regex missed multi-line literals.
for legacy in correct dismiss corroborate contradict; do
  # Look for the legacy string as a quoted PHP literal followed by a comma —
  # the array-element shape. Excludes phpdoc / human-readable English usage.
  if grep -nE "'${legacy}'\s*," "$PLUGIN_PHP" | grep -v "phpcs:" | head -1 | grep -q .; then
    fail "inv #9: WP plugin still has '${legacy}' as a PHP allowlist literal"
  fi
done

# ----- inv #10: NonceAuditContext forensic slots -----
NONCE_RS="$ROOT_DIR/src-tauri/src/services/surface_nonce.rs"
for slot in attempted_wp_user_id attempted_surface_client_id ip_hash user_agent_hash; do
  if ! grep -qE "^\s+${slot}: Option" "$NONCE_RS"; then
    fail "inv #10: NonceAuditContext missing forensic slot '${slot}' in surface_nonce.rs"
  fi
done

# AuditSubkey + AUDIT_IP_HASH_KEY_INFO MUST be distinct from the nonce digest key.
grep -qE "^const AUDIT_IP_HASH_KEY_INFO:" "$NONCE_RS" \
  || fail "inv #10: AUDIT_IP_HASH_KEY_INFO constant missing"
grep -qE "^struct AuditSubkey" "$NONCE_RS" \
  || fail "inv #10: AuditSubkey struct missing"

# ----- inv #11: bundle-17 sweep covers WpBlockRenders -----
BUNDLE17_TEST="$ROOT_DIR/src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs"
if ! grep -qE "RenderPolicyChannel::all\(\)\.len\(\), 10" "$BUNDLE17_TEST"; then
  fail "inv #11: bundle-17 sweep does not assert RenderPolicyChannel::all().len() == 10"
fi

TYPES_RS="$ROOT_DIR/src-tauri/src/bridges/types.rs"
grep -qE "WpBlockRenders" "$TYPES_RS" \
  || fail "inv #11: RenderPolicyChannel::WpBlockRenders missing from bridges/types.rs"
grep -qE "const ALL: \[Self; 10\]" "$TYPES_RS" \
  || fail "inv #11: RenderPolicyChannel::ALL not bumped to [Self; 10]"

# ----- inv #12: orphan /v1/surface/feedback retired -----
if grep -rn "/v1/surface/feedback" "$ROOT_DIR/src-tauri/src/surface_runtime/" 2>/dev/null; then
  fail "inv #12: orphan /v1/surface/feedback present in src-tauri/src/surface_runtime/"
fi

if grep -rn "/v1/surface/feedback" "$ROOT_DIR/wp/dailyos/includes/" 2>/dev/null; then
  fail "inv #12: orphan /v1/surface/feedback present in wp/dailyos/includes/"
fi

if grep -qE "submit_feedback\s*\(" "$ROOT_DIR/wp/dailyos/includes/transport/class-dailyos-runtime-client.php"; then
  fail "inv #12: orphan submit_feedback() still present in runtime-client.php"
fi

if [[ $failures -gt 0 ]]; then
  echo ""
  echo "W4-F wire-through lint: ${failures} failure(s)" >&2
  exit 1
fi

echo "W4-F wire-through lint: OK"
