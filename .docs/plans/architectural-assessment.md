# DailyOS Architectural Assessment

**Date:** 2026-02-20
**Auditor:** Systems Engineering Team (5-agent parallel audit)
**Scope:** Full codebase, all ADRs, backlog, database, signal system, frontend, backend

---

## 1. Executive Summary

DailyOS is a **surprisingly coherent codebase that has outgrown its architecture**. The product vision is clear, the documentation is excellent (88 ADRs, honest backlog), and the core pipeline (Prepare -> Deliver -> Enrich) is well-designed. But the implementation has accumulated significant structural debt from fast iteration without refactoring.

**The good news:** The bones are solid. The three-phase workflow, the signal bus, the migration framework, the design token system -- these are well-architected foundations.

**The bad news:** Everything else is crammed into three god files (`commands.rs` at 11,469 lines, `db.rs` at 9,747 lines, `MeetingDetailPage.tsx` at 2,113 lines), there's no service layer, helper functions are copy-pasted instead of imported, and the AppState struct has 28 fields acting as a god object.

**Overall Grade: B-** -- Strong architecture on paper, undisciplined implementation in practice.

---

## 2. Architecture Map (What Actually Exists)

### Backend (Rust, ~55,000 lines across 122 files)

```
src-tauri/src/
  commands.rs .......... 11,469 lines  IPC handlers + business logic + helpers (GOD FILE)
  db.rs ................ 9,747 lines   404 SQL operations, all data access (GOD FILE)
  state.rs ............. 943 lines     AppState: 28 fields, god object
  executor.rs .......... 1,487 lines   Workflow orchestration (specialized, not generic)
  entity_intel.rs ...... 3,946 lines   Entity intelligence I/O + prompt building (mixed concerns)
  pty.rs ............... 637 lines     Claude Code subprocess (well-abstracted)

  prepare/ ............. 10 files, ~3,646 lines   Calendar/email fetch, classify, directive
  processor/ ........... 12 files, ~2,400 lines   Inbox routing, AI enrichment, transcripts
  signals/ ............. 17 files, ~2,000 lines   Signal bus, decay, fusion, propagation
  workflow/ ............ 8 files, ~7,562 lines    Three-phase pipeline execution
  queries/ ............. 6 files, ~650 lines      Specialized DB queries

  Integrations: google_api/, clay/, granola/, quill/, linear/, gravatar/ (~4,890 lines total)
  Infrastructure: migrations/ (32 SQL files), presets/, proactive/, mcp/
```

### Frontend (React + TypeScript, ~18,000 lines)

```
src/
  pages/ ............... 16 pages, 10,270 lines total
    MeetingDetailPage ... 2,113 lines (GOD COMPONENT)
    InboxPage ........... 1,219 lines
    WeekPage ............ 1,056 lines

  components/ ........... ~80 components across editorial/, ui/, entity/, dashboard/, layout/
    SystemStatus ........ 1,034 lines
    YouCard ............. 800 lines
    DailyBriefing ....... 757 lines
    StakeholderGallery .. 641 lines

  hooks/ ................ ~15 hooks, each scoped to one data domain
  types/index.ts ........ 1,632 lines (comprehensive, aligned with Rust structs)
  styles/design-tokens.css .. 142 lines (single source of truth, universally used)
```

### Database (SQLite, WAL mode)

```
29 tables, 30+ indexes, 10 FK constraints
32 sequential migrations (well-managed framework, ADR-0071)
Single connection per process, Mutex-guarded in AppState
```

### Signal System

```
signal_events table (append-only event log)
6 propagation rules (cross-entity derivation)
Bayesian log-odds fusion (5 signal sources for entity resolution)
Confidence-scored, decay-aware, supersession-tracked
```

---

## 3. Critical Issues (Will Cause Pain at Scale)

### C1: commands.rs is 11,469 lines with 229 direct DB calls

This file is the single biggest liability in the codebase. It contains:
- 212 Tauri IPC command handlers
- Business logic (keyword extraction, signal summarization, entity hints)
- Data transformation and validation
- Its own private `normalize_key()` function (duplicated from helpers.rs)
- Direct DB access bypassing any service abstraction

**Impact:** Every feature addition touches this file. No developer can hold it in their head. Merge conflicts are guaranteed. Testing is impossible without the full Tauri runtime.

**Fix:** Extract service modules: `AccountService`, `PersonService`, `MeetingService`, `ActionService`, `WorkflowService`. Commands.rs becomes a thin dispatch layer (~2,000 lines).

