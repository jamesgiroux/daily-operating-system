# EbrQbrPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `EbrQbrPage`
**`data-ds-spec`:** `surfaces/EbrQbrPage.md`
**Source files:**
- `src/pages/EbrQbrPage.tsx`
- `src/pages/report-slides.module.css`

## Job

EbrQbrPage renders an executive/business review report for an account. It turns account context into a structured narrative deck with source-backed claims.

## Layout regions

1. Report slide shell.
2. Executive headline and account framing.
3. Health, risk, movement, and recommendation sections.
4. Evidence, caveats, and source/freshness callouts.
5. Export/share affordances where available.

## Patterns and primitives

Consumes the shared report-slide module, report title/section patterns, trust/evidence primitives, and account-health report vocabulary.

## States

Supports loading/generating, stale report, regeneration, error, empty inputs, and completed report states.
