# ClaimRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `ClaimRow`
**`data-ds-spec`:** `patterns/ClaimRow.md`
**Variants:** `density="compact" | "default" | "rich"`; `expandable` (toggleable inspection)
**Design system version introduced:** 0.2.0

## Job

The canonical render for a single claim — the core unit of v1.4.0 intelligence. Combines the claim's value, subject attribution, trust signals, and (when expandable) drill-in to receipt detail. Every claim-rendering surface consumes this pattern.

## When to use it

- Every place a claim is rendered as a discrete unit (account detail facts, briefing intelligence items, project state, person stakeholder info, meeting prep facts)
- Lists of claims (account dossier sections, intelligence panels)
- When user benefits from seeing trust signals next to the claim value

## When NOT to use it

- For free-form intelligence prose (use editorial typography directly)
- For a single ungrouped fact without claim semantics (use raw text with optional `TrustBand`)
- For receipt inspection mode — that's `ReceiptCallout` (which wraps ClaimRow)

## Composition

Two reading orders depending on density:

**Compact / default** (single line):
```
[Subject • field — value]                          [TrustBand compact]
```

Subject and field render as muted prefix; value as primary text. TrustBand sits at the right edge.

**Rich** (multi-line):
```
[Subject — field]
[Value, prose-quality if narrative]
[TrustBand default — band + provenance + freshness]
[Optional: actions row — Correct / Dismiss / Inspect]
```

When `expandable`, clicking the row (or an explicit drill-in chevron) opens `ReceiptCallout` inline below.

## States

- **default** — normal claim render
- **corrected** — render with strikethrough on previous value + new value highlighted; consistency state pulled from claim metadata
- **flagged** — `ConsistencyFindingBanner` renders above the row; row gets terracotta left border
- **dismissed** — opacity 0.4, struck through; only visible in inspection mode

## Composition contract

- Always composes `TrustBand`
- Uses `Pill` (with `tone="neutral"`) for subject prefix when rendered
- Optional `EntityChip` for entity references in claim text
- For `expandable` mode, manages `ReceiptCallout` open/close state

## Tokens consumed

- `--font-serif` (claim value when prose-quality), `--font-sans` (default value), `--font-mono` (subject/field prefix)
- `--color-text-primary` (value), `--color-text-secondary` (prefix), `--color-text-tertiary` (metadata)
- `--color-spice-terracotta-8` (flagged left border), `--color-spice-terracotta` (flagged accent)
- `--color-rule-light` (row border-bottom)
- `--space-md` (vertical row padding), `--space-sm` (gap between value and trust band)
- All TrustBand-inherited tokens

## API sketch

```tsx
<ClaimRow
  claim={{
    subject: "Acme Corp",
    field: "renewal_date",
    value: "2026-06-15",
    band: "likely_current",
    source: "salesforce",
    asOf: "2026-05-02T03:00:00Z",
    consistencyState: "ok",
  }}
  density="default"
  expandable
  onCorrect={(claim) => /* corrections flow */}
  onDismiss={(claim) => /* dismissal flow */}
/>
```

## Source

- **Spec:** new for Wave 2
- **Code:** to be implemented in `src/components/intelligence/ClaimRow.tsx`
- **Existing similar:** `src/components/health/TriageCard.tsx` is row-shaped intelligence with trust-adjacent affordances (Audit 02 surfaced); future ClaimRow may absorb some of its API; for now, document distinct.

## Surfaces that consume it

DailyBriefing (entity portrait card threads — those events become claim rows), AccountDetail (Health, Context — most fact rendering), ProjectDetail, PersonDetail, MeetingDetail (Findings, Predictions sections), every intelligence panel.

## Naming notes

`ClaimRow` — matches the v1.4.0 substrate vocabulary (claims are the unit). Don't rename to `IntelligenceRow` (broader) or `FactRow` (legacy framing pre-v1.4.0).

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
