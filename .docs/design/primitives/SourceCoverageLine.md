# SourceCoverageLine

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `SourceCoverageLine`
**`data-ds-spec`:** `primitives/SourceCoverageLine.md`
**Variants:** `default`; `withStaleCount`; `empty`
**Design system version introduced:** 0.2.0

## Job

Render a compact line summarizing source coverage for a piece of intelligence, so users can see which upstream system contributed, how many sources are present, and whether any are stale before opening deeper inspection.

## When to use it

- In receipt and inspection surfaces where a claim needs a quick coverage summary.
- Inside dossier and intelligence panels that explain what contributed to the rendered result.
- When source count, source family, and stale count all fit as a single inline metadata line.
- Near `ProvenanceTag`, `FreshnessIndicator`, or `TrustBandBadge` when users need both attribution and coverage depth.

## When NOT to use it

- For naming only one source without counts; use `ProvenanceTag`.
- For the age of a single source timestamp; use `FreshnessIndicator` or `AsOfTimestamp`.
- For trust judgment; use `TrustBandBadge` or `ConfidenceScoreChip`.

## States / variants

- `default` - renders source label plus total count, for example `Glean · 4 sources`.
- `withStaleCount` - appends stale count when non-zero, for example `Glean · 4 sources · 2 stale`.
- `empty` - renders a muted mono label such as `No source coverage` when the contract has no source manifest.
- Future variant TBD for multi-system coverage once the substrate exposes stable grouping.

## Tokens consumed

- `--font-mono` from `tokens/typography.md` - compact label typography.
- `--color-text-tertiary` from `tokens/color.md` - default muted metadata color.
- `--color-text-quaternary` from `tokens/color.md` - empty or unavailable state.
- `--color-spice-saffron` from `tokens/color.md` - stale-count emphasis.
- `--space-xs` - separators and inline gaps.

## API sketch

```tsx
type SourceCoverageLineProps = {
  sourceLabel: string;
  sourceCount: number;
  staleCount?: number;
  emptyLabel?: string;
};

<SourceCoverageLine sourceLabel="Glean" sourceCount={4} staleCount={2} />
```

## Source

- **Spec:** new for Wave 2
- **Substrate contract:** v1.4.0 source manifest and `source_asof` metadata used by receipt and inspection surfaces
- **Mockup origin:** `.docs/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Code:** to be implemented in `src/components/ui/SourceCoverageLine.tsx`

## Surfaces that consume it

Wave 2 receipts and inspection panels, especially `DossierSourceCoveragePanel` and `AboutThisIntelligencePanel`; likely inline support for DailyBriefing and AccountDetail inspection affordances.

## Naming notes

Canonical name is `SourceCoverageLine`. It is coverage depth, not source attribution; keep it separate from `ProvenanceTag`. If future UI needs a denser chip treatment, that should be a variant of this primitive unless the job changes.

## History

- 2026-05-03 — Proposed primitive for Wave 2 (v1.4.4 trust UI substrate).