### C2: No service layer between commands and database

The current call chain is:
```
commands.rs -> state.with_db_read/write() -> db.rs methods
```

There's no place for business rules, validation, cross-cutting concerns, or testable logic between the IPC boundary and raw SQL. This forces business logic into either commands.rs (where it doesn't belong) or db.rs (where it definitely doesn't belong).

**Impact:** Can't unit test business logic. Can't reuse logic across different entry points (commands vs scheduler vs signals). Can't add middleware (logging, caching, authorization).

**Fix:** Introduce trait-based services that commands.rs and scheduler.rs both consume.

### C3: AppState is a 28-field god object

```rust
AppState {
  config, db, google_auth, workflow_status, execution_history,
  last_scheduled_run, calendar_events, capture_dismissed, capture_captured,
  transcript_processed, intel_queue, embedding_model, embedding_queue,
  last_hygiene_report, hygiene_scan_running, last_hygiene_scan_at,
  next_hygiene_scan_at, hygiene_budget, week_calendar_cache,
  hygiene_full_orphan_scan_done, pre_dev_workspace, clay_poller_wake,
  prep_invalidation_queue, signal_engine, entity_resolution_wake,
  quill_poller_wake, linear_poller_wake, active_preset
}
```

Every subsystem reaches into AppState for its dependencies. Adding a new feature means adding another field. There's no encapsulation.

**Impact:** Thread contention (single Mutex for DB, separate locks for everything else). Impossible to reason about which subsystem owns which state. Testing requires constructing the entire world.

**Fix:** Split into domain-specific state containers: `DbState`, `WorkflowState`, `HygieneState`, `IntegrationState`, `SignalState`. AppState becomes a facade over these.

### C4: db.rs is 9,747 lines with no query abstraction

400+ SQL operations in a single file. SELECT column lists are duplicated across 41 action queries, 36 account queries. Row mappers exist but column lists are still copy-pasted.

**Impact:** Schema changes require updating dozens of queries. No way to verify query correctness without running them. Cognitive load for any developer touching the data layer.

**Fix:** Extract `const ACTIONS_SELECT`, `const ACCOUNTS_SELECT`, etc. for column lists. Consider splitting db.rs into domain modules: `db/actions.rs`, `db/accounts.rs`, `db/meetings.rs`, `db/signals.rs`.

### C5: Only 3 explicit transactions despite many multi-step operations

Operations like `merge_accounts()` execute 7+ SQL statements without wrapping them in a transaction. If the process crashes between statements, data becomes inconsistent.

**Impact:** Silent data corruption risk. Merge operations, team updates, entity reassignment are all vulnerable.

**Fix:** Wrap all compound operations in `with_transaction()`.

---

## 4. DRY Violations

| Duplicated Logic | Locations | Fix |
|-----------------|-----------|-----|
| `normalize_key()` | `helpers.rs:10`, `commands.rs:6566`, `entity_resolver.rs:682` | Delete copies, import from helpers |
| `normalize_domains()` | `helpers.rs:19`, `util.rs:472` (identical implementations) | Pick one canonical location |
| `build_entity_hints()` | `helpers.rs:31`, `google.rs:90` (state-coupled variant), `prepare/orchestrate.rs:74` (filesystem variant) | Centralize with trait/strategy pattern |
| Entity signal prose formatting | `commands.rs:61` (71 lines), similar in `intelligence.rs` | Extract to shared `SignalFormatter` |
| Auto-increment ID generation | `commands.rs:6592`, `processor/mod.rs:313`, multiple `db.rs` upsert functions | Centralize in `util::generate_unique_id()` |
| SELECT column lists | 41 action queries, 36 account queries all repeat same columns | Extract to const strings |
| PtyManager instantiation | 7+ call sites each construct independently | Factory method: `PtyManager::with_tier()` in state |
| Tauri event listener pattern | 5+ hooks repeat identical listen/unlisten boilerplate | Extract `useTauriEvent()` utility hook |

---

## 5. SRP Violations

### Backend

| File | Lines | Responsibilities (Should Be 1) | Actual |
|------|-------|--------------------------------|--------|
| `commands.rs` | 11,469 | IPC dispatch | IPC + business logic + validation + data transformation + helper functions |
| `db.rs` | 9,747 | Data access | Data access + schema types + row mapping + transaction management + query building |
| `entity_intel.rs` | 3,946 | Entity intelligence I/O | I/O + prompt building + user edit tracking + intelligence schema |
| `state.rs` | 943 | App state container | State + initialization + serialization + helper methods + wake signal management |
| `executor.rs` | 1,487 | Workflow execution | Execution + error handling + model fallback + email signal sync + domain query building |

### Frontend

| File | Lines | Responsibilities (Should Be 1) | Actual |
|------|-------|--------------------------------|--------|
| `MeetingDetailPage.tsx` | 2,113 | Meeting detail rendering | Rendering + prep management + transcript sync + agenda editing + intelligence refresh + outcome tracking |
| `SystemStatus.tsx` | 1,034 | System status display | Display + backend queries + state management + feature detection + config display |
| `InboxPage.tsx` | 1,219 | Inbox view | View + drag-drop handling + file processing + batch operations + deduplication logic |

---

## 6. Missing Infrastructure

### Services That Should Exist (Backend)

| Service | Current State | What It Would Own |
|---------|--------------|-------------------|
| `AccountService` | Logic scattered in commands.rs | CRUD, merge, hierarchy, domain mapping, intelligence triggers |
| `PersonService` | Logic scattered in commands.rs | CRUD, merge, relationship classification, enrichment triggers |
| `MeetingService` | Logic in commands.rs + executor.rs | Prep lifecycle, intelligence quality, entity linking, transcript attachment |
| `ActionService` | Logic in commands.rs + workflow/today.rs | CRUD, status transitions, temporal grouping, source tracking |
| `IntelligenceService` | Split across 3 files (intelligence.rs, entity_intel.rs, intelligence_lifecycle.rs) | Quality assessment, staleness detection, enrichment orchestration, prompt building |
| `SignalService` | Exists partially (signals/ module) | Signal emission, propagation, fusion, callout generation |

### Abstractions That Should Exist (Backend)

| Abstraction | Current State | Benefit |
|-------------|--------------|---------|
| `Pipeline` trait | Each workflow hardcodes step sequence in executor.rs | Pluggable steps, testable stages, retry per stage |
| `QueryBuilder` / column constants | 400+ raw SQL strings with duplicated column lists | Schema change = one edit, not 41 |
| Central error type for commands | Mix of `Result<T, String>`, `ExecutionError`, `anyhow::Result` | Consistent error serialization to frontend |
| `useTauriEvent()` hook | 5+ hooks repeat listen/unlisten boilerplate | Single utility, consistent cleanup |

### Stores That Should Exist (Frontend)

| Store | Current State | What It Would Own |
|-------|--------------|-------------------|
| None critical | Hooks are well-scoped; no global store needed | PersonalityProvider is the only Context, and it's appropriate |

**Frontend assessment: The frontend actually doesn't need a centralized store.** The hook-per-domain pattern works well for a Tauri app where the backend is the source of truth. This is one area where the team got it right.

---

## 7. Frontend-Backend Disconnect

**Surprisingly minimal.** The frontend audit found:

- Zero hardcoded/mock data in production components
- All pages call `invoke()` through well-typed hooks
- TypeScript types in `src/types/index.ts` (1,632 lines) align with Rust structs
- Only 6 instances of `any`/`unknown` type coercion (all framework necessities)
- Design tokens used universally -- zero hardcoded colors found
- Event-driven updates (calendar-updated, prep-ready, entity-updated, workflow-completed) keep UI synchronized

**The frontend is the best-architected layer of this application.** Clean separation, consistent patterns, type-safe, design-token compliant. The engineering discipline that's missing from the backend is present here.

---

## 8. Database Issues

### Strengths
- Well-designed migration framework (ADR-0071) with 32 sequential migrations
- Foreign keys enforced (`PRAGMA foreign_keys = ON`)
- WAL mode for concurrent reads
- Centralized data access (99% of SQL in db.rs)
- Parameterized queries throughout (no SQL injection risk)
- Appropriate junction tables (all justified, proper composite PKs)

### Issues

| Issue | Severity | Detail |
|-------|----------|--------|
| Only 3 transactions for compound ops | Medium | merge_accounts(), team updates, entity reassignment unprotected |
| Polymorphic FKs not enforced | Medium | entity_id in meeting_entities, entity_people references accounts OR projects |
| No index on people.last_seen | Low | Temporal people queries do full table scan |
| Row mapper column lists duplicated | Low | 41 action queries repeat same 17 columns |
| No automated rollback | Low | Manual backup restore only |
| Ad-hoc queries via conn_ref() | Low | 40 files bypass ActionDb methods (intentional, small queries) |

---

## 9. Recommendations (Priority Order)

### P0: Immediate (Before Next Feature)

1. **Delete duplicate `normalize_key()` from commands.rs:6566 and entity_resolver.rs:682.** Import from `helpers.rs`. This is a 5-minute fix that prevents a class of bugs.

2. **Wrap compound DB operations in transactions.** Start with `merge_accounts()`, `merge_people()`, and any multi-entity update. Prevents silent data corruption.

### P1: Next Sprint (High Impact)

3. **Extract service layer from commands.rs.** Start with `AccountService` (most complex) and `ActionService` (most used). This is the single highest-impact refactoring. commands.rs drops from 11,469 to ~3,000 lines. Business logic becomes testable.

4. **Split db.rs into domain modules.** `db/actions.rs`, `db/accounts.rs`, `db/meetings.rs`, `db/signals.rs`, `db/content.rs`. Extract `const` column lists. Each module is <2,000 lines.

5. **Consolidate intelligence modules.** Move prompt building out of entity_intel.rs into `intelligence/prompts.rs`. Keep entity_intel.rs as pure I/O. Keep intelligence.rs as pure computation. intelligence_lifecycle.rs becomes the orchestrator.

### P2: Next Release Cycle

6. **Split AppState into domain containers.** `DbState`, `WorkflowState`, `HygieneState`, `IntegrationState`. Reduces contention, improves testability, makes ownership clear.

7. **Standardize error types.** All commands return `Result<T, CommandError>`. All workflows return `Result<T, ExecutionError>`. Kill ad-hoc `Result<T, String>`.

8. **Extract `useTauriEvent()` hook** for frontend. Reduces boilerplate in 5+ hooks.

### P3: Architectural Evolution

9. **Introduce Pipeline trait** for workflow execution. Steps become pluggable. Retry, timeout, and telemetry are cross-cutting. New workflows don't require executor.rs modifications.

10. **Consider splitting MeetingDetailPage.tsx** (2,113 lines) into sub-components: `MeetingPrepPanel`, `MeetingTranscriptPanel`, `MeetingOutcomesPanel`, `MeetingAgendaEditor`.

11. **Update ADR-0080 status** from "Proposed" to "Accepted" in the ADR README. It's been implemented since v0.10.0.

---

## 10. Target Architecture

### What This Should Look Like

```
commands.rs (thin IPC dispatch, ~2,000 lines)
  |
  v
services/ (business logic, testable without Tauri)
  account_service.rs
  person_service.rs
  meeting_service.rs
  action_service.rs
  intelligence_service.rs
  |
  v
db/ (data access, split by domain, ~2,000 lines each)
  actions.rs
  accounts.rs
  meetings.rs
  signals.rs
  content.rs
  mod.rs (shared connection, transaction, column constants)
  |
  v
state/ (domain-specific state containers)
  db_state.rs
  workflow_state.rs
  hygiene_state.rs
  integration_state.rs
  mod.rs (AppState facade)
```

### What Stays The Same

- **Frontend architecture** -- hooks, components, design tokens are well-done. Don't touch this.
- **Workflow pipeline** (Prepare -> Deliver -> Enrich) -- solid three-phase design. Keep it.
- **Signal bus** -- well-architected append-only event log with propagation. Keep it.
- **Migration framework** -- solid, versioned, backed up. Keep it.
- **PtyManager** -- good abstraction for Claude Code subprocess. Keep it.
- **prepare/, processor/, signals/ module boundaries** -- clean separation. Keep it.

### What Changes

- **commands.rs** splits into commands/ + services/
- **db.rs** splits into db/ module tree
- **state.rs** splits into state/ module tree
- **entity_intel.rs** loses prompt building (moves to intelligence/)
- **executor.rs** gets a Pipeline trait (optional, P3)
- **Duplicate helpers** get deleted, canonical imports enforced

---

## Appendix: Audit Methodology

Five specialized agents conducted parallel deep-dive audits:

1. **Backend Auditor** (architect-reviewer): Read all 122 .rs files, mapped dependencies, identified DRY/SRP violations
2. **Database Auditor** (database-optimizer): Read all 32 migrations, mapped 29 tables, analyzed query patterns and index coverage
3. **Signals Auditor** (architect-reviewer): Read all 17 signal files, traced signal flow, mapped workflow orchestration
4. **Frontend Auditor** (react-specialist): Read all 16 pages, all hooks, all types, verified design token compliance
5. **Documentation Auditor** (architect-reviewer): Read all 88 ADRs, full backlog (6,163 lines), all plans, cross-referenced with codebase

Total tokens consumed: ~400,000+. Every file in the codebase was read by at least one agent.
