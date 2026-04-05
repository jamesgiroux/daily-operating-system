# I513 — DB as Sole Source of Truth for App-Generated State

**Priority:** P0
**Area:** Backend / Architecture
**Version:** v1.0.0 (Phase 1)
**Depends on:** I511 (schema decomposition — `entity_assessment` table must exist), I512 (ServiceLayer — all reads go through services)
**Blocks:** I508 (intelligence schema redesign needs clean read paths)
**Absorbs:** I436 (workspace file deprecation)

## Reframe: What I513 Is and Is Not

The original framing was "workspace file elimination." That's wrong. The workspace directory serves two distinct purposes:

1. **App-generated state** — `intelligence.json`, `dashboard.json`, `person.json`, `schedule.json`, `actions.json`, `preps/*.json`. These duplicate DB data on disk. They're the #1 source of "stale data" bugs (MEMORY.md: "stale skeleton prep files block real prep from loading"). This is what I513 eliminates as a *read path*.

2. **User-contributed knowledge** — markdown files, PDFs, decks, documents dropped into entity directories or `_inbox/`. These are source material that feeds intelligence. There is no alternative local storage for this with tools like Claude Desktop. This is core functionality that I513 does not touch.

**I513 makes the DB the sole *read path* for app-generated state.** The filesystem remains the read/write surface for user-contributed documents. App-generated files may continue to be *written* as human-readable backups or exports, but the app never *reads* them as a data source.

### The Principle

**Workspace files are a user-facing file system for knowledge management. App state lives in the DB. The two coexist — files feed intelligence, the DB stores the results.**

## What Stays (Explicitly Out of Scope)

| Category | Files | Why It Stays |
|----------|-------|-------------|
| User-contributed documents | `_inbox/*`, `Accounts/{Name}/*.md`, `*.pdf`, `*.docx`, `Documents/`, `_user/attachments/*` | No alternative local storage. Claude Desktop, manual drops, Google Drive exports produce files. The filesystem is the natural interface. |
| Processed transcripts | `Call-Transcripts/{date}-{slug}-transcript.md` | Permanent meeting records. Users read, annotate, share these. |
| Companion markdown | `.md` files alongside routed PDFs/DOCXs | Make non-text files readable in entity context. |
| Audit trail | `_audit/*.txt` | Compliance artifact, not app state. |
| Derivative work product | `Leadership/02-Performance/Weekly-Impact/*.md`, prep snapshots | User-facing, shareable documents. |
| `_inbox/` processing pipeline | `watcher.rs`, `processor/router.rs`, `processor/classifier.rs` | Core feature: user drops file → classify → route → index → feed intelligence. |
| `_user/context.json` | User entity serialized for AI context | Written by settings, consumed by context_provider. Small, stable, no stale-data risk. Evaluate later. |

## What Changes

### Category 1: Entity intelligence files — stop reading

**Files:** `intelligence.json` in every entity directory (Accounts, Projects, People).

**Current state:** `intel_queue.rs` writes `intelligence.json` after AI enrichment. 15 files read it: `services/accounts.rs`, `services/projects.rs`, `prepare/meeting_context.rs`, `reports/*.rs`, `people.rs`, `projects.rs`, `accounts.rs`, `mcp/main.rs`, etc. This is the primary intelligence data store alongside the DB's `entity_intelligence` table (which becomes `entity_assessment` after I511).

**Problem:** `intelligence.json` and `entity_intelligence` contain overlapping but not identical data. The JSON has the full LLM narrative; the DB has structured fields. Consumers read from whichever is convenient, creating divergence. After I511, `entity_assessment` has all the structured fields. The narrative text fields (`executive_assessment`, `value_delivered`, `relationship_depth`) are already columns — not JSON-only.

**Change:**
- All reads of `intelligence.json` replaced with `entity_assessment` queries via ServiceLayer
- `intel_queue.rs` continues writing `intelligence.json` as a human-readable backup (write-only)
- Alternatively, stop writing entirely if `entity_assessment` is complete. Decision during implementation: verify that every field in `intelligence.json` has a corresponding column in `entity_assessment`.

