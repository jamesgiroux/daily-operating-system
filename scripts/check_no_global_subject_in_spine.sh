#!/usr/bin/env bash
#  spine restriction (interim guard until  ships
# CLAIM_TYPE_REGISTRY): v1.4.0 production code must not construct
# `SubjectRef::Global` directly. Match arms for the variant are
# permitted (they route to the correct epoch path); construction is
# what triggers an actual bump of `migration_state.global_claim_epoch`,
# which v1.4.0 spine has decided not to use.
#
# Catches:
#   - SubjectRef::Global { ... }
#   - SubjectRef::Global,    (struct/struct literal)
#   - = SubjectRef::Global   (assignment)
#   - return SubjectRef::Global  (return)
# Allowlist:
#   - db/claim_invalidation.rs   — definition + match arms + tests
#   - tests/                     — integration tests
#   - .docs/, scripts/           — documentation and tooling
#
# Replaced by the CLAIM_TYPE_REGISTRY-aware compile-time guard introduced
# by (W3) per ADR-0125.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Match SubjectRef::Global as a constructor (followed by `,`, `;`, `(`,
# `{`, `}`, ` `, end-of-line, or `)`). Excludes match-arm patterns
# `SubjectRef::Global =>` because those are routing, not construction.
PATTERN='SubjectRef::Global([[:space:]]*[,;(){}}\)\b]|$)'

violations=0
while IFS= read -r line; do
  case "$line" in
    "$ROOT_DIR/src-tauri/src/db/claim_invalidation.rs"*) continue ;;
    "$ROOT_DIR/src-tauri/tests/"*) continue ;;
    "$ROOT_DIR/.docs/"*) continue ;;
    "$ROOT_DIR/scripts/"*) continue ;;
  esac
  # Skip match-arm patterns (-> or =>) within the same line
  if echo "$line" | grep -qE 'SubjectRef::Global[[:space:]]*=>'; then
    continue
  fi
  echo "$line"
  violations=$((violations + 1))
done < <(grep -rEn "$PATTERN" "$ROOT_DIR/src-tauri/src/" 2>/dev/null || true)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: ${violations} construction(s) of SubjectRef::Global outside the allowlist."
  echo "Spine restriction (DOS-310): v1.4.0 does not register any claim_type"
  echo "with canonical_subject_types containing Global. Production code must"
  echo "not construct this variant until DOS-7 (W3) ships the CLAIM_TYPE_REGISTRY"
  echo "guard per ADR-0125."
  exit 1
fi
