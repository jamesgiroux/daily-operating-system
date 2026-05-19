#!/usr/bin/env bash
# .claude/hooks/k-out-reminder.sh — Stop hook (Tier 2 of K-out automation).
#
# Scans the conversation transcript for trigger phrases that suggest a K-out
# capture candidate. When matched, prints a one-line nudge so the next turn
# can decide whether to run /ce-compound and write to docs/solutions/.
#
# Tier 1 (autonomous) is Claude running /ce-compound as part of L3 retro work.
# Tier 3 (blocking) is the K-out checklist in `.docs/plans/v1.4.0-waves.md` §
#   "Proof-bundle template" → "K-out captures" — retro doesn't close until
#   the section is filled in.
# This hook is the safety net: it nudges when triggers fire OUTSIDE a retro
# context, where the autonomous + checklist layers don't apply.
#
# Hook contract (Claude Code Stop hook):
#   - Receives JSON on stdin with `transcript_path` (path to the conversation
#     transcript .jsonl file).
#   - exit 0 = no action; emit nothing.
#   - exit 0 with stdout = additional context surfaced to the user.
#   - exit 2 = block stop (NOT what we want — we never block).
#
# See https://docs.claude.com/claude/claude-code/hooks for the schema.

set -euo pipefail

# Read stdin payload. The transcript path is what we care about.
PAYLOAD="$(cat 2>/dev/null || echo '{}')"

# Extract transcript_path. If jq is available use it; otherwise grep+sed.
TRANSCRIPT_PATH=""
if command -v jq >/dev/null 2>&1; then
  TRANSCRIPT_PATH="$(echo "$PAYLOAD" | jq -r '.transcript_path // empty' 2>/dev/null || true)"
else
  TRANSCRIPT_PATH="$(echo "$PAYLOAD" | grep -o '"transcript_path"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"transcript_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || true)"
fi

# If we can't resolve the transcript, silently exit — never block.
if [ -z "$TRANSCRIPT_PATH" ] || [ ! -r "$TRANSCRIPT_PATH" ]; then
  exit 0
fi

# Only scan the last assistant turn — older turns may have stale triggers
# the orchestrator already acted on. Read the last ~3000 chars of the file
# to bound work; transcript is JSONL, last entry is the most recent.
LAST_TURN="$(tail -c 8000 "$TRANSCRIPT_PATH" 2>/dev/null || true)"

if [ -z "$LAST_TURN" ]; then
  exit 0
fi

# Trigger phrases. Lowercased; matched case-insensitively. Each phrase is a
# concrete signal that a K-out candidate is in play. Keep this list narrow —
# false positives erode the nudge's credibility.
#
# Update with care: noise here trains the operator to ignore the nudge.
TRIGGERS=(
  'class-pattern finding'
  'same-shape twice'
  'same-shape finding'
  'substrate already existed'
  'substrate-already-existed'
  'reinvented documented substrate'
  'L3 retro complete'
  'l3 retro complete'
  'wave retro close'
  'documented substrate'
)

MATCHED=""
for trigger in "${TRIGGERS[@]}"; do
  if echo "$LAST_TURN" | grep -qiF "$trigger"; then
    MATCHED="$trigger"
    break
  fi
done

if [ -z "$MATCHED" ]; then
  exit 0
fi

# Don't fire if the same turn already mentions /ce-compound — assume the
# autonomous tier already handled it.
if echo "$LAST_TURN" | grep -qiE '/ce-compound|ce_compound|docs/solutions/'; then
  exit 0
fi

# Emit the nudge. stdout from a Stop hook is surfaced as additional context.
cat <<EOF
→ K-out candidate detected (trigger: "$MATCHED").
  Run /ce-compound (or /ce-compound mode:headless) to capture this finding to docs/solutions/.
  See .docs/plans/engineering-ladder.md § K-out for the obligation.
EOF

exit 0
