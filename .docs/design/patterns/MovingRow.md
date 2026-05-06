# MovingRow

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-423 (W1)
**Last updated:** 2026-05-06
**`data-ds-name`:** `MovingRow`
**`data-ds-spec`:** `patterns/MovingRow.md`
**Module CSS (canonical):** `src/components/dashboard/MovingRow.module.css` (ships W1)
**Composes:** `Pill`, `SignalDot`, `ProvenanceStat`, `EntityChip` (optional)

## Job

Render one entity's "what's moving" row in the Daily Briefing Moving section. Three-column grid: entity identity (left), narrative + signal feed (center), stacked provenance stats (right). Restraint: ≤3 entities visible per Moving section, lede ≤2 sentences.

## Anatomy

```
┌─────────────────┬──────────────────────────────────┬───────────────────┐
│ ENTITY NAME     │ Lede sentence(s).                │ Health   71 +3    │
│ [State pill ↑]  │                                  │ Stage    Renewal  │
│                 │ • 10:00  Pricing — in progress   │ Conf.    82%      │
│                 │ • 2d     Send memo — overdue     │ Owner    You      │
│                 │ • 3h ago Legal flagged 3 clauses │                   │
└─────────────────┴──────────────────────────────────┴───────────────────┘
```

Three CSS grid columns: `minmax(120px, 0.2fr) minmax(0, 1fr) minmax(120px, auto)`.

**Click target — resolved.** The whole row is the click target via a wrapping `<div>` with `role="link"` + `tabindex="0"` + click handler navigating to the entity detail page. **Not** a wrapping `<a>` — that would invalidate any nested interactive element. Inside the row, `MovingSignalViewModel.threadAction` (if present) is a separate `<button>` that stops event propagation; clicking the button performs the thread action without triggering the row navigation. This avoids invalid nested-anchor HTML.

Hover treatment matches the editorial-row hover (border-color shift, no transform).

## Variants

Variants come from the entity kind (drives left-column accent bar + state pill tone):

- `customer` — turmeric accent
- `person` — larkspur accent
- `project` — olive accent
- `internal` — neutral
- `lifecycle` — saffron (lifecycle transitions render as their own row when not folded into another entity's signal feed)

Accent bar implemented via `::before` pseudo on the row, color from `--moving-accent` CSS variable set by the `data-kind` attribute.

## Contract type

```ts
interface MovingEntityViewModel {
  kind: MovingEntityKind;
  entity: LinkedEntity;
  href: string;
  statePill: { label: string; tone: PillTone };
  lede: string;                      // 1-2 sentences
  signals: MovingSignalViewModel[];  // 3-5
  provenanceStats: ProvenanceStat[];
}
```

## States

- **Default present** — full row with all fields.
- **Loading / error / empty** — handled by the parent MovingViewModel state, not per-row. (No per-row loading state.)

## Composition rules

- `statePill` is a `Pill` primitive in compact size with the entity's `tone`.
- Each `signals[]` item is a `SignalDot` primitive (kind-tinted dot + when + whatSegments).
- Each `provenanceStats[]` item is a `ProvenanceStat` primitive.
- The lede sentence is service-rendered; view does not compose.

## What it doesn't do

- Sort or filter signals — service ordered.
- Decide which stats to show — service-curated per entity kind (customer carries Health/Stage/Confidence/Owner; person carries Tenure/Last 1:1/Mtgs moved; etc.).
- Compute change magnitude — that's `MovingService`'s job in W2.

## Open questions

- Mobile layout collapse: should the right column reflow below the center column (current d-spine HTML approach), or stay 2-column with stats inline?
- Click target: whole row vs only the entity name? Current spec is whole row.

## Spec status

**proposed** — TSX ships in W1 (DOS-423). Reference HTML at `briefing-redesign.html` uses `.dspine-moving-row` provisional class today; W1 cuts over to `MovingRow_*` scoped class names.
