# PostMeetingIntelligence

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `PostMeetingIntelligence`
**`data-ds-spec`:** `patterns/PostMeetingIntelligence.md`
**Variants:** summary; threads; predictions; conversation; findings; commitments; role changes
**Design system version introduced:** 0.5.0

## Job

Render the actual shipped post-meeting intelligence system inside MeetingDetail. This is the source-of-truth umbrella for agenda threads, predictions vs reality, signal grids, escalation quotes, findings, champion health, commitments, and role transitions.

## Composition

Composes local `PostMeetingIntelligence.module.css` class families, `TalkBalanceBar`, and `IntelligenceFeedback`.

## Source

- **Code:** `src/components/meeting/PostMeetingIntelligence.tsx`
- **Styles:** `src/components/meeting/PostMeetingIntelligence.module.css`

## Surfaces that consume it

MeetingDetailPage.

