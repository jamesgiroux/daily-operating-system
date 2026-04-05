# ADR-0061: Calendar Event ID as Meeting Primary Key

**Date:** 2026-02-10
**Status:** Accepted
**Deciders:** James, Claude

## Context

Meetings are identified throughout the system by a derived slug: `make_meeting_id(title, start_time, type)` → e.g., `1030-customer-acme-weekly-sync`. This slug is:

- **Fragile.** A title change ("Acme Sync" → "Acme Weekly Sync") produces a different ID. Timezone handling differences between prepare and deliver produce different IDs. The same meeting generates different IDs in `deliver_schedule` vs `deliver_preps` when event-ID matching fails in the latter.
- **Ambiguous.** Two meetings with the same title at the same time (unlikely but not impossible) collide.
- **Disconnected from Google Calendar.** The `calendar_event_id` column already exists in `meetings_history` but is treated as optional metadata, not as the authoritative key.

Meanwhile, Google Calendar event IDs are:
- **Stable across renames and reschedules.** The ID persists even when the meeting title, time, or attendees change.
- **Unique per calendar.** No collision risk.
- **Already flowing through the system.** `CalendarEvent.id`, `DirectiveMeeting.event_id`, `calendarEventId` in schedule.json and prep files — the data is there, just not authoritative.

The current slug-based approach is actively causing bugs:
1. `deliver_schedule` sets `hasPrep: true` and generates a prep filename using one code path. `deliver_preps` generates the actual file using a different code path. When the derived meeting IDs diverge, the "View Prep" button links to a file that doesn't exist.
2. Prep files break when meetings are renamed in Google Calendar (new slug, old file orphaned).
3. The `meetings_history` table accumulates duplicate rows for the same meeting when its title or start time shifts between polls.

## Decision

**Use Google Calendar event IDs as the primary key for meetings throughout the system.**

### Scope

1. **Prep file naming.** `preps/{calendar_event_id}.json` replaces `preps/{make_meeting_id(...)}.json`. Calendar event IDs are safe for filenames (alphanumeric + underscores).

2. **`meetings_history.id`.** Use calendar event ID as the primary key. The old slug becomes a display-only `slug` field for URL readability (optional, not authoritative).

3. **Schedule JSON.** `meeting.id` becomes the calendar event ID. The `calendarEventId` field becomes redundant — keep it during migration for backwards compatibility, then remove.

4. **Route params.** `/meeting/$prepFile` already accepts the prep filename. No route change needed — just the filename content changes from slug to event ID.

5. **`make_meeting_id` retained for non-calendar meetings.** Manually created meetings or imported meetings without calendar IDs still need a derived ID. `make_meeting_id` becomes the fallback, not the default.

6. **`ensure_meeting_in_history` and `record_meeting_attendance`** (I160) already work with string IDs — the switch is transparent to the attendance pipeline.

### What does NOT change

- **Meeting classification.** Still based on title + attendees + domain signals.
- **Meeting context gathering.** Still matches by account name / event ID.
- **Entity intelligence.** Consumes meetings by account, not by meeting ID.
- **Archive reconciliation.** Already uses calendar event ID for matching.

## Consequences

**Easier:**
- Prep file lookup is deterministic — `deliver_schedule` and `deliver_preps` always agree on the filename because they use the same event ID, not a derived slug.
- Meeting history is stable across renames and reschedules — no more duplicate rows.
- `record_meeting_attendance` (I160) works correctly because the meeting ID used during calendar polling is the same ID used during delivery.
- Future features (meeting series tracking, recurring meeting intelligence) are straightforward since the event ID is stable.

**Harder:**
- Meetings without Google Calendar events (manually created, imported from other sources) need the fallback `make_meeting_id` path. Two ID schemes must coexist.
- Migration: existing `meetings_history` rows have slug-based IDs. Need a migration that re-keys rows using `calendar_event_id` where available, keeps slug for rows without one.
- Existing prep files on disk use slug names. First delivery after migration will write new-named files and stale cleanup will remove old ones — clean but users lose "reviewed" state on existing preps.

**Trade-offs:**
- Accepting a dependency on Google Calendar's ID scheme. If a user switches calendar providers, their meeting history keys change. Acceptable for MVP (Google-only, A4).
