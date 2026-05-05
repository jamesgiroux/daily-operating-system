# WeeklyImpactPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `WeeklyImpactPage`
**`data-ds-spec`:** `surfaces/WeeklyImpactPage.md`
**Source files:**
- `src/pages/WeeklyImpactPage.tsx`
- `src/pages/report-slides.module.css`

## Job

WeeklyImpactPage renders the user's weekly impact report. It summarizes completed work, relationship movement, risks reduced, and follow-up momentum.

## Layout regions

1. Report slide shell.
2. Weekly headline and impact summary.
3. Outcomes, meetings, actions, and relationship sections.
4. Evidence/freshness callouts.
5. Export/share affordances where available.

## Patterns and primitives

Consumes the shared report-slide module, report title/section patterns, trust/evidence primitives, and weekly rollup vocabulary.

## States

Supports loading/generating, empty week, stale report, regeneration, error, and completed report states.
