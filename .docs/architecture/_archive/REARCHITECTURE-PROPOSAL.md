# DailyOS Rearchitecture Proposal

**Date:** 2026-03-02
**Author:** Architecture audit synthesis (98 ADRs, 14 architecture docs, 7 research docs, full codebase read)
**Status:** Active — feeds v1.0.0 implementation plan. ADR-0099 (remote-first) was proposed and withdrawn after first-principles review (2026-03-03). The 6 original workstreams survive as-is: schema decomposition (WS1→I511), ServiceLayer (WS2→I512), workspace file elimination (WS5→I513), module decomposition (WS3→I514), pipeline reliability (WS4→I515), frontend cleanup (WS6→I521). No sync-aware modifications, no Postgres, no auth/teams. DailyOS stays local-first. See `.docs/research/2026-03-03-architecture-first-principles-review.md` and `.docs/plans/v1.0.0.md`.

---

## Executive Summary

DailyOS was built iteratively over 98 ADRs across 12 months. The foundational choices were sound: Tauri + React + SQLite, local-first, signal bus, editorial design language. But fast iteration produced structural debt that now compounds: an 8,940-line command file, voluntary service boundaries, a data model with triple-representation of relationships, silent failure modes in critical pipelines, and a workspace file system that is the #1 source of bugs.

This proposal doesn't recommend a rewrite. It recommends a **systematic structural refactor** organized into 6 workstreams that can be executed incrementally while the app continues to ship features. Each workstream has a thesis, concrete file changes, a migration strategy, and a definition of done.

The goal: **make the architecture enforceable, not conventional.** Every structural problem in the current system traces to the same root cause — contracts exist as conventions that developers must remember, not as type-system or schema constraints that the compiler or database enforces.

---

## What We Built Right

Before diagnosing problems, acknowledge what held:

1. **Tauri + SQLite + local-first** (ADR-0001, 0003, 0007) — No regrets. Local-first eliminates an entire class of sync, auth, and latency problems. SQLCipher adds encryption at rest. The stack is correct.

2. **Signal bus architecture** (ADR-0030, 0066, 0080) — The Bayesian fusion model with source-weighted confidence, time decay, and propagation rules is genuinely sophisticated. The problem isn't the design — it's incomplete coverage.

3. **Intelligence as shared service** (ADR-0086) — The decision that meeting prep consumes entity intelligence (not generates its own) was the right architectural call. One source of truth for intelligence, multiple consumers.

4. **Editorial design language** (ADR-0073, 0076, 0077) — The magazine aesthetic differentiates the product. Design tokens as CSS custom properties is the right pattern. 85/100 compliance score from the styles audit.

5. **Role presets** (ADR-0079, 0090) — Preset-agnostic infrastructure with preset-specific vocabulary is the right abstraction. CS-first was the right rollout choice.

6. **Product vocabulary** (ADR-0083) — Separating system terminology from user-facing language was a mature decision, even if it came as a correction rather than upfront planning.

These are load-bearing walls. The refactor works around them, not through them.

---

## The Six Structural Problems

### Problem 1: Voluntary Service Boundaries

**Diagnosis:** The service layer exists (`src-tauri/src/services/`) with 16 domain modules. But it's optional. Commands can and do bypass it — ~35 commands have mixed responsibilities (handler makes decisions then calls service), ~18 do raw DB work inside the handler. The signal bus has defined weights for every source but ~25 commands that mutate entities emit no signals at all.

**Root cause:** There is no type-system enforcement that mutations go through services. `state.db.lock()` is accessible from anywhere. A developer writing a new command can choose to call `services::accounts::update()` or write raw SQL — nothing stops them.

**Why it matters now:** v1.1.0 builds health scoring, relationship intelligence, and a report suite on top of entity intelligence. If 25 commands silently mutate entities without emitting signals, the intelligence layer has blind spots. Reports built on incomplete signals are unreliable reports.

### Problem 2: God Modules

**Diagnosis:** Three files account for 20,000+ lines:
- `commands.rs` (8,940 lines, ~220 functions) — all IPC entry points in one flat file
- `workflow/deliver.rs` (5,731 lines) — all Phase 2/3 delivery logic
- `prepare/orchestrate.rs` (4,218 lines) — all Phase 1 preparation logic

Plus secondary god modules: `hygiene.rs` (1,800+ lines mixing duplicate detection, name resolution, readiness checking, and overnight scanning), `intelligence/prompts.rs` (2,982 lines of prompt templates), `signals/rules.rs` (1,570 lines of rule definitions).

**Root cause:** Each module grew along a single axis (entry points, delivery, preparation) without internal decomposition checkpoints. Tauri's `#[tauri::command]` pattern encourages a single command file. Workflow delivery naturally touches many systems.

**Why it matters now:** Navigation friction, IDE performance, grep noise, and merge conflicts when multiple features touch the same file. Not a correctness bug, but a productivity multiplier for every future change.

### Problem 3: Data Model Entropy

**Diagnosis:** Three specific problems compound:

1. **`meetings_history`** has 26 columns across 4 concerns (calendar identity, user layer, AI prep, intelligence lifecycle). Every meeting query loads all 26 columns. Writes for prep and writes for intelligence contend on the same row.