**Affected files (~15):**

| File | Current Read Pattern | New Read Pattern |
|------|---------------------|-----------------|
| `services/accounts.rs` | `read_intelligence_json()` for narrative fields in account detail | `services::intelligence::get_assessment(entity_id)` |
| `services/projects.rs` | `read_intelligence_json()` for project detail | Same |
| `prepare/meeting_context.rs` | `read_intelligence_json()` for meeting prep context | Same |
| `reports/account_health.rs` | `read_intelligence_json()` for report data | Same |
| `reports/swot.rs` | `read_intelligence_json()` for SWOT input | Same |
| `reports/ebr_qbr.rs` | `read_intelligence_json()` for EBR/QBR input | Same |
| `people.rs` | `read_intelligence_json()` for person markdown | Same |
| `projects.rs` | `read_intelligence_json()` for project markdown | Same |
| `accounts.rs` | `read_intelligence_json()` for account markdown | Same |
| `mcp/main.rs` | `read_intelligence_json()` for MCP tool responses | Same |
| `intelligence/io.rs` | `read_intelligence_json()` definition + `write_intelligence_json()` | Keep write, remove or deprecate read |
| `risk_briefing.rs` | May read intelligence for risk context | Same |

### Category 2: Entity dashboard/person files — stop reading

**Files:** `dashboard.json`, `dashboard.md` in Accounts/Projects dirs. `person.json`, `person.md` in People dirs.

**Current state:** These files are written by `accounts.rs`, `projects.rs`, `people.rs` as serialized views of DB data. The `watcher.rs` also watches for *external* edits to `dashboard.json` and `person.json` — if a user edits them outside the app, the watcher syncs changes back to the DB.

**Problem:** Dual source of truth. The DB has the canonical data, but some code paths read from the JSON file instead of the DB. `dashboard.md` and `person.md` are markdown renders used as AI context input — these are useful as intermediate artifacts but shouldn't be the primary data source.

**Change:**
- All app reads of `dashboard.json` and `person.json` go through DB queries via ServiceLayer
- Continue writing these files for: (a) human readability in entity dirs, (b) external tool compatibility, (c) watcher-based external edit sync
- `dashboard.md` and `person.md` continue to be written and consumed as AI context input (these are *derived views*, not *data sources* — acceptable)
- The watcher's reverse-sync (external edit → DB) remains — this is the "user-contributed" direction

**Affected files (~8):**

| File | Current Read Pattern | New Read Pattern |
|------|---------------------|-----------------|
| `services/accounts.rs` | `read_account_json()` for dashboard fields | DB query via `db::accounts::get_account()` |
| `services/projects.rs` | `read_project_json()` for dashboard fields | DB query via `db::projects::get_project()` |
| `services/people.rs` | `read_person_json()` for person fields | DB query via `db::people::get_person()` |
| `google.rs` | Reads account/person JSON for entity linking | DB query |
| `watcher.rs` | Reads JSON on external edit to sync to DB | Stays — this is the reverse direction |

### Category 3: Daily pipeline files — stop reading as data layer

**Files:** Everything in `_today/data/`: `schedule.json`, `actions.json`, `emails.json`, `preps/*.json`, `manifest.json`, `email-disposition.json`, `next-morning-flags.json`, `week-overview.json`.

**Current state:** `prepare/orchestrate.rs` assembles data from Google Calendar/Gmail and writes directive JSON files. `workflow/deliver.rs` processes AI output and writes result JSON files. `json_loader.rs` (1,225 lines) loads these files for the frontend. `services/dashboard.rs` reads through `json_loader.rs`. This is the primary data flow for the Daily Briefing, Week, and Meeting Detail pages.

**Problem:** `json_loader.rs` is an entire filesystem-based read layer that duplicates what the DB should provide. Stale `_today/data/` files from yesterday (before archive runs) show wrong data. The archive cycle introduces a window where data is absent. The DB already stores meetings, actions, and email data — the JSON files are a parallel system.

**Change — phased approach:**

