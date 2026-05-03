# ChapterHeading

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `ChapterHeading`
**`data-ds-spec`:** `patterns/ChapterHeading.md`
**Variants:** default (heavy rule + serif title + epigraph); `light` (medium rule, less prominent)
**Design system version introduced:** 0.1.0

## Job

Section opener for editorial surfaces — a heavy horizontal rule, a serif title, and an optional italic epigraph (the magazine-style sub-headline). Establishes visual hierarchy below the page hero.

## When to use it

- Top of any editorial chapter / section that warrants formal heading treatment (Health & Outlook, Context, The Record, Findings, etc.)
- Used widely by AccountDetail, ProjectDetail, PersonDetail, MeetingDetail

## When NOT to use it

- Sub-section headings within a chapter (use simpler heading-without-rule pattern, or eyebrow + title only)
- Briefing's section headings — D-spine uses `.section-heading` (mono uppercase eyebrow + serif summary), reconcile if these consolidate; current Wave 1 spec keeps D-spine's variant separate

## Composition

```
[Heavy horizontal rule, color rule-heavy]
[Serif title — 28px, weight 400]
[Italic serif epigraph — 16px, color text-tertiary, optional]
```

## Variants

- **Default** — heavy rule, full title + optional epigraph
- **Light** — medium rule (less visual weight), shorter spacing, used for sub-divisions

## Tokens consumed

- `--color-rule-heavy` (default rule)
- `--color-rule-light` (sub-rules)
- `--font-serif` (title + epigraph)
- `--color-text-primary` (title), `--color-text-tertiary` (epigraph)
- `--space-lg` (vertical rhythm)

## API sketch

Mockup form:

```html
<header class="chapter" data-ds-name="ChapterHeading" data-ds-spec="patterns/ChapterHeading.md">
  <hr class="chapter-rule" />
  <h2 class="chapter-title">Health & Outlook</h2>
  <p class="chapter-epigraph">What the room is feeling and what the data shows.</p>
</header>
```

Production form:

```tsx
<ChapterHeading title="Health & Outlook" epigraph="What the room is feeling and what the data shows." />
```

## Source

- **Code:** `src/components/editorial/ChapterHeading.tsx` + `ChapterHeading.module.css`
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/primitives.css` (`.chapter`)

## Surfaces that consume it

AccountDetail (Health, Context, Work chapters), ProjectDetail, PersonDetail, MeetingDetail (chapter headers).

## Naming notes

`ChapterHeading` — keeps the editorial / publishing metaphor (chapter > section). D-spine briefing uses a separate `.section-heading` (mono uppercase eyebrow) + `.section-summary` (serif paragraph) treatment for its sections — those are NOT this pattern; they're a briefing-specific composition. Reconciliation is a Wave 1 follow-up.

## History

- 2026-05-02 — Promoted to canonical from existing `src/components/editorial/ChapterHeading.tsx` + mockup `_shared/.chapter`.
- Audit 03 — surfaced reconciliation question with D-spine `.section-heading`.