2. **`entity_intelligence`** has 21 columns mixing AI output (executive_assessment, risks_json) with operational state (coherence_score, coherence_flagged) and report-layer data (health_score, health_trend). The v1.1.0 intelligence schema redesign (I508) adds 6 new dimensions but has nowhere clean to put them.

3. **Three representations of account-person relationships:** `entity_people` (generic junction), `account_team` (specialized with role/title), and `people.organization/role` (denormalized on person). No FK constraints, no triggers, no consistency enforcement. Code in different modules reads from different tables and sees different truths.

**Root cause:** Each new feature added columns to existing tables (faster than creating new tables). Migrations piled up without consolidation. Relationship representation was never resolved — code worked around the schema instead of fixing it.

**Why it matters now:** I508 (intelligence schema redesign), I499-I503 (health scoring), and I504-I506 (relationship intelligence) all need to write structured data to the intelligence and relationship tables. Building on a 21-column table that mixes 3 concerns guarantees more entropy.

### Problem 4: Silent Pipeline Failures

**Diagnosis:** Three critical pipelines use `let _ =` to discard errors:

- **Transcript processing** silently drops capture insertions, signal emissions, and impact log entries
- **Signal bus** silently fails to flag future meetings with `has_new_signals`
- **Prep invalidation queue** silently drops all invalidations for the rest of the session if the mutex is poisoned

Additionally, the intelligence enrichment pipeline has three phases (gather → PTY → write) that are logically a transaction but not coordinated as one. Phase 2 success doesn't guarantee Phase 3 runs.

**Root cause:** Error handling was deferred during rapid iteration. `let _ =` is Rust's way of saying "I know this can fail and I'm choosing to ignore it." In library code this is acceptable. In application pipelines that feed user-visible intelligence, it's a data loss vector.

**Why it matters now:** Users see "enriched" status in the UI but the system may have lost the actual intelligence data. The v1.1.0 report suite will query intelligence that may be silently incomplete.

### Problem 5: Workspace File Duality

**Diagnosis:** The system maintains two sources of truth:

- **SQLite database** — operational queries, fast reads, service layer writes
- **Workspace files** (`~/.dailyos/{workspace}/`) — durable storage, offline access, prep JSON files

Data flows both directions: `prepare/orchestrate.rs` writes directive JSON to disk, `workflow/deliver.rs` reads it and writes prep/intelligence JSON, `services/dashboard.rs` reads prep JSON from disk and hydrates with DB data, and on archive, data flows back from disk to DB.

The MEMORY.md explicitly calls this out: *"Dual write/read pattern is the #1 source of stale data on disk bugs."* Stale skeleton prep files on disk (`_today/data/preps/*.json` with 5 fields and no intelligence) deserialize successfully into `FullMeetingPrep` with all content as `None`, preventing richer DB sources from loading.

**Root cause:** ADR-0024 chose prep files as offline-readable JSON. ADR-0054 formalized the disk cache pattern. This was the right choice when the DB layer was simpler, but as the DB grew richer (prep_frozen_json, intelligence quality, signal counts), the two sources diverged.

**Why it matters now:** Every new feature that touches prep or intelligence must reason about which source to read from and which to write to. The load order (frozen > context > disk) is implicit, not enforced. New developers (or AI agents) will write to the wrong source.

### Problem 6: Frontend Structural Debt

**Diagnosis from the four frontend audits:**

- **23 ghost components** (3,440 lines) — exported but never imported. Includes 6 complete onboarding chapters (2,801 lines) that were built but never wired in.
- **42 components over 200 lines** — 4 exceed 1,000 lines. `MeetingDetailPage.tsx` (1,751 lines) mixes time parsing, attendee unification, prep data reconciliation, and transient DB retry logic with presentation.
- **60 Tauri commands called directly from components** without hook wrappers. `update_entity_metadata` has 6 call sites across 3 pages.
- **6 duplicate pattern clusters** — entity detail pages share 80%+ structure, report slide pages have identical load-generate-save patterns, hero components have nearly identical 10-15 prop interfaces.
- **Cross-page staleness** — `complete_action` called from 4 locations; non-hook callers don't refresh the action list.
- **3 broken design token references** — `--color-surface-linen`, `--color-turmeric`, `--color-cream-wash` referenced but don't exist.
- **Spacing token gap** — 8px → 16px with nothing between, explaining ~200 hardcoded pixel values.
- **Triple-inlined `IntelligenceQuality`** — same 8-field type defined in 3 separate places.
- **Phantom `DbMeeting.accountId`** — exists in TypeScript but not in the Rust struct; always `undefined`.

**Root cause:** Features were built page-by-page without extracting shared patterns. Business logic landed in the nearest component rather than in hooks. Types were copy-pasted rather than shared.

---

## The Target Architecture

### Principle: Enforced, Not Conventional

Every change below converts a convention ("developers should do X") into an enforcement mechanism ("the compiler/schema/type system requires X").

---

### Workstream 1: Mandatory Service Layer

**Thesis:** Make the service layer the only write path. Commands cannot access `ActionDb` directly for mutations.

**Design:**

