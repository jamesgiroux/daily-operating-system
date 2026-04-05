# I371 — Meeting Email Context Rendering — Wire recentEmailSignals to Meeting Detail UI

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Frontend / UX

## Summary

Meeting prep data (`FullMeetingPrep`) already contains a `recentEmailSignals` field populated by the backend with email signals from the meeting's attendees. This field is currently not rendered in the meeting detail UI — it exists in the data but has no frontend consumer. This issue wires `recentEmailSignals` to a "Recent Correspondence" section on the meeting detail page, showing contextual email summaries from attendees rather than raw email rows.

This is a frontend rendering task — the data already flows from the backend; it just needs to be displayed.

## Acceptance Criteria

From the v0.13.1 brief, verified with real data in the running app:

1. Open a meeting detail page for a meeting with attendees who have recent email activity.
2. A "Recent Correspondence" (or equivalent) section appears showing email signals from those attendees — contextual summaries, not raw email rows.
3. The data comes from `FullMeetingPrep.recentEmailSignals` (already populated by the backend). This is a frontend rendering task.
4. If no email signals exist for the meeting's attendees, the section does not render (no empty state).
5. Clicking an email signal in this section navigates to the email page, not a dead link.

## Dependencies

- The `recentEmailSignals` field exists in the backend type and is populated. This is purely a frontend wiring task.
- Benefits significantly from I369 (contextual synthesis) — the summaries shown will be contextual rather than raw if I369 ships first.

## Notes / Rationale

`recentEmailSignals` was a case of data flowing through the system but not surfacing to the user — the backend was doing the work but the frontend wasn't rendering the output. The meeting detail is the right place for this: before a meeting with Jack, seeing "Jack confirmed the agenda this morning" directly on the meeting detail page is exactly what a chief-of-staff would surface.
