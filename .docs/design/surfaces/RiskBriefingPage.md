# RiskBriefingPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `RiskBriefingPage`
**`data-ds-spec`:** `surfaces/RiskBriefingPage.md`
**Source files:**
- `src/pages/RiskBriefingPage.tsx`
- `src/pages/report-slides.module.css`

## Job

RiskBriefingPage renders an executive risk briefing for an account. It translates risk signals into a situation narrative, stakes, recovery plan, and specific asks.

## Layout regions

1. Report slide shell with risk briefing title and regeneration controls.
2. Cover and bottom-line summary.
3. What-happened narrative.
4. Stakes and exposure framing.
5. Recovery plan and ask slides.

## Patterns and primitives

Consumes the shared report-slide module, risk-briefing slide components, feedback controls, `GeneratingProgress`, report freshness/staleness patterns, and `FinisMarker`.

## States

Supports loading, generating, cached report, stale report, live generation events, parse error, regeneration, save feedback, and completed report states.