```rust
// NEW: src-tauri/src/services/mod.rs

/// ServiceLayer is the ONLY way to mutate application state.
/// Commands receive &ServiceLayer, not &AppState.
/// Read-only queries still go through state.with_db_read().
pub struct ServiceLayer {
    state: Arc<AppState>,
}

impl ServiceLayer {
    // Every mutation method:
    // 1. Validates input
    // 2. Performs the DB write via state.db
    // 3. Emits appropriate signals via emit_signal_and_propagate()
    // 4. Returns the result
    //
    // Commands CANNOT skip step 3 because they don't have direct DB access for writes.

    pub async fn update_account(&self, id: &str, fields: AccountUpdate) -> Result<DbAccount, ServiceError> {
        let account = self.state.with_db_write(|db| {
            db.update_account(id, &fields)
        })?;

        self.emit_signal_and_propagate(SignalEmission {
            entity_type: "account",
            entity_id: id,
            signal_type: "account_updated",
            source: "user_action",
            confidence: 1.0,
            ..Default::default()
        }).await?;

        Ok(account)
    }

    // ... every mutation operation follows this pattern
}
```

**Migration path:**
1. Create `ServiceLayer` struct wrapping `Arc<AppState>`
2. Add `ServiceLayer` to Tauri managed state alongside `AppState`
3. For each command that currently does `state.db.lock()` for writes, migrate to `service.method()`
4. Commands retain `state.with_db_read()` for reads (no service needed for queries)
5. Once all write commands use `ServiceLayer`, remove `pub` from `state.db` — make it `pub(crate)` accessible only from `services/`

**Enforcement mechanism:** `AppState.db` becomes `pub(crate)` — only the `services` module can access it for writes. Commands receive `State<ServiceLayer>` for mutations and `State<AppState>` for reads.

**Signal coverage guarantee:** Every `ServiceLayer` method emits signals. It's impossible to mutate through the service layer without signaling — the method won't compile without the emission call.

**Files to create/modify:**
| File | Change |
|------|--------|
| `src-tauri/src/services/mod.rs` | New `ServiceLayer` struct with all mutation methods |
| `src-tauri/src/services/{domain}.rs` | Migrate existing service functions into `ServiceLayer` methods |
| `src-tauri/src/commands.rs` → `src-tauri/src/commands/` | Refactored (see Workstream 2) |
| `src-tauri/src/state.rs` | `db` field visibility changes to `pub(crate)` |
| `src-tauri/src/lib.rs` | Register `ServiceLayer` as Tauri managed state |

**Definition of done:** `grep -r "state.db.lock" src-tauri/src/commands/` returns zero results for write operations. Every entity mutation emits a signal (verified by the signal coverage matrix in COMMAND-REFERENCE.md).

---

### Workstream 2: Module Decomposition

**Thesis:** Split god modules into domain-scoped files with clear ownership boundaries.

**Target structure:**

```
src-tauri/src/
├── commands/
│   ├── mod.rs                    # Re-exports all command registration
│   ├── accounts.rs               # Account CRUD, hierarchy, team (~30 commands)
│   ├── meetings.rs               # Meeting CRUD, linking, capture (~25 commands)
│   ├── people.rs                 # Person CRUD, relationships (~20 commands)
│   ├── projects.rs               # Project CRUD, hierarchy (~15 commands)
│   ├── actions.rs                # Action CRUD, state transitions (~15 commands)
│   ├── email.rs                  # Email sync, enrichment, dismiss (~15 commands)
│   ├── intelligence.rs           # Enrichment, reports (~15 commands)
│   ├── config.rs                 # Settings, auth, workspace (~20 commands)
│   ├── user_entity.rs            # User profile, context, priorities (~15 commands)
│   ├── onboarding.rs             # Wizard, demo data (~10 commands)
│   └── diagnostics.rs            # Dev tools, system status (~10 commands)
├── workflow/
│   ├── deliver/
│   │   ├── mod.rs                # Orchestration
│   │   ├── briefing.rs           # Daily briefing delivery
│   │   ├── meeting_prep.rs       # Per-meeting prep delivery
│   │   ├── email_synthesis.rs    # Email brief delivery
│   │   ├── intelligence.rs       # Entity intelligence delivery
│   │   └── actions.rs            # Action enrichment delivery
│   └── reconcile.rs              # Archive/reconciliation (kept together)
├── prepare/
│   ├── orchestrate/
│   │   ├── mod.rs                # Phase 1 orchestration
│   │   ├── today.rs              # Today workflow preparation
│   │   ├── week.rs               # Week workflow preparation
│   │   └── email.rs              # Email refresh
│   ├── meeting_context.rs        # (kept — single responsibility)
│   ├── entity_resolver.rs        # (kept)
│   └── email_classify.rs         # (kept)
├── hygiene/
│   ├── mod.rs                    # Orchestrator + budget
│   ├── duplicates.rs             # Duplicate detection
│   ├── name_resolution.rs        # Name/domain resolution
│   ├── readiness.rs              # Meeting readiness checks
│   └── scanner.rs                # Overnight scanning
├── intelligence/
│   ├── prompts/
│   │   ├── mod.rs                # Prompt builder orchestration
│   │   ├── account.rs            # Account-specific prompts
│   │   ├── person.rs             # Person-specific prompts
│   │   ├── project.rs            # Project-specific prompts
│   │   └── meeting.rs            # Meeting-specific prompts
│   ├── io.rs                     # (kept — JSON I/O)
│   ├── compute.rs                # (kept — PTY calls)
│   ├── lifecycle.rs              # (kept)
│   └── validation.rs             # (kept)
└── signals/
    ├── rules/
    │   ├── mod.rs                # Rule registry
    │   ├── account_rules.rs      # Account-domain signal rules
    │   ├── meeting_rules.rs      # Meeting-domain rules
    │   ├── people_rules.rs       # People-domain rules
    │   └── email_rules.rs        # Email-domain rules
    ├── bus.rs                    # (kept — core emission)
    ├── propagation.rs            # (kept — made async, see Workstream 4)
    ├── feedback.rs               # (kept)
    └── decay.rs                  # (kept)
```

