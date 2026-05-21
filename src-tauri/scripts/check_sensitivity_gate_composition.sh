#!/usr/bin/env bash
#
# Sensitivity-gate composition lint (AC-477.11).
#
# Forbids `match … sensitivity` against `ClaimSensitivity` variants outside the
# canonical sensitivity gate at `abilities-runtime/src/sensitivity*`. The
# canonical primitives (`render_policy_for_surface`, `renderable_claim_text_with_value`)
# are the ONLY allowed branch points for rendering decisions; any parallel
# `match` on sensitivity that branches behavior is the same shape of drift the
# `prompt-channel-sensitivity-class-sweep` precedent (2026-05-18) called out.
#
# Pairs with `check_claim_writer_allowlist.sh` — same allowlist-with-rationale
# discipline: pre-existing non-policy mappings (serde naming, ordinal ranks,
# fixture seeding) are allowlisted; any new file matching the pattern fails CI
# until either folded into the canonical gate OR added to the allowlist with a
# rationale.
#
# AC-477.11 references this script by path.

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src" "src-tauri/tests")
else
  roots=("src" "tests")
fi

# Allowed-by-design files:
# - abilities-runtime/src/sensitivity*       — the canonical gate itself
# - services/sensitivity.rs                  — legacy parallel gate; consolidation tracked
#                                              under W2 surface migration follow-up
# - services/claims.rs                       — sensitivity_restriction_rank ordinal mapper
# - services/claims/canonicalization_parity.rs — fixture canonicalization
# - tests/dos412_mcp_static_surface_test.rs  — fixture name mapper
# - tests/w5_c_detect_risk_shift_test.rs     — fixture name mapper
# - claim_receipt/auth.rs tests              — fixture seeding (sensitivity_name)
# - services/entity_intelligence/auth.rs     — boundary helper (allowlist itself
#                                              is the gate's allowlist; only invokes the
#                                              canonical gate through a synthetic claim)
allowed_basename_regex='abilities-runtime/src/sensitivity\.rs|abilities-runtime/src/sensitivity/|src/services/sensitivity\.rs|src/services/claims\.rs|src/services/claims/canonicalization_parity\.rs|src/services/entity_intelligence/auth\.rs|src/services/claim_receipt/auth\.rs|tests/dos412_mcp_static_surface_test\.rs|tests/w5_c_detect_risk_shift_test\.rs'

# Pattern: `match` token on a binding ending in `sensitivity` (claim.sensitivity,
# value, &claim.sensitivity, etc.). Captures the policy-decision shape and
# misses unrelated `match` calls that happen to mention sensitivity in
# comments. Trailing-context `match sensitivity {` and `match claim.sensitivity {`
# both hit.
pattern='\<match[[:space:]]+([&*]?[[:space:]]*)?([A-Za-z_][A-Za-z0-9_]*(\.|::))*sensitivity[[:space:]]*\{'

matches="$(
  grep -rEni --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -Ev "($allowed_basename_regex)" \
    | grep -v 'sensitivity-gate-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Direct \`match … sensitivity\` outside the canonical gate is forbidden."
  echo
  echo "Compose the shipped abilities_runtime::sensitivity::render_policy_for_surface"
  echo "or services::entity_intelligence::auth::redact_provenance_for_surface instead"
  echo "of branching on ClaimSensitivity variants in-place. AC-477.11 / CSO Finding 1."
  echo
  echo "If this is a non-policy mapping (serde naming, ordinal rank, fixture seed),"
  echo "add the file to the allowlist in this script OR annotate the offending"
  echo "line with \`sensitivity-gate-allowed: <rationale>\`."
  echo
  echo "$matches"
  exit 1
fi

echo "All claim-sensitivity branching routes through the canonical gate."
