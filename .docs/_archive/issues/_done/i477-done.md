# I477 — Meeting entity switch should hot-swap briefing content

**Status:** Open  
**Priority:** P1  
**Version:** 0.15.1  
**Area:** Backend / Meeting + Frontend / UX

## Summary

Switching linked entities on the Meeting Briefing page updates chips immediately, but briefing content often remains stale until the user manually runs a full refresh. The page is intended to be a consumer of `intelligence.json`-derived meeting prep, so entity reassignment should deterministically swap the displayed briefing context without page reloads or manual intervention.

This issue implements two coordinated fixes:

1. **Stale disk fallback guard** (Plan item 3): prevent `get_meeting_intelligence` from hydrating stale `_today/data/preps/<meetingId>.json` content immediately after entity mutation invalidates prep.
2. **Single mutation-and-refresh service** (Plan item 4): one backend orchestration path for meeting entity mutations that atomically performs mutation + prep invalidation + immediate mechanical rebuild + event emission.

## Problem Statement

Current flow:

- UI chips call `add_meeting_entity` / `remove_meeting_entity`.
- Service invalidates DB prep and queues background regeneration.
- `MeetingDetailPage` immediately reloads.
- `load_meeting_prep_from_sources` falls back to stale disk prep when DB prep is null.
- Scheduler-driven regeneration may lag (up to poll interval + workflow execution), so user sees old briefing.

Result: entity switched, but content still reflects prior entity context.

## Scope

### In scope

- New service for entity mutation + immediate meeting prep rebuild.
- Freshness guard so stale disk prep is not used during invalidated/rebuilding state.
- Event contract for deterministic UI update (`prep-ready` or new meeting-specific event payload).
- MeetingDetailPage wiring so content updates automatically when rebuild completes.

### Out of scope

- Rewriting all historical `prepare_today` workflows.
- Replacing `meeting_prep_queue` architecture globally.
- Removing disk prep files entirely.

## Implementation Plan

### Phase A — Stale disk fallback guard (Plan item 3)

1. Add an explicit **briefing freshness gate** in meeting prep loading:
   - If meeting prep was invalidated due to entity mutation and no new `prep_frozen_json` exists yet, do **not** read disk fallback.
   - Return `None` (or rebuilding state) so UI can show "Updating briefing..." instead of stale content.
2. Ensure invalidation path marks state clearly (`intelligence_state = refreshing` or dedicated state marker) before reload.
3. Only allow disk fallback when:
   - no invalidation is in progress, and
   - fallback prep is known current for current entity links (or no stronger source exists).

### Phase B — Single mutation-and-refresh service (Plan item 4)

1. Add unified service method (example name):
   - `mutate_meeting_entities_and_refresh_briefing(...)`
2. Service performs in-order:
   - mutate entity links (add/remove/replace mode)
   - invalidate prep snapshot + stale disk path metadata
   - enqueue linked entity enrichment asynchronously (non-blocking)
   - **rebuild mechanical prep immediately** (spawn_blocking/block_in_place safe path)
   - emit completion event with `meetingId` and refresh metadata
3. Route existing command handlers (`add_meeting_entity`, `remove_meeting_entity`, `update_meeting_entity`) through this orchestration service.
4. Keep existing background queue as fallback only when immediate rebuild fails.

### Phase C — UI consumer behavior

1. `MeetingDetailPage` subscribes to rebuild completion event and reloads only for matching `meetingId`.
2. On entity mutation, show in-place transient state ("Updating briefing...").
3. Remove requirement for manual full refresh after entity changes.

## Acceptance Criteria

1. **No manual refresh required:** After adding/removing/swapping a meeting entity, the displayed briefing sections (`intelligenceSummary`, risks, context, talking points) update automatically to match new entity context.
2. **No stale flash:** Immediately after mutation, UI never renders old disk prep content for the previous entity.
3. **Deterministic update path:** Entity mutation command returns only after immediate mechanical rebuild succeeds, or returns a clear fallback status when queued.
4. **Safe fallback:** If immediate rebuild fails, background queue handles regeneration and UI updates on completion event.
5. **Performance guardrails:** Mutation path does not block UI navigation; heavy work runs via `spawn_blocking` and respects heavy-work semaphore where applicable.
6. **Regression safety:** Existing `refresh_meeting_briefing` manual flow remains functional.
7. **Quality gate:** `cargo check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.

## Proposed Files

- `src-tauri/src/services/meetings.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/meeting_prep_queue.rs`
- `src-tauri/src/services/intelligence.rs` (if orchestration requires shared heavy-work control)
- `src/pages/MeetingDetailPage.tsx`
- `src/components/ui/meeting-entity-chips.tsx`

## Risks / Tradeoffs

- Immediate rebuild adds synchronous work to mutation path; must stay on blocking threads to avoid runtime starvation.
- Guarding disk fallback too aggressively could show temporary empty/loading state more often; UI messaging must be explicit.
- Multiple rapid chip mutations need dedupe/coalescing to avoid redundant rebuild work.

## Notes

- This issue intentionally makes Meeting Briefing a strict consumer of current meeting prep state, not "best effort" fallback state.
- Aligns with ADR-0086 direction: meeting prep is a consumer projection and should refresh deterministically when entity context changes.
