# I475 — Inbox entity-gating follow-ups

**Priority:** P2
**Area:** Backend / Pipeline + Frontend / UX
**Parent:** Inbox never creates entities (45e20a3)

## Context

The core inbox entity-gating shipped: router validates entity existence, NeedsEntity keeps files in inbox, entity picker lets users assign. These are the remaining gaps found during audit.

## Items

### 1. Transcript NeedsEntity path (Medium)

**Current:** `process_transcript` routes to `_archive/` with `status: "completed"` when the account isn't in the DB. The user gets no signal that they should create the account and re-process.

**Fix:** Add a `NeedsEntity`-like path to the transcript result. When `meeting.account` doesn't resolve in DB, return a result the frontend can surface — either a new `TranscriptResult` status or log `needs_entity` in `processing_log` for the transcript filename.

**Files:** `src-tauri/src/processor/transcript.rs` (lines 63-84, 288-303)

### 2. onAssignEntity should check result status (Low)

**Current:** The `onAssignEntity` callback in `InboxPage.tsx` calls `process_inbox_file` with `entityId` and unconditionally sets `status: "processed"`. If the entity has no `tracker_path`, the resolve returns `None` and `process_file` runs without `entity_tracker_path` — potentially returning `NeedsEntity` again.

**Fix:** Check the result status after re-processing. If `NeedsEntity`, show an error message like "Account has no workspace folder — create one first."

**Files:** `src/pages/InboxPage.tsx` (lines 948-957)

### 3. enrich.rs opens redundant DB connection (Low)

**Current:** `enrich_file` opens a second `ActionDb::open()` at line 138 to pass to `resolve_destination`, even though `state.db.lock()` is available and used elsewhere in the same function.

**Fix:** Extract the DB reference from `state.db.lock()` and pass it to `resolve_destination` instead of opening a second connection.

**Files:** `src-tauri/src/processor/enrich.rs` (line 132-138)

### 4. ActionItems account not validated (Low)

**Current:** `Classification::ActionItems { account }` routes to `_archive/` without validating the account. The extracted actions get `account_id` set to the raw tag, which may not match any DB record.

**Fix:** In `extract_and_sync_actions` (mod.rs), resolve `account_fallback` through `db.get_account_by_name()` before using it as `account_id` — same pattern as `extract_transcript_actions` in transcript.rs.

**Files:** `src-tauri/src/processor/mod.rs` (lines 456-460)

## Acceptance criteria

1. Transcripts with unrecognized accounts surface a user-visible signal (not silently archived as "completed")
2. Entity picker assignment handles edge cases (no tracker_path, re-process returning NeedsEntity)
3. No redundant DB connections in the enrichment path
4. Action items extracted with validated account IDs only
