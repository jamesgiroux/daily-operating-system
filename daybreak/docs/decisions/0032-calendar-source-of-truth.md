# ADR-0032: Calendar source of truth: hybrid overlay

**Date:** 2026-02
**Status:** Proposed

## Context

The app has two calendar data sources that can disagree:
- `schedule.json`: Generated at briefing time (6 AM). Stale within hours. Has AI enrichment.
- `calendar_events` (AppState): Polled every 5 minutes. Near-real-time. No enrichment.

A meeting cancelled at 7 AM still shows on the dashboard. A meeting added at 2 PM shows in the header count but not the timeline.

## Decision

Hybrid overlay (recommended): Live calendar is source of truth for *which meetings exist*. Briefing enrichment (prep, classification, context) is overlaid onto live events by matching on calendar event ID.

- Meeting in briefing but not live → cancelled, hide or grey out
- Meeting in live but not briefing → new, show bare with "No prep" indicator
- Meeting in both → live timing + briefing enrichment

## Consequences

- Dashboard is both current AND enriched
- Requires stable event ID matching between Google Calendar API and `prepare_today.py` output
- Requires refactoring how the dashboard consumes meeting data
- Open question: does `schedule.json` currently include Google Calendar event IDs?
- Depends on: this decision should be resolved before ADR-0033 (meeting entity unification)
