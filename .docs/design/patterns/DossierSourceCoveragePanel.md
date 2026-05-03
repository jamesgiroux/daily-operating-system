# DossierSourceCoveragePanel

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `DossierSourceCoveragePanel`
**`data-ds-spec`:** `patterns/DossierSourceCoveragePanel.md`
**Variants:** `dossierType="account" | "project" | "person"`; `density="default" | "compact"`
**Design system version introduced:** 0.2.0

## Job

Explain dossier-level source coverage for an account, project, or person. The panel shows which systems contributed, what important data gaps remain, and when the dossier was last enriched.

## When to use it

- On AccountDetail, ProjectDetail, or PersonDetail when users need to understand source coverage for the whole dossier.
- When source availability changes how confidently users should interpret the dossier.
- When the surface needs to show data capture gaps such as uncharacterized stakeholders or missing meetings.
- When last enrichment time is meaningful for the dossier's currentness.

## When NOT to use it

- For a chapter-scoped explanation — use `AboutThisIntelligencePanel`.
- For a per-claim inspection flow — use `ReceiptCallout`.
- For inline claim trust summaries — use `TrustBand` inside `ClaimRow`.

## Composition

Composes:

```
[AsOfTimestamp]
[SourceCoverageLine]
[SourceCoverageLine]
[DataGapNotice, if any]
```

`AsOfTimestamp` anchors the panel with last enrichment. Each `SourceCoverageLine` represents one source and its coverage state. `DataGapNotice` renders below source lines so gaps read as consequences of coverage, not separate alerts.

## States / variants

- **account** — emphasizes CRM, meetings, stakeholders, and account context sources.
- **project** — emphasizes project status, owners, linked accounts, and work context.
- **person** — emphasizes stakeholder details, interaction history, and role confidence.
- **default** — full source labels, descriptions, gap details, and timestamp.
- **compact** — source labels and status only; used in narrow side panels.
- **no-gaps** — hides the gap region and tightens the bottom spacing.
- **loading** — skeleton source lines and timestamp.

## Tokens consumed

- `--color-text-primary` — panel title and source names.
- `--color-text-secondary` — coverage descriptions.
- `--color-text-tertiary` — timestamp and muted metadata.
- `--color-rule-light` — source row dividers.
- `--color-trust-likely-current` — covered source status.
- `--color-trust-use-with-caution` — partial source status.
- `--color-trust-needs-verification` — missing or verification-needed status.
- `--space-xs`, `--space-sm`, `--space-md` — row gaps and panel rhythm.
- `--font-mono` — source system labels and timestamps.

## API sketch

```tsx
<DossierSourceCoveragePanel
  dossierType="account"
  enrichedAt="2026-05-02T08:00:00Z"
  sources={[
    { source: "salesforce", status: "covered", label: "CRM fields" },
    { source: "meetings", status: "partial", label: "Recent meeting notes" },
    { source: "glean", status: "missing", label: "Related docs" },
  ]}
  gaps={[
    { code: "uncharacterized_stakeholders", label: "2 stakeholders need characterization" },
  ]}
  density="default"
/>
```

## Source

- **Code:** `src/components/context/AboutThisDossier.tsx`
- **Mockup origin:** `.docs/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Note:** Wave 2 follow-on for v1.4.4

## Surfaces that consume it

DailyBriefing chapter callouts, AccountDetail/ProjectDetail/PersonDetail intelligence panels, MeetingDetail findings sections.

## Naming notes

`DossierSourceCoveragePanel` is the canonical dossier-level explanation pattern. Keep `Dossier` in the name to distinguish it from chapter-level `AboutThisIntelligencePanel` and per-claim `ReceiptCallout`.

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
