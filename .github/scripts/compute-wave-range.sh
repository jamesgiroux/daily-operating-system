#!/usr/bin/env bash
# Compute the integrated git diff range for a wave.
#
# Strategy: use the wave's PR set (from `gh pr list --label wave-WN`),
# find the OLDEST merged PR's parent commit, and use that as the wave's
# "first parent". The wave's tip is current dev HEAD.
#
# Usage: .github/scripts/compute-wave-range.sh --wave WN [--repo owner/repo]
#
# Output (single line): <first-parent-sha>..<dev-head-sha>
# Stderr: human-readable explanation of how the range was derived.

set -euo pipefail

WAVE=""
REPO="${GH_REPO:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wave) WAVE="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$WAVE" ]] || { echo "--wave required" >&2; exit 2; }

REPO_ARG=""
[[ -n "$REPO" ]] && REPO_ARG="--repo $REPO"

# Fetch oldest merged PR with the wave label.
oldest=$(gh pr list $REPO_ARG --state merged --label "wave-${WAVE}" --json mergeCommit,mergedAt \
  --jq 'sort_by(.mergedAt) | first // empty')

if [[ -z "$oldest" ]]; then
  echo "No merged PRs found with label wave-${WAVE}" >&2
  exit 3
fi

first_merge_sha=$(printf '%s' "$oldest" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['mergeCommit']['oid'])")

# The parent of the first merge commit is the wave's "before" state.
first_parent=$(git rev-parse "${first_merge_sha}^1" 2>/dev/null) || {
  echo "Failed to resolve parent of $first_merge_sha; ensure git history is fetched" >&2
  exit 4
}

dev_head=$(git rev-parse HEAD)

echo "wave=${WAVE} first-merge=${first_merge_sha} first-parent=${first_parent} dev-head=${dev_head}" >&2
printf '%s..%s\n' "$first_parent" "$dev_head"
