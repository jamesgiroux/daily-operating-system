# DailyOS Refactoring Plan

**Date:** 2026-02-20
**Based on:** Architectural Assessment (same date)
**Current version:** 0.12.1 (dev branch)
**Relationship to backlog:** Subsumes I280 (Beta Hardening Umbrella), feeds I335 (Entity-Generic Data Model)

---

## Guiding Principles

1. **No feature freezes.** Refactoring ships alongside features, not instead of them.
2. **Each phase is independently shippable.** No phase depends on completing the next.
3. **Backend only (mostly).** The frontend is well-architected — leave it alone except for one utility hook.
4. **Tests gate each phase.** No phase merges without `cargo test` + `cargo clippy -- -D warnings` passing.
5. **No public API changes.** Tauri command signatures stay the same. Frontend sees zero impact.

---

## Phase 0: Hygiene (1-2 hours, do today)

Zero-risk fixes that should have been done yesterday. No architectural changes — just deleting duplicates and adding safety.

### 0A: Kill duplicate functions

| What | Where | Action |
|------|-------|--------|
| `normalize_key()` | `commands.rs:6566` | Delete. Replace call sites with `crate::helpers::normalize_key()` |
| `normalize_key()` | `prepare/entity_resolver.rs:682` | Delete. Replace call sites with `crate::helpers::normalize_key()` |
| `normalize_domains()` | `util.rs:472` | Delete. Replace call sites with `crate::helpers::normalize_domains()` |

**Verification:** `cargo test` passes. `grep -rn "fn normalize_key" src-tauri/src/` returns exactly 1 result (helpers.rs).

### 0B: Wrap compound DB operations in transactions

| Operation | File | Current | Fix |
|-----------|------|---------|-----|
| `merge_accounts()` | `db.rs` | 7+ sequential statements | Wrap in `with_transaction()` |
| `merge_people()` | `db.rs` | 5+ sequential statements | Wrap in `with_transaction()` |
| `update_account_field("parent_id", ...)` | `db.rs` | Multi-step with circular ref check | Wrap in `with_transaction()` |
| `delete_person()` | `db.rs` | Cascading deletes across tables | Wrap in `with_transaction()` |

**Verification:** `cargo test`. Spot-check by merging two test accounts and killing the process mid-merge (should rollback cleanly or not corrupt).

### 0C: Add missing temporal index

```sql
-- Migration 033
CREATE INDEX IF NOT EXISTS idx_people_last_seen ON people(last_seen DESC);
```

**Verification:** Migration applies cleanly on fresh + existing DBs.

---

## Phase 1: Split db.rs into domain modules (3-5 hours)

The single highest-leverage structural change. db.rs at 9,747 lines is unnavigable. Split it by domain while keeping the public API identical.

### Target structure

```
src-tauri/src/db/
  mod.rs .............. ActionDb struct, open(), open_at(), connection management,
                        with_transaction(), with_db_read/write helpers, re-exports
  columns.rs .......... const ACTIONS_SELECT, ACCOUNTS_SELECT, PEOPLE_SELECT, MEETINGS_SELECT
  actions.rs .......... All action CRUD (upsert, get, list, complete, archive, filter)
  accounts.rs ......... All account CRUD (upsert, get, list, merge, hierarchy, domains, team)
  people.rs ........... All person CRUD (upsert, get, list, merge, archive, enrichment)
  meetings.rs ......... All meeting CRUD (upsert, get, list, attendees, entities, prep_state)
  signals.rs .......... Signal events, weights, email_signals, cadence, derivations
  content.rs .......... Content index, embeddings, chat sessions/turns
  hygiene.rs .......... Hygiene queries (unnamed people, unknown relationships, stale intel)
  types.rs ............ DbAction, DbAccount, DbPerson, DbProject, etc. (moved from db.rs)
```

### Approach

1. Create `src-tauri/src/db/` directory
2. Move `db.rs` to `db/mod.rs` (temporary — everything still works)
3. Extract `columns.rs` first — define `const` column lists, replace in queries
4. Extract `types.rs` — move all `Db*` struct definitions
5. Extract domain files one at a time: actions.rs, accounts.rs, people.rs, meetings.rs
6. Each extraction: cut `impl ActionDb` methods, paste into new file, add `use super::*`, verify `cargo check`
7. Last: extract signals.rs, content.rs, hygiene.rs (smaller, lower risk)

