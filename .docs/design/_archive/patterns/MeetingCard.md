# MeetingCard

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `MeetingCard`
**`data-ds-spec`:** `patterns/MeetingCard.md`
**Variants:** customer/internal/personal/1:1 accents; upcoming/in-progress/past; navigable/clickable; subtitle extras; child content
**Design system version introduced:** 0.5.0

## Job

Render a meeting as an editorial schedule row with time, duration, title, entity byline, temporal state, optional intelligence quality, and caller-supplied slot content.

`MeetingCard` does **not** own meeting-detail depth. Routed meeting context belongs to `MeetingDetailPage`; this pattern remains a compact schedule/timeline row.

## Composition

Composes `MeetingStatusPill`, `IntelligenceQualityBadge`, title/subtitle slots, type accents, and an optional child slot. WeekPage uses the subtitle and child slots for health, "No prep", days-until, outcome summaries, and follow-up counts.

## Source

- **Code:** `src/components/shared/MeetingCard.tsx`
- **Styles:** `src/components/shared/MeetingCard.module.css`

## Surfaces that consume it

WeekPage timeline/direct meeting lists.