**Migration path:** This is mechanical refactoring. Each step:
1. Create the target directory/file
2. Move functions with `pub(crate)` visibility
3. Add re-export in `mod.rs`
4. Verify `cargo check` passes
5. No behavior change — pure structural reorganization

**Command consolidation (188 → ~120):**
| Current Commands | Consolidated To | Reduction |
|-----------------|----------------|-----------|
| `link_meeting_entity` + `add_meeting_entity` + `update_meeting_entity` | `set_meeting_entities` (replaces all) + `remove_meeting_entity` | 5 → 2 |
| `complete_action` + `reopen_action` + `accept_proposed_action` + `reject_proposed_action` | `transition_action_state(id, new_state)` | 4 → 1 |
| `create_entity_context_entry` + `update_entity_context_entry` + `delete_entity_context_entry` | `save_entity_context_entry` (upsert) + `delete_entity_context_entry` | 3 → 2 |
| Multiple `get_*_detail` commands with overlapping queries | Parameterized `get_entity_detail(type, id, sections)` | ~6 → 1 |

**Definition of done:** No file over 2,000 lines in `commands/`, `workflow/`, `prepare/`, `hygiene/`, or `signals/`. `commands.rs` deleted. All commands registered via domain modules.

---

### Workstream 3: Data Model Consolidation

**Thesis:** Decompose bloated tables, unify duplicate representations, and add missing constraints.

#### 3a: Decompose `meetings_history` (26 columns → 4 tables)

```sql
-- Core meeting record (calendar identity + metadata)
CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    calendar_event_id TEXT,
    google_event_id TEXT,
    organizer TEXT,
    organizer_email TEXT,
    location TEXT,
    meeting_type TEXT DEFAULT 'external',
    attendee_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- User-authored content (notes, agenda)
CREATE TABLE meeting_user_content (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id),
    user_agenda_json TEXT,
    user_notes TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- AI-generated prep (frozen snapshots, context)
CREATE TABLE meeting_prep (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id),
    prep_frozen_json TEXT,
    prep_frozen_at TEXT,
    prep_snapshot_participant_count INTEGER,
    prep_snapshot_created_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Intelligence lifecycle (enrichment state, signal tracking)
CREATE TABLE meeting_intelligence (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id),
    intelligence_state TEXT DEFAULT 'pending',
    intelligence_quality TEXT,
    last_enriched_at TEXT,
    enriched_at TEXT,
    signal_count INTEGER DEFAULT 0,
    has_new_signals INTEGER DEFAULT 0,
    last_viewed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Drop: prep_context_json (dead column, never read)
-- Drop: attendees JSON blob (replaced by meeting_attendees junction table)
```

**Migration strategy:** Single migration that:
1. Creates the 4 new tables
2. `INSERT INTO meetings SELECT ... FROM meetings_history`
3. Same for meeting_user_content, meeting_prep, meeting_intelligence
4. Creates views: `CREATE VIEW meetings_history_compat AS SELECT ... FROM meetings JOIN meeting_user_content ... JOIN meeting_prep ... JOIN meeting_intelligence ...`
5. Keeps the compat view for 2 versions, then drops it

#### 3b: Decompose `entity_intelligence` (21 columns → 2 tables)

```sql
-- AI-generated assessment (the intelligence itself)
CREATE TABLE entity_assessment (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth_json TEXT,
    value_delivered TEXT,
    -- v1.1.0 I508 dimensions live here:
    strategic_assessment_json TEXT,
    relationship_health_json TEXT,
    engagement_cadence_json TEXT,
    value_outcomes_json TEXT,
    commercial_context_json TEXT,
    external_health_json TEXT,
    enriched_at TEXT,
    last_enriched_at TEXT,
    refresh_needed INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Operational quality state (self-healing, coherence)
CREATE TABLE entity_quality (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    coherence_score REAL,
    coherence_flagged INTEGER DEFAULT 0,
    last_coherence_check TEXT,
    health_score REAL,
    health_trend TEXT,
    health_confidence REAL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**This aligns directly with I508** (intelligence schema redesign). The 6 new dimensions have a clean home in `entity_assessment`. Health scoring (I499-I503) lives in `entity_quality` where it belongs — separate from the AI narrative.

#### 3c: Unify relationship representations

```sql
-- One table for account-person relationships (replaces entity_people + account_team)
CREATE TABLE account_stakeholders (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    person_id TEXT NOT NULL REFERENCES people(id),
    role TEXT,                           -- from account_team
    title TEXT,                          -- from account_team
    email TEXT,                          -- from account_team
    data_source TEXT DEFAULT 'user',     -- ADR-0098 provenance
    engagement_level TEXT,               -- 'active' | 'warm' | 'cold' (computed)
    last_meeting_date TEXT,              -- denormalized for query speed
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(account_id, person_id)
);

