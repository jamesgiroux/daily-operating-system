# Cross-Cutting Gap Analysis

**Status**: Authoritative -- synthesized 2026-03-02 from architecture audit artifacts.

**Source artifacts**:
- `DATA-MODEL.md` -- Schema inventory, duplication issues, missing indexes/constraints
- `PIPELINES.md` -- Async pipeline flows, error handling, concurrency, testing
- `MODULE-MAP.md` -- Module dependency graph, boundary analysis, code health
- `DATA-FLOWS.md` -- End-to-end data flow diagrams, known issues
- `LIFECYCLES.md` -- State machines for all domain objects, transition gaps
- `COMMAND-REFERENCE.md` -- IPC command inventory, signal coverage analysis
- `SIGNAL-SCORING-REFERENCE.md` -- Entity resolution scoring, auto-link guardrails

---

## Table of Contents

1. [Critical Issues (P0)](#1-critical-issues-p0)
2. [Functional Gaps (P1)](#2-functional-gaps-p1)
3. [Architectural Debt (P2)](#3-architectural-debt-p2)
4. [Signal Bus Coverage Gaps](#4-signal-bus-coverage-gaps)
5. [Data Consistency Risks](#5-data-consistency-risks)
6. [Testing Gaps](#6-testing-gaps)
7. [Performance Risks](#7-performance-risks)
8. [Recommended Remediation Roadmap](#8-recommended-remediation-roadmap)

---

## 1. Critical Issues (P0)

Issues that could cause data loss, corruption, or security vulnerabilities.

---

### P0-1: Broken FK in `drive_watched_sources` (Migration 048)

**Description**: Migration `048_google_drive_sync.sql` declares a foreign key referencing `entity_intel(entity_id)`, which is not a table name. The correct table is `entity_intelligence`. SQLite silently ignores FK references to nonexistent tables, so the constraint is never enforced.

**Source artifacts**: DATA-MODEL.md (Section 4: Schema Health, P0 Active Bugs), DATA-MODEL.md (Section 2: Relationship Map, explicit FK list)

**Impact**: Every `drive_watched_sources` row can reference a nonexistent entity. If an entity is deleted, its Drive watch records persist as orphans. Drive sync operations may attempt to associate files with entities that no longer exist.

**Recommended fix**: Migration 054 should `ALTER TABLE drive_watched_sources` to drop and recreate the FK with the correct table name. Run a cleanup pass to delete rows whose `entity_id` has no match in `entity_intelligence`.

---

### P0-2: Silent Signal Failures in Transcript Processing

**Description**: In `src-tauri/src/processor/transcript.rs`, the transcript pipeline uses `let _ =` to silently discard errors from three critical operations: capture insertion (`insert_capture`), signal emission (`emit_signal`), and impact log append. A failed signal emission means the propagation engine, prep invalidation, and self-healing subsystems are never notified that a transcript was processed.

**Source artifacts**: PIPELINES.md (Pipeline 4: Transcript Processing, Error Handling -- three `[SILENT FAILURE]` tags), DATA-FLOWS.md (Section 4: User Action to Signal flow -- signal bus is the cascade trigger)

**Impact**: Post-meeting intelligence data (wins, risks, decisions) can be lost without any indication. The user sees a "processed" status in the UI, but the system's intelligence layer has no record of the outcomes. Downstream effects: entity intelligence is not refreshed, meeting prep for follow-up meetings is not invalidated, reports are not marked stale.

**Recommended fix**: Replace `let _ =` with explicit error logging and a partial-failure status returned to the caller. At minimum, signal emission failures should log at `error` level and set a flag on the `processing_log` entry indicating incomplete processing.

---

### P0-3: Silent Signal Bus Failure on Future Meeting Flagging

**Description**: In `src-tauri/src/signals/bus.rs`, `emit_signal()` uses `let _ =` to discard errors when setting `has_new_signals = 1` on future meetings linked to a signal's entity. If this UPDATE fails (SQLite busy, concurrent write, lock timeout), future meetings are never flagged, and the 30-minute pre-meeting auto-refresh in the scheduler will not detect that the meeting needs attention.

**Source artifacts**: PIPELINES.md (Pipeline 2: Signal Bus, Error Handling -- `[SILENT FAILURE]` on flag future meetings)

**Impact**: Users may enter meetings with stale prep because the pre-meeting refresh did not fire. This is a silent data staleness issue with no user-visible indicator.

**Recommended fix**: Propagate the error or retry the flag operation. At minimum, log at `warn` level with the meeting IDs that could not be flagged.

---

### P0-4: Prep Invalidation Queue Lock Poison Silently Drops Invalidations

**Description**: The `prep_invalidation_queue` (`Mutex<Vec<String>>`) in `signals/invalidation.rs` silently returns if the mutex is poisoned. The scheduler drains this queue every 60 seconds to trigger prep regeneration. If the mutex poisons (panic in another thread that held the lock), ALL subsequent prep invalidation requests are silently dropped for the remainder of the application session.

**Source artifacts**: PIPELINES.md (Pipeline 2: Signal Bus, Error Handling -- `[SILENT FAILURE]` on prep queue lock)

**Impact**: Once poisoned, no signal-driven prep invalidation can occur until the app restarts. The user sees stale meeting preps with no indication of the underlying failure.

**Recommended fix**: Use `mutex.lock().unwrap_or_else(|e| e.into_inner())` to recover from poison, or replace with a `tokio::sync::Mutex` that does not have the poison concept.

---

## 2. Functional Gaps (P1)

Features that are partially implemented, dead code paths, or disconnected subsystems.

---

### P1-1: Entity CRUD Operations Bypass Signal Bus Entirely

**Description**: The majority of entity create/update/delete commands write directly to the database without emitting signals. This means the propagation engine, prep invalidation, self-healing, and report staleness detection are not triggered by these operations.

**What exists**: Signal emission is implemented for: action state changes (`complete_action`, `reopen_action`, `accept_proposed_action`, `reject_proposed_action`, `update_action_priority`), entity context entries (CRUD), person relationship changes, and transcript processing.

**What is missing**: No signal emission from:
- `create_account`, `update_account_field`, `archive_account`, `merge_accounts`
- `create_project`, `update_project_field`, `update_project_notes`, `archive_project`
- `create_person`, `update_person`, `delete_person`, `merge_people`
- `create_action`, `update_action`
- `update_capture`
- `link_person_entity`, `unlink_person_entity`
- `link_meeting_entity`, `unlink_meeting_entity`
- `add_account_team_member`, `remove_account_team_member`
- `record_account_event`
- `update_intelligence_field`, `update_stakeholders`
- `bulk_create_accounts`, `bulk_create_projects`

**Source artifacts**: COMMAND-REFERENCE.md (Signal Coverage Gaps section), LIFECYCLES.md (Entity Lifecycle -- no archival cascade), DATA-FLOWS.md (Section 4 -- user correction flow shows only correction-type signals)

**Impact on user experience**: When a user updates an account's health status, edits a person's role, or archives an entity, downstream intelligence is not refreshed. Meeting preps for affected entities remain stale. Reports are not marked stale. The system "forgets" the change from an intelligence perspective until the next scheduled enrichment cycle.

**Recommended approach**: Introduce `emit_signal_and_propagate()` calls for the ~25 missing commands. Prioritize by user impact:
1. Entity mutations (`update_account_field`, `update_project_field`, `update_person`) -- these are the most common user actions
2. Structural changes (`link_meeting_entity`, `archive_*`, `merge_*`) -- these affect entity graph topology
3. Bulk operations (`bulk_create_*`) -- lower frequency but high data volume

---

### P1-2: `captures.owner` and `captures.due_date` Are Dead Schema Columns

**Description**: The `captures` table has `owner` (TEXT) and `due_date` (TEXT) columns in the SQL schema, but the Rust `DbCapture` struct in `src-tauri/src/db/signals.rs` does not include these fields. No SELECT query reads them; no INSERT populates them.

**Source artifacts**: DATA-MODEL.md (Section 4: Schema Health, "Columns written but never read"), LIFECYCLES.md (Section 7: Action/Capture Lifecycle -- captures have no status progression)

**Impact**: Post-meeting capture processing cannot assign owners or due dates to wins, risks, or decisions, even though the schema supports it. This is a product-level gap: captures have no accountability tracking.

**Recommended approach**: Either add `owner` and `due_date` to `DbCapture` and wire them through the transcript processing pipeline, or drop the columns via migration to reduce schema noise.

---

### P1-3: No Meeting Deletion/Cancellation State

**Description**: When a Google Calendar event is deleted, the corresponding `meetings_history` row persists with no deletion marker. There is no explicit "cancelled" or "deleted" state in the `intelligence_state` lifecycle.

**Source artifacts**: LIFECYCLES.md (Section 3: Meeting Lifecycle, Known Gaps -- "No explicit Cancelled state"), DATA-FLOWS.md (Section 2: Calendar to Meeting Prep flow -- upserts only)

**Impact**: Deleted meetings accumulate in the database. The meeting prep queue may process prep for meetings that no longer exist on the calendar. The user sees ghost meetings in historical views.

**Recommended approach**: Add a `cancelled_at` column to `meetings_history`. During calendar sync, mark meetings whose `calendar_event_id` is no longer in the fetched events as cancelled. Exclude cancelled meetings from prep sweep, enrichment, and dashboard queries.

---

### P1-4: No Time-Based Prep Expiry

**Description**: Mechanical prep generated a week before a meeting is not automatically refreshed as the meeting approaches and new signals accumulate. The only triggers for prep refresh are: signal-driven invalidation (confidence >= 0.7), manual user refresh, and boot sweep (only for meetings with no prep at all).

**Source artifacts**: LIFECYCLES.md (Section 3: Meeting Lifecycle, Known Gaps -- "No time-based prep expiry"), PIPELINES.md (Pipeline 3: Meeting Prep -- debounce and priority levels)

**Impact**: A meeting prep generated on Monday for a Thursday meeting will not reflect Tuesday/Wednesday activity unless a high-confidence signal happens to fire. Low-confidence signals (which don't trigger invalidation) accumulate silently.

**Recommended approach**: Add a staleness check to the 30-minute pre-meeting scheduler sweep in `scheduler.rs`. If `prep_frozen_json` is older than a configurable threshold (e.g., 12 hours) and the meeting is within 2 hours, enqueue a prep refresh at `PageLoad` priority.

---

### P1-5: No Capture Deletion or Archival Pathway

**Description**: The `captures` table has no delete or archive mechanism. Captures accumulate indefinitely with no way for users to remove incorrect or irrelevant entries.

**Source artifacts**: LIFECYCLES.md (Section 7: Action/Capture Lifecycle, Known Gaps -- "No capture deletion"), DATA-MODEL.md (captures table -- no archived/deleted column)

**Impact**: Over time, stale captures pollute account dashboards and meeting history views. Users cannot correct AI extraction errors in the capture layer.

**Recommended approach**: Add `archived_at` column to `captures`. Add `archive_capture` and `delete_capture` IPC commands.

---

### P1-6: No Archival Cascade on Entities

**Description**: Archiving an account does not cascade to child projects, linked people, pending actions, or entity_intelligence rows. These remain active in queries and enrichment pipelines.

**Source artifacts**: LIFECYCLES.md (Section 1: Entity Lifecycle, Known Gaps -- "No archival cascade"), DATA-MODEL.md (Section 2: accounts FK -- child projects have no CASCADE on archive)

**Impact**: An archived account's projects continue appearing in dashboards, actions remain in the active queue, and enrichment budget is consumed on entities the user has explicitly dismissed.

**Recommended approach**: When archiving an account, cascade `archived=1` to child projects and cancel/archive pending actions linked to the account. Exclude archived entity IDs from `IntelligenceQueue` enqueue and `MeetingPrepQueue` sweep.

---

### P1-7: Keyword Extraction Is One-Shot

**Description**: Entity keywords (`accounts.keywords`, `projects.keywords`) are extracted once during initial setup and stored with a `keywords_extracted_at` timestamp. They are never refreshed as entity context evolves through meetings, signals, and enrichment.

**Source artifacts**: LIFECYCLES.md (Section 1: Entity Lifecycle, Known Gaps -- "Keyword extraction is one-shot")

**Impact**: Entity auto-linking accuracy degrades over time as the keyword set becomes stale relative to current entity activity. The entity resolver (`src-tauri/src/prepare/entity_resolver.rs`) uses keywords with a base confidence of 0.65-0.80 (SIGNAL-SCORING-REFERENCE.md), so stale keywords can cause both false positives and missed links.

**Recommended approach**: Re-extract keywords during intelligence enrichment (post-PTY). Add a keyword refresh step to `write_enrichment_results()` in `intel_queue.rs`.

---

### P1-8: Enrichment Staleness Uses Flat 30-Day Window

**Description**: Person enrichment checks a single 30-day TTL regardless of enrichment source. A person enriched by Gravatar 29 days ago is not re-enriched even if Clay data (higher-priority source) is available.

**Source artifacts**: LIFECYCLES.md (Section 2: Person Lifecycle, Known Gaps -- "flat 30-day window")

**Impact**: Higher-quality enrichment sources are blocked by lower-quality prior enrichment. Clay data that could provide LinkedIn URLs, title history, and company intelligence is not fetched because Gravatar already touched the record.

**Recommended approach**: Implement per-source TTLs. Clay enrichment should be eligible after 90 days regardless of Gravatar freshness. Use the `enrichment_sources` JSON provenance map to check per-source staleness.

---

## 3. Architectural Debt (P2)

God modules, boundary violations, missing abstractions, inconsistent patterns.

---

### P2-1: `commands.rs` Is a 8940-Line God Module

**Current state**: `src-tauri/src/commands.rs` contains ~220 `#[tauri::command]` functions in a single file. While each function is a thin delegate to `services/`, the file itself is enormous and navigation-hostile.

**Target state**: Split into domain-scoped command files: `commands/accounts.rs`, `commands/meetings.rs`, `commands/people.rs`, `commands/intelligence.rs`, etc. with a `commands/mod.rs` re-export.

**Migration path**: Mechanical extraction -- move command functions into domain files, update `lib.rs` handler registrations to import from submodules.

**Effort estimate**: M (medium -- large file, but no logic changes needed)

**Source artifacts**: MODULE-MAP.md (Modules that do too much, #1), COMMAND-REFERENCE.md (8940 lines noted)

---

### P2-2: `hygiene.rs` Is Five Modules in One

**Current state**: `src-tauri/src/hygiene.rs` (~1800+ lines) combines: mechanical fixes, duplicate detection, overnight scanning, name resolution, domain linking, meeting readiness checks, and the background loop.

**Target state**: Extract into `hygiene/` directory with: `duplicates.rs`, `name_resolution.rs`, `readiness.rs`, `scanner.rs`, `mod.rs`.

**Migration path**: Extract functions into domain files. The hygiene background loop remains in `scanner.rs`.

**Effort estimate**: M

**Source artifacts**: MODULE-MAP.md (Modules that do too much, #2)

---

### P2-3: `meetings_history` Table Has 26 Columns Across 8 Migrations

**Current state**: A single table serves as: calendar event identity, user layer (agenda, notes), AI prep (two overlapping blobs), snapshot pointers, transcript pointers, and intelligence lifecycle state (5 columns).

**Target state**: Decompose into: `meetings` (identity + calendar fields), `meeting_prep` (prep_frozen_json, prep_context_json, prep_frozen_at, prep_snapshot_*), `meeting_intelligence` (intelligence_state, intelligence_quality, last_enriched_at, signal_count, has_new_signals, last_viewed_at), `meeting_user_layer` (user_agenda_json, user_notes).

**Migration path**: Create new tables, populate from existing columns, update all read/write paths. This is a major refactor but prevents further column accretion.

**Effort estimate**: L (large -- touches DB layer, services, commands, and frontend)

**Source artifacts**: DATA-MODEL.md (Section 4: Tables Beyond Original Design, #1)

---

### P2-4: `entity_intelligence` Mixes Operational State with AI Output

**Current state**: 21 columns spanning AI output fields (`executive_assessment`, `risks_json`, etc.), operational state (`coherence_score`, `coherence_flagged`), and report-layer fields (`health_score`, `health_trend`, `value_delivered`, etc.) that duplicate the purpose of the `reports` table.

**Target state**: Split AI output from operational state. Move report-layer fields into the `reports` pipeline (I508 intelligence schema redesign is already planned for v1.1.0).

**Migration path**: Align with I508. The `health_score`/`health_trend`/`value_delivered`/`success_metrics`/`open_commitments`/`relationship_depth` columns should be absorbed into the health scoring architecture (ADR-0097, I499-I503).

**Effort estimate**: L (coupled with v1.1.0 intelligence foundation work)

**Source artifacts**: DATA-MODEL.md (Section 4: Tables Beyond Original Design, #2)

---

### P2-5: Three Representations of Account-Person Relationships

**Current state**: `entity_people` (generic junction), `account_team` (structured with role), and `people.organization` + `people.role` (denormalized on person) all represent the relationship between people and accounts. Code in `db/accounts.rs::add_account_team_member` writes to `account_team` but may not always sync `entity_people`.

**Target state**: `account_team` is the canonical source for team membership. `entity_people` serves as a generic bridge for signal resolution. The two must be explicitly kept in sync, or `entity_people` should derive from `account_team` via a VIEW or trigger.

**Migration path**: Audit all write paths to `entity_people` and `account_team`. Add a sync step to `add_account_team_member` and `remove_account_team_member`. Consider an SQLite TRIGGER.

**Effort estimate**: S

**Source artifacts**: DATA-MODEL.md (Section 3: Issue 1)

---

### P2-6: Dual Prep JSON Columns (`prep_context_json` / `prep_frozen_json`)

**Current state**: `meetings_history.prep_context_json` is the legacy column, `prep_frozen_json` is the current authoritative source. `load_meeting_prep_from_sources` reads `prep_frozen_json` first. The old column is still written by legacy code paths.

**Target state**: Deprecate `prep_context_json`. Migrate all reads and writes to `prep_frozen_json`. Eventually drop the column.

**Migration path**: Grep for all `prep_context_json` references, redirect to `prep_frozen_json`. Add a migration that drops the column after confirming no remaining references.

**Effort estimate**: S

**Source artifacts**: DATA-MODEL.md (Section 3: Issue 5, Section 4: P2 debt)

---

### P2-7: Vestigial Tables (`chat_sessions`, `chat_turns`)

**Current state**: Created in migration `007_chat_interface.sql`. No active read or write paths in the current codebase.

**Target state**: Drop tables in a migration.

**Migration path**: Confirm no code references (grep), add a migration dropping both tables.

**Effort estimate**: S

**Source artifacts**: DATA-MODEL.md (Section 4: Vestigial tables)

---

### P2-8: `is_internal` / `account_type` Column Redundancy

**Current state**: `accounts.is_internal` (INTEGER boolean) and `accounts.account_type` (TEXT enum) are kept in sync by application code. They represent the same information.

**Target state**: Drop `is_internal`. Use `account_type = 'internal'` as the sole representation.

**Migration path**: Replace all `is_internal` reads with `account_type = 'internal'` checks. Drop column and index in migration.

**Effort estimate**: S

**Source artifacts**: DATA-MODEL.md (Section 3: Issue 9, Section 4: P1 gap)

---

### P2-9: `risk_briefing.rs` Duplicates Report Infrastructure

**Current state**: `risk_briefing.rs` implements its own two-phase gather/generate pattern, separate from the `reports/` module which provides the same pattern generically. The risk briefing reads/writes `entity_intelligence` directly instead of using the `reports` table.

**Target state**: Absorb `risk_briefing.rs` into `reports/risk.rs`. Use the `reports` table for storage.

**Migration path**: The `reports/risk.rs` module already exists. Migrate the remaining callers of `risk_briefing.rs` functions to use `generate_report(entity_id, "risk_briefing")`.

**Effort estimate**: S

**Source artifacts**: MODULE-MAP.md (reports/ and risk_briefing.rs listed as separate modules with overlapping dependencies)

---

## 4. Signal Bus Coverage Gaps

Synthesis of COMMAND-REFERENCE signal gaps, PIPELINES reliability issues, and LIFECYCLES known gaps.

---

### User Actions That Bypass the Signal Bus Entirely

These are user-initiated mutations that write to the database without emitting any signal. The propagation engine, prep invalidation, self-healing re-enrichment, and report staleness detection are not triggered.

| Action Category | Commands | Tables Written | Expected Signal |
|----------------|----------|---------------|-----------------|
| **Account CRUD** | `create_account`, `update_account_field`, `archive_account`, `merge_accounts`, `record_account_event` | accounts, account_events | `account_created`, `account_updated`, `account_archived`, `account_merged`, `account_event_recorded` |
| **Project CRUD** | `create_project`, `update_project_field`, `update_project_notes`, `archive_project` | projects | `project_created`, `project_updated`, `project_archived` |
| **People CRUD** | `create_person`, `update_person`, `delete_person`, `merge_people` | people, entity_people | `person_created`, `person_updated`, `person_deleted`, `person_merged` |
| **Team Changes** | `add_account_team_member`, `remove_account_team_member` | account_team | `team_member_added`, `team_member_removed` (note: these signal types exist in the invalidation list but are never emitted) |
| **Meeting Links** | `link_meeting_entity`, `unlink_meeting_entity`, `update_meeting_entity`, `add_meeting_entity`, `remove_meeting_entity` | meeting_entities, meetings_history | Should emit entity resolution signals |
| **Person Links** | `link_person_entity`, `unlink_person_entity` | entity_links | `entity_person_linked`, `entity_person_unlinked` |
| **Action CRUD** | `create_action`, `update_action` | actions | `action_created`, `action_updated` |
| **Intelligence Edits** | `update_intelligence_field`, `update_stakeholders` | entity_intelligence | `intelligence_edited` |
| **Bulk Ops** | `bulk_create_accounts`, `bulk_create_projects`, `populate_workspace`, `backfill_historical_meetings` | varies | Should batch-emit after completion |

### Dead Propagation Rules

The following propagation signal types are listed in `signals/invalidation.rs` as prep-invalidating but have no confirmed emission source in the current command layer:

| Signal Type | Listed As Invalidating | Emitted By |
|-------------|----------------------|------------|
| `team_member_added` | Yes | **Never emitted** -- `add_account_team_member` has no signal call |
| `team_member_removed` | Yes | **Never emitted** -- `remove_account_team_member` has no signal call |
| `stakeholders_updated` | Yes | Only emitted by `rule_person_profile_discovered` propagation (Clay/Gravatar enrichment path) -- never by direct user edit to stakeholders |

### Signal Reliability Issues

From PIPELINES.md error handling analysis:

1. **Silent future-meeting flagging failure** (bus.rs): `has_new_signals` UPDATE uses `let _ =` (P0-3 above)
2. **Silent prep invalidation queue poison** (invalidation.rs): Mutex poison silently drops all invalidations (P0-4 above)
3. **Synchronous propagation blocking**: All 9 propagation rules fire inline within the caller's DB transaction. A slow or panicking rule blocks the caller and all other rules. No timeout, no panic catch. (PIPELINES.md Pipeline 2, Concurrency section)
4. **No signal garbage collection**: `signal_events` grows unbounded. Decayed signals are never deleted. (LIFECYCLES.md Section 5, Known Gaps)
5. **Supersession is manual**: The system does not automatically detect when a new signal should supersede an old one. Callers must explicitly call `supersede_signal()`. (LIFECYCLES.md Section 5, Known Gaps)

---

## 5. Data Consistency Risks

Where data can get out of sync, synthesized from DATA-MODEL duplication issues, DATA-FLOWS silent failure points, and LIFECYCLES missing transitions.

---

### DC-1: `entity_people` and `account_team` Divergence

**Tables**: `entity_people`, `account_team`

`add_account_team_member` writes to `account_team` but may not update `entity_people`. Signal resolution and prep context use `entity_people` to look up linked entities. If the two tables diverge, a person who is on an account's team (visible in account detail) may not be recognized as associated with that account during meeting entity resolution.

**Source**: DATA-MODEL.md Issue 1, SIGNAL-SCORING-REFERENCE.md (attendee organization matching relies on entity_people)

---

### DC-2: `accounts.contract_end` vs `account_events` Renewal Dates

**Tables**: `accounts`, `account_events`

Recording a renewal via `record_account_event` writes to `account_events` and recalculates ARR on the `accounts` row, but `contract_end` is not updated from the new event's date. Proactive detectors (`detect_renewal_gap`, `detect_renewal_proximity`) read `contract_end`, so a logged renewal may not update the renewal monitoring logic.

**Source**: DATA-MODEL.md Issue 2

---

### DC-3: `meetings_history.attendees` Blob vs `meeting_attendees` Table

**Tables**: `meetings_history`, `meeting_attendees`

Both are written during calendar sync, but a partial sync failure could leave them inconsistent. Some display code reads the JSON blob; entity resolution reads the junction table. A person appearing in one but not the other creates display vs. intelligence divergence.

**Source**: DATA-MODEL.md Issue 3

---

### DC-4: `entities` Mirror Table Staleness

**Tables**: `entities`, `accounts`, `projects`, `people`

Any direct SQL UPDATE to `accounts` or `projects` (bypassing `upsert_account`/`upsert_project`) leaves the `entities` mirror stale. `entities.updated_at` is used as a last-contact signal in `get_stakeholder_signals`, so staleness here affects displayed relationship temperature and proactive detection.

**Source**: DATA-MODEL.md Issue 7

---

### DC-5: Batch PTY Enrichment Cross-Entity Corruption

**Scenario**: `intel_queue.rs` batches up to 3 entities per PTY call. If the AI produces a hallucination or malformed response for entity 2, the parse failure can corrupt or discard the output for entities 1 and 3.

**Source**: LIFECYCLES.md (Section 4: Intelligence Lifecycle, Known Gaps -- "Batch enrichment shares one PTY call")

---

### DC-6: Report Staleness Detection Misses Non-Assessment Changes

**Tables**: `reports`, `entity_intelligence`

`intel_hash` is computed from `enriched_at + executive_assessment` only. If risks, wins, stakeholder insights, or other intelligence fields change without updating `enriched_at` or the assessment text, the hash remains the same and the report is not marked stale.

**Source**: DATA-FLOWS.md (Section 6: Report Generation, Known Issues -- "intel hash granularity")

---

### DC-7: Prep Frozen Immutability Blocks Signal-Driven Invalidation

**Scenario**: Once `freeze_meeting_prep_snapshot()` sets `prep_frozen_at`, the meeting prep is immutable. Signal-driven invalidation clears `prep_frozen_json` but cannot clear `prep_frozen_at`. The gate check `WHERE prep_frozen_at IS NULL` in `freeze_meeting_prep_snapshot` means the prep cannot be re-frozen with updated content.

**Source**: LIFECYCLES.md (Section 3: Meeting Lifecycle, Known Gaps -- "Prep invalidation clears prep_frozen_json but not prep_frozen_at")

This is documented as intentional (immutability after freeze), but the interaction with signal-driven invalidation creates a gap: high-confidence signals about a frozen meeting's entity are acknowledged (the signal is emitted) but have no effect on the user-visible prep.

---

## 6. Testing Gaps

Untested code paths identified from PIPELINES.md error handling analysis and LIFECYCLES.md edge transitions.

---

### Critical Untested Paths (Pipeline Integration)

| Pipeline | Untested Area | Risk |
|----------|--------------|------|
| Intelligence Enrichment | Full processor loop (`run_intel_processor`) | Cannot verify end-to-end enrichment without integration test |
| Intelligence Enrichment | Write results + side effects (`write_enrichment_results`) | Report invalidation, prep requeue, coherence check untested |
| Intelligence Enrichment | Validation retry logic (I470) | Cannot verify retry behavior on parse failure |
| Signal Bus | Full propagation chain with real rules | 9 rules tested individually but chain interaction untested |
| Signal Bus | `emit_signal_propagate_and_evaluate` chain | Three-phase emission never tested together |
| Meeting Prep | `generate_mechanical_prep` | Core prep generation function has zero test coverage |
| Meeting Prep | `sweep_meetings_needing_prep` | Boot sweep logic untested |
| Meeting Prep | `prep_frozen_json` deserialization round-trip | Serialization compatibility between write and read paths untested |
| Transcript Processing | Full `process_transcript` with AI | End-to-end transcript flow untested |
| Transcript Processing | Action extraction from AI output | Parser → DB write path untested |
| Transcript Processing | Signal emission from transcript | Post-processing signal chain untested |
| Reports | All report type parsing | No tests for any report type |
| Reports | Intel hash computation and staleness detection | Core freshness mechanism untested |
| Reports | Report invalidation on enrichment | The `mark_reports_stale` → report staleness flow untested |
| Google API | Token refresh flow | Requires HTTP mocking |
| Google API | Calendar event fetching and Gmail fetching | Requires API mocking |
| Google API | Retry policy (`send_with_retry`) | Exponential backoff untested |
| Scheduler | Scheduler loop, sleep/wake detection, day change handling | All scheduler orchestration logic untested |
| Scheduler | Pre-meeting refresh | Scheduler → intelligence → prep chain untested |
| Self-Healing | Coherence check with real embeddings | Requires model loading |
| Proactive Engine | Detector panic recovery | A panicking detector crashes the entire scan with no catch |

### Untested State Transitions (Lifecycle Edge Cases)

| Lifecycle | Untested Transition | Risk |
|-----------|-------------------|------|
| Entity | Created → Stale (never linked to any meeting) | Entities without meetings may never be enriched |
| Entity | Merged → cascade effects on actions, meetings, people | Merge correctness across all junction tables |
| Meeting | PrepGenerated → PrepFrozen → signal invalidation arrives | Immutability vs. invalidation conflict |
| Meeting | Calendar event deleted → row persists | No cancelled state tested |
| Intelligence | Batch enrichment → partial parse failure | Cross-entity corruption on shared PTY |
| Intelligence | Circuit breaker trip → 72h auto-expire → re-enqueue | Full circuit breaker lifecycle |
| Signal | Propagation rule panic → engine crash | No panic recovery |
| Action | Proposed → auto-archive (stale threshold) | `auto_archive_old_proposed` timing logic |

---

## 7. Performance Risks

Long DB locks, synchronous propagation, unbounded queries, and missing indexes.

---

### PERF-1: Missing Index on `meetings_history(intelligence_state)`

The intelligence lifecycle regularly queries for meetings in `detected` or `stale` state. With no index, this is a full table scan on a table that grows with every calendar sync. For a user with 2+ years of meeting history, this could be 5000+ rows scanned every 60 seconds by the scheduler.

**Source**: DATA-MODEL.md (Section 4: Missing Indexes)

**Fix**: `CREATE INDEX idx_meetings_intelligence_state ON meetings_history(intelligence_state);`

---

### PERF-2: Missing Index on `emails(sender_email)`

Entity resolution lookups by sender email occur during every Gmail sync cycle. No index exists.

**Source**: DATA-MODEL.md (Section 4: Missing Indexes)

**Fix**: `CREATE INDEX idx_emails_sender ON emails(sender_email);`

---

### PERF-3: Missing Compound Index on `signal_events(entity_id, signal_type)`

Propagation rules filter by both entity and signal type. The existing index `(entity_type, entity_id, created_at DESC)` requires `entity_type` to be specified first and does not optimize signal_type queries.

**Source**: DATA-MODEL.md (Section 4: Missing Indexes)

**Fix**: `CREATE INDEX idx_signal_events_entity_signal ON signal_events(entity_id, signal_type, created_at DESC);`

---

### PERF-4: Missing Index on `captures(captured_at)`

Range queries on `captured_at` for time-bounded capture retrieval have no index.

**Source**: DATA-MODEL.md (Section 4: Missing Indexes)

**Fix**: `CREATE INDEX idx_captures_captured_at ON captures(captured_at);`

---

### PERF-5: Synchronous Propagation Blocks Signal Emission

All 9 propagation rules fire inline within the caller's context during `emit_signal_and_propagate()`. Each rule queries the DB (entity lookups, hierarchy traversal, renewal checks). A slow rule or a rule that triggers many derived signals blocks the original command handler.

**Source**: PIPELINES.md (Pipeline 2: Concurrency section -- "Signal emission: Synchronous within the caller's context"), LIFECYCLES.md (Section 5: Known Gaps -- "Propagation is synchronous")

**Impact**: User-facing commands that emit signals (action state changes, entity context edits, person relationship changes) experience variable latency depending on how many propagation rules match.

**Recommendation**: Move propagation to an async task spawned after the signal INSERT commits. The trade-off is eventual consistency for derived signals.

---

### PERF-6: Unbounded `signal_events` Table Growth

Signals decay in weight but are never physically deleted. Over months of use, this table grows linearly with every calendar sync, email enrichment, user action, and proactive detection. Queries that scan `signal_events` (propagation rules, fusion scoring, prep context assembly) slow as the table grows.

**Source**: LIFECYCLES.md (Section 5: Known Gaps -- "No explicit signal expiry/GC"), DATA-MODEL.md (signal_events table)

**Recommendation**: Add a periodic garbage collection job to the scheduler. Delete signals where `decayed_weight < 0.01` (effectively expired) and `superseded_by IS NOT NULL`. Retain user_correction signals indefinitely for feedback integrity.

---

### PERF-7: No Per-Entity Enrichment Budget

The `HygieneBudget` is global -- a few expensive entities can exhaust the daily budget, preventing any other entity from being enriched. A portfolio with 50 accounts where 3 are very content-rich will starve the other 47.

**Source**: LIFECYCLES.md (Section 4: Intelligence Lifecycle, Known Gaps -- "No per-entity enrichment budget")

**Recommendation**: Add per-entity or per-priority-tier budget slicing to `HygieneBudget`.

---

## 8. Recommended Remediation Roadmap

---

### Before v1.1.0 (Must Fix Before Building New Features)

These issues affect the foundation that v1.1.0 intelligence work builds on.

| ID | Issue | Effort | Priority |
|----|-------|--------|----------|
| P0-1 | Fix `drive_watched_sources` FK (migration 054) | S | P0 |
| P0-2 | Replace `let _ =` in transcript signal/capture insertion with error logging | S | P0 |
| P0-3 | Replace `let _ =` in `emit_signal` future meeting flagging with logged retry | S | P0 |
| P0-4 | Replace poisonable `Mutex` on `prep_invalidation_queue` with recoverable lock | S | P0 |
| PERF-1 | Add index on `meetings_history(intelligence_state)` | S | P1 |
| PERF-2 | Add index on `emails(sender_email)` | S | P1 |
| PERF-3 | Add index on `signal_events(entity_id, signal_type, created_at DESC)` | S | P1 |
| PERF-4 | Add index on `captures(captured_at)` | S | P1 |
| P1-1a | Add signal emission to top-10 most impactful entity CRUD commands (update_account_field, update_project_field, update_person, archive_*, link_meeting_entity, add/remove_account_team_member) | M | P1 |
| DC-1 | Sync `entity_people` writes from `add_account_team_member`/`remove_account_team_member` | S | P1 |
| P2-7 | Drop vestigial `chat_sessions` / `chat_turns` tables | S | P2 |
| P2-6 | Deprecate `prep_context_json` reads, migrate all to `prep_frozen_json` | S | P2 |

### During v1.1.0 (Fix as Part of Intelligence Foundation Work)

These align with the planned I508 intelligence schema redesign and related issues.

| ID | Issue | Effort | Priority |
|----|-------|--------|----------|
| P2-4 | Split `entity_intelligence` operational state from AI output (aligned with I508) | L | P2 |
| P1-1b | Complete signal bus coverage for remaining ~15 commands | M | P1 |
| P1-7 | Re-extract keywords during intelligence enrichment | S | P1 |
| P1-8 | Implement per-source enrichment TTLs | M | P1 |
| DC-5 | Add per-entity response parsing in batch PTY mode (fail one, keep others) | M | P1 |
| DC-6 | Expand `intel_hash` to include `risks_json` hash + `recent_wins_json` hash | S | P1 |
| PERF-6 | Add signal GC job to scheduler (delete expired + superseded signals) | S | P2 |
| PERF-5 | Move signal propagation to async task (eventual consistency for derived signals) | M | P2 |
| P1-2 | Wire `captures.owner` and `captures.due_date` through transcript pipeline, or drop columns | S | P1 |
| P1-3 | Add `cancelled_at` column to `meetings_history`, detect deleted calendar events | M | P1 |
| P1-4 | Add time-based prep staleness check to pre-meeting scheduler sweep | S | P1 |

### Post v1.1.0 (Technical Debt That Can Wait)

These improve code health and maintainability but do not block feature work.

| ID | Issue | Effort | Priority |
|----|-------|--------|----------|
| P2-1 | Split `commands.rs` into domain-scoped command files | M | P2 |
| P2-2 | Extract `hygiene.rs` into `hygiene/` directory module | M | P2 |
| P2-3 | Decompose `meetings_history` into narrower tables | L | P2 |
| P2-5 | Resolve `entity_people` / `account_team` duality | S | P2 |
| P2-8 | Drop `is_internal` column, use `account_type` exclusively | S | P2 |
| P2-9 | Absorb `risk_briefing.rs` into `reports/risk.rs` | S | P2 |
| P1-5 | Add capture deletion/archival pathway | S | P2 |
| P1-6 | Implement archival cascade for accounts → children, actions | M | P2 |
| DC-2 | Auto-sync `contract_end` from newest `account_events` renewal | S | P2 |
| DC-3 | Reconcile `attendees` blob with `meeting_attendees` junction during sync | S | P2 |
| DC-4 | Add SQLite TRIGGER to keep `entities` mirror in sync on direct UPDATE | S | P2 |
| PERF-7 | Per-entity enrichment budget slicing | M | P2 |
| DC-7 | Document frozen prep immutability policy; decide if invalidation should clear `prep_frozen_at` | S | P2 |

---

### Testing Priorities

The following test coverage additions have the highest risk-reduction value:

1. **`generate_mechanical_prep` integration test** -- the core user-facing feature (meeting prep) has zero test coverage on its generation path
2. **`prep_frozen_json` deserialization round-trip** -- a serde failure here silently returns empty prep
3. **Full propagation chain integration test** -- 9 rules interact; individual tests miss chain effects
4. **Report staleness detection test** -- the intel hash → staleness → regeneration flow is the entire report lifecycle
5. **Signal emission from transcript processing** -- the post-meeting intelligence cascade depends on this

---

*End of Cross-Cutting Gap Analysis.*
