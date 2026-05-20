#!/usr/bin/env bash
# check-chrome-block-collision.sh — enforce the Pill duality invariant from ADR 0132.
#
# A primitive may live in EITHER the chrome layer (runtime-injected via
# chrome.js, CSS at wp/dailyos/theme/assets/chrome/styles/<Name>.module.css)
# OR the block layer (Gutenberg block at wp/dailyos/blocks/<name>/), but
# not both — with the explicit allowlist below as the only exception.
#
# Pill is grandfathered because it serves two genuinely different roles
# (content-authoring primitive + chrome-internal primitive). Future
# dual-existence requests must amend the allowlist via a follow-on ADR.
#
# Exits 0 when every collision is on the allowlist; exits 1 with a
# per-collision diagnostic otherwise.
#
# Anchored in:
#   - ADR 0132 (Pill primitive dual-existence)
#   - L0 packet v1.4.4-wp-surface-migration §5.2 AC #6 + §10 invariants + AC #28

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CHROME_STYLES="$REPO_ROOT/wp/dailyos/theme/assets/chrome/styles"
BLOCKS_DIR="$REPO_ROOT/wp/dailyos/blocks"

# Allowlist — keep in sync with ADR 0132. Lowercase primitive names only.
ALLOWLIST=(
  "pill"
)

is_allowed() {
  local name="$1"
  for allowed in "${ALLOWLIST[@]}"; do
    if [[ "$name" == "$allowed" ]]; then
      return 0
    fi
  done
  return 1
}

if [[ ! -d "$CHROME_STYLES" ]]; then
  echo "chrome-block collision check skipped: $CHROME_STYLES not found"
  exit 0
fi

if [[ ! -d "$BLOCKS_DIR" ]]; then
  echo "chrome-block collision check skipped: $BLOCKS_DIR not found"
  exit 0
fi

violations=0

for chrome_module in "$CHROME_STYLES"/*.module.css; do
  [[ -e "$chrome_module" ]] || continue
  module_name="$(basename "$chrome_module" .module.css)"
  primitive="$(echo "$module_name" | tr '[:upper:]' '[:lower:]')"
  block_dir="$BLOCKS_DIR/$primitive"
  if [[ -d "$block_dir" ]]; then
    if is_allowed "$primitive"; then
      continue
    fi
    echo "chrome-block collision: $primitive exists in BOTH" >&2
    echo "  chrome CSS:  ${chrome_module#$REPO_ROOT/}" >&2
    echo "  block dir:   ${block_dir#$REPO_ROOT/}" >&2
    echo "  Resolve by removing one side, or amend ADR 0132 + the ALLOWLIST in this script." >&2
    violations=$((violations + 1))
  fi
done

if (( violations > 0 )); then
  echo "" >&2
  echo "chrome-block collision check FAILED: $violations primitive(s) violate the dual-existence invariant." >&2
  exit 1
fi

echo "chrome-block collision check passed (allowlist: ${ALLOWLIST[*]})"
