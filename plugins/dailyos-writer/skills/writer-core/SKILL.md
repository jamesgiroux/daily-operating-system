---
name: writer-core
description: Multi-phase editorial workflow orchestrator with specialized voices, internal review cycles, and challenger gates, plus a voice system that strips AI-isms. Creates thought leadership, strategic documents, status reports, and customer/executive communications with writer's room quality control. Activates when the user initiates a writing project, requests content creation, or wants writing that sounds human rather than AI-generated.
---

# Writer Core - Editorial Workflow Orchestrator

An orchestrated writing workflow with specialized voices, internal review cycles, and challenger gates. Work goes through "writer's room" review before reaching the human. The job is not competent prose — it is prose that sounds like a specific person thinking, that a reader actively suspicious of AI can't dismiss.

## Activation

This skill activates when the user initiates a writing project, requests content creation, or invokes the write command. It orchestrates the full editorial pipeline from discovery through polish, then capture.

---

## Core principle: writer's room quality control

Each phase has **internal review cycles before the human sees output**. The human is not the first reviewer; they are the decision-maker receiving already-debated, refined work. The reviewers exist to catch AI-isms and voice drift before they reach the human, because every one that lands costs credibility and time.

## The pipeline

| Phase | What happens | Gate |
|------|--------------|------|
| 0 · Discovery | Scan for existing drafts; don't duplicate work | — |
| 1 · Ideation | Content type, voice profiles, thesis, brief; challenger tests premise | **Human: approve direction** |
| 2 · Research | Workspace-first (DailyOS), then internal + web; flag gaps | **Human: fill gaps** |
| 3 · Structure | Template + outline; challenger + structural review | **Human: approve outline** |
| 4 · Drafting | Write to outline; apply voice profile (+ author layer) | — |
| 5 · Review | Six passes: mechanical, structural, voice, authenticity, scrutiny, challenger | **Human: review flagged items** |
| 6 · Revision | Incorporate feedback; re-run affected passes | loop |
| 7 · Polish | Final mechanics, format, metadata, assets | **Human: confirm ready** |
| 8 · Capture | Log any voice corrections into the learning loop | — |

**Full phase detail — process, outputs, exit criteria — is in [`skills/references/phases.md`](../references/phases.md). Read it when running a phase.**

**DailyOS workspace-first research**: when operating in a DailyOS workspace, Phase 2 reads entity intelligence, meeting archives, and stakeholder quotes BEFORE any web search, so content is grounded in real organizational context.

---

## The voice system (the part that matters most)

Voice is a **stack of layers plus a feedback loop**, not a single profile. This is what keeps AI-isms — "this meeting is the gate," setup-colons, oracle reveals — out of the output. **Full explanation in [`skills/references/voice-system.md`](../references/voice-system.md); read it before drafting or running the Voice/Authenticity passes.**

Load order (later wins ties):
1. **`skills/voices/<type>.yaml`** — content-type profile. Sets structure and register.
2. **`skills/voices/author.yaml`** — *optional* personal author layer, applied on top of *every* content type (including exec/customer). The plugin ships only a template (`author.example.yaml`); a project copies it to `author.yaml` and fills it in. When present, its `hard_rules` override the content-type profile.
3. **`skills/voices/LEARNINGS.md`** — voice corrections logged in this project. **Wins all ties. Read every run.** Ships empty.

The detection references the review passes use:
- **`skills/shared/AI-TELLS.md`** — current-generation AI-tell taxonomy (aphorism mode vs. narrator mode; the "X is the gate" class; classes adapted from the [Wikipedia "Signs of AI writing"](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing) catalog). Primary reference for the Authenticity pass.
- **`skills/shared/ANTI-PATTERNS.md`** — older line-level patterns still in force.

**The one idea behind all of it:** AI writes in *aphorism mode* — crisp, confident declaratives that sound like insight and belong to no one. The target is *narrator mode* — a specific person thinking out loud. When a sentence would look good on a slide, rewrite it as something a person would say to a colleague.

### Content type → voice profile

| Content type | Content-type profile | Default template |
|--------------|---------------------|------------------|
| Thought leadership | `voices/thought-leadership.yaml` | hook-problem-reframe |
| Strategic update | `voices/strategic.yaml` | bluf-standard |
| Status report | `voices/status-report.yaml` | weekly-impact / monthly-rollup |
| Vision document | `voices/strategic.yaml` | strategy-memo |
| Video script | `voices/narrative.yaml` | documentary-arc |
| Customer / exec communication | `voices/customer.yaml` | qbr-narrative / executive-briefing |

