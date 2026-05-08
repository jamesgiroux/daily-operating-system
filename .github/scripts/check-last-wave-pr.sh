#!/usr/bin/env bash
# Detect whether a just-merged PR was the LAST open PR with its wave label.
#
# Used by the L3 workflow's auto-trigger gate. We must avoid races: between
# the merge event firing and this check running, another wave-WN PR could
# have been opened or closed. We accept that race as low-impact: at worst,
# L3 fires once when the user thought the wave wasn't done — operator can
# cancel; or L3 doesn't fire when the user thinks it should — operator can
# manual-dispatch.
#
# Usage: .github/scripts/check-last-wave-pr.sh --wave WN [--exclude-pr N] [--repo owner/repo]
#
# Stdout: "true" if no other open PRs carry this wave label, else "false"
# Exit: always 0 (caller branches on stdout)

set -euo pipefail

WAVE=""
EXCLUDE=""
REPO="${GH_REPO:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wave) WAVE="$2"; shift 2 ;;
    --exclude-pr) EXCLUDE="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$WAVE" ]] || { echo "--wave required" >&2; exit 2; }

REPO_ARG=""
[[ -n "$REPO" ]] && REPO_ARG="--repo $REPO"

open_prs=$(gh pr list $REPO_ARG --state open --label "wave-${WAVE}" --json number --jq '[.[].number]')

if [[ -n "$EXCLUDE" ]]; then
  remaining=$(printf '%s' "$open_prs" | python3 -c "import sys,json; nums=json.load(sys.stdin); ex=int('$EXCLUDE'); print(json.dumps([n for n in nums if n != ex]))")
else
  remaining="$open_prs"
fi

count=$(printf '%s' "$remaining" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))")

echo "wave=${WAVE} open-prs-remaining=${count} (excluding PR ${EXCLUDE:-none})" >&2

if [[ "$count" -eq 0 ]]; then
  printf 'true\n'
else
  printf 'false\n'
fi
