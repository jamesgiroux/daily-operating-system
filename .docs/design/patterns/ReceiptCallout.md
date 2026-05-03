# ReceiptCallout

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `ReceiptCallout`
**`data-ds-spec`:** `patterns/ReceiptCallout.md`
**Variants:** `mode="collapsed" | "expanded"`; `position="inline" | "drawer"`
**Design system version introduced:** 0.2.0

## Job

The inspectable receipt for a claim — drill-in from a `ClaimRow` to see *why* DailyOS believes what it believes. Surfaces the underlying trust math: resolver-band confidence, consistency findings, source attribution detail, freshness chain, and corrections / contradictions affordances.

This is the v1.4.4 inspection layer — the receipt that makes claims auditable.

## When to use it

- When the user clicks an `expandable` `ClaimRow` to drill in
- When trust banding is `use_with_caution` or `needs_verification` and the user benefits from understanding why
- For corrections flow (showing "you previously said X; we now infer Y because Z")
- For consistency findings ("this claim contradicts <other claim>")

## When NOT to use it

- For default trust render — use `TrustBand` (compact, no drill-in)
- For chapter-level coverage explanation — use `AboutThisIntelligencePanel` (broader scope)
- For dossier-level source coverage — use `DossierSourceCoveragePanel`
- For staleness warnings on full reports — use `StaleReportBanner`

## Composition

Two-state pattern:

**Collapsed (default)** — renders as the parent `ClaimRow` with an inspection chevron at the right edge.

**Expanded** — opens inline below the claim row OR in a side drawer (per `position`):

```
─── Receipt ───
[ResolverConfidenceBadge]  [ConsistencyFindingBanner if any]
[Source attribution detail — full source name + capture timestamp + raw value]
[Freshness chain — e.g., "fetched 3h ago · last verified 2d ago · stale threshold 7d"]
[Contradictions list (if any) — links to conflicting claims]
[Actions: Confirm / Correct / Dismiss / Flag for review]
─── End receipt ───
```

Subtle background tint (`--color-paper-warm-white`), border per band color, mono-typed metadata, serif-typed quoted source values.

## Composition contract

Composes:
- `ClaimRow` (the parent it expands from)
- `ResolverConfidenceBadge` (Wave 2 primitive — not yet specified; deferred from Audit 04)
- `ConsistencyFindingBanner` (Wave 2 pattern)
- `ProvenanceTag` (full-form, not suppressed even for synthesized)
- `FreshnessIndicator` (in `format="both"` mode)
- Action buttons (`Pill`-shaped with mono labels)

## States

- **collapsed** — parent ClaimRow renders normally with inspection chevron
- **expanded** — full receipt visible
- **loading** — skeleton receipt while fetching detail
- **error** — "Unable to load receipt — try refresh" inline message
- **corrected** — receipt shows the correction history (previous values, when corrected, by whom)

## Tokens consumed

- `--color-paper-warm-white` (background)
- Per-band border tints (likely_current → sage-12, use_with_caution → saffron-12, needs_verification → terracotta-12)
- `--color-rule-light` (internal section dividers)
- `--font-mono` (metadata, timestamps), `--font-serif` (quoted source values)
- `--color-text-secondary` (most receipt content)
- `--space-md`, `--space-lg` (internal padding)
- `--shadow-sm` (drawer mode only)
- `--transition-normal` (open/close animation)

## API sketch

```tsx
<ReceiptCallout
  claim={claim}
  mode="expanded"
  position="inline"
  onCorrect={(correction) => /* corrections service */}
  onDismiss={() => /* dismissal */}
  onFlagForReview={() => /* surface to user */}
/>
```

When `mode="collapsed"`, renders as ClaimRow with chevron; click toggles to `expanded`. Drawer mode renders in a side panel (used on Account Detail when the user opens "inspect" from any claim).

## Source

- **Spec:** new for Wave 2
- **v1.4.4 contract:** receipt experience per the Linear v1.4.4 project description ("inspectable receipts for provenance, trust, freshness, contradictions, corrections, and audit boundaries")
- **Code:** to be implemented in `src/components/intelligence/ReceiptCallout.tsx`

## Surfaces that consume it

Every claim-rendering surface in v1.4.4: AccountDetail (drill-in from any fact), DailyBriefing (drill-in from claim items in entity portraits), ProjectDetail, PersonDetail, MeetingDetail. The drawer position is used when a surface has many claims and inline expansion would crowd; otherwise inline.

## Naming notes

`ReceiptCallout` — "receipt" (the auditable record) + "callout" (the visually distinct expanded section). Don't rename to `ClaimDetail` (too generic), `Inspector` (overlaps with the design system inspector module), or `AuditTrail` (not the same concept — audit trail is the history; receipt is the current state's evidence).

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate). Foundational for the receipts experience.
