#!/usr/bin/env bash
# Purpose: fixture-test the ability description lint gate against blocked and clean descriptions.
# Exit codes: 0 when all fixture assertions pass; 1 when any assertion fails.
# How to run: ./scripts/check_ability_descriptions_test.sh

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
LINT_SCRIPT="${ROOT_DIR}/scripts/check_ability_descriptions.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ability-description-lint.XXXXXX")"
PII_BLOCKLIST="${ROOT_DIR}/.claude/pii-blocklist.txt"
CREATED_PII_BLOCKLIST=0

cleanup() {
  rm -rf "$TMP_DIR"
  if [ "$CREATED_PII_BLOCKLIST" -eq 1 ]; then
    rm -f "$PII_BLOCKLIST"
    rmdir "${ROOT_DIR}/.claude" 2>/dev/null || true
  fi
}
trap cleanup EXIT

fail() {
  echo "ability description lint fixture test failed: $1" >&2
  if [ "${2:-}" != "" ] && [ -f "$2" ]; then
    sed 's/^/  /' "$2" >&2
  fi
  exit 1
}

if [ ! -f "$PII_BLOCKLIST" ]; then
  mkdir -p "$(dirname "$PII_BLOCKLIST")"
  printf '# Test fixture PII blocklist.\npii-fixture-marker\n' > "$PII_BLOCKLIST"
  CREATED_PII_BLOCKLIST=1
fi

BAD_RS="${TMP_DIR}/bad_ability.rs"
BAD_OUT="${TMP_DIR}/bad.out"
cat > "$BAD_RS" <<'RS'
#[ability(name = "foo", description = "covers our intelligence pipeline run for pii-fixture-marker")]
fn foo() {}
RS

set +e
ABILITY_DESC_LINT_SCAN_PATHS="$BAD_RS" "$LINT_SCRIPT" > "$BAD_OUT" 2>&1
BAD_STATUS=$?
set -e

if [ "$BAD_STATUS" -ne 1 ]; then
  fail "expected blocked fixture to exit 1, got ${BAD_STATUS}" "$BAD_OUT"
fi

if ! grep -q 'intelligence pipeline' "$BAD_OUT"; then
  fail 'expected blocked fixture output to mention "intelligence pipeline"' "$BAD_OUT"
fi

if ! grep -q 'pii-fixture-marker' "$BAD_OUT"; then
  fail 'expected blocked fixture output to mention "pii-fixture-marker"' "$BAD_OUT"
fi

CLEAN_RS="${TMP_DIR}/clean_ability.rs"
CLEAN_OUT="${TMP_DIR}/clean.out"
cat > "$CLEAN_RS" <<'RS'
#[ability(name = "bar", description = "Return entity context for a subject reference.")]
fn bar() {}
RS

set +e
ABILITY_DESC_LINT_SCAN_PATHS="$CLEAN_RS" "$LINT_SCRIPT" > "$CLEAN_OUT" 2>&1
CLEAN_STATUS=$?
set -e

if [ "$CLEAN_STATUS" -ne 0 ]; then
  fail "expected clean fixture to exit 0, got ${CLEAN_STATUS}" "$CLEAN_OUT"
fi

echo "ability description lint fixture test: ok"
