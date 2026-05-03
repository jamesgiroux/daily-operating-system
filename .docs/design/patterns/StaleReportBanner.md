# StaleReportBanner

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `StaleReportBanner`
**`data-ds-spec`:** `patterns/StaleReportBanner.md`
**Variants:** `severity="info" | "warning"`; `action="regenerate" | "refresh"`
**Design system version introduced:** 0.2.0

## Job

Warn users when a generated report is based on intelligence that has changed or gone stale since the report was created. The banner explains the stale condition and gives a clear regeneration affordance.

## When to use it

- Above generated reports when underlying intelligence is newer than the report.
- When a report was generated before the latest enrichment or source refresh.
- When the user can regenerate or refresh the report in place.
- When the stale state affects interpretation but does not block reading the report.

## When NOT to use it

- For claim-level freshness — use `FreshnessIndicator` inside `TrustBand`.
- For chapter-level source and gap explanation — use `AboutThisIntelligencePanel`.
- For dossier-level coverage gaps — use `DossierSourceCoveragePanel`.

## Composition

Composes:

```
[FreshnessIndicator] [AsOfTimestamp] [Button action]
```

`FreshnessIndicator` communicates stale status. `AsOfTimestamp` compares report generation time against latest enrichment. The action button is required when regeneration is available and optional only for read-only historical reports.

## States / variants

- **info** — intelligence changed but the report remains broadly usable.
- **warning** — source data is stale enough that report conclusions may be outdated.
- **regenerate** — primary action requests a new generated report.
- **refresh** — secondary action refreshes report context before generation.
- **loading** — action shows pending state and disables repeated submission.
- **error** — inline failure text appears after a failed regeneration attempt.
- **dismissed** — allowed only per session; stale state remains visible on reload.

## Tokens consumed

- `--color-text-primary` — banner message.
- `--color-text-secondary` — timestamp explanation.
- `--color-text-tertiary` — supporting metadata.
- `--color-rule-light` — banner border.
- `--color-trust-use-with-caution` — info stale accent.
- `--color-trust-needs-verification` — warning stale accent.
- `--space-sm`, `--space-md` — icon, copy, and action gaps.
- `--font-mono` — generated-at and enriched-at timestamps.

## API sketch

```tsx
<StaleReportBanner
  reportGeneratedAt="2026-05-01T14:30:00Z"
  latestEnrichedAt="2026-05-02T08:00:00Z"
  severity="warning"
  action="regenerate"
  onRegenerate={() => generateReport()}
  isRegenerating={false}
/>
```

## Source

- **Code:** `src/components/reports/ReportShell.tsx`
- **Mockup origin:** `.docs/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Note:** Wave 2 follow-on for v1.4.4

## Surfaces that consume it

DailyBriefing chapter callouts, AccountDetail/ProjectDetail/PersonDetail intelligence panels, MeetingDetail findings sections.

## Naming notes

`StaleReportBanner` is report-scoped and action-oriented. Do not rename to `FreshnessBanner`; that overlaps with general freshness primitives and loses the generated-report contract.

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
