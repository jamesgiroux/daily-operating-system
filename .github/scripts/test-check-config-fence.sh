#!/usr/bin/env bash
# test-check-config-fence.sh — smoke tests for check-config-fence.sh
#
# Exercises the three documented behaviors:
#   1. Non-fenced paths exit 0.
#   2. Exact-pattern fenced files exit 2.
#   3. Prefix-fenced files exit 2.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FENCE="${SCRIPT_DIR}/check-config-fence.sh"

failures=0

assert_exit() {
  local description="$1"
  local expected="$2"
  local input="$3"
  local actual
  set +e
  printf '%s\n' "$input" | bash "$FENCE" >/dev/null 2>&1
  actual=$?
  set -e
  if [ "$actual" -ne "$expected" ]; then
    printf 'FAIL: %s — expected exit %s, got %s\n' "$description" "$expected" "$actual" >&2
    failures=$((failures + 1))
  else
    printf 'PASS: %s (exit %s)\n' "$description" "$actual"
  fi
}

assert_exit "non-fenced path (README.md) exits 0" 0 "README.md"
assert_exit "exact-pattern fenced (.github/workflows/l2-review.yml) exits 2" 2 ".github/workflows/l2-review.yml"
assert_exit "prefix-fenced (.github/scripts/check-config-fence.sh) exits 2" 2 ".github/scripts/check-config-fence.sh"

if [ "$failures" -ne 0 ]; then
  printf '\n%s test(s) failed.\n' "$failures" >&2
  exit 1
fi

printf '\nAll tests passed.\n'
