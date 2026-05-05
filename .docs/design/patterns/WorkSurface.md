# WorkSurface

**Tier:** pattern
**Status:** canonical/shipped
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `WorkSurface`
**`data-ds-spec`:** `patterns/WorkSurface.md`
**Variants:** commitment card; suggestion card; program pill; shared reference row; recently-landed row; report card; exported focus/nudge rows
**Design system version introduced:** 0.5.0

## Job

Render the AccountDetail Work tab vocabulary currently consumed by the shipped account detail page: commitments, suggestions, programs, shared tracker references, recently-landed work, and report outputs.

`NumberedFocusList` and `NudgeRow` remain exported from `WorkSurface.tsx`, but the active AccountDetail Work view no longer renders the focus or nudges chapters.

## Source

- **Code:** `src/components/work/WorkSurface.tsx`
- **Styles:** `src/components/work/WorkSurface.module.css`

## Surfaces that consume it

AccountDetailPage.
