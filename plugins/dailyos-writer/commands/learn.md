---
description: Capture a voice correction into the writer's learning log
---

# /learn - Capture a Voice Correction

Record a voice correction the author just made so future drafts don't repeat it. This is the writer's learning loop: corrections captured here are read on every future run and applied to the next draft. See `skills/references/voice-system.md`.

$ARGUMENTS: Optionally, the correction to capture (e.g. a before/after, or "remember: no setup-colons"). If omitted, infer the correction from the most recent edits the author made to the current draft.

## Workflow

### Step 1: Identify the correction

- If `$ARGUMENTS` describes a correction, use it.
- Otherwise, diff what the author changed against what the skill produced in this session and isolate the **voice** corrections — how something was said, not what facts were changed. Ignore content/fact edits.

If you cannot find a clear voice correction, ask the author what they'd like the writer to learn.

### Step 2: Generalize it

Turn the specific edit into a reusable rule. Capture:
- **scope**: which content types it applies to (`all`, or a specific type)
- **before**: the AI-ism that was cut (verbatim)
- **after**: what replaced it (or "cut entirely")
- **rule**: the generalizable instruction
- **status**: `active`

Keep it example-driven — a future run should be able to apply the rule from the before/after alone. Do not inflate the correction into something grander than it was.

### Step 3: Write the entry

Append to `skills/voices/LEARNINGS.md` under "Active learnings", using the entry format documented at the top of that file:

```
### YYYY-MM-DD — short title
- scope: ...
- context: ...
- before: "..."
- after: "..."
- rule: ...
- status: active
```

### Step 4: Graduate if it recurs

If this same shape of correction is already in `LEARNINGS.md` (same shape twice = a class), promote it:
- Author-voice rule → move into `skills/voices/author.yaml` `hard_rules`, mark the LEARNINGS entry `graduated -> voices/author.yaml`.
- Mechanical/detectable tell → add to `skills/shared/AI-TELLS.md` (and `skills/scripts/detect_patterns.py` if regex-able), mark `graduated -> AI-TELLS.md`.

### Step 5: Confirm

Report what was captured (and whether it graduated), and confirm it will apply to the next draft.