### Key constraints

- `ActionDb` struct stays in `mod.rs` — all domain files implement methods on it
- `conn` field access via `self.conn` works across files (same `impl ActionDb` block)
- Row mappers (`map_person_row`, `map_account_row`, `map_action_row`) move to their domain files
- External imports (`use crate::db::ActionDb`) continue to work — `mod.rs` re-exports everything

### Verification

- `cargo test` passes (all 886 tests)
- `cargo clippy --workspace --all-features --lib --bins -- -D warnings` clean
- `wc -l src-tauri/src/db/*.rs` — no file exceeds 2,500 lines
- `grep -rn "use crate::db" src-tauri/src/ | wc -l` — same count before and after (no broken imports)

---

## Phase 2: Extract service layer from commands.rs (8-12 hours)

This is the main event. commands.rs drops from 11,469 to ~2,500 lines. Business logic becomes independently testable.

### Target structure

```
src-tauri/src/services/
  mod.rs .............. Re-exports
  accounts.rs ......... AccountService: CRUD, merge, hierarchy, domain mapping, keywords
  people.rs ........... PersonService: CRUD, merge, relationship classification, enrichment
  meetings.rs ......... MeetingService: prep lifecycle, intelligence quality, entity linking
  actions.rs .......... ActionService: CRUD, status transitions, temporal grouping
  entities.rs ......... EntityService: shared entity operations (archive, metadata, signals)
```

### What moves where

**Into `services/accounts.rs`:**
- `auto_extract_title_keywords()` (from commands.rs ~line 163)
- `create_child_account_record()` (from commands.rs ~line 6574)
- Account merge orchestration logic
- Account field update validation (parent_id circular ref check)
- Domain mapping and team management logic
- `build_entity_signal_prose()` for accounts (from commands.rs ~line 61)

**Into `services/people.rs`:**
- Person merge logic
- Relationship classification
- Duplicate detection orchestration
- Name resolution from email headers

**Into `services/meetings.rs`:**
- `get_meeting_intelligence()` orchestration (~8 sequential DB calls)
- Prep review state management
- Entity linking and attendee management
- Transcript attachment logic

**Into `services/actions.rs`:**
- Action creation with source tracking
- Status transition rules (proposed -> pending -> completed)
- Temporal grouping logic (overdue, today, this week, upcoming)
- Proposed action accept/reject

**Into `services/entities.rs`:**
- `normalize_key()` calls (now imports from helpers)
- Archive/unarchive with cascade
- Metadata update (JSON column)
- Shared intelligence field update pattern

### What stays in commands.rs

Each Tauri `#[command]` function becomes a thin wrapper:

```rust
#[tauri::command]
pub async fn update_account_field(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    field: String,
    value: String,
) -> Result<(), String> {
    let svc = services::AccountService::new(&state);
    svc.update_field(&id, &field, &value).map_err(|e| e.to_string())
}
```

### Extraction order (least to most coupled)

1. **actions.rs** — fewest cross-service dependencies, most self-contained
2. **entities.rs** — shared patterns, used by accounts/people/meetings
3. **accounts.rs** — most complex, but well-bounded domain
4. **people.rs** — depends on accounts (for domain mapping) but otherwise isolated
5. **meetings.rs** — depends on people (attendees) and accounts (entities), extract last

### Service function signatures

Services take `&AppState` (or specific state slices) and return `Result<T, ServiceError>` where `ServiceError` maps to `String` at the command boundary:

```rust
pub struct AccountService<'a> {
    state: &'a AppState,
}

impl<'a> AccountService<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    pub fn update_field(&self, id: &str, field: &str, value: &str) -> Result<(), ServiceError> {
        self.state.with_db_write(|db| {
            db.update_account_field(id, field, value)
                .map_err(|e| ServiceError::Database(e.to_string()))
        })
    }
}
```

### Verification

- `cargo test` passes
- `cargo clippy -- -D warnings` clean
- `wc -l src-tauri/src/commands.rs` < 3,000
- `wc -l src-tauri/src/services/*.rs` — each file < 1,500
- `pnpm dev` — app works end-to-end (every page loads, actions complete, meetings show prep)

---