This is the largest change. It requires the DB to store everything the frontend currently reads from JSON files. Some of this already exists (meetings, actions). Some doesn't (enriched email triage, briefing narrative).

**Phase A — Meetings + Actions (already in DB):**
- `schedule.json` data is already in `meetings` table (post-I511). Frontend reads from DB via `get_meetings_for_date()`.
- `actions.json` data is already in `actions` table. `workflow/today.rs::sync_actions_to_db()` already syncs JSON → DB. Remove the reverse: frontend reads from DB, not JSON.
- `preps/*.json` data is already in `prep_frozen_json` column on `meetings` (post-I511: `meeting_prep` table). Frontend reads from DB.

**Phase B — Emails + Briefing (EXTRACTED — separate issue):**
- **Extracted to a new issue (TBD).** Email triage, briefing narrative, and week overview have no DB schema today. This is new data model design work — not "move reads to DB." Requires its own schema spec with table designs, migration, and write-path changes in `workflow/deliver.rs`.
- The "DB as sole source" principle is 80% achieved with Phases A + C alone. The remaining email/briefing JSON reads are a known, bounded exception until the extracted issue ships.
- See "Phase B Extraction" section below for rationale.

**Phase C — Pipeline artifacts become ephemeral:**
- `today-directive.json`, `week-directive.json`, `email-refresh-directive.json` are *input* to the AI pipeline, not *output* for the app. These stay as ephemeral files: written, consumed by PTY, then archived. They're temporary IPC, not a data layer.
- `.email-context.json`, `.briefing-context.json` — same: ephemeral AI input.
- `manifest.json` — replaced by DB-based freshness tracking.
- `next-morning-flags.json` — move to DB (simple JSON column on a daily_state table or app_state).

**`json_loader.rs` disposition:**
- Phase A removes schedule, actions, and preps loading (~400 lines)
- Phase B (extracted) removes emails and briefing loading (~300 lines) — deferred
- Phase C removes manifest and utility functions (~200 lines)
- After A + C: `json_loader.rs` reduced to email/briefing loading only (~300 lines). Fully deleted when Phase B ships.

## Implementation Plan

### Sub-issues for parallelization

| Sub-issue | Title | Depends On | Can Parallel With |
|-----------|-------|-----------|-------------------|
| **I513a** | DB as sole read path for entity intelligence | I511b (entity_assessment exists) | I513c |
| **I513b** | DB as sole read path for daily pipeline data | I511a (meetings table exists), I512 (service reads) | I513a after Phase A |
| **I513c** | DB as sole read path for entity dashboards | I512 (service reads) | I513a |

### I513a — Entity intelligence (estimated: ~15 file changes)

1. Create `services::intelligence::get_assessment(entity_id)` that reads from `entity_assessment` table
2. Replace all `read_intelligence_json()` calls with the service method
3. Verify every field in `intelligence.json` has a column in `entity_assessment`. If any are missing, add them in the I511b migration.
4. Keep `write_intelligence_json()` as write-only backup (or remove if all fields are in DB)
5. Update `intel_queue.rs` write path: DB write is primary, file write is optional backup

### I513b — Daily pipeline data (estimated: ~14 file changes, phased)

