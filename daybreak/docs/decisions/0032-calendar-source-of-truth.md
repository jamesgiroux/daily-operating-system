# ADR-0032: Calendar source of truth: hybrid overlay

**Date:** 2026-02
**Status:** Proposed

## Context

The app has two calendar data sources that can disagree:
- `schedule.json`: Generated at briefing time (6 AM). Stale within hours. Has AI enrichment (prep, classification, context).
- `calendar_events` (AppState): Polled every 5 minutes via `calendar_poll.py`. Near-real-time. No enrichment.

A meeting cancelled at 7 AM still shows on the dashboard. A meeting added at 2 PM shows nowhere — the dashboard only reads `schedule.json`.

**Event ID status:** Google Calendar event IDs are preserved by `prepare_today.py` and `calendar_poll.py`. Both data sources can be matched by event ID. However, `deliver_today.py` may generate local slugs (e.g., "0900-acme-sync") rather than preserving the Google Calendar event ID in `schedule.json`. This needs verification.

## Decision

Hybrid overlay: Live calendar is source of truth for *which meetings exist*. Briefing enrichment (prep, classification, context) is overlaid onto live events by matching on calendar event ID.

- Meeting in briefing but not live → cancelled, hide or grey out
- Meeting in live but not briefing → new, show bare with "No prep" indicator
- Meeting in both → live timing + briefing enrichment

## Consequences

- Dashboard is both current AND enriched
- Requires stable event ID matching between Google Calendar API and `schedule.json`
- Requires refactoring `get_dashboard_data()` to merge `schedule.json` + `state.calendar_events`
- Must resolve the ID format question: `deliver_today.py` needs to preserve Google Calendar event IDs, not generate local slugs
- This decision should be resolved before ADR-0033 (meeting entity unification)
