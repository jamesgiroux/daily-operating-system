# DailyBriefingAttentionSection

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `DailyBriefingAttentionSection`
**`data-ds-spec`:** `patterns/DailyBriefingAttentionSection.md`
**Variants:** lifecycle confirmation; prioritized action; raw action; scored email; aging notice; view-all links
**Design system version introduced:** 0.5.0

## Job

Collect the real shipped "needs attention" rows in DailyBriefing: lifecycle updates that need confirmation/correction, prioritized or raw action rows, scored email intelligence rows, aging notices, and links out to the full Actions/Emails surfaces.

Suggested actions are intentionally not part of this pattern anymore. They were removed from DailyBriefing and now live on `/actions`.

## Source

- **Code:** local components inside `src/components/dashboard/DailyBriefing.tsx`
- **Styles:** `src/styles/editorial-briefing.module.css` plus `src/components/dashboard/DailyBriefing.module.css`
- **Extraction note:** this is real shipped UI but not yet a named exported component.

## Surfaces that consume it

DailyBriefing.
