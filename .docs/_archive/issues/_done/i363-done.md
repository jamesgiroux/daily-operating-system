# I363 — Timeline Data Enrichment — Display Time + Meeting Type on TimelineMeeting

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend / Types

## Summary

`TimelineMeeting` rows in the weekly forecast were missing formatted start time and duration — meetings appeared without time context, making it impossible to see at a glance when they were scheduled. This issue added formatted time and duration to every meeting in the weekly forecast timeline. The time format was standardized to match the daily briefing's display format exactly.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Every meeting in the weekly forecast timeline displays a formatted time (e.g., "9:30 AM").
2. Meetings with a known end time display duration (e.g., "30m", "1h").
3. The time format matches the daily briefing's time display format exactly.

## Dependencies

- Foundational for I364 (weekly forecast timeline adoption) and I362 (shared meeting card).
- Depends on meeting data having `start_time` and `end_time` fields populated from the calendar poller.

## Notes / Rationale

A meeting card without a time is barely useful — time context is the most fundamental piece of information about a scheduled event. The data existed in the DB; it just wasn't being passed through the `TimelineMeeting` type to the component. This was a type and data wiring fix more than a new feature.