## Phase 3: Consolidate intelligence modules (3-4 hours)

Three files with overlapping concerns become a clean package.

### Current state

| File | Lines | Does |
|------|-------|------|
| `intelligence.rs` | 559 | Pure Rust computation (signals from DB). No AI. |
| `entity_intel.rs` | 3,946 | Intelligence I/O + prompt building + user edit tracking + schema |
| `intelligence_lifecycle.rs` | 634 | Quality assessment + staleness detection |

### Target structure

```
src-tauri/src/intelligence/
  mod.rs .............. Re-exports
  compute.rs .......... From intelligence.rs — pure signal computation (unchanged)
  lifecycle.rs ........ From intelligence_lifecycle.rs — quality + staleness (unchanged)
  io.rs ............... From entity_intel.rs — JUST types, read/write, user edit tracking
  prompts.rs .......... Extracted from entity_intel.rs — build_intelligence_prompt(),
                        build_intelligence_context() (the 1,525-line function that needs splitting)
```

### What changes

1. `entity_intel.rs` splits: types + I/O stay as `intelligence/io.rs`, prompt building moves to `intelligence/prompts.rs`
2. `intelligence.rs` moves to `intelligence/compute.rs` (no code changes)
3. `intelligence_lifecycle.rs` moves to `intelligence/lifecycle.rs` (no code changes)
4. `build_intelligence_context()` (1,525 lines) gets broken into:
   - `build_meeting_context()` — meeting-specific context assembly
   - `build_entity_context()` — entity-specific context assembly
   - `build_signal_context()` — signal/callout context assembly

### Verification

- `cargo test` passes
- `cargo clippy -- -D warnings` clean
- No changes to public API — all imports updated via `use crate::intelligence::*`

---

## Phase 4: Standardize error types (2-3 hours)

Kill `Result<T, String>` in commands. Introduce `CommandError` that maps cleanly to frontend.

### Target

```rust
// src-tauri/src/error.rs (extend existing)

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Validation: {0}")]
    Validation(String),
    #[error("Service unavailable: {0}")]
    Unavailable(String),
}

impl From<CommandError> for String {
    fn from(e: CommandError) -> String { e.to_string() }
}

// Also impl serde::Serialize for structured frontend errors
```

### Approach

1. Define `CommandError` in error.rs
2. Define `ServiceError` (used by services, converts to `CommandError`)
3. Update services (from Phase 2) to return `Result<T, ServiceError>`
4. Update commands.rs wrappers to convert `ServiceError` -> `String` (Tauri requires String)
5. Gradually replace `Result<T, String>` in db.rs methods with `Result<T, DbError>` (already partially done)

### Verification

- `cargo test` passes
- No frontend changes needed (errors are still strings at IPC boundary)

---

## Phase 5: Split AppState (4-6 hours)

This is P2 priority — do after Phase 1-4 are stable.

### Target structure

```rust
pub struct AppState {
    pub core: CoreState,       // config, db, active_preset
    pub workflow: WorkflowState, // workflow_status, execution_history, last_scheduled_run
    pub hygiene: HygieneState,  // all hygiene_* fields, budget
    pub integrations: IntegrationState, // google_auth, clay/quill/linear pollers
    pub signals: SignalState,   // signal_engine, prep_invalidation_queue, entity_resolution_wake
    pub cache: CacheState,     // calendar_events, week_calendar_cache, capture_dismissed/captured
}
```

### Approach

1. Define sub-state structs with their fields and lock types
2. Implement accessor methods on each sub-state (mirroring current AppState helpers)
3. Update AppState to compose sub-states
4. Update all call sites — this is the tedious part but is mechanical find-and-replace:
   - `state.db.lock()` -> `state.core.db.lock()`
   - `state.hygiene_budget` -> `state.hygiene.budget`
   - etc.
5. Move `with_db_read/write` to CoreState

### Verification

- `cargo test` passes
- `cargo clippy -- -D warnings` clean
- No public API changes

---

## Phase 6: Frontend utility hook (30 minutes)

The only frontend change in this plan.

### `useTauriEvent()` utility hook

```typescript
// src/hooks/useTauriEvent.ts
import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export function useTauriEvent(event: string, callback: () => void) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen(event, () => callback()).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [event, callback]);
}
```

