# I401 — Show Internal Attendees for Internal Meetings

**Status:** Done (v0.13.6)
**Priority:** P2
**Version:** 0.13.6
**Area:** Backend / UX

## Summary

`hydrate_attendee_context` in `services/meetings.rs` filters out all people with `relationship = "internal"`. For external customer meetings this is correct (internal colleagues are noise in "The Room"). For internal team meetings (team_sync, internal, one_on_one with internal attendees), this empties the attendee list entirely — "The Room" shows nobody even though 4-6 people accepted.

## Acceptance Criteria

1. Internal meetings (meeting_type = team_sync, internal, or one_on_one where all attendees are internal) show their attendees in "The Room" on the meeting detail page.
2. External meetings continue to filter out internal attendees.
3. The determination should use meeting_type, not a heuristic on attendee domains.

## Resolution

Shipped in v0.13.6. `services/meetings.rs:57-71` — `matches!(meeting.meeting_type.as_str(), "team_sync" | "internal" | "one_on_one")` gates the internal filter. Internal meetings return all contexts; external meetings filter out `relationship = "internal"`.