**Phase A (meetings + actions — data already in DB):**
1. `services/dashboard.rs` reads schedule from `meetings` table, not `json_loader::load_schedule_json()`
2. `services/dashboard.rs` reads actions from `actions` table, not `json_loader::load_actions_json()`
3. `services/meetings.rs` reads prep from `meeting_prep` table, not `json_loader::load_meeting_prep_json()`
4. Remove corresponding `json_loader.rs` functions
5. `workflow/deliver.rs` still writes JSON (consumed by archive, but app doesn't read it)

**Phase B (emails + briefing — needs schema work):**
1. Design email triage storage (likely `email_triage` table or columns on existing email tables)
2. `workflow/deliver.rs::deliver_emails()` writes to DB, not just JSON
3. Store briefing narrative in DB (daily_briefing table or similar)
4. Store week overview in DB
5. Frontend reads all daily data from DB via services

**Phase C (cleanup):**
1. `manifest.json` → DB-based freshness (last_briefing_at timestamp)
2. `next-morning-flags.json` → DB column
3. Gut `json_loader.rs` — delete or reduce to stub
4. `_today/data/` becomes write-only pipeline scratch (directives in, archived nightly)

### I513c — Entity dashboards/people (estimated: ~8 file changes)

1. Ensure `services/accounts.rs`, `services/projects.rs`, `services/people.rs` read all data from DB
2. Remove `read_account_json()`, `read_project_json()`, `read_person_json()` calls from service code
3. Keep writing `dashboard.json`, `person.json` for external compatibility + watcher reverse-sync
4. Keep writing `dashboard.md`, `person.md` as derived markdown views for AI context

## Critical Cross-Cutting Concern: Signal Chain Integrity

I513 doesn't just move reads to the DB. It fixes two systemic problems that make CRUD buggy today.

### Problem 1: intel_queue.rs emits zero signals

The intelligence enrichment pipeline — the most important mutation in the system — writes to the DB with zero signal emissions:

```rust
// intel_queue.rs line 1062-1067 (current state)
write_intelligence_json(&input.entity_dir, &final_intel)?;  // file = canonical
let db = crate::db::ActionDb::open()?;
let _ = db.upsert_entity_intelligence(&final_intel);         // DB = "cache update"
// NO emit_signal(). NO emit_and_propagate(). Nothing.
```

Consequences:
- Intelligence update → no signal → no prep invalidation → meetings show stale briefings
- Intelligence update → no signal → no propagation → parent accounts don't refresh
- Intelligence update → no signal → no feedback loop → source reliability weights don't update

**Fix (requires I512):** `intel_queue.rs` must call `services::intelligence::upsert_assessment()` instead of raw `db.upsert_entity_intelligence()`. The service method emits `entity_intelligence_updated` signal via `emit_signal_and_propagate()`. This is not captured in I512's spec (which focuses on `commands.rs` direct DB calls) — I512 must be expanded to cover background processors too, not just command handlers.

### Problem 2: Watcher feedback loop on app-generated files

The watcher treats all file changes identically — user drops and app writes:

```
intel_queue.rs writes intelligence.json to Accounts/Acme/
  → watcher detects AccountContent change
  → enqueues IntelPriority::ContentChange for Acme
  → intel_queue.rs runs again, writes intelligence.json again
  → (loop broken only by queue deduplication)
```

Similarly, when `services/accounts.rs` writes `dashboard.json` after a DB update, the watcher detects it and tries to sync it *back* to the DB — redundant work that could cause subtle race conditions.

**Fix (part of I513):** When app-generated file writes are eliminated or reduced, the feedback loop dies. Specifically:

- **I513a:** If `intelligence.json` stops being written (or is write-only backup), the watcher no longer triggers re-enrichment on intel updates.
- **I513c:** `dashboard.json` and `person.json` continue to be written for external compatibility. The watcher needs a **write guard** — a flag that suppresses watcher events during app-initiated file writes. Pattern: set `self.writing = true` before write, clear after. Watcher ignores events while flag is set.

### The Complete Signal Chain (Post I511 + I512 + I513)

After all three Phase 1 issues ship, the mutation → signal → propagation chain should be:

```
User edits account field
  → services::accounts::update_field()
  → DB write (entity_assessment / accounts table)
  → emit_signal_and_propagate("field_updated", "user_edit", 0.8)
  → propagation rules: invalidate prep for next meeting with this account
  → MeetingPrepQueue: regenerate affected briefings
  → Frontend: Tauri event → re-render

Intelligence enrichment completes
  → services::intelligence::upsert_assessment()
  → DB write (entity_assessment table)
  → emit_signal_and_propagate("entity_intelligence_updated", "ai_enrichment", 0.7)
  → propagation rules: invalidate prep, enqueue parent refresh
  → MeetingPrepQueue: regenerate affected briefings
  → Frontend: Tauri event → re-render

User drops file in entity dir
  → watcher detects AccountContent change
  → enqueue embedding + intel refresh (unchanged — correct behavior)
  → intel_queue.rs runs enrichment
  → (chain above)
```

### I512 Covers Signal Chain (Resolved)

The revised I512 spec explicitly scopes its audit crate-wide — not just commands.rs. The six hotspot files (`commands.rs`, `intel_queue.rs`, `transcript.rs`, `deliver.rs`, `reconcile.rs`, `hygiene.rs`) are all in I512's scope with zero direct DB mutation calls as an acceptance criterion. The signal chain integrity concerns identified here are now I512 deliverables.

I513's signal chain ACs (21-25 below) verify the end-to-end chain works after I512 delivers the signal wiring.

## Risk Assessment

**Highest risk: I513b Phase B.** Email triage and briefing narrative have no DB schema today. This requires new tables and new write paths in `workflow/deliver.rs`. This is new schema work beyond I511's scope.

**Mitigation:** I513b Phase A (meetings + actions) can ship independently — these already have DB storage. Phase B is a separate PR with its own schema migration.

**Medium risk: I513a.** `intelligence.json` may contain fields not in `entity_assessment`. The I511b migration defines the column set, but the LLM output is freeform JSON. Need to verify completeness.

**Mitigation:** During I513a implementation, diff the `intelligence.json` schema against `entity_assessment` columns. Any gaps become additional columns in a follow-up migration.

**Low risk: I513c.** Dashboard and person JSON reads are already thin wrappers around DB data. The files are written *from* DB data, then read *back*. Cutting the read-back is straightforward.

## Acceptance Criteria

### I513a — Entity intelligence
1. Zero calls to `read_intelligence_json()` from any service, command handler, report, or prep module
2. `services::intelligence::get_assessment(entity_id)` returns all fields previously read from `intelligence.json`
3. `intel_queue.rs` writes to `entity_assessment` table as primary; `intelligence.json` write is optional backup
4. Account detail page renders executive assessment, risks, stakeholder insights — sourced from DB, not file
5. Meeting prep context includes entity intelligence — sourced from DB, not file
6. Reports (account_health, swot, ebr_qbr) render with intelligence data — sourced from DB, not file
7. Delete `intelligence.json` from an entity dir. App still shows all intelligence data. (Critical test: file absence doesn't break anything.)

### I513b — Daily pipeline data (Phase A + C only; Phase B extracted)
8. (Phase A) Daily Briefing page renders schedule from `meetings` table, not `schedule.json`
9. (Phase A) Actions page renders from `actions` table, not `actions.json`
10. (Phase A) Meeting detail page renders prep from `meeting_prep` table, not `preps/*.json`
11. (Phase A) Delete `_today/data/schedule.json`. Briefing page still renders today's meetings.
12. (Phase C) `manifest.json` replaced by DB-based freshness tracking. `next-morning-flags.json` moved to DB.
13. (Phase C) `_today/data/` files (except emails.json and briefing) are write-only pipeline artifacts — app never reads them as data source
14. `json_loader.rs` reduced to email/briefing loading only. Schedule, actions, preps, manifest loading removed.
~~15. (Phase B — EXTRACTED) Email triage and briefing narrative storage. Tracked in separate issue.~~

### I513c — Entity dashboards
16. Account detail loads all data from DB, not `dashboard.json`
17. Project detail loads all data from DB, not `dashboard.json`
18. Person detail loads all data from DB, not `person.json`
19. Entity JSON files still written for external tool compatibility
20. Watcher reverse-sync (external edit → DB) still works

### Signal chain integrity
21. `intel_queue.rs` writes intelligence through `services::intelligence::upsert_assessment()`, not raw `db.upsert_entity_intelligence()`. Signal emitted on every enrichment.
22. Enrichment completes → signal emitted → prep invalidated for entity's next meeting → briefing regenerated. (End-to-end test: enrich account, verify meeting briefing updates within one cycle.)
23. Watcher does not trigger re-enrichment when app writes `intelligence.json` (feedback loop eliminated — either by not writing the file, or by write-guard suppression).
24. Watcher does not trigger redundant DB sync when app writes `dashboard.json` or `person.json` (write-guard or equivalent).
25. User drops file in entity dir → watcher → enqueue intel refresh → enrichment → signal → prep invalidation. Full chain works.

### Inline editing integrity
26. All editable fields on entity detail pages (account, project, person) save correctly and persist on page reload — no split-brain between file and DB
27. EditableText and EditableList components on entity pages write to DB via ServiceLayer, not to JSON files on disk
28. Editing an intelligence field on any entity detail page saves immediately and is visible after navigating away and back

### Cross-cutting
29. App works end-to-end: briefing renders, meeting detail renders, account detail renders, enrichment runs
30. `cargo test` — all pass
31. `cargo clippy -- -D warnings` — clean
32. `pnpm tsc --noEmit` — clean

## What This Does NOT Do

- **Does not remove entity directories.** Directories stay. User files stay. The workspace is still a file system for knowledge management.
- **Does not remove the `_inbox/` pipeline.** File drop → classify → route → index → enrich is core functionality.
- **Does not remove `watcher.rs`.** The watcher monitors for user-contributed file changes. It stays.
- **Does not remove `dashboard.md` / `person.md` writes.** These derived markdown files are useful as AI context input and human-readable exports. They're derived views, not data sources.
- **Does not remove directive files.** `today-directive.json` and friends are ephemeral AI pipeline input — temporary IPC, not a data layer. They're written, consumed, archived.
- **Does not change the daily archive cycle.** `workflow/archive.rs` still archives `_today/` content nightly.
- **Does not require new frontend types.** If services return the same data shapes, frontend changes are zero. The Tauri command handlers join internally.

## Phase B Extraction Rationale

Phase B (email triage + briefing narrative + week overview → DB) was originally scoped as part of I513's "daily pipeline data" elimination. After senior engineer review, it was extracted because:

1. **It's new schema design, not read-path migration.** Phases A and C move reads from files to existing DB tables. Phase B requires designing and implementing new tables (`email_triage`, `daily_briefing`, `week_overview` or similar) that don't exist anywhere in the current schema or spec. That's a different class of work.

2. **The 80% principle.** Phases A + C eliminate entity intelligence file reads, schedule/actions/preps file reads, and make pipeline artifacts ephemeral. The remaining email/briefing JSON reads are a known, bounded exception — not a data integrity risk.

3. **Independent deliverability.** Phases A + C can ship and be verified without Phase B. Phase B can ship later (early Phase 2 or as a standalone issue) without invalidating anything A + C delivered.

The extracted issue should include: email triage table schema design, briefing narrative storage, week overview storage, write-path changes in `workflow/deliver.rs`, and the final `json_loader.rs` deletion.

## MCP Sidecar Compatibility

`mcp/main.rs` reads `intelligence.json` from the filesystem via `read_entity_intelligence()`. The MCP sidecar is a separate binary without the main app's DB connection.

**Decision: Continue writing `intelligence.json` as a human-readable backup.**

- `intel_queue.rs` continues to call `write_intelligence_json()` after DB write
- The DB write (`entity_assessment`) is the canonical source for the main app
- The file write is the read source for the MCP sidecar and for human inspection
- I513a eliminates file *reads* from the main app, not file *writes*
- When the MCP sidecar gets DB access (future work, not v1.0.0), the file write becomes truly optional

This means `intelligence.json` in entity dirs is not deleted, not deprecated — it's reclassified from "canonical data source" to "derived output for external consumers."

## Relationship to Other Issues

- **I511** — Creates `entity_assessment` table that I513a reads from. Creates `meetings` + `meeting_prep` tables that I513b reads from. Hard dependency.
- **I512** — Ensures all reads go through ServiceLayer. I513 implements the "read from DB" side of that contract. Soft dependency (can start I513a before I512 completes if service methods exist).
- **I514** — Module decomposition. Independent but complementary. I513 removes `json_loader.rs` (or guts it); I514 splits `commands.rs`. No conflict.
- **I436** — Absorbed. I436 was "workspace file deprecation." I513 is the refined version with clear boundaries.
