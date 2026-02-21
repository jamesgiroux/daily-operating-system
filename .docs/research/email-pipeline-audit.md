# Email Pipeline Audit — v0.13.1 Research

**Date:** 2026-02-21
**Scope:** End-to-end email: Gmail API → classification → enrichment → signals → DB → frontend → refresh
**Purpose:** Identify gaps, propose v0.13.1 issues

---

## Current Architecture

### Data Flow

```
Gmail API (is:unread newer_than:1d)
  │
  ▼
google_api/gmail.rs::fetch_unread_emails()
  │  (metadata-only: From, Subject, Date, List-Unsubscribe, Precedence)
  │
  ▼
prepare/email_classify.rs
  │  Tier 1: Mechanical rules (high/medium/low)
  │  Tier 2: Signal-context boost (medium → high if entity has active signals)
  │  Tier 3: Optional AI reclassification (I357, feature flag)
  │
  ▼
_today/data/emails.json                      ← PRIMARY STORE (JSON file, not DB)
  │
  ├──► Email poller: AI enrichment on NEW emails only
  │    processor/email_actions.rs (commitments, questions)
  │    executor.enrich_emails_with_fallback()
  │    executor.sync_email_signals_from_payload() → email_signals table
  │
  ├──► signals/email_bridge.rs
  │    Correlates email signals (7d) with upcoming meetings (48h)
  │    Emits pre_meeting_context signals
  │
  └──► Frontend reads via:
       get_dashboard_data() → basic Email[] (no signals)
       get_emails_enriched() → EmailBriefingData (full signals, entity threads)
```

### Polling & Refresh

| Mechanism | Frequency | Trigger |
|-----------|-----------|---------|
| Email poller (`run_email_poller`) | Every 15 min (configurable, work hours only) | Background loop in google.rs |
| /today workflow | Once daily (scheduler) | Includes full email fetch + classify |
| Manual refresh button | On-demand | Frontend calls `refresh_emails` command → wakes poller |
| Tauri events | Real-time | `emails-updated` emitted after each poll cycle |

### Storage

| Store | Content | Persistence |
|-------|---------|-------------|
| `_today/data/emails.json` | Classified emails (high/medium/low arrays) | Workspace file, replaced each poll |
| `_today/data/email-refresh-directive.json` | Intermediate refresh output | Workspace file |
| `_today/data/today-directive.json` | Contains `emails.narrative`, `emails.replies_needed` | Workspace file |
| `email_signals` table (SQLite) | Extracted signals per email (entity-linked) | Persistent DB |
| `email_threads` table (SQLite) | Thread state: subject, last sender, ball position | Persistent DB |
| `email_dismissals` table (SQLite) | User dismissals of items | Persistent DB |

---

## What Works Well

1. **Email poller is a proper background task** — runs on its own cadence, doesn't require /today workflow
2. **Classification system is solid** — 3-tier mechanical rules + signal-context boosting + optional AI
3. **Auto-archive of low-priority** — removes noise from Gmail (opt-in via `autoArchiveEnabled` flag, default off)
4. **AI enrichment pipeline** — commitments, questions, sentiment extraction with fallback
5. **Email bridge to meetings** — correlates emails with upcoming meetings for prep context
6. **Tauri event system** — `emails-updated` events keep frontend surfaces reactive
7. **Dismissal UX** — commitments, questions, replies can be dismissed (persisted)
8. **Email page editorial layout** — well-structured editorial page with Replies Needed, Commitments, Signals

---

## Critical Gaps

### Gap 1: Gmail Query Is Wrong for Inbox Mirror

**Current:** `is:unread newer_than:1d`

This means DailyOS only sees emails that are:
- Unread AND
- Less than 24 hours old

