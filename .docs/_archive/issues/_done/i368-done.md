# I368 — Persist Email Metadata to SQLite — DB as Source of Truth, Not JSON Files

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Backend / DB

## Summary

Email data currently lives primarily in `_today/data/emails.json` — a flat file that's regenerated on each sync cycle and doesn't survive across days. This makes cross-day email history impossible, enrichment state tracking unreliable, and entity email history (e.g., "show me emails from this account over the last week") unavailable. This issue migrates email storage to SQLite as the source of truth, with `emails.json` generated from the DB for backward compatibility rather than being the primary store.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. An `emails` table exists in the database with columns: `email_id`, `thread_id`, `sender_email`, `sender_name`, `subject`, `snippet`, `priority`, `is_unread`, `received_at`, `enrichment_state`, `last_seen_at`, `entity_id`, `entity_type`, `contextual_summary`, `created_at`, `updated_at`.
2. After a poll cycle, query the DB: `SELECT count(*) FROM emails` returns a count matching the number of emails fetched. The DB is the source of truth.
3. `_today/data/emails.json` is still written (for backward compatibility with frontend) but is generated FROM the database, not the other way around. Verify: delete `emails.json`, trigger a refresh (NOT a Gmail fetch), and confirm `emails.json` is regenerated from DB contents. If the file can only be produced by fetching from Gmail, the DB is not the source of truth.
4. Email data survives across days. Open the app on Tuesday, see Monday's emails that are still in the inbox. Verify by checking `emails.received_at` includes dates older than today.
5. Entity detail pages (account, person) can query email history: open an account detail, see emails from that account's domain across multiple days — not just today's snapshot.

## Dependencies

- Foundational for I366 (reconciliation) — reconciliation reads/writes the `emails` table.
- Foundational for I369 (contextual synthesis) — synthesis reads entity context from DB and writes `contextual_summary`.
- Foundational for I373 (sync status) — sync status reads enrichment state from DB.

## Notes / Rationale

ADR-0085 decision 6: "Email metadata is durable." JSON files are ephemeral — they don't accumulate history, don't support cross-day queries, and don't provide the enrichment state tracking needed for the retry mechanism in I367. Moving to SQLite is the correct architectural step that enables everything else in v0.13.1.
