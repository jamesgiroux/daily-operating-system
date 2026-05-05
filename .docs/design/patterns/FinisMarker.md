# FinisMarker

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `FinisMarker`
**`data-ds-spec`:** `patterns/FinisMarker.md`
**Variants:** mark only; mark plus `enrichedAt`
**Design system version introduced:** 0.5.0

## Job

End an editorial surface with the DailyOS three-mark finis and optional last-updated text. It makes the page finite.

## Source

- **Code:** `src/components/editorial/FinisMarker.tsx`
- **Extraction note:** source currently uses inline layout styles and `BrandMark`; CSS extraction is a cleanup target, not a blocker for documenting the shipped pattern.

## Surfaces that consume it

DailyBriefing, MeetingDetail, Settings, Actions, reports, entity pages, and other editorial surfaces.