-- Keep entity_people for non-account entity types (projects, user entity)
-- But add FK constraints:
ALTER TABLE entity_people ADD CONSTRAINT fk_person FOREIGN KEY (person_id) REFERENCES people(id);

-- Drop: people.organization and people.role columns (now in account_stakeholders)
-- These were denormalized on the person row, but a person can be stakeholder in multiple accounts
```

**Migration strategy:**
1. Create `account_stakeholders` from `account_team` data
2. Backfill engagement_level and last_meeting_date from meeting_attendees
3. Create trigger: INSERT/UPDATE on `account_stakeholders` → sync to `entity_people` for backward compat
4. Migrate all reads from `entity_people WHERE entity_type = 'account'` to `account_stakeholders`
5. After 2 versions, remove the sync trigger and the redundant `entity_people` rows for accounts

#### 3d: Fix FK bug and add missing indexes

```sql
-- Fix P0-1: FK bug in drive_watched_sources
-- (migration already needed for table decomposition above)

-- Missing indexes (from PERF-1 through PERF-4 in GAP-ANALYSIS):
CREATE INDEX idx_meetings_intelligence_state ON meeting_intelligence(intelligence_state);
CREATE INDEX idx_meetings_start_time ON meetings(start_time);
CREATE INDEX idx_signal_events_entity_created ON signal_events(entity_id, created_at);
CREATE INDEX idx_signal_events_type ON signal_events(signal_type);
CREATE INDEX idx_entity_assessment_refresh ON entity_assessment(refresh_needed) WHERE refresh_needed = 1;
```

**Definition of done:** `meetings_history` table dropped (replaced by 4 tables + compat view). `entity_intelligence` table dropped (replaced by 2 tables). `account_team` table dropped (replaced by `account_stakeholders`). Zero `entity_people` rows where `entity_type = 'account'` (all in `account_stakeholders`). All FK constraints pass `PRAGMA foreign_key_check`.

---

### Workstream 4: Pipeline Reliability

**Thesis:** Replace silent failures with explicit error handling. Make multi-phase operations atomic or explicitly compensating.

#### 4a: Eliminate `let _ =` in pipelines

Every `let _ =` in a pipeline (not in cleanup/best-effort code) becomes one of:
- `?` (propagate error, fail the operation)
- `if let Err(e) = ... { log::error!("..."); }` (log and continue, when the side effect is non-critical)
- An explicit `// BEST_EFFORT: ...` comment when silence is intentional

**Concrete changes:**

```rust
// BEFORE (transcript processing):
let _ = insert_capture(...);
let _ = emit_signal(...);
let _ = append_impact_log(...);

// AFTER:
insert_capture(...)?;  // Critical: capture is the point of transcript processing
if let Err(e) = emit_signal_and_propagate(...) {
    log::error!("Signal emission failed for transcript capture {}: {}", capture_id, e);
    // Signal failure is non-critical to the capture itself, but must be visible
}
append_impact_log(...)?;  // Critical: impact log is audit trail
```

```rust
// BEFORE (signal bus, future meeting flagging):
let _ = db.execute("UPDATE meetings_history SET has_new_signals = 1 WHERE ...");

// AFTER:
let rows_affected = db.execute(
    "UPDATE meeting_intelligence SET has_new_signals = 1 WHERE meeting_id IN (...)",
    params,
)?;
if rows_affected == 0 {
    log::warn!("Signal flagging matched no future meetings for entity {}", entity_id);
}
```

```rust
// BEFORE (prep invalidation queue):
if let Ok(mut queue) = prep_invalidation_queue.lock() {
    // process
} else {
    // poisoned: silently return
}

// AFTER:
match prep_invalidation_queue.lock() {
    Ok(mut queue) => { /* process */ }
    Err(poisoned) => {
        log::error!("Prep invalidation queue poisoned — recovering with into_inner()");
        let mut queue = poisoned.into_inner();
        // Recover the mutex and continue processing
        // Mutex poisoning means a thread panicked while holding the lock,
        // but the data inside may still be usable
    }
}
```

#### 4b: Async signal propagation

```rust
// CURRENT: Synchronous propagation blocks the caller
pub fn emit_signal_and_propagate(state: &AppState, emission: SignalEmission) -> Result<()> {
    let signal_id = emit_signal(&state.db, &emission)?;
    // 9 propagation rules run INLINE, each querying DB
    for rule in &PROPAGATION_RULES {
        rule(state, &emission)?;
    }
    Ok(())
}

// TARGET: Emit synchronously, propagate asynchronously
pub async fn emit_signal_and_propagate(state: &AppState, emission: SignalEmission) -> Result<()> {
    let signal_id = emit_signal(&state.db, &emission)?;

    // Spawn propagation as a background task
    let state_clone = state.clone();
    let emission_clone = emission.clone();
    tokio::spawn(async move {
        if let Err(e) = run_propagation_rules(&state_clone, &emission_clone).await {
            log::error!("Signal propagation failed for {}: {}", signal_id, e);
        }
    });

    Ok(())
}
```

