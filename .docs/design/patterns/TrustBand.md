# TrustBand

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `TrustBand`
**`data-ds-spec`:** `patterns/TrustBand.md`
**Variants:** `density="compact" | "default" | "expanded"`; `align="inline" | "row"`
**Design system version introduced:** 0.2.0

## Job

Render the canonical trust signal cluster for a piece of intelligence — the user-facing answer to "should I trust this?" Composed of three Wave 1 primitives in a consistent layout: trust band judgment + provenance + freshness.

This is the **default trust render** consumed by every claim-rendering surface. v1.4.4 receipts (`ReceiptCallout`) wrap this for inspection.

## When to use it

- Inline next to any claim, intelligence summary, or AI-produced fact on every editorial surface
- Wherever the user benefits from a quick, scannable answer to "how much do I trust this?"
- As the trust signal inside `ClaimRow` — every claim row composes one
- Inside `ReceiptCallout` (v1.4.4) as the collapsed-state header

## When NOT to use it

- For non-claim content (chrome, navigation, marketing copy) — trust signals don't apply
- When only one of the three primitives is needed (use the primitive directly)
- For raw provenance display without judgment — use `ProvenanceTag` alone

## Composition

Three Wave 1 primitives in a consistent left-to-right layout:

```
[TrustBandBadge]  [ProvenanceTag]  [FreshnessIndicator]
```

**Compact** (default for in-line claim rows): all three as small chips, single row, gap `--space-xs`.

**Default** (for chapter-level signals): all three with labels visible, gap `--space-sm`.

**Expanded** (for receipt headers, prominent surfaces): all three with full labels, plus optional provenance link to drill in (becomes `ReceiptCallout`).

`align="row"` stacks vertically on narrow widths.

## Tokens consumed

Inherits from composed primitives:
- `--color-trust-likely-current`, `--color-trust-use-with-caution`, `--color-trust-needs-verification` (via TrustBandBadge)
- `--color-text-tertiary` (via ProvenanceTag default)
- Staleness colors via FreshnessIndicator
- `--space-xs`, `--space-sm` (gaps)

## API sketch

```tsx
<TrustBand
  band="likely_current"
  source="glean"
  asOf="2026-05-02T08:00:00Z"
  density="compact"
/>

{/* Or accept a single Claim object: */}
<TrustBand claim={claim} density="default" />
```

When `claim` prop is passed, the pattern reads `band`, `source`, `asOf` from it. Synthesized claims (`pty_synthesis`) hide ProvenanceTag per existing primitive behavior.

## Source

- **Spec:** new for Wave 2
- **Composed primitives:** `TrustBandBadge`, `ProvenanceTag`, `FreshnessIndicator` (all Wave 1)
- **Code:** to be implemented in `src/components/intelligence/TrustBand.tsx` (Wave 2 follow-on for v1.4.4)

## Surfaces that consume it

DailyBriefing (per-claim trust signals), AccountDetail (claim rows + chapter freshness), MeetingDetail (intelligence sections), ProjectDetail, PersonDetail. Wrapped by `ReceiptCallout` for inspection.

## Naming notes

`TrustBand` — keeps the v1.4.0 vocabulary (the underlying contract is "trust bands"). Don't rename to `TrustSignal`, `TrustHeader`, or `TrustChip` — those collide with primitive names.

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
