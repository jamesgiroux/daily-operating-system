# ADR-0032: Calendar source of truth: hybrid overlay

**Date:** 2026-02
**Status:** Accepted

## Context

The app has two calendar data sources that can disagree:
- `schedule.json`: Generated at briefing time (6 AM). Stale within hours. Has AI enrichment (prep, classification, context).
- `calendar_events` (AppState): Polled every 5 minutes via `calendar_poll.py`. Near-real-time. No enrichment.

A meeting cancelled at 7 AM still shows on the dashboard. A meeting added at 2 PM shows nowhere — the dashboard only reads `schedule.json`.

**Event ID status:** Google Calendar event IDs are preserved through the full pipeline — `prepare_today.py`, `deliver_today.py` (schedule.json + preps/*.json), and `calendar_poll.py`. Both data sources can be matched by event ID (resolved by I24).

## Decision

Hybrid overlay: Live calendar is source of truth for *which meetings exist*. Briefing enrichment (prep, classification, context) is overlaid onto live events by matching on calendar event ID.

- Meeting in briefing but not live → cancelled, hide or grey out
- Meeting in live but not briefing → new, show bare with "No prep" indicator
- Meeting in both → live timing + briefing enrichment

## Consequences

- Dashboard is both current AND enriched
- Requires stable event ID matching between Google Calendar API and `schedule.json` (implemented via `calendar_merge.rs`)
- `get_dashboard_data()` merges `schedule.json` + `state.calendar_events` with 4-state overlay (Enriched, Cancelled, New, BriefingOnly)
- Event ID format resolved: Google Calendar event IDs flow through the full pipeline (I24)
- This decision should be resolved before ADR-0033 (meeting entity unification)
