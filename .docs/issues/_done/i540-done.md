# I540 — Actions Pipeline Integrity + Lifecycle

**Priority:** P0
**Area:** Backend / Pipeline / Frontend
**Version:** v1.0.0 (Phase 3)
**Depends on:** I512 (ServiceLayer — signal emissions), I511 (schema — done)
**Complements:** I448 (ActionsPage editorial rebuild), I454 (vocabulary pass)

## Problem Statement

The actions system has 6 broken or incomplete paths that undermine the chief-of-staff experience. Meeting outcomes don't reliably produce useful actions, the briefing can't see DB-stored actions, the lifecycle policy is implemented but never called, and the UI makes promises the backend doesn't keep.

A chief of staff who forgets your commitments, can't explain why something matters, and lies about cleaning up old tasks is not a chief of staff — it's a liability.

## Root Cause Analysis

### Bug 1: Granola actions lose all metadata

**Location:** `granola/poller.rs:282-311`

When Granola transcripts are processed through the AI pipeline, the resulting `CapturedAction` struct carries only `title`, `owner`, `due_date`. The poller creates `DbAction` with:
- Priority **hardcoded to P2** (ignoring AI-extracted priority)
- Context **None** (dropping the AI's reasoning)
- No entity linking beyond owner name matching

The direct transcript processor (`processor/transcript.rs:362`) uses `parse_action_metadata()` which extracts priority annotations (`P1 @Acme due: 2026-03-15 #"CFO context"`), inline context quotes, and structured metadata. This path is never called for Granola-sourced actions.

**Fix:** After `process_granola_document` returns `TranscriptResult`, re-parse each `CapturedAction` through `parse_action_metadata()` to extract inline annotations. If the title contains priority/context markers, they should be preserved — not stripped to a bare string.

### Bug 2: `fetch_categorized_actions` queries non-existent column

**Location:** `prepare/actions.rs:257`

```sql
SELECT id, title, priority, status, due_date, account_id, source_context
FROM actions WHERE status != 'completed' AND due_date IS NOT NULL
```

The column `source_context` does not exist on the `actions` table (it exists on `signal_events`). The query silently fails via the `Err(_) => return result` fallback at line 265, so `fetch_categorized_actions` always returns empty.

**Effect:** DB-stored actions never merge into the briefing directive. Only workspace markdown actions feed the daily briefing. Completing or updating an action in the UI has zero effect on the next day's briefing context.

**Fix:** Replace `source_context` with `context` (the actual column name on `actions`). Verify the query returns expected rows.

### Bug 3: `archive_stale_actions()` exists but is never called

**Location:** `db/actions.rs:539`

The function archives pending actions older than N days. It has zero callers outside tests. The scheduler (`scheduler.rs:190`) only calls `auto_archive_old_proposed(7)` for proposed actions.

**Effect:** Pending actions accumulate in the DB forever. The frontend hides them after 30 days via `isPending30DaysStale()` in `useActions.ts:231`, but they remain in the DB. The tooltip text "30+ days auto-archived" is a lie — no archival happens.

**Fix:** Add `archive_stale_actions(30)` to the scheduler alongside `auto_archive_old_proposed(7)`. Update the tooltip to match reality.

### Bug 4: Granola free-tier content produces thin summaries

**Location:** `granola/cache.rs` content selection, `processor/transcript.rs` prompt

When Granola only has `notes_markdown` (free tier) rather than raw transcripts (paid tier), the AI extraction prompt receives already-processed AI notes. The prompt asks for a meeting summary of what is itself a summary — producing repetitive or empty output.

**Fix:** Tag content type in `GranolaContent` (transcript vs notes). When notes-only content is detected, adjust the extraction prompt to acknowledge it's working from meeting notes rather than a verbatim transcript. Set expectations: "Extract action items and key decisions from these meeting notes" rather than "Summarize this meeting transcript."

### Bug 5: `rejection_source` always records `"unknown"`

**Location:** `db/actions.rs:503-505`

`reject_proposed_action()` calls `reject_proposed_action_with_source(id, "unknown")`. The correction learning system (I307/I529/I530) can't learn from rejections without proper source attribution.

**Fix:** Thread rejection source from the frontend through the Tauri command. Actions page dismissals → `"actions_page"`. Briefing dismissals → `"daily_briefing"`. Meeting detail dismissals → `"meeting_detail"`.

### Bug 6: Deceptive auto-archive tooltip

**Location:** Frontend — ActionsPage or action components

The UI claims "30+ days auto-archived" but this behavior doesn't exist in the backend.

**Fix:** After Bug 3 is fixed (scheduler calls `archive_stale_actions(30)`), the tooltip becomes truthful. If the policy changes, update the tooltip to match.

## Lifecycle Policy

### Proposed actions (AI-suggested)
- **7 days** → auto-archived (already implemented, scheduler calls `auto_archive_old_proposed(7)`)
- No change needed

### Pending actions (user-accepted or user-created)
- **30 days past due date** → auto-archived by scheduler
- Actions with no due date: **30 days from creation** → auto-archived
- Archival is soft — archived actions are queryable, just not shown in active views
- Remove the frontend `isPending30DaysStale()` display filter — let the backend handle lifecycle

### Completed actions
- Shown for 48 hours on the completed tab (already implemented)
- Stay in DB permanently for historical queries and briefing context

## Action Context Enrichment

The chief of staff doesn't hand you a bare "Follow up on renewal." They say: "Follow up on Acme renewal — the CFO mentioned budget concerns in yesterday's meeting."

### For Granola/transcript-sourced actions:
- Preserve the AI-extracted `#"context"` annotation from `parse_action_metadata()`
- Store in the `context` column on `actions`
- Frontend renders context as secondary text below the action title

### For briefing-surfaced actions:
- The briefing directive should include the action's `context` and `source_label` when surfacing top actions
- This requires Bug 2 to be fixed first (briefing must see DB actions)

## Implementation Plan

### Wave 1: Backend bug fixes (mechanical)

| Task | File | Change |
|------|------|--------|
| Fix `source_context` → `context` | `prepare/actions.rs:257` | Column name fix |
| Wire `archive_stale_actions(30)` into scheduler | `scheduler.rs` | Add call alongside `auto_archive_old_proposed` |
| Thread `rejection_source` from frontend | `commands.rs`, `services/actions.rs`, `db/actions.rs` | Add `source` param to reject command |
| Remove `isPending30DaysStale` display filter | `src/hooks/useActions.ts` | Delete — backend handles lifecycle |

### Wave 2: Granola pipeline enrichment

| Task | File | Change |
|------|------|--------|
| Re-parse Granola actions through `parse_action_metadata()` | `granola/poller.rs:282-311` | Use metadata parser instead of bare DbAction construction |
| Tag content type (transcript vs notes) | `granola/cache.rs` | Add `content_type` field to `GranolaContent` |
| Adjust prompt for notes-only content | `processor/transcript.rs` | Conditional prompt preamble based on content type |

### Wave 3: Briefing integration

| Task | File | Change |
|------|------|--------|
| Verify `fetch_categorized_actions` returns rows after Bug 2 fix | `prepare/actions.rs` | Integration test |
| Include `context` and `source_label` in briefing directive actions | `prepare/actions.rs`, `workflow/deliver.rs` | Pass context through to actions.json |

### Wave 4: Frontend (pairs with I448 editorial rebuild)

| Task | File | Change |
|------|------|--------|
| Fix deceptive tooltip | `ActionsPage.tsx` or action components | Match tooltip to actual policy |
| Render action context as secondary text | Action row component | Show context below title |
| Thread rejection source in dismiss calls | `useProposedActions.ts`, `ActionsPage.tsx` | Pass source string to `reject_proposed_action` |

## Acceptance Criteria

### Pipeline integrity
1. Process a Granola transcript with inline action annotations (`P1 @Acme due: 2026-03-15 #"CFO mentioned budget"`). Proposed action appears in Suggested tab with P1 priority, Acme entity link, due date, and context text.
2. Accept the proposed action. It appears on the Pending tab. Next daily briefing includes it in the actions context.
3. Complete the action. Next daily briefing reflects the completion (no longer surfaces as pending).
4. Granola free-tier (notes only): actions extracted with reasonable context. Summary is not a repetition of the notes.

### Lifecycle
5. Proposed action untouched for 7 days → auto-archived. (Already works, verify.)
6. Pending action 30 days past due date → auto-archived by scheduler. Verify in DB.
7. Pending action with no due date, 30 days old → auto-archived.
8. No frontend display filter for stale actions — backend is sole arbiter of lifecycle.

### Corrections
9. Dismiss a proposed action from the Actions page. `rejection_source` in DB = `"actions_page"`, not `"unknown"`.
10. Dismiss a proposed action from the briefing. `rejection_source` = `"daily_briefing"`.

### UX integrity
11. No tooltip or UI text claims behavior that doesn't exist in the backend.
12. Action context text renders below action title when available.
13. Vocabulary: "Suggested" tab (not "Proposed"), "Dismiss" button (not "Reject") — verify after I454 ships.

### Cross-cutting
14. `cargo test` — all pass
15. `cargo clippy -- -D warnings` — clean
16. `pnpm tsc --noEmit` — clean

## What This Does NOT Do

- **Does not redesign the Actions page layout** — that's I448 (editorial rebuild)
- **Does not implement meeting-centric action grouping** — future work, requires design spec
- **Does not add new action sources** (email-to-action, etc.) — separate issue
- **Does not change the action data model** — uses existing columns (`context`, `source_type`, `source_label`, `rejected_at`, `rejection_source`)
- **Does not touch the vocabulary** — that's I454. This issue fixes behavior; I454 fixes labels.

## Relationship to Other Issues

- **I448** (ActionsPage editorial rebuild) — I540 fixes the data pipeline; I448 fixes the presentation. They pair naturally in Phase 3d.
- **I454** (Vocabulary pass) — I540's AC #13 verifies vocabulary after I454 ships.
- **I512** (ServiceLayer) — Bug 5 fix (rejection source threading) goes through ServiceLayer mutations. Signal emissions for action lifecycle depend on I512.
- **I529/I530** (Feedback + taxonomy) — Proper rejection source attribution (Bug 5) feeds the correction learning system these issues build.