**Trade-off:** Derived signals become eventually consistent (milliseconds, not synchronous). This is acceptable — derived signals inform future prep/intelligence, not immediate UI responses.

#### 4c: Intelligence pipeline saga pattern

```rust
// TARGET: Explicit three-phase saga with compensation
pub async fn enrich_entity(service: &ServiceLayer, entity_id: &str) -> Result<EnrichmentResult> {
    // Phase 1: Gather (read-only, can't fail destructively)
    let context = gather_intelligence_context(service, entity_id).await?;

    // Phase 2: Generate (PTY call, external dependency)
    let ai_response = match generate_intelligence(&context).await {
        Ok(response) => response,
        Err(e) => {
            // Mark entity as "enrichment_failed" — visible in UI
            service.mark_enrichment_failed(entity_id, &e.to_string()).await?;
            return Err(e);
        }
    };

    // Phase 3: Persist (writes, signals, cascades — all through ServiceLayer)
    let result = service.apply_intelligence(entity_id, &ai_response).await?;
    // ServiceLayer.apply_intelligence() internally:
    //   1. Writes to entity_assessment
    //   2. Updates entity_quality
    //   3. Emits intelligence_updated signal (which triggers prep invalidation via propagation)
    //   4. Marks linked reports as stale
    // All in a single DB transaction via service.state.with_db_write()

    Ok(result)
}
```

**Key change:** Phase 3 is a single `ServiceLayer` method that wraps all writes + signals in one transaction. If any part fails, the transaction rolls back and the entity retains its previous intelligence. No partial state.

#### 4d: Signal GC for unbounded growth

```rust
// New: periodic signal garbage collection
pub fn gc_expired_signals(db: &ActionDb) -> Result<usize> {
    let deleted = db.execute(
        "DELETE FROM signal_events
         WHERE decayed_weight < 0.01
         AND source != 'user_correction'
         AND created_at < datetime('now', '-90 days')",
        [],
    )?;
    Ok(deleted)
}
// Called from hygiene loop, budget: 1 call per day
```

**Definition of done:** Zero `let _ =` in pipeline code (verified by `grep -rn "let _ =" src-tauri/src/{processor,signals,intel_queue,meeting_prep_queue,workflow}/*.rs`). Signal propagation is async. Intelligence enrichment uses saga pattern with explicit failure states. Signal table has GC.

---

### Workstream 5: Eliminate Workspace File Duality

**Thesis:** DB is the sole source of truth. Workspace files become an export format, not a read source.

**Current flow:**
```
prepare/orchestrate.rs → writes directive JSON to disk
workflow/deliver.rs → reads directive, writes prep/intelligence JSON to disk
services/dashboard.rs → reads prep JSON from disk, hydrates with DB
frontend → receives hydrated data
```

**Target flow:**
```
prepare pipeline → writes to DB tables directly
services/dashboard.rs → reads from DB only
frontend → receives data from DB
workspace files → generated on demand for export/offline reading (write-only from app's perspective)
```

**Migration:**

1. **Directive files → DB table:**
```sql
CREATE TABLE workflow_directives (
    id TEXT PRIMARY KEY,
    workflow_type TEXT NOT NULL,  -- 'today' | 'week'
    directive_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT  -- auto-cleanup after 7 days
);
```

2. **Prep files → `meeting_prep.prep_frozen_json`** (already exists, just stop reading from disk):
   - `load_meeting_prep_from_sources()` reads DB only
   - Remove disk file fallback
   - Prep files written to disk only for offline export

3. **Intelligence files → `entity_assessment`** (from Workstream 3):
   - `entity_intelligence/{id}.json` becomes export-only
   - All intelligence reads come from DB

4. **Dashboard data → DB queries** (already partially there):
   - `services/dashboard.rs` stops reading `_today/data/meetings.json`
   - Reads from `meetings` + `meeting_prep` + `meeting_intelligence` tables

5. **Keep workspace files as export targets:**
   - On each workflow run, WRITE workspace files for offline reading
   - On archive, WRITE reconciled data to workspace files for portability
   - Never READ from workspace files for operational queries

**The key principle:** Workspace files are a **write-through cache for export**, not a read source. The DB is always authoritative.

**Definition of done:** `grep -rn "read_to_string" src-tauri/src/services/` returns zero results for `_today/data/` paths. All operational reads come from DB. Workspace files still exist for export/offline but are never the authoritative source.

---

### Workstream 6: Frontend Structural Refactor

#### 6a: Delete ghost components (23 files, 3,440 lines)

Immediate deletion. These are dead code:

**Onboarding chapters (never wired):** `DashboardTour.tsx`, `InternalTeamSetup.tsx`, `InboxTraining.tsx`, `PopulateWorkspace.tsx`, `PrimeBriefing.tsx`, `MeetingDeepDive.tsx`

**Dead UI primitives:** Identified in FRONTEND-COMPONENTS.md — 10 components exported but never imported.

**Dead report components:** 3 report slide components with no parent.

#### 6b: Extract business logic from presentation

