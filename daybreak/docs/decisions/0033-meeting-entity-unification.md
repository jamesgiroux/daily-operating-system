# ADR-0033: Meeting entity unification

**Date:** 2026-02
**Status:** Accepted

## Context

Meetings exist in three independent forms with no shared state:
- Daily dashboard card (`schedule.json`) — ID is a local slug, has enrichment, no state tracking
- Weekly grid cell (`week-overview.json`) — no ID at all, decorative prep badge (always "prep_needed")
- Meeting detail page (`preps/*.json`) — has `meetingId` field but no validation against schedule, orphaned (reachable only if `prepFile` exists on the dashboard card)

No shared lifecycle. Marking prep as "reviewed" on daily view has no effect on weekly view. MeetingCard links to detail only when `meeting.prepFile` is set. The Week page has no links to detail pages at all.

SQLite has a `meeting_history` table and a `captures` table (post-meeting wins/risks/actions) but no prep state tracking.

## Decision

Near-term: Shared ID mapping. Keep independent data sources but add a SQLite lookup table mapping calendar event IDs across views. A "prep reviewed" flag in SQLite can be queried by any view.

Long-term (Phase 3): Unified Meeting entity in SQLite with stable ID (calendar event ID), state (prep status, notes, outcomes), and lifecycle. All views read from the same source.

## Consequences

**Near-term (implemented):**
- `meeting_prep_state` SQLite table tracks which preps the user has reviewed, keyed by `prep_file` with optional `calendar_event_id`
- `get_meeting_prep` command records review on load; `get_dashboard_data` annotates meetings with `prep_reviewed: true`
- MeetingCard shows a checkmark icon when prep has been reviewed
- `meetings_history` table gains `calendar_event_id` column for cross-source matching
- Works without Google Calendar (keyed by prep_file, not event ID)
- I14 (meeting card links) was already resolved in Phase 1.5

**Long-term (Phase 3):**
- Unified Meeting entity in SQLite with stable ID, state, and lifecycle
- All views read from the same source — significant architectural change
- Depends on ADR-0032 (calendar source of truth determines the ID scheme)
