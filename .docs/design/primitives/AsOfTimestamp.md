# AsOfTimestamp

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `AsOfTimestamp`
**`data-ds-spec`:** `primitives/AsOfTimestamp.md`
**Variants:** `format="relative" | "absolute" | "both"`
**Design system version introduced:** 0.2.0

## Job

Render the static `as of` timestamp for intelligence, giving users a clear reference point such as `As of 3h ago` without applying staleness coloring or trust judgment.

## When to use it

- In receipt, briefing, and inspection surfaces that need a neutral timestamp label.
- When the source age is useful but staleness thresholds should not alter tone.
- In generated report footers, source summaries, or panel headers that need static metadata.
- When paired with other trust primitives where color is already carrying confidence or verification meaning.

## When NOT to use it

- When timestamp age should visually shift by stale threshold; use `FreshnessIndicator`.
- For source attribution; use `ProvenanceTag`.
- For confidence, trust, or verification state; use `ConfidenceScoreChip`, `TrustBandBadge`, or `VerificationStatusFlag`.

## States / variants

- `format="relative"` - default inline form, for example `As of 3h ago`.
- `format="absolute"` - exact time form, for example `As of Apr 22, 10:30am`.
- `format="both"` - inspection form, for example `As of 3h ago · Apr 22 10:30`.
- `unavailable` - muted `As of unavailable` only when the consuming surface needs a stable row.

## Tokens consumed

- `--font-mono` from `tokens/typography.md` - timestamp label typography.
- `--color-text-tertiary` from `tokens/color.md` - neutral timestamp text.
- `--color-text-quaternary` from `tokens/color.md` - unavailable state.
- `--space-xs` - separator spacing.

## API sketch

```tsx
type AsOfTimestampProps = {
  at: string | Date | null;
  format?: "relative" | "absolute" | "both";
  prefix?: "As of";
};

<AsOfTimestamp at="2026-05-03T12:00:00Z" />
<AsOfTimestamp at={sourceAsof} format="both" />
```

## Source

- **Spec:** new for Wave 2
- **Substrate contract:** v1.4.0 `source_asof` timestamp rendered in neutral receipt and inspection contexts
- **Mockup origin:** `.docs/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Code:** to be implemented in `src/components/ui/AsOfTimestamp.tsx`

## Surfaces that consume it

Wave 2 receipts and inspection panels, generated report metadata, DailyBriefing as-of labels, and account context panels that need neutral source timing.

## Naming notes

Canonical name is `AsOfTimestamp`. This may ultimately consolidate with `FreshnessIndicator`; for now it covers the static-label form while `FreshnessIndicator` owns staleness coloring.

## History

- 2026-05-03 — Proposed primitive for Wave 2 (v1.4.4 trust UI substrate).
