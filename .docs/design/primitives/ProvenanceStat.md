# ProvenanceStat

**Tier:** primitive
**Status:** proposed
**Owner:** DOS-423 (W1, ships alongside MovingRow)
**Last updated:** 2026-05-06
**`data-ds-name`:** `ProvenanceStat`
**`data-ds-spec`:** `primitives/ProvenanceStat.md`
**Module CSS (canonical):** `_shared/styles/ProvenanceStat.module.css`

## Job

Render a label + value pair representing a tracked metric on an entity (Health, Stage, Confidence, Owner, Last touch, Tenure, etc.). Composed in stacked groups in the right column of MovingRow patterns. Distinct from `ProvenanceTag` (which represents source attribution like "from Glean") — ProvenanceStat is a labeled metric.

## Anatomy

```
[label]   [value]
```

Two-column CSS grid: `80px auto`. Mono font, 11px, label muted, value emphasized.

## Variants

- **Default** — label + value.
- **Trending up** (`ProvenanceStat_up`) — value text in `--color-garden-sage`.
- **Trending down** (`ProvenanceStat_down`) — value text in `--color-spice-terracotta`.
- **Trending flat** (`ProvenanceStat_flat`) — value text muted (tertiary).

The trend variant is selected by the contract's `ProvenanceStat.trend` field; absent = default.

## Contract type

```ts
interface ProvenanceStat extends TrustMixin {
  label: string;          // "Health", "Stage", "Confidence", "Owner"
  value: string;          // "71 +3", "Renewal", "82%", "You"
  trend?: "up" | "down" | "flat";
}
```

The trust mixin lets a stat carry its own per-field trust band (Health and Confidence have different provenance and may have different trust bands).

## What it doesn't do

- Compose the value — service produces "71 +3" as a single rendered string. The view does not concatenate.
- Render the trust band — `TrustBandBadge` is a separate primitive composed alongside if visible UI is needed (typical: ProvenanceStat does not render the band visually; it carries the trust metadata for analytics + downstream sensitivity decisions).

## Open questions

- Should the label have a max-width truncation? Reference fixture uses 80px which works for "Last touch" but breaks on longer label strings.
- Should there be a `kind` discriminator (health/stage/confidence/owner) for analytics, or is `label` text-only?

## Spec status

**proposed** — TSX + final module CSS ship in W1 alongside MovingRow.
