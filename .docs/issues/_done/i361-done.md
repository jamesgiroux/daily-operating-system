# I361 — Timeline Meeting Filtering — Skip Personal/Focus/Blocked Types

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend

## Summary

The weekly forecast timeline was showing all calendar events including personal events (dentist appointments, gym blocks, personal reminders) and focus/blocked time. These are not work meetings and have no intelligence to show. Including them created visual noise and could surface personal information the user doesn't want visible in a work-focused briefing. This issue added backend filtering to exclude personal, focus, and blocked event types from the timeline.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open weekly forecast. No personal events appear in the timeline. Verify by checking against Google Calendar — any event classified as `personal` (dentist appointments, personal blocks, etc.) should be absent from the timeline.
2. Focus blocks and blocked time do not appear in the timeline.
3. Internal meetings, team syncs, 1:1s, all-hands, and all external meeting types continue to appear.
4. The daily briefing schedule section shows the same set of meetings (minus personal) — verify both pages show consistent results for today's meetings.

## Dependencies

- Related to meeting type classification system.
- I363 (timeline data enrichment) and I364 (timeline adoption) build on this filtering.

## Notes / Rationale

The filtering rule is intentionally conservative: exclude only types that are never work-related (personal, focus/blocked). Every meeting type that could involve another person or represent a work commitment is included. The calendar is the user's record of their time; DailyOS's job is to surface the work-relevant portion.
