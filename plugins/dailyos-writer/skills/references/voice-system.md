# The Voice System — How the Layers Compose and Learn

Voice is a stack of layers plus a feedback loop, not a single profile. This is what keeps AI-isms — "this meeting is the gate," setup-colons, oracle reveals — out of the output. Read this when drafting or running the Voice/Authenticity passes.

## The layers (load in this order)

```
┌─────────────────────────────────────────────────────────────┐
│  3. voices/LEARNINGS.md   — corrections logged in this        │
│     project. WINS ALL TIES. Read every run.                   │
├─────────────────────────────────────────────────────────────┤
│  2. voices/author.yaml    — OPTIONAL personal author LAYER.   │
│     If present, sits on top of every content type. Narrator-  │
│     not-aphorist, register dial. Its hard_rules override the   │
│     content-type profile. The plugin ships only a template     │
│     (author.example.yaml); a project copies + fills it in.    │
├─────────────────────────────────────────────────────────────┤
│  1. voices/<type>.yaml     — the content-type profile.         │
│     Sets structure and register (strategic / thought-         │
│     leadership / customer / status-report / narrative).        │
└─────────────────────────────────────────────────────────────┘
```

**Why an author layer and not just per-type profiles:** a writer sounds like themselves whether they're writing a blog post or an executive briefing. The thing that lets "this meeting is the gate" slip into an exec briefing is a generic-professional content profile with no personal voice on top of it. The optional author layer fixes that everywhere at once. The content type changes the *register* (how formal), never the *identity* (whose voice).

**The register dial** lives in `author.yaml`. Same narrator, different clothes: `personal` (most personality), `professional` (tighter), `executive` (calm, specific, concrete-over-clever — the register that must never slide back into corporate-aphorism mode).

## The detection layer (separate from voice)

Two reference files, read by the review passes, not "voice profiles":
- `shared/AI-TELLS.md` — the current-generation machine-tell taxonomy (aphorism vs. narrator, the "X is the gate" class, plus classes adapted from the [Wikipedia "Signs of AI writing"](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing) catalog). Primary reference for the Authenticity pass.
- `shared/ANTI-PATTERNS.md` — older line-level patterns (contrast framing, throat-clearing, evidence anti-patterns).

`scripts/detect_patterns.py` enforces the regex-detectable subset of both.

## The learning loop

The writer gets more author-shaped every time the author corrects it.

```
   Author corrects a draft's voice
              │
              ▼
   ┌──────────────────────┐     record
   │  /writer:learn  OR    │ ───────────────►  new entry in
   │  end-of-session       │                   voices/LEARNINGS.md
   │  Phase 8 capture      │                   (before / after / rule / scope)
   └──────────────────────┘
              │
              │  recurs (same shape twice)
              ▼
   ┌──────────────────────┐     graduation
   │  promote the rule     │ ───────────────►  voices/author.yaml     (author-voice rule)
   │                       │                   shared/AI-TELLS.md +    (mechanical tell)
   │                       │                   detect_patterns.py
   └──────────────────────┘
              │
              ▼
   every future run reads LEARNINGS.md first  ◄──── applied on next draft
```

- **Apply (every run):** the Voice and Authenticity passes load `voices/LEARNINGS.md` and apply every `active` rule. A correction made yesterday governs today's draft without anyone touching the profiles.
- **Record** — `/writer:learn` or Phase 8: when the author changes the voice of a draft, isolate the *voice* corrections (ignore content/fact edits) and write a `LEARNINGS.md` entry: `scope`, `before`, `after`, the generalizable `rule`, `status: active`. Keep it example-driven.
- **Graduation (same shape twice = a class):** when a rule recurs, promote it into `voices/author.yaml` (author-voice rule) or `shared/AI-TELLS.md` + `scripts/detect_patterns.py` (mechanical tell), and mark the entry `graduated`. Graduated entries stay for history.

`LEARNINGS.md` ships empty — its content is per-project and accrues as the skill is used.