Then update: `useDashboardData.ts`, `useInbox.ts`, `useCalendar.ts`, `PeoplePage.tsx` to use it. Each saves 8-12 lines of boilerplate.

### Verification

- `pnpm dev` — all event-driven updates still work (calendar refresh, inbox count, workflow completion)

---

## Sequencing & Dependencies

```
Phase 0 (Hygiene)           ← Do today. No dependencies.
    |
Phase 1 (Split db.rs)       ← Do next. Unblocks Phase 2.
    |
Phase 2 (Service layer)     ← Depends on Phase 1 (services call db/ modules).
    |                          This is the big one.
Phase 3 (Intelligence)      ← Independent of Phase 2. Can run in parallel.
    |
Phase 4 (Error types)       ← Depends on Phase 2 (services define error types).
    |
Phase 5 (Split AppState)    ← Depends on Phase 2 (services reference state slices).
                               P2 priority — do after 0.12.2 ships.

Phase 6 (Frontend hook)     ← Independent. Do anytime.
```

### Parallel tracks

Two engineers can work simultaneously:
- **Track A:** Phase 0 -> Phase 1 -> Phase 2 -> Phase 4
- **Track B:** Phase 3 (intelligence) + Phase 6 (frontend hook)

Phase 5 (AppState split) is best done after both tracks merge, during a quiet sprint.

---

## Relationship to Product Backlog

| Backlog Item | Relationship to This Plan |
|-------------|--------------------------|
| **I280** (Beta Hardening) | Phase 0-2 address I280's DRY + DB integrity sub-issues. Phase 4 addresses error handling. Closes I280 structurally. |
| **I335** (Entity-Generic Data Model) | **Blocked on Phase 1.** Migrating `account_id` columns to generic `entity_id` is 10x easier in domain-split db/ modules than in a 9,747-line monolith. Do Phase 1 first. |
| **I290** (DRY extraction — entity I/O) | Phase 2's `services/entities.rs` directly addresses this. |
| **I291** (DRY extraction — frontend lists) | Out of scope. Frontend is well-done; not worth the disruption. |
| **I352** (Shared entity detail hooks/components) | Phase 6 is a small version of this. Full I352 can follow. |
| **0.13.0** (Meeting Intelligence) | Phase 3 (intelligence consolidation) directly unblocks 0.13.0's I326-I332 by making the intelligence layer navigable and extensible. |

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| Phase 1 (db split) breaks imports across 40+ files | Mechanical find-and-replace. `mod.rs` re-exports everything. Zero public API change. |
| Phase 2 (service extraction) introduces subtle behavior changes | Extract verbatim first, refactor later. No logic changes during extraction. |
| Phase 5 (AppState split) causes thread contention regression | Profile before and after. Sub-states use same lock types as current fields. |
| Refactoring delays 0.12.2/0.13.0 feature work | Phase 0-1 are prerequisites that make feature work faster. Phase 2 can interleave with feature work. |
| Merge conflicts from parallel feature development | Sequence: land refactoring phases on dev first, then features on top. Each phase is a single PR. |

---

## Estimated Effort

| Phase | Hours | Can Parallelize? |
|-------|-------|-----------------|
| Phase 0: Hygiene | 1-2 | No (do first) |
| Phase 1: Split db.rs | 3-5 | No (do before Phase 2) |
| Phase 2: Service layer | 8-12 | No (main track) |
| Phase 3: Intelligence | 3-4 | Yes (with Phase 1-2) |
| Phase 4: Error types | 2-3 | After Phase 2 |
| Phase 5: Split AppState | 4-6 | After Phase 2 |
| Phase 6: Frontend hook | 0.5 | Anytime |
| **Total** | **22-33 hours** | |

---

## Definition of Done

Each phase is done when:

1. `cargo test` passes (all 886+ tests)
2. `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings` clean
3. `pnpm dev` — app launches and every page renders with real data
4. `git diff --stat` shows net-zero or negative line count (no bloat)
5. No `Result<T, String>` introduced (Phase 4+)
6. God file line counts reduced to targets:
   - After Phase 1: `db/mod.rs` < 500 lines, each domain file < 2,500
   - After Phase 2: `commands.rs` < 3,000 lines
   - After Phase 3: No intelligence file > 1,500 lines