**MeetingDetailPage.tsx (1,751 → ~500 lines):**
```
Extract to hooks:
  - useMeetingPrep(meetingId)     — prep loading, reconciliation, retry logic
  - useMeetingAttendees(meetingId) — attendee unification, entity resolution display
  - useMeetingTimeParsing(meeting) — time formatting, duration calculation, conflict detection

MeetingDetailPage becomes pure layout:
  const { prep, loading, error } = useMeetingPrep(meetingId);
  const { attendees, entities } = useMeetingAttendees(meetingId);
  return <MeetingBriefingLayout prep={prep} attendees={attendees} ... />;
```

**DailyBriefing.tsx — readiness score computation → `useReadinessScore()` hook**

**ActionsPage.tsx — action grouping/sorting → `useGroupedActions()` hook**

**InboxPage.tsx — file classification → `useClassifiedInbox()` hook**

**`cronToHumanTime()`** — deduplicate from SystemStatus.tsx and DiagnosticsSection.tsx into `src/utils/time.ts`

#### 6c: Shared component shells

```typescript
// NEW: src/components/entity/EntityDetailShell.tsx
// Replaces 80%+ duplicate structure across AccountDetailPage, PersonDetailPage, ProjectDetailPage
interface EntityDetailShellProps {
  entity: EntityBase;
  hero: React.ReactNode;        // Entity-specific hero component
  chapters: ChapterConfig[];     // Chapter definitions with render functions
  actions: EntityAction[];       // Toolbar actions
}

// NEW: src/components/reports/ReportSlideShell.tsx
// Replaces identical load-generate-save pattern across 5 report pages
interface ReportSlideShellProps {
  reportType: ReportType;
  entityId: string;
  slides: SlideDefinition[];
}

// NEW: src/components/entity/EntityHero.tsx
// Replaces 3 nearly-identical hero components with unified props
interface EntityHeroProps {
  type: 'account' | 'person' | 'project';
  name: string;
  subtitle?: string;
  health?: HealthIndicator;
  actions: HeroAction[];
  metadata: MetadataField[];
}
```

#### 6d: Hook wrapper for every Tauri command used in components

**Priority extractions (highest call-site count):**
```typescript
// NEW: src/hooks/useEntityMutation.ts
export function useEntityMutation() {
  return useMutation({
    mutate: (params: { entityId: string; field: string; value: any }) =>
      invoke('update_entity_metadata', params),
    onSuccess: () => {
      // Invalidate relevant queries — solves cross-page staleness
      queryClient.invalidateQueries(['entity', params.entityId]);
    }
  });
}

// NEW: src/hooks/useReportGeneration.ts
export function useReportGeneration(reportType: ReportType, entityId: string) { ... }

// NEW: src/hooks/useActionTransition.ts
export function useActionTransition() { ... }
```

**Pattern:** Every hook that wraps a mutation command should invalidate related queries. This solves the cross-page staleness bug (completing an action from MeetingDetailPage now invalidates the actions list on ActionsPage).

#### 6e: Fix design token issues

```css
/* Add missing tokens to design-tokens.css */
--color-surface-linen: var(--color-cream-light);  /* or correct value */
--space-xs: 4px;    /* Fill the 8px→16px gap */
--space-sm: 8px;    /* existing */
--space-md-sm: 12px; /* NEW: between sm and md */
--space-md: 16px;   /* existing */

/* Remove broken references */
/* WeekPage.module.css:99 — replace --color-surface-linen with correct token */
/* meeting-intel.module.css:1102-1103 — replace --color-turmeric with --color-accent-turmeric */
```

**Delete entity color alias tokens** (5 tokens, 0 usage) or wire them into the entity components where they belong.

#### 6f: Type consolidation

```typescript
// SINGLE definition of IntelligenceQuality (currently triple-inlined)
// src/types/index.ts
export interface IntelligenceQuality {
  level: 'sparse' | 'developing' | 'ready' | 'fresh';
  signalCount: number;
  sourceCount: number;
  lastEnrichedAt: string | null;
  hasFreshSignals: boolean;
  meetingCount: number;
  emailSignalCount: number;
  confidenceNote: string | null;
}

// Delete phantom field:
// DbMeeting.accountId — remove from TypeScript (doesn't exist in Rust)

// Delete ghost types:
// EmailSummaryData, EmailStats — superseded by EmailBriefingData/EmailBriefingStats
```

**Definition of done:** Zero ghost components. No component file over 500 lines (pages) or 200 lines (components). Every Tauri mutation command has a hook wrapper. Zero broken token references. `IntelligenceQuality` defined once. `DbMeeting.accountId` removed.

---

## Workstream Dependencies

```
Workstream 1 (Service Layer)
    ↓ enables
Workstream 2 (Module Decomposition) — can start in parallel for non-service files

Workstream 3 (Data Model) — independent, can start immediately
    ↓ enables
Workstream 5 (Workspace File Elimination) — depends on new table structure

Workstream 4 (Pipeline Reliability) — independent, can start immediately
    ↓ benefits from
Workstream 1 (Service Layer) — saga pattern uses ServiceLayer

Workstream 6 (Frontend) — independent, can start immediately
```

