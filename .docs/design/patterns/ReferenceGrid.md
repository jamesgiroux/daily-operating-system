# ReferenceGrid

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `ReferenceGrid`
**`data-ds-spec`:** `patterns/ReferenceGrid.md`
**Variants:** `data-columns="2 | 3 | 4"`; row variants `default | gap | accent`; optional feature-adoption sub-list; optional caveat banner
**Design system version introduced:** 0.3.0

## Job

Render dense, structured key-value reference data in a labeled grid — the "Commercial shape," "Technical footprint," "Pricing reference" type sub-cards that pair each label with its current value in a scan-friendly layout. Distinct from prose narrative; this is durable structured fact.

## When to use it

- On Account Detail for Commercial shape, Technical footprint, Pricing reference sub-cards
- On Project Detail for Commercial shape (co-build) sub-card under "What success looks like"
- In reports when an executive needs a structured fact reference alongside prose
- Anywhere paired label + value rows in a multi-column grid is the right shape (NOT a single column list)

## When NOT to use it

- For prose narrative — use chapter body or `StateOfPlayQuad`
- For trust-receipt-style metadata — use `ReceiptCallout`
- For a single key-value row inline — use `GlanceCell`
- For freshness / source attribution metadata — use `FreshnessIndicator` + `ProvenanceTag` inline

## States / variants

- **column count** — `data-columns="2 | 3 | 4"`. Default 2. D's Commercial shape uses 4
- **value variants:**
  - **default** — primary text color
  - **gap** (`.ReferenceGrid_valueGap`) — saffron italic, used when a field is missing data we expected to have
  - **accent** (`.ReferenceGrid_valueAccent`) — turmeric bold, used to draw attention to commercially significant numbers
- **feature-adoption sub-list** — optional 3-column dot-prefixed feature list under the grid (Technical footprint variant only)
- **caveat banner** — optional saffron-bordered "coming in next release" banner

## Composition

Self-contained pattern; does not compose other primitives. Optionally rendered alongside a `FreshnessIndicator` line above the grid (consumer-controlled).

## Tokens consumed

- `--font-mono` (label), `--font-sans` (value, feature item)
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`
- `--color-spice-saffron` (gap value tint, caveat banner)
- `--color-spice-turmeric` (accent value tint, feature trial dot)
- `--color-garden-sage` (feature live dot)
- `--color-rule-light` (row dividers)
- `--space-xs | sm | md | lg | 2xl | 3xl`
- `--radius-sm`, `--radius-editorial-sm`

## API sketch

```html
<div class="ReferenceGrid_section" data-ds-name="ReferenceGrid" data-ds-spec="patterns/ReferenceGrid.md">
  <div class="ReferenceGrid_grid" data-columns="4">
    <div class="ReferenceGrid_row">
      <span class="ReferenceGrid_label">Type</span>
      <span class="ReferenceGrid_value">Customer co-build</span>
    </div>
    <div class="ReferenceGrid_row">
      <span class="ReferenceGrid_label">Committed ARR</span>
      <span class="ReferenceGrid_value ReferenceGrid_valueAccent">$640K signed</span>
    </div>
    <div class="ReferenceGrid_row">
      <span class="ReferenceGrid_label">Coverage model</span>
      <span class="ReferenceGrid_value ReferenceGrid_valueGap">Awaiting account plan</span>
    </div>
    <!-- … more rows … -->
  </div>
</div>
```

React form:

```tsx
<ReferenceGrid
  columns={4}
  rows={[
    { label: 'Type', value: 'Customer co-build' },
    { label: 'Committed ARR', value: '$640K signed', variant: 'accent' },
    { label: 'Coverage model', value: 'Awaiting account plan', variant: 'gap' },
  ]}
/>
```

## Source

- **Code:** consumed by `src/components/context/CommercialShape.tsx` and `AccountTechnicalFootprint.tsx`
- **Reference CSS:** `.docs/design/reference/_shared/styles/ReferenceGrid.module.css`
- **Mockup origin:** `.docs/mockups/account-context-globex.html` (`.ref-grid`); extended for project-detail in `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.ref-grid` 4-col)

## Surfaces that consume it

- AccountDetail (Commercial shape, Technical footprint chapters)
- ProjectDetail (Commercial shape · co-build sub-card under "What success looks like")
- Reports (reference data sections)

## Naming notes

`ReferenceGrid` — "reference" (durable structured fact; the user references it, doesn't read it as prose) + "grid" (the layout primitive). Not `FactGrid` because facts can be incomplete; this pattern includes gap handling for missing data. Not `KeyValueGrid` because it's intentionally narrower: editorial-tone reference data, not arbitrary key-value pairs.

## History

- 2026-05-10 — Promoted to canonical pattern spec. Added 4-column variant (`data-columns="4"`). Heading and valueUnknown were also proposed, then retracted the same day after a `src/components/context/CommercialShape.tsx` audit showed no consumer use.
- pre-2026-05-10 — CSS shipped at `.docs/design/reference/_shared/styles/ReferenceGrid.module.css`. Code consumers existed at `src/components/context/`. No canonical spec markdown.
