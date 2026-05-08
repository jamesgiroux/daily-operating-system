#!/usr/bin/env bash
# Resolve an L3 review range from PR numbers.
#
# Given a comma-separated list of merged PR numbers, find the OLDEST one's
# merge-commit parent and use that as the integration "before" SHA. The
# integration "after" SHA is current dev HEAD.
#
# Usage: .github/scripts/resolve-l3-range.sh --prs N,M,K [--repo owner/repo]
#
# Stdout (single line): <first-parent-sha>..<dev-head-sha>
# Stderr: human-readable explanation.

set -euo pipefail

PRS=""
REPO="${GH_REPO:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --prs) PRS="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$PRS" ]] || { echo "--prs required (comma-separated PR numbers)" >&2; exit 2; }

REPO_ARG=""
[[ -n "$REPO" ]] && REPO_ARG="--repo $REPO"

oldest_sha=""
oldest_ts=""
IFS=',' read -ra PR_LIST <<< "$PRS"
for pr in "${PR_LIST[@]}"; do
  pr=$(echo "$pr" | tr -d ' ')
  [[ -z "$pr" ]] && continue
  data=$(gh pr view $REPO_ARG "$pr" --json mergeCommit,mergedAt 2>/dev/null)
  sha=$(printf '%s' "$data" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('mergeCommit',{}).get('oid','') or '')")
  ts=$(printf '%s' "$data" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('mergedAt') or '')")
  if [[ -z "$sha" || -z "$ts" ]]; then
    echo "PR #$pr is not merged (no mergeCommit or mergedAt). Aborting." >&2
    exit 3
  fi
  if [[ -z "$oldest_ts" || "$ts" < "$oldest_ts" ]]; then
    oldest_ts="$ts"; oldest_sha="$sha"
  fi
done

[[ -n "$oldest_sha" ]] || { echo "no merged PRs resolved from input" >&2; exit 4; }

first_parent=$(git rev-parse "${oldest_sha}^1" 2>/dev/null) || {
  echo "Failed to resolve parent of $oldest_sha; ensure git history is fetched (fetch-depth: 0)" >&2
  exit 5
}
dev_head=$(git rev-parse HEAD)

echo "prs=$PRS oldest-merge=$oldest_sha first-parent=$first_parent dev-head=$dev_head" >&2
printf '%s..%s\n' "$first_parent" "$dev_head"
