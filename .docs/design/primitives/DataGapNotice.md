# DataGapNotice

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `DataGapNotice`
**`data-ds-spec`:** `primitives/DataGapNotice.md`
**Variants:** `severity="info" | "warning"`
**Design system version introduced:** 0.2.0

## Job

Render an inline notice that a piece of intelligence is missing a critical input, so users understand why the system may be incomplete before acting on the result.

## When to use it

- Inside `DossierSourceCoveragePanel` when source coverage has a known gap.
- Inside `AboutThisIntelligencePanel` when enrichment or source metadata is missing.
- Next to intelligence that depends on meetings, stakeholder roles, source manifests, or capture metadata.
- When the copy names the missing input directly, such as `no recent meetings` or `stakeholder roles unknown`.

## When NOT to use it

- For stale data that exists; use `FreshnessIndicator` or `AsOfTimestamp`.
- For low trust or consistency findings; use `TrustBandBadge` or `VerificationStatusFlag`.
- For a full data capture explanation; use a Wave 2 panel pattern.

## States / variants

- `severity="info"` - muted inline notice for a gap that limits context but does not block interpretation.
- `severity="warning"` - amber inline notice for a gap that materially affects whether the user should rely on the intelligence.
- `dismissed` is not a primitive state; dismissal belongs to the consuming surface if needed.
- Future variant TBD for blocking gaps if a later substrate contract defines one.

## Tokens consumed

- `--font-mono` from `tokens/typography.md` - compact gap label.
- `--color-text-tertiary` from `tokens/color.md` - info state text.
- `--color-spice-saffron` from `tokens/color.md` - warning state text and icon.
- `--color-text-quaternary` from `tokens/color.md` - secondary context.
- `--space-xs`, `--space-sm` - icon gap and inline padding.

## API sketch

```tsx
type DataGapNoticeProps = {
  message: string;
  severity?: "info" | "warning";
};

<DataGapNotice message="No recent meetings" />
<DataGapNotice message="Stakeholder roles unknown" severity="warning" />
```

## Source

- **Spec:** new for Wave 2
- **Substrate contract:** v1.4.0 source and enrichment metadata can expose missing source manifests, missing capture inputs, and verification needs
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Code:** to be implemented in `src/components/ui/DataGapNotice.tsx`

## Surfaces that consume it

`DossierSourceCoveragePanel` and `AboutThisIntelligencePanel`; likely support for data capture gap panels and account context inspection surfaces.

## Naming notes

Canonical name is `DataGapNotice`. Keep this scoped to missing inputs. It should not absorb freshness, provenance, or verification jobs.

## History

- 2026-05-03 — Proposed primitive for Wave 2 (v1.4.4 trust UI substrate).
