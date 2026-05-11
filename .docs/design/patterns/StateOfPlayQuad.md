# StateOfPlayQuad

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `StateOfPlayQuad`
**`data-ds-spec`:** `patterns/StateOfPlayQuad.md`
**Variants:** per-cell `data-empty="true | false"` for empty-state styling
**Design system version introduced:** 0.6.0

## Job

Render a 4-quadrant narrative status grid where each quadrant is a single short prose paragraph with a mono uppercase label. The canonical four labels for project/account state are: **What's working / What's not / What's unclear / What we need.**

This is an editorial assessment shape — narrative prose that gives a stand-up-quality read of where things are. Distinct from:
- `StateOfPlay` — sibling pattern; a binary working/struggling state LIST (multiple bullet items per side). StateOfPlayQuad is exactly 4 cells, each holding a single paragraph
- `FindingsTriad` — 3 fixed buckets (wins / risks / decisions), spec-locked at 3 cells per its own contract
- `MarginGrid` — sidebar + content layout, not equal quadrants

## When to use it

- On Project Detail's "Where it stands today" chapter
- On Account Detail's stand-up assessment section when the account warrants the same read
- In report surfaces when a 4-up assessment anchors the narrative
- Whenever a 4-quadrant reading provides more shape than a 3-bucket FindingsTriad and the cells are narrative prose, not lists

## When NOT to use it

- When 3 buckets fit the content (wins / risks / decisions) — use `FindingsTriad`
- When the cells are bulleted lists rather than paragraphs — use `StateOfPlay`
- When you need a sidebar + content layout — use `MarginGrid`
- When the assessment is a single paragraph — use a `Callout` or chapter body prose directly

## States / variants

- **default** — full prose body, primary text color
- **empty** — `data-empty="true"` styles the body as italic tertiary text for "no signal yet" or "first sync pending" states

The four cell labels are a pattern convention, not an enforced contract. Override when the surface needs different prompts (e.g. "What changed / What didn't / What's next / What's at risk" for a meeting recap variant). Keep to exactly 4 cells.

## Composition

This pattern does not compose other primitives. Each cell is a label + paragraph. Optionally pairs with a `ChapterHeading` + `FreshnessLine` shell above it.

## Tokens consumed

- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`
- `--font-mono` (label), `--font-serif` (body)
- `--space-sm`, `--space-lg`, `--space-2xl`

## API sketch

```html
<div class="state-of-play-quad"
     data-ds-name="StateOfPlayQuad"
     data-ds-spec="patterns/StateOfPlayQuad.md">
  <div class="state-of-play-quad-cell">
    <div class="state-of-play-quad-label">What's working</div>
    <p class="state-of-play-quad-body">
      The partner integration is two weeks ahead. Their
      platform team approved our auth model on first review and the beta
      cohort is running 18 active sessions a day.
    </p>
  </div>
  <div class="state-of-play-quad-cell">
    <div class="state-of-play-quad-label">What's not</div>
    <p class="state-of-play-quad-body">
      The second partner's residency requirement was unspecified at kickoff and is now a
      hard gate for their pilot. We owe them a remediation plan by Friday.
    </p>
  </div>
  <div class="state-of-play-quad-cell">
    <div class="state-of-play-quad-label">What's unclear</div>
    <p class="state-of-play-quad-body">
      GTM positioning. Sales wants to lead with the integration partner; product
      marketing wants to lead with "agent of action."
    </p>
  </div>
  <div class="state-of-play-quad-cell">
    <div class="state-of-play-quad-label">What we need</div>
    <p class="state-of-play-quad-body">
      Legal to clear the residency response by Apr 26. PMM to commit to a
      positioning track by May 2.
    </p>
  </div>
</div>
```

React form:

```tsx
<StateOfPlayQuad
  cells={[
    { label: "What's working", body: "The partner integration is two weeks ahead…" },
    { label: "What's not",     body: "The second partner's residency requirement was unspecified at kickoff…" },
    { label: "What's unclear", body: "GTM positioning. Sales wants to lead with the integration partner…" },
    { label: "What we need",   body: "Legal to clear the residency response by Apr 26…" },
  ]}
/>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/StateOfPlayQuad.module.css`
- **Code:** to be shipped at `src/components/entity/StateOfPlayQuad.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.vD-stand-grid`)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2 — "Where it stands today" chapter)
- AccountDetail (potential consumer)
- Reports (Risk Briefing, Account Health) when a 4-up assessment anchors the chapter

## Naming notes

`StateOfPlayQuad` — sibling to existing `StateOfPlay` (binary working/struggling state list). The `Quad` suffix mirrors the cell-count vocabulary (FindingsTriad = 3, StateOfPlayQuad = 4). Not `FindingsQuad` because FindingsTriad's spec at line 86 explicitly forbids extension to a 4-up variant ("the three buckets are part of the pattern contract"). Not `WhereItStandsGrid` because that name couples the spec to one surface vocabulary; the pattern is reusable for any 4-up narrative assessment.

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. Chrome-overlap audit attempted to map this to `FindingsTriad + 4th cell composition` (rejected: spec contract forbids); to `StateBlock`/`StateOfPlay` (rejected: those are bulleted lists not narrative quadrants). StateOfPlayQuad is the family-consistent new pattern.
