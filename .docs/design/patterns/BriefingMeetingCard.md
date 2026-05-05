# BriefingMeetingCard

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `BriefingMeetingCard`
**`data-ds-spec`:** `patterns/BriefingMeetingCard.md`
**Variants:** up next; in progress; past; cancelled; empty prep
**Design system version introduced:** 0.5.0

## Job

Render the actual shipped DailyBriefing schedule item: a `MeetingCard` row that can expand inline into meeting context, the room, prep prompts, and before-meeting actions.

## Composition

Composes `MeetingCard`, `Pill`, `KeyPeopleFlow`, `PrepGrid`, and `MeetingActionChecklist` using the shipped `editorial-briefing.module.css` class family.

## Source

- **Code:** `src/components/dashboard/BriefingMeetingCard.tsx`
- **Styles:** `src/styles/editorial-briefing.module.css`

## Surfaces that consume it

DailyBriefing.

