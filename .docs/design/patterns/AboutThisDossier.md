# AboutThisDossier

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `AboutThisDossier`
**`data-ds-spec`:** `patterns/AboutThisDossier.md`
**Variants:** `mode="multi-card | narrative"`; optional inline action affordance ("Verify or dismiss…")
**Design system version introduced:** 0.4.0

## Job

Render the editorial metadata that lives at the foot of an entity dossier — the "About this dossier" footer that explains how the dossier was assembled, what coverage gaps exist, and what assessments need verification. Two modes:

1. **multi-card** (default) — separate left-bordered cards for distinct metadata facets (data capture gap / source coverage / freshness)
2. **narrative** — single italic-serif paragraph for tighter dossiers where the metadata reads as one short editorial note

Distinct from `AboutThisIntelligencePanel` (chapter-level trust panel that explains intelligence coverage for a specific chapter) and `DossierSourceCoveragePanel` (full source-coverage table). AboutThisDossier is the editorial dossier-level metadata footer; it can compose those others when the dossier needs richer detail.

## When to use it

- At the foot of Account Detail, Project Detail, or Person Detail when the dossier needs an editorial closeout
- When verification states or capture gaps deserve a dossier-level call-out without taking over the page
- For reports when an editorial signoff explains how the report was assembled

## When NOT to use it

- For chapter-level trust explanation — use `AboutThisIntelligencePanel`
- For dossier-level source coverage table — use `DossierSourceCoveragePanel`
- For a finite editorial-page ending — use `FinisMarker` (AboutThisDossier sits BEFORE FinisMarker)
- For trust-receipt drill-in — use `ReceiptCallout`

## States / variants

- **multi-card** — default. One section root with multiple `.AboutThisDossier_card` blocks, each with mono eyebrow + sans body. Used when the dossier has distinct metadata facets to separate
- **narrative** — `data-mode="narrative"` on section root. Single italic-serif paragraph with optional inline `<strong>` emphasis and optional `.AboutThisDossier_bodyAction` call-to-action link. Tighter; reads as editorial signoff. Used when the metadata is short and unified

## Composition

This pattern does not compose other primitives in the narrative variant; it is a self-contained editorial paragraph.

In multi-card mode, each card may compose:
- `FreshnessIndicator` (when a card carries timestamp data)
- `EntityChip` (when a card references stakeholders to verify)

## Tokens consumed

- `--font-mono` (eyebrow), `--font-sans` (multi-card body), `--font-serif` (narrative body)
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`
- `--color-desk-charcoal-4` (background tint)
- `--color-rule-light`
- `--color-garden-olive`, `--color-garden-rosemary` (action link tint — project-tinted)
- `--space-xs | sm | md | lg | 4xl | 5xl`
- `--radius-sm`, `--radius-editorial-sm`

## API sketch

**Narrative mode (project-d-spine canonical):**

```html
<section class="AboutThisDossier_section"
         data-ds-name="AboutThisDossier"
         data-ds-spec="patterns/AboutThisDossier.md"
         data-mode="narrative">
  <div class="AboutThisDossier_eyebrow">About this dossier</div>
  <p class="AboutThisDossier_body">
    Assembled from 32 signals across project standups, design partner calls,
    Slack threads and the steering Notion. Assessments for
    <strong>Sara Wu</strong> and one partner engineer are flagged for
    verification — both have been in the work for 6+ weeks but we haven't
    characterized the relationship.
    <a href="#" class="AboutThisDossier_bodyAction">Verify or dismiss →</a>
  </p>
</section>
```

**Multi-card mode (existing variant):**

```html
<section class="AboutThisDossier_section" data-ds-name="AboutThisDossier">
  <div class="AboutThisDossier_eyebrow">About this dossier</div>
  <div class="AboutThisDossier_card">
    <div class="AboutThisDossier_cardLabel">Data capture gap</div>
    <p class="AboutThisDossier_cardText">…</p>
  </div>
  <div class="AboutThisDossier_card">
    <div class="AboutThisDossier_cardLabel">Source coverage</div>
    <p class="AboutThisDossier_cardText">…</p>
  </div>
</section>
```

React form:

```tsx
<AboutThisDossier mode="narrative">
  Assembled from 32 signals across project standups…
  <AboutThisDossier.Action href="/stakeholders">Verify or dismiss</AboutThisDossier.Action>
</AboutThisDossier>
```

## Source

- **Code:** `src/components/context/AboutThisDossier.tsx`
- **Reference CSS:** `.docs/design/reference/_shared/styles/AboutThisDossier.module.css`
- **Mockup origin:** Multiple — narrative variant from `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.meta-section`, `.meta-body`); multi-card variant from earlier account-detail mockups

## Surfaces that consume it

- ProjectDetail (narrative variant — canonical v1.4.2 consumer)
- AccountDetail (multi-card variant)
- PersonDetail (multi-card or narrative depending on dossier richness)
- Reports (narrative variant for executive signoff)

## Naming notes

`AboutThisDossier` — clear-intent name; the section literally explains "about this dossier." Not `DossierMetadata` (too generic — could mean any metadata). Not `EditorialFooter` (too specific to UI position). Sibling to `AboutThisIntelligencePanel` (which is chapter-scoped, not dossier-scoped). Inline action affordance follows the editorial dotted-underline convention used elsewhere in the system.

## History

- 2026-05-10 — Promoted to canonical pattern spec. Added narrative variant (`data-mode="narrative"`) for project-detail d-spine usage, with optional inline action affordance.
- pre-2026-05-10 — Component shipped at `src/components/context/AboutThisDossier.tsx` with multi-card variant only. CSS at `.docs/design/reference/_shared/styles/AboutThisDossier.module.css`.
