# I331 — Daily Briefing Intelligence Assembly — Always-Live, No Empty State

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX / Backend

## Summary

The daily briefing previously could show an empty or error state if the morning prepare pipeline hadn't run or had failed. This issue made the briefing always-live: it assembles from pre-computed intelligence stored in the DB, never blocks on a pipeline run, and never shows an empty state. If intelligence is incomplete, it shows the best available data with appropriate quality indicators — not a blank screen.

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. Opening the daily briefing never blocks on a pipeline run — it renders immediately from cached data.
2. No empty state screen appears. Even with a fresh install (after setup), the briefing renders a reasonable starting state.
3. Meeting cards on the briefing show quality badges reflecting current intelligence state rather than hiding meetings without prep.
4. A briefing that generates while the user is looking at it updates incrementally — they don't need to refresh.

## Dependencies

- Depends on I326 (lifecycle), I327 (advance generation), I329 (quality indicators).
- Related to I332 (signal-triggered refresh) — the always-live approach means the briefing reflects signal-triggered updates automatically.

## Notes / Rationale

The always-live assembly model is a direct consequence of ADR-0086: meeting prep is mechanical assembly from entity intelligence, not an AI call. Mechanical assembly is instant. The briefing therefore has no reason to ever be empty or loading — it's always assembling from whatever entity intelligence is available.
