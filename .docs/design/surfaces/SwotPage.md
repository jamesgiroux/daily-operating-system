# SwotPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `SwotPage`
**`data-ds-spec`:** `surfaces/SwotPage.md`
**Source files:**
- `src/pages/SwotPage.tsx`
- `src/pages/report-slides.module.css`

## Job

SwotPage renders an account SWOT analysis deck. It organizes strengths, weaknesses, opportunities, threats, and the summary into an editable strategy artifact.

## Layout regions

1. Report slide shell with SWOT title and regeneration controls.
2. Cover and summary.
3. Strengths and weaknesses quadrants.
4. Opportunities and threats quadrants.
5. Source-backed observations and edit/save affordances.

## Patterns and primitives

Consumes the shared report-slide module, SWOT slide components, editable report text, feedback controls, `GeneratingProgress`, and `FinisMarker`.

## States

Supports loading, generating, cached report, stale report, empty SWOT sections, parse error, regeneration, save feedback, and completed report states.
