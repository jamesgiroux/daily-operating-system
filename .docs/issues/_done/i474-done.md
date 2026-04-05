# I474 — Inbox document → historical meeting matching

**Status:** Open
**Priority:** P2
**Version:** 0.15.1
**Area:** Backend / Pipeline

## Summary

When a file lands in `_inbox/` and the processor classifies it as `MeetingNotes`, it is routed to `Accounts/{Account}/Call-Transcripts/` (or `_archive/`) but is never linked to a specific meeting in `meetings_history`. The transcript AI pipeline runs, captures (wins/risks/decisions) and actions are written, but they have no `meeting_id` — they float unattached. If the same meeting was already in the system (past calendar event), the outcomes never appear on the meeting detail page and don't enrich prep for linked future meetings.

Quill and Granola already solve this for their own transcript sources using a multi-signal scoring algorithm (title similarity + time proximity + participant overlap, threshold ≥ 100). This issue applies the same algorithm to inbox-processed documents, closing the gap for files dropped in manually or arriving via other paths (e.g., email forwarding, Drive import to inbox).

## Acceptance Criteria

### Matching

1. **Candidate query** — after `MeetingNotes` classification, query `meetings_history` for candidates: same account (if resolved), `start_time` within the last 90 days, `intelligence_state != 'archived'`. Cap at 20 candidates.

2. **Scoring** — score each candidate using the same algorithm as `quill/matcher.rs`:
   - Title similarity (0–100 pts): exact match 100, contains/contained-by 70, token overlap > 50% → 50, else 0
   - Time proximity (0–80 pts): derive meeting date from filename or frontmatter if present; within 1 day → 80, within 3 days → 50, within 7 days → 20, else 0
   - Account match (0–40 pts): if classified account matches `meetings_history.account_id` → 40, else 0
   - Threshold: score ≥ 100 for automatic linking; below threshold → route without meeting link (current behaviour, no regression)

3. **Link and run** — on automatic match, use the matched `meeting_id` to:
   - Write captures (wins/risks/decisions) with `meeting_id` set (currently written with `meeting_id = None` or the inbox document ID)
   - Write actions with `source_id = meeting_id` and `source_type = "transcript"`
   - Update `meetings_history.transcript_path` and `transcript_processed_at` for the matched meeting (same as `attach_meeting_transcript` does)
   - Emit `transcript_outcomes` signal via `emit_signal_and_propagate` on the meeting's linked entity

4. **No match → current behaviour** — if no candidate scores ≥ 100, the file is routed and processed as today: captures and actions are written without a meeting link. No error, no retry.

5. **Verify end-to-end** — drop a file named `2026-02-20-acme-qbr-notes.md` into `_inbox/` for a meeting that exists in `meetings_history`. After processing: open the meeting detail page → "Meeting Outcomes" section is populated with the extracted wins/risks/decisions.

6. **No regressions** — `cargo test` passes. Existing Quill and Granola sync behaviour is unchanged. Files that don't classify as `MeetingNotes` are unaffected.

### Confidence logging

7. **Audit trail** — log the match attempt at `INFO` level: `"Inbox matcher: '{}' → meeting '{}' (score {}, {})". Abandoned matches (score < 100) are logged at DEBUG level with the top candidate and its score, so it's diagnosable when a file should have matched but didn't.

## Dependencies

- `quill/matcher.rs` — reuse the scoring algorithm directly or extract a shared `transcript_matcher.rs` crate-internal module
- `processor/mod.rs` — entry point for inbox classification; matching runs after `MeetingNotes` classification, before file routing
- `processor/transcript.rs` — `process_transcript` accepts an optional `meeting_id` override; if matched, it is passed in

## Notes / Rationale

**Why inbox files don't match today:**
The inbox processor classifies by filename pattern (`acme-qbr-notes.md` → `MeetingNotes { account: "Acme" }`) and routes to disk. It has no awareness of the calendar. Quill and Granola solve this differently — they start from the calendar (pending meetings) and search for a transcript. Inbox is document-first: we need to reverse the direction, going from document to calendar event.

**Key files:**
- `src-tauri/src/processor/mod.rs` — `process_file()`, classification and routing entry point
- `src-tauri/src/processor/transcript.rs` — `process_transcript()`, accepts `db: Option<&ActionDb>`; needs a `meeting_id: Option<&str>` param to write captures with meeting context
- `src-tauri/src/quill/matcher.rs` — scoring algorithm to reuse or extract
- `src-tauri/src/db/meetings.rs` — `update_meeting_transcript_metadata()` already exists

**Date extraction from filename:**
Many inbox files are named with dates (`2026-02-20-acme-qbr-notes.md`, `acme-call-2026-02-20.md`). A regex pass over the filename before scoring can extract a candidate date to weight time proximity. Without a date in the filename, time proximity scores 0 but title and account signals still fire.

**Shared matcher module:**
The scoring logic is currently duplicated between `quill/matcher.rs` and `granola/matcher.rs`. This issue is a natural forcing function to extract a `processor/matcher.rs` (or `transcript_matcher.rs`) with the shared algorithm, reducing the duplication. Not required for this issue but recommended.

**Threshold tuning:**
The Quill/Granola threshold of ≥ 100 works because those matchers have time proximity from actual meeting timestamps. Inbox matching relies on filename-derived dates which may be less reliable. If false positives emerge in practice, the threshold can be raised to 120 or 140 without changing the algorithm — just the gate.

**What this does NOT cover:**
Inbox files with no recognisable account, no date, and a generic title (e.g., `notes.txt`) will still not match. That is acceptable — the fallback is current behaviour, not an error. Fix 2 is intentionally scoped to cases where enough signal exists for a confident match.
