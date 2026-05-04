# MarginGrid

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `MarginGrid`
**`data-ds-spec`:** `patterns/MarginGrid.md`
**Variants:** default; `label-count` for "N items" sub-label
**Design system version introduced:** 0.1.0

## Job

Two-column section layout: a thin left margin label (mono uppercase) + a wide right content column. Used to establish the magazine-style "marginalia" rhythm of editorial surfaces.

## When to use it

- Section-level layout on any editorial surface where chapters or sections benefit from a consistent left-margin label
- Heaviest user: D-spine briefing (every section uses this)
- Also used by AccountDetail's `MarginSection` editorial component

## When NOT to use it

- Compact list rows (use list patterns directly)
- Modal content (no margin scale)
- Cards / tiles (use their own internal layout)

## Composition

CSS Grid with two columns:

- Left column: 100px fixed width, mono uppercase label, right-aligned, color tertiary
- Right column: `1fr`, min-width 0, content
- Gap: 32px

Optional `margin-label-count` sub-label inside the label column ("4 meetings", "3 entities", "anything").

## Variants

- **Default** — single label
- **With count** — label + sub-label count line

## Tokens consumed

- `--font-mono` (label)
- `--color-text-tertiary` (label color)
- `--space-md`, `--space-2xl` (gap, vertical rhythm between sections)

## API sketch

Mockup form:

```html
<section class="margin-grid" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">
  <div class="margin-label">
    Today
    <span class="margin-label-count">4 meetings</span>
  </div>
  <div class="margin-content">
    <!-- content -->
  </div>
</section>
```

Production form:

```tsx
<MarginGrid label="Today" count="4 meetings">
  {/* content */}
</MarginGrid>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/primitives.css` (`.margin-section`); also reused by D-spine as `.margin-grid` (slightly different class name, same shape)
- **Code:** existing in `src/components/editorial/MarginSection.tsx` — Wave 1 reconciles the two class names

## Surfaces that consume it

DailyBriefing (every section), AccountDetail (some chapters), ProjectDetail, PersonDetail. The signature layout pattern of editorial surfaces.

## Naming notes

D-spine uses `.margin-grid`; production uses `.margin-section`. Wave 1 picks one canonical name. Recommend `MarginGrid` (matches grid layout terminology). The production CSS class can stay `.margin-section` for backward compat or rename per Wave 1.

## History

- 2026-05-02 — Promoted to canonical from mockup `_shared/.margin-section` + production `MarginSection`.
