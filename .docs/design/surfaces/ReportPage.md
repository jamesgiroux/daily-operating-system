# ReportPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ReportPage`
**`data-ds-spec`:** `surfaces/ReportPage.md`
**Source files:**
- `src/pages/ReportPage.tsx`
- `src/pages/report-page.module.css`
- `src/components/reports/ReportShell.tsx`
- `src/components/reports/report-shell.module.css`
**Routes:** `/accounts/$accountId/reports/$reportType`, `/me/reports/$reportType`

## Job

ReportPage is the generic report route renderer. It resolves the scoped entity and report type, fetches the stored report, delegates named reports to specialized renderers, and falls back to a JSON/preformatted view for unknown report types.

## Layout Regions

1. Folio chrome derived from entity type and report title.
2. Loading and error states from `ReportPage`.
3. `ReportShell` stale banner and generation error banner.
4. Empty state with generate action when no report exists.
5. Report content area: specialized renderer or generic JSON fallback.
6. Parse-error state for invalid stored content.
7. Footer with generated date, regenerate, and export PDF actions.

## Patterns And Primitives

Consumes `ReportShell`, report-surface globals, report-page fallback states, and button controls. Named report routes continue to use dedicated references for their full deck layouts.

## States

Supports loading, fetch error, no report, generating handoff, stale report, generation error, generic JSON fallback, parse error, generated report footer, account-scoped breadcrumbs, project/person-scoped breadcrumbs, and user-scoped breadcrumbs.

