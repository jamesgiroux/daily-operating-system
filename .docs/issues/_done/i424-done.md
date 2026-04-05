# I424 — Granola hardening

**Status:** Open
**Priority:** P1
**Version:** 0.13.9
**Area:** Backend / Granola

## Summary

Granola has two critical issues: (1) it holds the DB mutex across an AI pipeline call (the exact bug Quill fixed months ago via the `process_fetched_transcript_without_db` pattern), and (2) it has no wake signal, so it always waits up to its fixed poll interval (default 10 minutes) after meetings end, while Quill is woken immediately by the calendar poller. This issue: port the Quill mutex fix, add the wake signal, and wire the calendar poller to trigger Granola when meetings end.

## Acceptance Criteria

1. **DB mutex fix:** `process_granola_document` in `granola/poller.rs` no longer holds the DB mutex across the AI pipeline call. The fix mirrors Quill's approach: acquire the mutex to read required data, drop it, run the pipeline without the lock. Verify: process a Granola transcript while the app is under load (another command running concurrently). The app does not hang or show slow response in other areas during transcript processing.

2. **Wake signal:** The calendar poller's `check_ended_meetings_for_sync` now also wakes the Granola poller (`state.granola_poller_wake.notify_one()`) after creating Quill sync rows, if Granola is enabled. Verify: end a meeting in Google Calendar. The Granola poller processes matching documents within 2 minutes of the calendar poll completing — not after a 10-minute wait.

3. `state.granola_poller_wake` (an `Arc<Notify>` or equivalent) is added to `AppState` and wired into `lib.rs` alongside the existing `quill_poller_wake`. Verify: `grep -n "granola_poller_wake" src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/src/google.rs` — appears in all three files.

4. `cargo test` passes. No regressions on existing Quill functionality. The fix is isolated to the Granola code path.

## Dependencies

Must not touch Quill code. The Quill fix is the reference implementation but Granola is a separate code path.

## Notes / Rationale

**Key files:**
- `src-tauri/src/granola/poller.rs` — line ~143, where `process_granola_document` holds the DB mutex
- `src-tauri/src/state.rs` — AppState definition
- `src-tauri/src/lib.rs` — event loop setup
- `src-tauri/src/google.rs` — calendar poller, `check_ended_meetings_for_sync` function

**The Quill fix reference:**
Quill solved this by creating `process_fetched_transcript_without_db`. The pattern: read all data needed for the pipeline while holding the lock, then drop the lock before calling the AI service. Apply the same pattern to Granola.

**Rationale:**
Granola's DB lock contention blocks other commands from accessing the database during transcript processing (which can take 30+ seconds with the AI call). By dropping the lock, other operations (calendar sync, enrichment, signal propagation) can proceed unblocked. The wake signal means meetings are processed within 2 minutes of ending rather than waiting for the next scheduled poll, improving the freshness of meeting intelligence.
