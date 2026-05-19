# Doc Authoring — Tier Ruleset

**Status:** Canonical
**Adopted:** 2026-05-18
**See also:** `.docs/plans/engineering-ladder.md`, `.docs/design/product/build-foundation-html.mjs` (precedent for MD-source → HTML-render pattern)

DailyOS planning docs sit in three tiers. Pick the tier by the **criteria below**, not by doc-type lists (lists drift; criteria stay).

The reason this matters: planning docs are read more than they're edited, and the rendered surface affects whether they get read at all. HTML-first when layout carries information; markdown + render when it's structured prose; markdown-only when the doc is code-adjacent.

---

## Tier 1 — HTML-first

**Author directly as `.html` against the shared design system.**

Apply when **any** of these are true:

- **Layout carries information that prose can't.** Status grids, timelines, reviewer matrices, gate boards, comparison cards, ladder visualizations, wave dashboards, side-by-side architecture overviews.
- **Intended for sharing visually with people who won't edit it.** Strategy decks, PRDs you'd send to a stakeholder, vision pieces, release-narrative artifacts.
- **Read-heavy reference doc.** Read often, edited rarely. The Engineering Ladder, the v1.4.x roadmap, the wave-W4 status board.

**Examples (illustrative, not exhaustive):**
- `.docs/plans/engineering-ladder.html` — Plan/Implement/Review/Capture matrix, K-channel diagram, rung cards
- Wave dashboards (gate state across W0–W6)
- PRDs with feature comparison grids
- Strategy decks
- Version roadmaps with milestone timelines
- Architecture overviews with embedded diagrams

**Authoring substrate:**
- Imports from `.docs/design/reference/_shared/styles/` (design tokens, fonts, MagazinePageLayout)
- Local `.docs/plans/plans.css` for plan-specific patterns
- Snippet patterns at `.docs/plans/_patterns/` — copy + adapt
- Starter templates at `.docs/plans/_templates/` — `pnpm new:plan <type> <slug>` (when wired up) clones the scaffold

**Dogfooding convention:** any inline style or one-off pattern in an HTML-first doc lands with a `<!-- TODO(ds): needs pattern -->` comment naming the gap. End of each wave, the gaps roll into design-system work items. Every HTML-first doc either consumes a DS pattern or surfaces a need.

---

## Tier 2 — Markdown + render

**Author as `.md`, render to `.html` via a build script. Commit both.**

Apply when:

- **Text-dominant prose with section structure.** Long-form essays where the value is the writing, not the layout.
- **Read often, edited occasionally.** Foundational docs that evolve slowly.
- **Content is words, not diagrams.**

**Examples:**
- `.docs/design/product/{MISSION,VISION,PHILOSOPHY,PRINCIPLES,PRODUCT-THESIS}.md` (current precedent; rendered via `build-foundation-html.mjs`)
- Policy explainers
- Longform strategy memos
- Narrative status updates intended for external sharing

**Authoring substrate:**
- Markdown source as the canonical edit surface (passes through Linear sync, codex review, grep)
- Build script per surface tier — e.g., `build-foundation-html.mjs` for product foundation. New surfaces get a new build script.
- HTML output goes alongside the source: `MISSION.md` → `mission.html` in the same directory

---

## Tier 3 — Markdown-only

**Author as `.md`. No HTML render.**

Apply when:

- **Code-adjacent.** Mixed with code blocks, file refs, line numbers, paths.
- **Diff-reviewed.** PR-shape changes where `git diff` legibility matters more than visual layout.
- **Linear-canonical.** Per-ticket plans where Linear's markdown view is the authoritative surface.
- **Grep-first.** Knowledge captures meant to be retrieved by keyword.

**Examples:**
- Wave retros (`.docs/plans/wave-WN/retro.md`)
- Proof bundles
- `docs/solutions/` K-out captures
- ADRs (`.docs/decisions/ADR-NNNN.md`)
- Per-ticket Linear plans for orchestration-driven work
- MEMORY.md and topic-file memories under `/Users/jamesgiroux/.claude/projects/.../memory/`
- Commit messages and PR bodies

---

## How to pick the tier (decision flow)

```
Is the doc code-adjacent, diff-reviewed, or grep-first?
├── YES → Tier 3 (markdown-only)
└── NO  → continue

Does the doc need layout primitives prose can't express?
(status grids, timelines, matrices, dashboards, diagrams)
├── YES → Tier 1 (HTML-first)
└── NO  → continue

Is the doc intended for visual sharing OR read-heavy reference?
├── YES → Tier 1 (HTML-first)
└── NO  → Tier 2 (markdown + render)
```

If unsure between Tier 1 and Tier 2, ask: *would I sketch this doc on a whiteboard before writing it?* If yes, Tier 1. If no, Tier 2.

---

## Authoring fast in Tier 1

Speed in HTML-first comes from scaffolds, not from compromising the tier:

- **`.docs/plans/_templates/<type>.html`** — starter HTML per doc type (`wave-plan.html`, `prd.html`, `ladder-viz.html`, `strategy-deck.html`), pre-wired to shared design-system imports. Authoring = fill in the slots.
- **`.docs/plans/_patterns/<name>.html`** — snippet library: `gate-badge.html`, `reviewer-matrix.html`, `status-pill.html`, `ladder-rung-card.html`, `k-channel-diagram.html`, `cycle-count-chip.html`, `timeline-strip.html`, etc. Copy + adapt.
- **`.docs/plans/_patterns/README.md`** — index of patterns with one-line descriptions of when to use each.

Don't write HTML by hand from scratch — clone a template, replace tokens, compose patterns.

---

## What stays markdown regardless

Tier 3 is non-negotiable for the following surfaces because they have tooling reasons that override visual considerations:

- **Linear-canonical plans** — Linear is canonical per CLAUDE.md; converting to HTML duplicates the surface and creates sync risk.
- **Retros + proof bundles** — wave-end review artifacts, consumed by L3 reviewers (codex challenge + architect) which expect markdown.
- **`docs/solutions/` K-out captures** — grep-retrieved by L0/L1/L2 reviewers per the Engineering Ladder K-in obligation. HTML breaks grep.
- **ADRs** — industry-wide markdown convention; tooling (`adr-tools`, etc.) assumes it.
- **MEMORY.md + topic-file memories** — Claude's auto-memory reads markdown.

---

## What this doesn't change

- Markdown remains the authoring surface for code-adjacent, diff-reviewed, and Linear-canonical docs.
- The Engineering Ladder's L0–L6 protocol, K-in / K-out obligations, and reviewer panels are unchanged.
- `feedback_set_and_forget_wave_protocol`, `feedback_check_substrate_before_authoring_primitives`, and other operational memories continue to apply.
- The `docs/solutions/` knowledge store stays markdown-only — it's grep-first reference, not consumption surface.

---

## Surfacing patterns back to the DS

When authoring an HTML-first doc surfaces a gap (inline style, ad-hoc layout, missing token):

1. Annotate with `<!-- TODO(ds): needs pattern — <one-line description> -->` in the source.
2. At wave close, sweep `grep -r 'TODO(ds):' .docs/plans/` and roll the gaps into design-system Linear issues.
3. When the DS pattern lands, replace the inline use with the canonical class.

This is the **dogfooding loop** — plan-doc authoring becomes a continuous feedback channel into the design system. Every plan that surfaces a gap either gets a new DS pattern or surfaces an over-abstraction (the gap is one-off, doesn't deserve a pattern). Both outcomes are signal.
