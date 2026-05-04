# VerificationStatusFlag

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `VerificationStatusFlag`
**`data-ds-spec`:** `primitives/VerificationStatusFlag.md`
**Variants:** `status="ok" | "corrected" | "flagged"`
**Design system version introduced:** 0.2.0

## Job

Render the consistency state for a piece of intelligence as a small icon plus mono label, so users can distinguish verified, auto-corrected, and flagged claims without reading the full receipt.

## When to use it

- In Wave 2 receipt and inspection surfaces that expose v1.4.0 consistency metadata.
- Next to a claim, row, or panel header where verification state is important for action.
- When deterministic repair or unresolved findings need a compact visible marker.
- Alongside `ConfidenceScoreChip` when score and consistency state both matter.

## When NOT to use it

- For overall trust banding; use `TrustBandBadge`.
- For stale or missing source data; use `FreshnessIndicator`, `AsOfTimestamp`, or `DataGapNotice`.
- For detailed consistency findings; use a Wave 2 finding pattern instead.

## States / variants

- `status="ok"` - neutral or positive icon with label `OK`; no known consistency issue.
- `status="corrected"` - repair icon with label `Corrected`; deterministic repair changed the claim or evidence.
- `status="flagged"` - warning icon with label `Flagged`; unresolved finding remains visible to the user.
- Future variant TBD for severity-specific flagged states if the substrate makes that necessary.

## Tokens consumed

- `--font-mono` from `tokens/typography.md` - status label typography.
- `--color-text-tertiary` from `tokens/color.md` - ok state.
- `--color-spice-saffron` from `tokens/color.md` - corrected state.
- `--color-trust-needs-verification` from `tokens/color.md` - flagged state.
- `--space-xs` - icon and label gap.

## API sketch

```tsx
type VerificationStatus = "ok" | "corrected" | "flagged";

type VerificationStatusFlagProps = {
  status: VerificationStatus;
  label?: string;
};

<VerificationStatusFlag status="ok" />
<VerificationStatusFlag status="corrected" />
<VerificationStatusFlag status="flagged" label="Flagged" />
```

## Source

- **Spec:** new for Wave 2
- **Substrate contract:** v1.4.0 consistency trust metadata: `ok`, `corrected`, or `flagged`, with findings carrying code, severity, claim text, evidence text, and auto-fix state
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Code:** to be implemented in `src/components/ui/VerificationStatusFlag.tsx`

## Surfaces that consume it

Wave 2 receipts and inspection panels, especially `LifecycleVerificationRow`, `ConsistencyFindingBanner`, and `EvidenceBackedClaimRow`.

## Naming notes

Canonical name is `VerificationStatusFlag`. Use `Flag` because the primitive marks consistency state inline; detailed evidence belongs in a pattern, not this primitive.

## History

- 2026-05-03 — Proposed primitive for Wave 2 (v1.4.4 trust UI substrate).
