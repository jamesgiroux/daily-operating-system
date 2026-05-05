# DailyBriefingAttentionSection

**Tier:** pattern
**Status:** shipped-local/extraction-needed
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `DailyBriefingAttentionSection`
**`data-ds-spec`:** `patterns/DailyBriefingAttentionSection.md`
**Variants:** action priority; priority email; lifecycle update
**Design system version introduced:** 0.5.0

## Job

Collect the real shipped "needs attention" rows in DailyBriefing: prioritized actions, priority email prompts, and lifecycle updates.

## Source

- **Code:** local components inside `src/components/dashboard/DailyBriefing.tsx`
- **Styles:** `src/styles/editorial-briefing.module.css`
- **Extraction note:** this is real shipped UI but not yet a named exported component.

## Surfaces that consume it

DailyBriefing.

