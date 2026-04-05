# I327 — Advance Intelligence Generation — Weekly Pipeline + Polling Cadence

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend / Intelligence

## Summary

Meeting intelligence was previously generated on-demand (when the user visited the meeting detail page) or day-of during the morning briefing run. This meant meetings scheduled for later in the week had no intelligence until the day arrived. This issue wired the weekly pipeline and calendar polling cadence to generate intelligence ahead of time — meetings scheduled for the coming week get their prep queued and enriched proactively, not reactively.

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. The calendar poller, when it detects a new meeting, adds it to `meeting_prep_queue` immediately — not waiting for the day-of briefing.
2. The weekly forecast page shows intelligence for future meetings across the ±7 day range without requiring a manual enrichment trigger.
3. Meetings scheduled for tomorrow have intelligence ready when the user opens the app that morning, not generated on page load.

## Dependencies

- Depends on I326 (meeting lifecycle state machine).
- Enables I330 (week page timeline) and I331 (always-live daily briefing).

## Notes / Rationale

The advance generation model is required by the ADR-0086 architecture: entity intelligence is the shared layer, meeting prep is mechanical assembly from that layer. Pre-computing the assembly ahead of time means the briefing is instant on load.
