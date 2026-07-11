# Voice Learnings — The Compounding Log

This is the learning channel for the writer. Every time the author corrects the voice of a draft, the correction is captured here as a durable rule so the *next* draft doesn't repeat it. This file is **read on every run** by the Voice pass and the Authenticity pass, and its rules **win ties** against the static profiles.

It is the antidote to static tell-lists: the classes in `shared/AI-TELLS.md` and `shared/ANTI-PATTERNS.md` will always lag what an author actually reacts to. This file closes that gap, per project.

> This file ships **empty** with the plugin. It fills up as you use the skill in a real project. The corrections are project-specific, so nothing here is bundled by default.

## How it works (the loop)

- **Capture (every run):** Voice + Authenticity passes load this file and apply every `active` rule. New corrections take effect on the very next draft.
- **Record:** At the end of a writing session, or on `/writer:learn`, a correction the author made gets extracted into a new entry here — before/after, the extracted rule, and which content types it applies to.
- **Graduation:** When a rule recurs across 2+ pieces (same shape twice = a class), promote it: move it into `voices/author.yaml` (if it's an author-voice rule) or `shared/AI-TELLS.md` + `scripts/detect_patterns.py` (if it's a mechanical tell), and mark the entry here `graduated`.

## Entry format

```
### YYYY-MM-DD — short title
- scope: [all | thought-leadership | strategic | customer | status-report | narrative]
- context: one line on where this came up
- before: "the AI-ism that was cut"
- after: "what replaced it (or 'cut entirely')"
- rule: the generalizable instruction
- status: active | graduated -> <where>
```

---

## Active learnings

_(none yet — entries land here as you correct drafts)_

---

## Graduated (kept for history)

_(none yet)_
