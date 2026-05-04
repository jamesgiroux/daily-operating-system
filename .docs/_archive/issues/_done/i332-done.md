# I332 — Signal-Triggered Meeting Intelligence Refresh

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend / Pipeline

## Summary

Meeting intelligence was previously static after initial generation — it didn't update when new signals arrived (new emails from attendees, calendar changes, transcript uploads). This issue wired signal-triggered refresh: when a signal arrives that's relevant to a meeting's linked entity, the prep is marked stale and re-queued for re-assembly. This ensures meeting prep always reflects the current state of entity intelligence without user intervention.

The pre-ship audit (February 20, 2026) found that `check_and_invalidate_preps()` was fully implemented but never called — it existed in the codebase but was not wired into the signal processing path. This was remediated before v0.13.0 tagged.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Add a new calendar event for tomorrow with a known contact. Within the next polling cycle (≤5 min), that meeting appears in the week timeline with intelligence triggered — without manually running a refresh.
2. Receive an email from a meeting attendee. The prep for that meeting is marked stale and re-queues for enrichment on next cycle. Verify by inspecting the prep file timestamp and `intelligence_state` in the DB before and after.
3. `check_and_invalidate_preps()` is called as part of the signal processing path. Verify by adding a log line and confirming it fires when a signal arrives.

## Dependencies

- Depends on I326 (lifecycle state machine).
- Depends on signal bus (ADR-0080) being active.
- Related to I372 (email-entity signal compounding) in v0.13.1.

## Notes / Rationale

This was the most significant gap found in the pre-0.13.0 audit: prep invalidation was coded but dead. The fix required wiring `check_and_invalidate_preps()` into the signal processing path — a one-line call in the right place. The lesson: having the function is not the same as the function being called.