**What's missed:**
- Read-but-not-archived emails (you opened it in Gmail but haven't dealt with it)
- Emails older than 24h that are still in your inbox unresolved
- Emails you starred or flagged for follow-up

**Impact:** If you read an email in Gmail but don't archive it, DailyOS thinks it doesn't exist. The "Replies Needed" section can't track threads where you read the email but haven't replied, because `is:unread` filters them out.

**Fix direction:** Query `in:inbox newer_than:3d` (or configurable window) to get actual inbox state. This also naturally handles archive sync — if it's not `in:inbox`, it won't appear.

### Gap 2: No Inbox Reconciliation

When a user archives, deletes, or moves an email in Gmail directly:
- DailyOS still shows it in `emails.json` until the next poll replaces the file
- `email_signals` persist indefinitely (no archival mechanism)
- `email_threads` table never marks threads as resolved
- "Replies Needed" can show stale threads the user already addressed

**Impact:** The email surface gets out of sync with reality. Users learn not to trust it.

**Fix direction:** If we switch to `in:inbox` query, each poll cycle naturally represents the current inbox. Emails absent from the latest fetch should be removed from the JSON. For `email_threads`, mark threads not seen in latest fetch as `resolved_at = now()`.

### Gap 3: Thread State Doesn't Reflect User Replies

`email_threads.user_is_last_sender` is set during email fetch by checking sent mail. But:
- If the user replies to a thread between poll cycles, the "Replies Needed" section still shows it
- The 15-minute poll interval means up to 15 minutes of stale "ball in your court" state
- No mechanism to detect sent mail in real-time

**Impact:** "Replies Needed" shows threads you already replied to. This is the most visible staleness issue.

**Fix direction:** During each poll cycle, also check recent sent mail to update thread positions. Or query the thread endpoint to get the latest message for each tracked thread.

### Gap 4: Emails Not Persisted to Database

The primary email data lives in `_today/data/emails.json`, not SQLite. Consequences:
- No email history beyond the current day's snapshot
- Entity detail pages can only reference emails from today's fetch
- Cross-day email continuity impossible (can't track a thread across days)
- If workspace files are cleared, all email context is lost
- Dual-store problem: signals in DB, emails in JSON, can get out of sync

**Impact:** The system can't answer "what emails did we get from Acme last week?" or "has the thread about the contract been resolved?" Email intelligence is ephemeral.

**Fix direction:** Persist email metadata (not body) to SQLite as the source of truth. JSON files become a cache/delivery format, not the primary store. This unblocks email history, cross-day tracking, and entity-level email timelines.

### Gap 5: Enrichment Failures Are Silent

If AI enrichment fails (Claude timeout, PTY error):
- The email appears in the mechanical classification (high/medium/low) without enrichment
- No retry mechanism — the email stays un-enriched until a new poll detects "new" emails
- But the email isn't new anymore (same ID), so it won't trigger enrichment again

**Impact:** Some emails never get commitments/questions extracted. The user sees a mechanical classification that never upgrades to enriched.

**Fix direction:** Track enrichment state per email. On each poll, check for un-enriched emails and retry. Or maintain an enrichment queue similar to `intel_queue`.

### Gap 6: Meeting Detail Doesn't Render Email Context

`FullMeetingPrep.recentEmailSignals?: EmailSignal[]` field exists in the type system. The backend populates it (via `email_context.rs`). But `MeetingDetailPage.tsx` doesn't render it.

**Impact:** Before a meeting, you can't see recent email exchanges with attendees on the meeting page. You have to navigate to the email page or entity pages to find this context.

**Fix direction:** Add an "Email Context" section to meeting detail showing recent email signals from meeting attendees.

### Gap 7: Dashboard Emails Are De-Enriched

The daily briefing shows `Email[]` objects without signals. The EmailsPage shows `EnrichedEmail[]` with full signal context. Two different data contracts for the same domain.

**Impact:** The daily briefing's email section is less useful than the email page. Users who only use the briefing miss signal context, entity linking, and enrichment details.

**Fix direction:** Either enrich the dashboard email data to include signals, or make the briefing email section link more clearly to the full email page for context.

---

## Minor Gaps

### Signal-context boosting not persisted
The I320 priority boost (medium → high based on entity signals) only updates the JSON payload. The DB's `email_signals` table retains the original classification. On re-fetch, the same email might not get boosted if signal state changed.

### Email dismissals table created but not used for learning
Migration 030 creates `email_dismissals` with columns for learning (sender_domain, email_type, entity_id). The table is written to when users dismiss items, but no code reads from it to adjust future classifications.

### No email search or filtering on EmailsPage
The page shows all enriched emails with no ability to search by sender, subject, entity, or signal type. As email volume grows, this becomes a usability problem.

### Email sync status not visible
`EmailSyncStatus` type exists in the frontend (with `state`, `stage`, `code`, `message` fields) but isn't displayed. Users can't tell when emails were last refreshed or if a fetch error occurred.

---

## Proposed Issues for v0.13.1

Reframed per ADR-0085: email is an intelligence input, not a display surface. AI enrichment is mandatory, not optional. Entity context is the primary classification axis.

### I365 — Inbox-anchored email fetch: `in:inbox` replaces `is:unread newer_than:1d`
**Priority:** P0 — foundational change that unblocks archive sync and thread accuracy
**Area:** Backend / Gmail API

Switch the Gmail query from `is:unread newer_than:1d` to `in:inbox newer_than:3d` (configurable window). This means DailyOS sees what Gmail considers your inbox — read or unread. Emails archived/deleted in Gmail naturally disappear from the next fetch. Preserve the unread flag as metadata for display priority but don't use it as a fetch filter.

### I366 — Inbox reconciliation: remove vanished emails from DailyOS
**Priority:** P0 — direct consequence of I365
**Area:** Backend / Pipeline

After each poll, compare fetched email IDs with the DB. Emails in DB but absent from the latest fetch are "vanished" (archived/deleted in Gmail). Mark them inactive, mark corresponding `email_threads` as resolved, and mark stale `email_signals` as inactive. This is the "sync back" half of bidirectional email state.

### I367 — Mandatory email enrichment with retry
**Priority:** P0 — AI enrichment is the core product, not an optional enhancement
**Area:** Backend / Pipeline

Every email is AI-processed. Mechanical classification (rules-based priority) exists only as bootstrap for the initial sync. From the second cycle onward, enrichment runs on every email. Track enrichment state per email (pending → enriching → enriched | failed). Failed enrichments retry with exponential backoff (max 3 attempts). Remove the `semanticEmailReclass` feature flag — reclassification is part of the mandatory pipeline. Absorbs I357.

### I368 — Persist email metadata to SQLite
**Priority:** P1 — eliminates dual-store, enables cross-day history
**Area:** Backend / DB

Create an `emails` table storing email metadata (id, thread_id, sender, subject, snippet, priority, is_unread, received_at, enrichment_state, entity_id, entity_type, contextual_summary, last_seen_at). Make the DB the source of truth. `emails.json` becomes a delivery cache generated from DB. This unblocks: email history across days, entity-level email timelines, enrichment state tracking, and consistent data for all frontend surfaces.

### I369 — Contextual email synthesis: entity-aware smart summaries
**Priority:** P1 — this is the "chief of staff" intelligence
**Area:** Backend / Intelligence

An email summary that says "Jack sent a message about the EBR" is mechanical. A contextual synthesis says "Jack is confirming the Acme EBR agenda. This aligns with the renewal discussion from Tuesday." The synthesis prompt includes: email content, resolved entity's current intelligence (from `entity_intel`), recent meeting history with the sender, and active signals for the entity. This runs as part of the enrichment pipeline (I367), producing a `contextual_summary` per email.

### I370 — Thread position refresh: detect user replies between polls
**Priority:** P1 — fixes the most visible staleness issue
**Area:** Backend / Gmail API

During each poll cycle, query recent sent mail (or thread state for tracked threads) to update `email_threads.user_is_last_sender`. If the user replied to a thread since the last poll, flip the ball position immediately. This keeps "Replies Needed" accurate within one poll cycle.

### I371 — Meeting email context rendering
**Priority:** P1 — wires up existing data that isn't displayed
**Area:** Frontend / UX

Render `recentEmailSignals` on the meeting detail page. Show recent email context from meeting attendees as a "Recent Correspondence" section — contextual summaries (from I369), not raw email rows. Data already flows from backend via `email_context.rs` → `FullMeetingPrep.recentEmailSignals`.

### I372 — Email-entity signal compounding
**Priority:** P1 — email signals enrich entity intelligence
**Area:** Backend / Signals

Email signals (commitment, sentiment, urgency, topic) flow into the entity signal graph. An email about Acme updates Acme's intelligence. The email bridge runs on every enriched email — not just emails linked to upcoming meetings. Signal propagation fires after email signals are emitted, cascading through existing propagation rules.

### I373 — Email sync status indicator
**Priority:** P2 — visibility into system state
**Area:** Frontend / UX

Display `EmailSyncStatus` on the email page and daily briefing. Show: last successful fetch timestamp, enrichment progress, error state if applicable, and whether the displayed data uses stale fallback.

### I374 — Email dismissal learning loop
**Priority:** P2 — completes the dismissal feedback loop
**Area:** Backend / Intelligence

Read from `email_dismissals` table to adjust future classifications. If a user consistently dismisses emails from a sender domain or of a certain type, learn to classify those as lower priority. Learning is additive to mechanical classification, not a hard override.

---

## Architecture Alignment

### How this fits with SERVICE-CONTRACTS.md

The service contracts doc doesn't define an `EmailService`. v0.13.1 would implicitly create one:

**EmailService** would own:
- Email fetch + classify pipeline
- Email storage (`emails` table)
- Thread tracking and reconciliation
- Enrichment queue management
- Email sync status

**Does NOT touch:**
- Signal bus (that's SignalService — email_bridge stays in signals/)
- Meeting prep (that's MeetingService — email_context stays in signals/)
- Action extraction from emails (that's the processor pipeline)

### DB module alignment

`db/emails.rs` already appears in the SERVICE-CONTRACTS.md DB split plan (estimated ~800 lines). Persisting emails to SQLite (I368) would populate this module.

---

## Recommended Scope for v0.13.1

**Must have (P0):** I365, I366, I367
**Must have (P1):** I368, I369, I370, I371, I372
**Nice to have (P2):** I373, I374

The P0 issues fix the plumbing: DailyOS sees the actual inbox, stays in sync, and enriches every email. The P1 issues deliver the intelligence: contextual synthesis, entity compounding, meeting context, thread accuracy. The P2 issues are refinements.

Full brief and acceptance criteria: `.docs/plans/v0.13.1.md`
