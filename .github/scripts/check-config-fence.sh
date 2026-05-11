#!/usr/bin/env bash
# check-config-fence.sh — block PRs that modify the L2 gate's own configuration
# from being reviewed by the modified config. Such PRs must go through a
# separate maintainer-approval flow.
#
# Reads the changed-files list from stdin. Exits 0 if the PR doesn't touch any
# fenced path; exits 2 (fail-closed) if it does, with a clear error message.
#
# Fenced paths (these define the gate; if a PR modifies them, the modified
# version cannot be used to review the same PR):
#   - .github/workflows/l2-review.yml
#   - .github/reviewer-prompts/**
#   - .github/scripts/**
#   - .claude/git-hooks/**

set -euo pipefail

FENCED_PATTERNS=(
  ".github/workflows/l2-review.yml"
  ".github/scripts/configure-branch-protection.sh"
)

FENCED_PREFIXES=(
  ".github/reviewer-prompts/"
  ".github/scripts/"
  ".github/actions/"
  ".claude/git-hooks/"
)

hits=()

while IFS= read -r path; do
  [ -z "$path" ] && continue

  # Exact-pattern matches
  for pat in "${FENCED_PATTERNS[@]}"; do
    if [ "$path" = "$pat" ]; then
      hits+=("$path")
      continue 2
    fi
  done

  # Prefix matches
  for pre in "${FENCED_PREFIXES[@]}"; do
    case "$path" in
      "${pre}"*) hits+=("$path"); continue 2 ;;
    esac
  done
done

if [ "${#hits[@]}" -eq 0 ]; then
  exit 0
fi

cat >&2 <<EOF
🔒 config-fence: this PR modifies one or more L2 gate configuration files.

The L2 gate cannot review a PR that modifies its own configuration — the modified
version would be reviewing itself, defeating the trust boundary.

Fenced paths touched in this PR:
$(printf '   - %s\n' "${hits[@]}")

Resolution:
  1. Land the gate-configuration change as its own PR with maintainer co-sign
     and admin override (this PR's only path through to merge today).
  2. Once merged on dev, the new configuration becomes the base for subsequent
     reviews. Open the original work as a fresh PR.

For the long-term fix (a path that handles config + non-config changes safely
in a single PR), see Phase 2.1 in v1-lite §4.
EOF

exit 2
