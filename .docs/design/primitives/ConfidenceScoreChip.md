# ConfidenceScoreChip

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `ConfidenceScoreChip`
**`data-ds-spec`:** `primitives/ConfidenceScoreChip.md`
**Variants:** `band="likely_current" | "use_with_caution" | "needs_verification"`
**Design system version introduced:** 0.2.0

## Job

Render a numerical confidence score in a compact chip, preserving the exact score while mapping color to the four-tier trust vocabulary users already see elsewhere.

## When to use it

- In receipt and inspection views where the exact confidence value matters.
- Next to resolver, lifecycle, or source-level evidence where users need more than a band label.
- When a claim has a normalized confidence score that can be expressed as a percentage.
- Beside `TrustBandBadge` when the interface needs both the band and the numeric value.

## When NOT to use it

- For the user-facing trust judgment alone; use `TrustBandBadge`.
- For source attribution; use `ProvenanceTag`.
- For source freshness or age; use `FreshnessIndicator` or `AsOfTimestamp`.

## States / variants

- `band="likely_current"` - score is `>= 0.85`; green family treatment.
- `band="use_with_caution"` - score is `>= 0.60` and `< 0.85`; amber family treatment.
- `band="needs_verification"` - score is `< 0.60`; red family treatment.
- `unavailable` - renders muted `--` when a score is not present but the layout needs a stable slot.
- Future variant TBD for resolver-specific labels such as Resolved or ResolvedWithFlag.

## Tokens consumed

- `--font-mono` from `tokens/typography.md` - numeric label typography.
- `--color-trust-likely-current` from `tokens/color.md` - high-confidence tone.
- `--color-trust-use-with-caution` from `tokens/color.md` - mid-confidence tone.
- `--color-trust-needs-verification` from `tokens/color.md` - low-confidence tone.
- `--color-text-tertiary` from `tokens/color.md` - unavailable state.
- `--space-xs`, `--space-sm` - chip padding and inline gap.

## API sketch

```tsx
type ConfidenceScoreChipProps = {
  score: number | null;
  format?: "percent";
};

<ConfidenceScoreChip score={0.82} />
<ConfidenceScoreChip score={null} />
```

Band mapping is derived by the primitive: `>= 0.85` maps to `likely_current`, `0.60-0.85` maps to `use_with_caution`, and `< 0.60` maps to `needs_verification`.

## Source

- **Spec:** new for Wave 2
- **Substrate contract:** v1.4.0 trust thresholds from resolver and render contracts: `likely_current >= 0.85`, `use_with_caution 0.60-0.85`, `needs_verification < 0.60`
- **Mockup origin:** `.docs/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Code:** to be implemented in `src/components/ui/ConfidenceScoreChip.tsx`

## Surfaces that consume it

Wave 2 receipts and inspection panels, especially lifecycle verification rows, evidence-backed claim rows, and intelligence detail panels that need exact confidence.

## Naming notes

Canonical name is `ConfidenceScoreChip`. It is numerical confidence, not the banded user judgment; keep `TrustBandBadge` as the primary readable trust label.

## History

- 2026-05-03 — Proposed primitive for Wave 2 (v1.4.4 trust UI substrate).
