# BriefingMeetingCard

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `BriefingMeetingCard`
**`data-ds-spec`:** `patterns/BriefingMeetingCard.md`
**Variants:** up next expanded; in-progress expanded; collapsed upcoming; past summary; cancelled; empty prep
**Design system version introduced:** 0.5.0

## Job

Render the actual shipped DailyBriefing schedule item: a `MeetingCard` row that can expand inline into meeting context, the room, prep prompts, before-meeting actions, and a bridge into the meeting detail page.

This is the expanded meeting-card contract for DailyBriefing. The shared `MeetingCard` row only supplies the row shell and slots; `BriefingMeetingCard` owns expansion state, measured `maxHeight`, empty-prep handling, and past/cancelled behavior.

## Composition

Non-cancelled rows compose `MeetingCard`, `Pill`, `KeyPeopleFlow`, `PrepGrid`, `MeetingActionChecklist`, the expansion panel, and bridge links using the shipped `editorial-briefing.module.css` class family. Cancelled rows are the exception: they render the legacy `scheduleRow` fallback directly because they do not expand or navigate.

## Source

- **Code:** `src/components/dashboard/BriefingMeetingCard.tsx`
- **Styles:** `src/styles/editorial-briefing.module.css`

## Surfaces that consume it

DailyBriefing.
