# I538: Meeting briefing refresh — rollback on failure

## Problem

`refresh_meeting_briefing_full` eagerly clears `prep_frozen_json` in Phase 1 before enrichment runs. If enrichment fails (PTY unavailable, entity has no workspace files, network error), the user loses their existing briefing and sees a blank "Building context" page with no way to recover except waiting for a retry that may never succeed.

This was discovered during I511 testing: linking an entity to a meeting and clicking Refresh cleared the prep, enrichment failed silently, and the meeting detail page showed the empty state instead of the previous briefing.

## Root Cause

In `services/meetings.rs::refresh_meeting_briefing_full`:

1. **Phase 1** (line ~1800): Clears `prep_frozen_json = NULL, prep_frozen_at = NULL` immediately
2. **Phase 2** (line ~1857): Calls `enrich_entity()` for each linked entity — may fail
3. **Phase 3** (line ~1935): Calls `generate_mechanical_prep_now()` — may fail if no intelligence exists
4. **Phase 4** (line ~1967): Updates intelligence state to "enriched"

If Phase 2 or 3 fails, the prep is already gone. The frontend re-renders, sees no content, and shows the empty state.

## Fix

Snapshot-then-swap: keep the existing prep until the new one is ready.

### Implementation

1. **Phase 1 — snapshot, don't clear.** Read the current `prep_frozen_json` into a local variable. Set `intelligence_state = 'enriching'` but do NOT null the prep. The frontend continues showing the existing briefing.

2. **Phase 2 — enrich as before.** No change to entity enrichment loop. Failed entities still queued for retry.

3. **Phase 3 — rebuild, then swap.** Call `generate_mechanical_prep_now()`. If it succeeds and produces non-empty prep, write the new `prep_frozen_json`. If it fails or produces empty prep AND enrichment also failed (zero entities refreshed), restore the snapshot and set state back to previous value.

4. **Phase 4 — finalize.** Only mark "enriched" if new prep was written. If snapshot was restored, mark state as previous value (not "enriched").

### Files to modify

- `src-tauri/src/services/meetings.rs` — `refresh_meeting_briefing_full`: snapshot-then-swap logic
- `src-tauri/src/meeting_prep_queue.rs` — `generate_mechanical_prep_now`: return whether prep was non-empty (currently returns `Result<(), String>`)

### What NOT to change

- Entity enrichment error handling (already queues retries correctly)
- Frontend empty state logic (correct — it should show empty when there's genuinely no content)
- The "Building context" page itself (correct for genuinely new meetings with no prep yet)

## Acceptance Criteria

1. Click Refresh on a meeting with existing briefing. If enrichment fails, existing briefing is still visible — not replaced by "Building context" empty state.
2. Click Refresh on a meeting with existing briefing. If enrichment succeeds, new briefing replaces old one.
3. Click Refresh on a meeting with no existing briefing. Behavior unchanged — "Building context" shows during enrichment, new prep appears when ready.
4. Toast messaging reflects actual outcome: "Briefing updated" on success, "Update failed — showing previous briefing" on full failure.
5. Partial success (some entities refreshed, some failed): new prep is written with whatever intelligence is available, failed entities queued for retry.

## Scope

- Backend only (Rust). No frontend changes.
- ~50 lines changed across 2 files.
- No schema changes.
