#!/bin/bash
# Snippet for .claude/hooks/pre-commit-gate.sh — paste before the final
# "# Block commit if any errors" block.
#
# Triggers when staged files touch reference surfaces, scoped CSS modules,
# chrome.js, or the canonical TSX/module CSS that any reference mirrors.
# Blocks commits that regress a reference HTML's audit severity vs baseline, or
# introduce global manifest/spec/token/router drift.
#
# To re-baseline (e.g. after intentionally accepting drift):
#   python3 .docs/design/_audits/audit-reference.py --write-baseline

if echo "$STAGED_FILES" | grep -qE '\.docs/design/reference/(surfaces|_shared)/|^src/(pages|components)/.*\.(tsx|module\.css)$'; then
  AUDIT_SCRIPT="${CWD}/.docs/design/_audits/audit-reference.py"
  if [ -f "$AUDIT_SCRIPT" ]; then
    if ! python3 "$AUDIT_SCRIPT" --enforce-baseline 2>&1; then
      ERRORS="${ERRORS}\n❌ Reference fidelity gate failed (see output above)."
      ERRORS="${ERRORS}\n   Either fix the reference HTML, or if the regression is intentional,"
      ERRORS="${ERRORS}\n   re-baseline:  python3 .docs/design/_audits/audit-reference.py --write-baseline"
    fi
  fi
fi
