# AboutThisIntelligencePanel

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `AboutThisIntelligencePanel`
**`data-ds-spec`:** `patterns/AboutThisIntelligencePanel.md`
**Variants:** `tone="neutral" | "caution" | "incomplete"`; `density="default" | "compact"`
**Design system version introduced:** 0.2.0

## Job

Explain what made a chapter's intelligence trustworthy, current, or incomplete. The panel gives users a surface-level answer to what sources contributed, when the chapter was last enriched, what is missing, and why the trust level is what it is.

## When to use it

- At the top or bottom of a DailyBriefing chapter when the chapter contains generated intelligence.
- When a user needs chapter-level trust context without opening every claim receipt.
- When the system has meaningful source coverage, enrichment, or gap metadata to summarize.
- When consolidating existing health and context explanations into a single trust vocabulary.

## When NOT to use it

- For a single claim's provenance or trust math — use `TrustBand` or `ReceiptCallout`.
- For dossier-wide source coverage across an account, project, or person — use `DossierSourceCoveragePanel`.
- For a stale generated report warning with a regeneration action — use `StaleReportBanner`.

## Composition

Composes trust primitives in a short explanatory stack:

```
[TrustBandBadge]  [AsOfTimestamp]
[SourceCoverageLine list]
[DataGapNotice list, if missing inputs]
```

`TrustBandBadge` leads with the current trust judgment. `AsOfTimestamp` explains freshness. `SourceCoverageLine` lists contributing sources in the order users expect. `DataGapNotice` calls out missing data only when it changes how the chapter should be read.

## States / variants

- **neutral** — sources and enrichment are present; no blocking gaps.
- **caution** — chapter is usable but has stale or partial source metadata.
- **incomplete** — key source metadata is missing; gaps are visually emphasized.
- **compact** — omits explanatory body copy and keeps source lines to labels plus status.
- **loading** — skeleton rows reserve space for source lines and timestamp.
- **empty** — hidden unless the surface explicitly needs to explain that no enrichment has run.

## Tokens consumed

- `--color-text-primary` — panel heading and trust summary.
- `--color-text-secondary` — explanation text and source line labels.
- `--color-text-tertiary` — timestamp and secondary metadata.
- `--color-rule-light` — panel border and row dividers.
- Trust band colors inherited through `TrustBandBadge`.
- `--space-sm`, `--space-md`, `--space-lg` — internal gaps and padding.
- `--font-mono` — source names, timestamps, and metadata fragments.

## API sketch

```tsx
<AboutThisIntelligencePanel
  title="About this intelligence"
  trustBand="use_with_caution"
  trustReason="Salesforce is current, but meeting notes are incomplete."
  sources={[
    { source: "salesforce", status: "covered", detail: "Opportunity and account fields" },
    { source: "glean", status: "partial", detail: "3 related docs" },
  ]}
  gaps={[
    { code: "missing_recent_meeting", label: "No characterized meeting notes this week" },
  ]}
  enrichedAt="2026-05-02T08:00:00Z"
  density="default"
/>
```

## Source

- **Code:** `src/components/health/AboutIntelligence.tsx`
- **Code:** `src/components/context/AboutThisDossier.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Note:** Wave 2 follow-on for v1.4.4

## Surfaces that consume it

DailyBriefing chapter callouts, AccountDetail/ProjectDetail/PersonDetail intelligence panels, MeetingDetail findings sections.

## Naming notes

`AboutThisIntelligencePanel` is the canonical chapter-level explanation pattern. Current code names `AboutIntelligence` and `AboutThisDossier` should consolidate into this only when the scope is a chapter or bounded intelligence block, not an entire dossier.

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
