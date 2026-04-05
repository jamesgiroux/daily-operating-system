# I326 — Per-Meeting Intelligence Lifecycle — Detect, Enrich, Update, Archive

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend / Intelligence

## Summary

Previously, meeting intelligence was generated as a point-in-time enrichment with no lifecycle management. Meetings had no state machine — they were either enriched or not, with no mechanism for detecting when intelligence was stale, re-enriching when signals changed, or archiving intelligence after the meeting passed. This issue established the `meeting_prep_queue` state machine with four states: `new → enriching → enriched → archived`, and wired background enrichment with real AI replacing a prior mechanical row-count that had been incorrectly marking meetings as "enriched."

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. `meeting_prep_queue` table exists with a proper state machine (new → enriching → enriched → archived).
2. New meetings detected by the calendar poller enter the queue in `new` state.
3. The background enrichment processor picks up `new` meetings, runs real AI enrichment (not a mechanical row-count), and transitions them to `enriched`.
4. Meetings that fail enrichment enter a retry path with exponential backoff.
5. Past meetings transition to `archived` state after a configurable window.

## Dependencies

- Foundation for I327 (advance generation), I328 (classification expansion), I329 (quality indicators), I332 (signal-triggered refresh).

## Notes / Rationale

This issue was the centerpiece of the v0.13.0 pre-ship engineering audit remediation. The February 20, 2026 audit found that the prior implementation was marking meetings "enriched" after a mechanical row-count check with no AI involvement whatsoever. All gaps were remediated before v0.13.0 tagged.
