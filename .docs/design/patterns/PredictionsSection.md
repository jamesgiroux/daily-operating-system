# PredictionsSection

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `PredictionsSection`
**`data-ds-spec`:** `patterns/PredictionsSection.md`
**Variants:** `default` (collapsed/expanded is interaction state, not a variant)
**Design system version introduced:** 0.6.0

## Job

Render the Daily Briefing Predictions section: a minimal one-line collapsed default, click-to-expand inline list of predictions sourced from the abilities runtime (DOS-218 / DOS-219 outputs). Restraint contract: the collapsed default takes <32px vertical space so the section doesn't dominate the briefing when predictions aren't the user's focus.

## When to use it

- Inside the Daily Briefing, immediately below `Lead` and above `Moving` / `Watch`
- When the surface needs to surface ability-runtime predictions without forcing them into the user's primary focus
- When the count is small (≤10 — service-capped) and the payload is eager-loadable

## When NOT to use it

- For a deep-dive predictions surface — that's a dedicated route, not this section
- For ad-hoc inline predictions on entity pages — use a smaller pattern (TBD post-W6)
- For predictions that aren't claim-bearing (must be `TrustMixin`-typed)

## States / variants

The pattern has one variant; collapsed vs expanded is interaction state, not a structural variant.

- **Collapsed (default)** — single line, no payload visible. `aria-expanded="false"`.
- **Expanded** — card list visible inline. `aria-expanded="true"`.
- **Empty (count = 0)** — collapsed line shows "0 predictions today" muted, trigger disabled.

Loading / error are handled by the parent `BriefingLoadState`; this section never renders a per-section loading state.

## Composition

Composes:

- `MarginGrid` — parent layout (label column + content column)
- `TrustBandBadge` — per prediction item

Collapsed:

```
3 predictions today    EXPAND
```

Expanded:

```
3 predictions today    COLLAPSE

  Northwind QBR likely raises pricing pushback once
  Kevin sees the renewal terms.
  72% confidence · via predict_meeting_friction · basis    [LIKELY CURRENT]

  [next prediction...]
```

Each prediction item: text (serif 17px) + meta line (mono 11px, confidence + ability source + basis link) + `TrustBandBadge`.

The collapsed trigger is a `<button>` with `aria-expanded` + `aria-controls`. Payload is eager-loaded — predictions count is small, expand intent is friction-free, no on-expand fetch.

## Tokens consumed

- `--color-text-primary` — prediction text
- `--color-text-secondary` — collapsed label
- `--color-text-tertiary` — meta line, confidence, ability source
- `--color-spice-saffron` — expand/collapse hint
- `--font-serif` — prediction text
- `--font-sans` — collapsed label, trigger
- `--font-mono` — meta line
- `--space-md`, `--space-lg` — item spacing in expanded list
- `--margin-grid-label-width` — label column width (inherited)

## API sketch

```tsx
<PredictionsSection
  label="Predictions"
  countLabel="3 today"
  collapsedLabel="3 predictions today"
  expandHint="expand"
  count={3}
  predictions={[
    {
      id: "pred_1",
      text: "Northwind QBR likely raises pricing pushback once Kevin sees the renewal terms.",
      confidence: { value: 0.72, label: "72%" },
      abilitySource: { id: "predict_meeting_friction", label: "predict_meeting_friction" },
      basisLink: { label: "basis", href: "/predictions/pred_1" },
      trustBand: "likely_current",
    },
    /* ... */
  ]}
/>
```

Contract type:

```ts
interface PredictionsViewModel {
  label: string;            // "Predictions"
  countLabel: string;       // "3 today"
  collapsedLabel: string;   // "3 predictions today"
  expandHint: string;       // "expand"
  count: number;
  predictions: PredictionItem[];   // service-capped at ≤10
}

interface PredictionItem extends TrustMixin {
  id: string;
  text: string;
  confidence: { value: number; label: string };
  abilitySource: { id: string; label: string };
  basisLink: { label: string; href: string };
}
```

The view does not fetch — `PredictionsService` (DOS-418) shapes data. The view does not update — basis link navigates, dismiss button emits `predictions::ack(id)`. No animation beyond editorial default (no slide, no fade).

## Source

- **Code:** ships W3 (DOS-425) at `src/components/dashboard/PredictionsSection.tsx` + `src/components/dashboard/PredictionsSection.module.css`
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign.html` (Predictions section, between Lead and Moving)

## Surfaces that consume it

- DailyBriefing (Predictions section)

## Naming notes

`PredictionsSection` is the canonical name. The `Section` suffix is intentional — this pattern wraps a `MarginGrid` row with predictions-specific contract semantics (collapsed / expanded behavior, ability sourcing, trust integration). Distinct from a raw `<section>` element. See `NAMING.md`.

The pattern is briefing-resident today but is named generically so a future surface can adopt it. Restraint contract (collapsed <32px) is a property of the pattern, not the briefing.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W3 under DOS-425.
