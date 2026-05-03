# ProvenanceTag

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `ProvenanceTag`
**`data-ds-spec`:** `primitives/ProvenanceTag.md`
**Variants:** plain (label only); `discrepancy` (highlights when sources disagree)
**Design system version introduced:** 0.1.0

## Job

Render a muted source-attribution label for a piece of intelligence — "where did this come from?" — when the source is meaningful to users. Suppresses synthetic-source labels by default (no point telling the user "we generated this").

## When to use it

- Inline next to claims, intelligence summaries, or rendered facts that have a distinct external source (Glean, email, calendar, CRM, manual entry, etc.)
- When the user benefits from knowing the source for trust calibration or follow-up
- Composed inside `TrustBand` (Wave 2) and `ClaimRow` (Wave 2)

## When NOT to use it

- For raw recency / age — use `FreshnessIndicator`
- For trust judgment — use `TrustBandBadge`
- For completeness — use `IntelligenceQualityBadge`
- When the source is `pty_synthesis` (synthesized intelligence) — the primitive intentionally suppresses these by default

## States / variants

- **default** — small muted label with source name (e.g., "Glean", "Salesforce", "Email")
- **discrepancy** — when sources disagree on a fact, render with attention treatment (subtle outline or warn color)

Note from Audit 04: production behavior intentionally hides `pty_synthesis` provenance — synthesized intelligence renders without the tag by default. Document this explicitly.

## Tokens consumed

- `--font-mono` (label, lowercase or small-caps)
- `--color-text-tertiary` (default)
- `--color-text-secondary` (hover/discrepancy emphasis)
- `--space-xs`

## API sketch

```tsx
<ProvenanceTag itemSource="glean" />
<ProvenanceTag itemSource="salesforce" discrepancy />
{/* Renders nothing for synthesized: */}
<ProvenanceTag itemSource="pty_synthesis" />
```

## Source

- **Code:** `src/components/ui/ProvenanceTag.tsx`
- **Existing usage:** AccountDetail, briefing, intelligence-rendering surfaces

## Surfaces that consume it

Audit 04 surfaced: AccountDetail (claim attribution), DailyBriefing, ProjectDetail, PersonDetail. Foundational for `TrustBand` and `ClaimRow` patterns (Wave 2).

## Naming notes

Existing primitive in `src/`. The mockup substrate proposes a related `ProvenancePill` — that's just the visual treatment if implemented as a pill; keep the canonical name `ProvenanceTag` to match what surface code already imports.

## History

- 2026-05-02 — Documented as canonical (existing src/ primitive).
- Audit 04 — confirmed pty_synthesis suppression behavior.
