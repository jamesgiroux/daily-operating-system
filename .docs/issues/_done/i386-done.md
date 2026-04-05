# I386 — Calendar Lifecycle Gaps — Future Meeting Cancellation Detection, Rescheduling Sync, Continuous Future Polling

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Backend / Calendar

## Summary

Three gaps exist in how the calendar poller handles future meeting lifecycle changes:

**Gap 1:** Cancellation detection only covers today. `detect_cancelled_meetings()` runs after each poll but only compares today's DB meetings against today's poll results. A meeting cancelled for next week persists on the timeline indefinitely until the day arrives.

**Gap 2:** The poller only fetches today. Future meetings only enter `meetings_history` when the user visits the Weekly Forecast page (live-fetch) or when `prepare_week` runs. A meeting added for next Wednesday won't appear between page visits.

**Gap 3:** No rescheduling detection for future meetings. `get_meeting_timeline` skips events whose `calendar_event_id` already exists in the DB, so a rescheduled future meeting shows stale time/title until the day-of poller catches it.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Google Calendar data in the running app:

1. The calendar poller fetches **today through +7 days** on each cycle (not just today). New meetings added to the calendar for the coming week appear in the timeline within one poll interval (default 5 min) — without requiring a page visit.
2. `detect_cancelled_meetings()` runs against the **full ±7 day range**, not just today. Cancel a meeting scheduled for next Tuesday in Google Calendar. Within one poll cycle, the meeting disappears from the Weekly Forecast timeline (archived state).
3. Reschedule a future meeting in Google Calendar (change its time by 1 hour). Within one poll cycle, the timeline shows the **updated time**, not the stale original. Verify `meetings_history.start_time` and `end_time` match the Google Calendar values.
4. Rename a future meeting in Google Calendar. Within one poll cycle, the timeline shows the **updated title**. Verify `meetings_history.title` matches the new Google Calendar title.
5. The expanded poll range does not cause duplicate meetings. Verify: `SELECT id, COUNT(*) FROM meetings_history GROUP BY calendar_event_id HAVING COUNT(*) > 1` returns 0 rows.
6. Meeting prep regeneration fires when a meeting's entity links change due to reclassification after a title change. If "Team Sync" is renamed to "Acme QBR", the meeting should be reclassified and re-linked to the Acme account, and its prep should regenerate.

## Dependencies

- Independent — backend calendar poller changes, no dependencies on other v0.13.1 issues.

## Notes / Rationale

The calendar is the ground truth for the user's schedule. When it changes — a cancellation, a reschedule, a rename — DailyOS should reflect that change within the next poll cycle, not on the day of the meeting. The three gaps combined mean a user planning their week using DailyOS may be working from stale data for days before the day-of poller corrects it.
