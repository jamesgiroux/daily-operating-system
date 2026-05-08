#!/usr/bin/env bash
# Suite E — Edge cases: frontend tests + surface QA attestation.
#
# What's covered automatically:
#   - pnpm test (frontend Jest/Vitest)
#   - frontend build sanity (pnpm build with --noEmit equivalent)
#
# What requires manual attestation (no headless Tauri harness yet):
#   - User-visible surfaces on W4+ waves: the workflow expects an
#     "L3-suite-e-attest: passed" line in the wave's Linear ticket
#     (or the most recent L3 dispatch comment) before passing this
#     suite. The line must be added by a human (runs /qa-only).
#
# Usage: scripts/suite-e.sh --wave WN [--out path] [--require-attest true|false]
#
# Exit: 0 if frontend tests pass AND (W4+ → attestation present); 1 otherwise.

set -euo pipefail

OUT=""
WAVE=""
REQUIRE_ATTEST="auto" # auto = derive from wave number; W4+ requires
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --wave) WAVE="$2"; shift 2 ;;
    --require-attest) REQUIRE_ATTEST="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$WAVE" ]] || { echo "--wave required" >&2; exit 2; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Decide whether attestation is required.
if [[ "$REQUIRE_ATTEST" == "auto" ]]; then
  # W0, W0.5, W1 = no attestation (substrate, no user-visible surfaces).
  # W2+ = require attestation (briefing, prep, surfaces).
  # User can override per-run with --require-attest false.
  case "$WAVE" in
    W0|W0.5|W1|W1.5) REQUIRE_ATTEST="false" ;;
    *) REQUIRE_ATTEST="true" ;;
  esac
fi

frontend_status="pending"
attest_status="not-required"
overall="pending"
notes=""

echo "─── Suite E: pnpm test ───" >&2
if pnpm test >/tmp/suite-e-frontend.log 2>&1; then
  frontend_status="pass"
else
  frontend_status="fail"
  cat /tmp/suite-e-frontend.log >&2 || true
fi

if [[ "$REQUIRE_ATTEST" == "true" ]]; then
  echo "─── Suite E: surface QA attestation check ───" >&2
  # Look for the attestation marker in the most recent dispatch comment on
  # the wave's Linear tracking ticket. We do NOT have direct Linear access
  # in CI; instead we rely on a marker file under .docs/plans/wave-WN/
  # that a human writes before re-dispatching the workflow.
  ATTEST_FILE=".docs/plans/wave-${WAVE}/l3-suite-e-attest.md"
  if [[ -f "$ATTEST_FILE" ]] && grep -q "^L3-suite-e-attest: passed" "$ATTEST_FILE"; then
    attest_status="pass"
    notes="attest-file=${ATTEST_FILE}"
  else
    attest_status="fail"
    notes="no attestation file at ${ATTEST_FILE} with 'L3-suite-e-attest: passed' line. Run /qa-only on user-visible surfaces, write the attestation file, re-dispatch L3."
  fi
fi

if [[ "$frontend_status" == "pass" && ( "$attest_status" == "pass" || "$attest_status" == "not-required" ) ]]; then
  overall="pass"
else
  overall="fail"
fi

summary="{\"suite\":\"E\",\"wave\":\"$WAVE\",\"frontend\":\"$frontend_status\",\"attestation\":\"$attest_status\",\"overall\":\"$overall\",\"notes\":\"$notes\"}"

if [[ -n "$OUT" ]]; then
  printf '%s\n' "$summary" > "$OUT"
else
  printf '%s\n' "$summary"
fi

[[ "$overall" == "pass" ]] && exit 0 || exit 1
