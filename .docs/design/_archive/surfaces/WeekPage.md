# WeekPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `WeekPage`
**`data-ds-spec`:** `surfaces/WeekPage.md`
**Source files:**
- `src/pages/WeekPage.tsx`
- `src/pages/WeekPage.module.css`

## Job

WeekPage is the weekly planning and review surface. It gives the user a scan of meetings, preparation state, movement, and work across the current week.

## Layout regions

1. Folio chrome with week label and readiness counters.
2. Week header and summary stats.
3. Day-by-day timeline.
4. Meeting and action clusters.
5. Empty and loading states for quiet weeks or unavailable schedule data.

## Patterns and primitives

Consumes `ChapterHeading`, timeline rows, readiness/status badges, meeting links, and action metadata. Promote timeline subpatterns only after reuse outside WeekPage.

## States

Supports loading, empty week, current-day highlight, meetings needing prep, complete prep, overdue action, and filtered/no-match states.
