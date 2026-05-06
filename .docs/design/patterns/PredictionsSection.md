# PredictionsSection

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-425 (W3)
**Last updated:** 2026-05-06
**`data-ds-name`:** `PredictionsSection`
**`data-ds-spec`:** `patterns/PredictionsSection.md`
**Module CSS (canonical):** `_shared/styles/PredictionsSection.module.css`
**Composes:** `MarginGrid` (parent), `TrustBandBadge` (per item)

## Job

Render the Daily Briefing Predictions section: a minimal one-line collapsed default, click-to-expand inline list of predictions sourced from the abilities runtime (DOS-218/219 outputs). Restraint: collapsed default takes <32px vertical space.

## Anatomy

Collapsed default:

```
3 predictions today    EXPAND
```

Single button with `aria-expanded="false"`, label + hint span. Click toggles `aria-expanded="true"` and reveals the list inline (no route change).

Expanded:

```
3 predictions today    COLLAPSE

  Northwind QBR likely raises pricing pushback once
  Kevin sees the renewal terms.
  72% confidence · via predict_meeting_friction · basis    [LIKELY CURRENT]

  [next prediction...]
```

Each prediction item: text (serif 17px) + meta line (mono 11px, confidence + ability source + basis link) + TrustBandBadge.

## Variants

- **Collapsed** (default): single line, no payload visible.
- **Expanded**: card list visible inline.

The variant is UI state, not data state. Payload is eager-loaded (per ADR-0109 rationale: predictions count is small, payload size acceptable, expand intent is friction-free).

## Contract type

```ts
interface PredictionsViewModel {
  label: string;            // "Predictions" — margin grid label
  countLabel: string;       // "3 today" — margin grid count
  collapsedLabel: string;   // "3 predictions today" — default state line
  expandHint: string;       // "expand" — affordance hint
  count: number;            // for type-narrowing + analytics
  predictions: PredictionItem[];
}

interface PredictionItem extends TrustMixin {
  text: string;
  confidence: { value: number; label: string };
  abilitySource: { id: string; label: string };
  basisLink: { label: string; href: string };
}
```

## States

- **Loading / error / empty** — handled at the top-level envelope. When the parent BriefingViewModel is in `success`, this section's `count: 0` indicates no predictions; the collapsed label shows "0 predictions today" and the trigger is disabled.

## Composition rules

- The collapsed trigger is a `<button>` with `aria-expanded` + `aria-controls`.
- Each item composes a `TrustBandBadge` showing the band level.
- The basis link text is service-rendered (don't compose "view basis" in view).

## What it doesn't do

- Fetch predictions — `PredictionsService` (DOS-418) shapes the data.
- Update state — clicking the basis link navigates; clicking dismiss emits `predictions::ack`.
- Animate the expand/collapse beyond the editorial default (no slide, no fade).

## Open questions

- When count is 0, hide the section entirely or show "0 predictions today" muted? Current spec leans toward the latter for predictability.
- Keyboard shortcut for expand? TBD with accessibility review.

## Spec status

**proposed** — TSX ships in W3 (DOS-425). Reference HTML at `briefing-redesign.html` consumes the canonical module CSS today.
