# ProvenanceStat

**Tier:** primitive
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `ProvenanceStat`
**`data-ds-spec`:** `primitives/ProvenanceStat.md`
**Variants:** `trend="up" | "down" | "flat"` (optional; absent = default)
**Design system version introduced:** 0.6.0

## Job

Render a label + value pair representing a tracked metric on an entity (Health, Stage, Confidence, Owner, Last touch, Tenure). Composed in stacked groups in the right column of `MovingRow`. The value can carry a trend hint via color so a stack of stats is scannable at a glance without numeric scrutiny.

## When to use it

- Right-column metric stack on an entity row pattern (today: `MovingRow`)
- When the value is a short rendered string (≤14 chars) and the label is a short noun (≤14 chars)
- When per-stat trust attribution matters — Health vs Confidence may carry different trust bands

## When NOT to use it

- For source attribution ("from Glean", "from CRM") — that's `ProvenanceTag`
- For a composite trust band on the entity as a whole — that's `TrustBandBadge`
- For freeform key/value display where the value is multi-line or rich — use a generic definition list pattern instead
- For action-bearing rows (clickable, mutable) — that's an action pattern, not a stat

## States / variants

- **Default** — label + value, no trend tint.
- **`ProvenanceStat_up`** — value text in `--color-garden-sage`. Driven by `trend: "up"`.
- **`ProvenanceStat_down`** — value text in `--color-spice-terracotta`. Driven by `trend: "down"`.
- **`ProvenanceStat_flat`** — value text muted via `--color-text-tertiary`. Driven by `trend: "flat"`.

## Composition

Primitive — no sub-primitives. Renders:

```html
<div class="ProvenanceStat" data-ds-name="ProvenanceStat" data-ds-spec="primitives/ProvenanceStat.md">
  <span class="ProvenanceStat_label">Health</span>
  <span class="ProvenanceStat_value ProvenanceStat_up">71 +3</span>
</div>
```

Two-column grid: `80px auto`. Both columns mono 11px. Label `--color-text-tertiary`, value `--color-text-primary` (or trend tint).

## Tokens consumed

- `--color-text-primary` — value (default)
- `--color-text-tertiary` — label
- `--color-garden-sage` — value (trend up)
- `--color-spice-terracotta` — value (trend down)
- `--font-mono` — label + value text
- `--space-xs` — vertical gap between stacked stats

## API sketch

```tsx
<ProvenanceStat label="Health" value="71 +3" trend="up" />
<ProvenanceStat label="Stage" value="Renewal" />
<ProvenanceStat label="Confidence" value="82%" trend="up" />
<ProvenanceStat label="Owner" value="You" />
```

Contract type:

```ts
interface ProvenanceStat extends TrustMixin {
  label: string;              // ≤14 chars (typography contract)
  value: string;              // ≤14 chars rendered
  trend?: "up" | "down" | "flat";
}
```

The TrustMixin carries per-stat provenance (Health and Confidence may have different trust bands and `source_asof` values).

## Source

- **Code:** ships W1 (DOS-423) at `src/components/dashboard/ProvenanceStat.tsx` + `src/components/dashboard/ProvenanceStat.module.css`
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign.html` (right column of each MovingRow)

## Surfaces that consume it

- DailyBriefing (via `MovingRow`)

## Naming notes

`ProvenanceStat` is the canonical name. Distinct from `ProvenanceTag` (source attribution label, e.g. "from Glean"). The `Stat` suffix is intentional and uncommon in the system — it signals "labeled metric on an entity" and is reserved for that role. See `NAMING.md`.

The `label` field is the kind discriminator (no enum). Analytics consumers join on the rendered label string — keeps the contract simple and lets services emit open-ended stat sets per entity kind. Truncation: label column is 80px with `text-overflow: ellipsis`; service-side budget is ≤14 characters per label. If a future use needs longer labels, lift to 96px and update the typography contract.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W1 alongside `MovingRow` under DOS-423.