**Recommended execution order:**
1. **Parallel start:** Workstream 3 (data model), Workstream 4 (pipeline fixes), Workstream 6 (frontend)
2. **After data model tables exist:** Workstream 5 (workspace file elimination)
3. **When ready for larger refactor:** Workstream 1 (service layer) → Workstream 2 (module decomposition)

Workstreams 3, 4, and 6 are independent and can be executed concurrently. Workstream 1 is the most impactful single change but also the largest. Workstream 5 is the most satisfying — it eliminates the #1 bug source — but depends on Workstream 3 for the new table structure.

---

## Relationship to v1.1.0

This proposal does **not** replace the v1.1.0 plan. It provides the structural foundation that makes v1.1.0 features more reliable:

| v1.1.0 Issue | Benefits From |
|-------------|---------------|
| I508 (intelligence schema redesign) | Workstream 3b — `entity_assessment` table is the clean home for 6 dimensions |
| I499-I503 (health scoring) | Workstream 3b — `entity_quality` table separates scoring from narrative |
| I504-I506 (relationship intelligence) | Workstream 3c — `account_stakeholders` is the unified relationship table |
| I487 (Glean signal emission) | Workstream 1 — ServiceLayer guarantees signal emission |
| I489-I491 (report suite) | Workstream 4 — saga pattern ensures intelligence is complete before reports consume it |
| I492-I493 (portfolio surfaces) | Workstream 5 — DB-only reads mean no stale file surprises |

**The question is sequencing.** Two options:

**Option A: Refactor first, then v1.1.0 features.**
Build the structural foundation (4-6 weeks), then build v1.1.0 features on solid ground. Higher upfront cost, lower feature risk.

**Option B: Interleave refactor with v1.1.0.**
Execute Workstreams 3 and 4 first (they directly enable I508 and pipeline reliability), then build v1.1.0 Phase 1 on the new tables, then continue Workstreams 1, 2, 5, 6 as capacity allows. Lower upfront cost, requires more careful sequencing.

**Recommendation: Option B.** Workstreams 3 and 4 are prerequisites for v1.1.0 anyway — they're not overhead, they're the foundation. Workstreams 1, 2, 5, and 6 improve developer velocity but don't block v1.1.0 features.

---

## What This Proposal Does NOT Recommend

1. **A rewrite.** The Tauri + React + SQLite stack is correct. The signal bus architecture is sound. The editorial design language works. This is a structural refactor, not a new system.

2. **Changing the PTY-based AI model.** Claude Code via PTY (ADR-0005) works. The IntelligenceProvider abstraction (ADR-0091) is designed but deferred to v2.1.0. This refactor doesn't touch the AI layer.

3. **Cloud sync or SaaS architecture.** Local-first (ADR-0007) is a load-bearing architectural decision. This refactor strengthens local-first by making the DB the sole source of truth.

4. **Removing workspace files entirely.** Workspace files remain as an export/offline format. The change is that they stop being a read source for operational queries.

5. **A new frontend framework.** React + TypeScript is fine. The frontend changes are structural (extract hooks, shared shells, delete dead code), not framework-level.

---

## Verification Criteria

After all 6 workstreams complete:

1. **Signal coverage:** Every entity mutation command emits at least one signal. Verified by `COMMAND-REFERENCE.md` signal matrix — zero gaps.

2. **No direct DB writes from commands:** `grep -r "state.db.lock" src-tauri/src/commands/` returns zero write operations. All mutations go through `ServiceLayer`.

3. **No god modules:** No `.rs` file over 2,000 lines except `types.rs` (pure data definitions).

4. **Data model normalized:** `meetings_history` dropped. `entity_intelligence` dropped. `account_team` dropped. All FKs pass `PRAGMA foreign_key_check`.

5. **Zero silent pipeline failures:** `grep -rn "let _ =" src-tauri/src/{processor,signals,intel_queue,meeting_prep_queue,workflow}/` returns zero results (excluding intentional best-effort cleanup).

6. **DB as sole read source:** `grep -rn "read_to_string.*_today" src-tauri/src/services/` returns zero results.

7. **Frontend cleanup:** Zero ghost components. `IntelligenceQuality` defined once. Every mutation command has a hook wrapper. Zero broken token references.

8. **All existing tests pass:** `cargo test`, `cargo clippy -- -D warnings`, `pnpm tsc --noEmit`.

---

## Appendix: Metrics Before vs. After

| Metric | Before | Target After |
|--------|--------|-------------|
| Largest `.rs` file (non-type) | 8,940 lines (commands.rs) | < 2,000 lines |
| Commands without signal emission | ~25 | 0 |
| `let _ =` in pipeline code | 8+ instances | 0 |
| Representations of account-person | 3 tables | 1 table + 1 view |
| `meetings_history` columns | 26 | 12 (core table) |
| `entity_intelligence` columns | 21 | Split: 16 (assessment) + 8 (quality) |
| Ghost frontend components | 23 | 0 |
| Components over 1,000 lines | 4 | 0 |
| Frontend types defined > once | 3 (`IntelligenceQuality`) | 0 |
| Workspace files read as source | ~12 read paths | 0 (export-only) |
| Signal propagation model | Synchronous | Async |
| Intelligence pipeline error model | Silent (`let _ =`) | Saga with compensation |