If the content type is unclear from the prompt, ask which it is before loading profiles.

---

## The review passes (Phase 5)

Six passes run before the human sees the draft. **Detail and exit criteria in [`skills/references/phases.md`](../references/phases.md).**

| Pass | Skill / tool | Catches |
|------|--------------|---------|
| A · Mechanical | `scripts/lint_typography.py` + `detect_patterns.py` | em-dashes, terminology, abstract-noun equations, setup-colons, inflated diction, copula avoidance, vague attribution |
| B · Structural | structural-review | logic, flow, evidence, transitions |
| C · Voice | voice-review | content-type fidelity + author layer + register |
| D · Authenticity | authenticity-review | AI-tells via `AI-TELLS.md`; **rewrites** flagged lines into narrator mode |
| E · Scrutiny | scrutiny | exec-facing only: vague claims, missing timelines/metrics/owners |
| F · Challenger | challenger | did we deliver on the promise? genuine insight? |

---

## The learning loop

The writer gets more author-shaped every time the author corrects it. **Full mechanics in [`skills/references/voice-system.md`](../references/voice-system.md).**

- **Apply (every run):** Voice + Authenticity passes read `voices/LEARNINGS.md` and apply every `active` rule. A correction made yesterday governs today's draft.
- **Record:** `/learn` (the learn command) or Phase 8 extracts a voice correction into a new `LEARNINGS.md` entry (before / after / rule / scope).
- **Graduation:** when a rule recurs (same shape twice), promote it into `voices/author.yaml` (author-voice rule) or `shared/AI-TELLS.md` + `detect_patterns.py` (mechanical tell).

---

## Content type detection

If unclear from the user's prompt, ask:

```
What type of content are we creating?

- Thought Leadership (HBR-style articles for practitioners)
- Strategic Update (Partnership, competitive, executive summary)
- Status Report (Weekly, monthly, quarterly)
- Vision Document (Strategy, planning, roadmap)
- Video Script (Documentary, thought leadership video)
- Podcast Outline (Interview, discussion)
- Customer Communication (QBR, renewal, executive briefing)
- Other (describe it)
```

## Input sources

| Source | How to invoke | What happens |
|--------|---------------|--------------|
| Topic from calendar | "Week 5 from Leadership Content" | Load topic context, thread, related articles |
| Document/transcript | "I have a transcript…" | Extract key points, quotes, evidence |
| Prompt/idea | "I want to write about…" | Ideation from scratch |
| Existing outline | "Here's my outline…" | Skip to drafting |
| Content brief | "Here's the brief…" | Skip ideation, go to research |

---

## Specialized skills

| Skill | Purpose |
|-------|---------|
| research | Evidence gathering from workspace data, internal docs, and web |
| challenger | Premise testing, "so what" assessment, value gate |
| scrutiny | Executive specificity: vague claims, timelines, metrics |
| mechanical-review | Pattern detection, typography, linting scripts |
| structural-review | Logic, flow, coherence |
| voice-review | Voice fidelity: content-type profile + author layer + learnings |
| authenticity-review | AI-tell detection and narrator-mode rewrites |

---

## Resource index

| Location | What's there | Read when |
|----------|--------------|-----------|
| `skills/references/phases.md` | Full phase + review-pass detail | running any phase |
| `skills/references/voice-system.md` | How the voice layers compose + the learning loop | drafting or reviewing voice |
| `skills/references/frameworks.md` | Borrowed-vs-invented framework guidance | tempted to coin a framework |
| `skills/voices/author.example.yaml` | Template for the optional personal author layer | setting up a project's voice |
| `skills/voices/LEARNINGS.md` | Logged voice corrections (win ties) | every run |
| `skills/voices/<type>.yaml` | Content-type profiles | drafting that type |
| `skills/shared/AI-TELLS.md` | Current-gen AI-tell taxonomy | Authenticity pass |
| `skills/shared/ANTI-PATTERNS.md` | Older line-level patterns | Mechanical/Voice passes |
| `skills/shared/MECHANICS.md`, `skills/shared/TERMINOLOGY.md` | Grammar, typography, terminology | mechanical pass |
| `skills/shared/DISTRIBUTION.md` | Format adaptation, repurposing, sanitization | Polish / cross-channel |
| `skills/templates/<type>/` | Per-content-type templates | Structure phase |
| `skills/scripts/` | `lint_typography.py`, `detect_patterns.py` | Mechanical pass |
