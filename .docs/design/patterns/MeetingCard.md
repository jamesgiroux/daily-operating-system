# MeetingCard

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `MeetingCard`
**`data-ds-spec`:** `patterns/MeetingCard.md`
**Variants:** customer/internal/personal/1:1 accents; upcoming/in-progress/past; navigable/clickable
**Design system version introduced:** 0.5.0

## Job

Render a meeting as an editorial schedule row with time, duration, title, entity byline, temporal state, and optional intelligence quality.

## Composition

Composes `MeetingStatusPill`, `IntelligenceQualityBadge`, title/subtitle slots, type accents, and an optional child slot.

## Source

- **Code:** `src/components/shared/MeetingCard.tsx`
- **Styles:** `src/components/shared/MeetingCard.module.css`

## Surfaces that consume it

DailyBriefing through `BriefingMeetingCard`, and WeekPage timeline/direct meeting lists.

