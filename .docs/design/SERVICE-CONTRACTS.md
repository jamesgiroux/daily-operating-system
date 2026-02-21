# Service Contracts

**Purpose:** Actionable contracts for the target service layer extraction.
**Status:** Specification. Partially underway — `services/` module exists with initial extractions.
**Source:** Architectural assessment Section 10 + codebase inspection (2026-02-20)

---

## Current State

```
Frontend hook → invoke() → commands.rs → state.with_db_read/write() → db/ methods
```

**Problem:** `commands.rs` (~11,500 lines) contains IPC dispatch + business logic + validation + data transformation. There is no place for business rules between the IPC boundary and raw SQL. Business logic is untestable without the full Tauri runtime.

**What exists today in `services/`:**
- `services/entities.rs` — `build_entity_signal_prose()` (extracted from commands.rs)
- `services/actions.rs` — Action business logic (partial)
- `services/accounts.rs` — Account business logic (partial)
- `services/people.rs` — People business logic (partial)
- `services/meetings.rs` — Meeting business logic (partial)

These are early extractions. Most business logic still lives in `commands.rs`.

---

## Target Services

### AccountService

**Owns:** Account lifecycle, hierarchy, domain mapping, intelligence triggers

**Public methods:**
```
create(name, parent_id?, metadata) → Account
get_detail(id) → AccountDetail (with team, children, events, intel)
update_field(id, field, value) → Account
update_notes(id, notes) → ()
update_programs(id, programs) → ()
list(filters?) → Vec<Account>
list_for_picker() → Vec<AccountPickerItem>
get_children(id) → Vec<Account>
get_ancestors(id) → Vec<Account>
get_descendants(id) → Vec<Account>
merge(source_id, target_id) → Account       // MUST be transactional
archive(id) → ()
restore(id) → ()
get_archived() → Vec<Account>
bulk_create(accounts) → Vec<Account>
add_team_member(account_id, person_id, role) → ()
remove_team_member(account_id, person_id) → ()
get_team(account_id) → Vec<TeamMember>
record_event(account_id, event) → ()
get_events(account_id) → Vec<AccountEvent>
enrich(id) → ()                              // triggers intel_queue
sync_from_workspace(path, db) → usize        // startup sync
sync_content_indexes(path, db) → usize       // startup content sync
```

**Does NOT touch:**
- Meeting prep generation (that's MeetingService)
- Signal emission (that's SignalService)
- Email classification (that's the prepare pipeline)

---

### PersonService

**Owns:** Person lifecycle, merge, relationship classification, enrichment triggers

**Public methods:**
```
create(name, email?, entity_links?) → Person
get_detail(id) → PersonDetail (with entities, meetings, emails, intel)
update(id, fields) → Person
search(query) → Vec<Person>
list(filters?) → Vec<Person>
merge(source_id, target_id) → Person         // MUST be transactional
delete(id) → ()
archive(id) → ()
get_archived() → Vec<Person>
link_entity(person_id, entity_id, entity_type) → ()
unlink_entity(person_id, entity_id) → ()
get_for_entity(entity_id) → Vec<Person>
get_meeting_attendees(meeting_id) → Vec<Person>
get_frequent_correspondents() → Vec<Person>
get_duplicates() → Vec<DuplicatePair>
get_duplicates_for(person_id) → Vec<DuplicatePair>
enrich(id) → ()                              // triggers intel_queue
enrich_from_clay(id) → ()                    // Clay enrichment
get_avatar(id) → Option<AvatarUrl>
bulk_fetch_gravatars(person_ids) → ()
sync_from_workspace(path, db, domains) → usize
```

**Does NOT touch:**
- Account hierarchy (that's AccountService)
- Meeting prep (that's MeetingService)
- Signal propagation rules (that's SignalService)

---

### MeetingService

**Owns:** Prep lifecycle, intelligence quality, entity linking, transcript attachment, outcomes

**Public methods:**
```
get_intelligence(meeting_id) → MeetingIntelligence
generate_intelligence(meeting_id) → MeetingIntelligence  // AI call
get_prep(meeting_id) → FullMeetingPrep
list_preps() → Vec<PrepSummary>
backfill_prep_semantics() → ()
get_history(filters?) → Vec<Meeting>
get_history_detail(meeting_id) → MeetingHistoryDetail
search(query) → Vec<Meeting>
get_current() → Option<CalendarEvent>
get_next() → Option<CalendarEvent>
link_entity(meeting_id, entity_id, entity_type, role?) → ()
unlink_entity(meeting_id, entity_id) → ()
add_entity(meeting_id, entity_id) → ()       // additive (no overwrite)
remove_entity(meeting_id, entity_id) → ()
get_entities(meeting_id) → Vec<MeetingEntity>
update_entity(meeting_id, entity_id, fields) → ()
attach_transcript(meeting_id, transcript) → ()
get_outcomes(meeting_id) → Vec<CapturedOutcome>
capture_outcome(meeting_id, outcome) → ()
update_capture(meeting_id, fields) → ()
update_user_agenda(meeting_id, agenda) → ()
update_user_notes(meeting_id, notes) → ()
apply_prep_prefill(meeting_id) → ()
generate_agenda_message_draft(meeting_id) → String
get_timeline(meeting_id) → Vec<TimelineEvent>
trigger_quill_sync(meeting_id) → ()
backfill_historical() → ()
```

**Does NOT touch:**
- Calendar polling (that stays in `google.rs`)
- Workflow orchestration (that stays in `executor.rs`)
- Post-meeting capture detection loop (that stays in `capture.rs`)

---

### ActionService

**Owns:** Action CRUD, status transitions, temporal grouping, source tracking

**Public methods:**
```
create(title, priority, account_id?, project_id?) → Action
update(id, fields) → Action
complete(id) → Action
reopen(id) → Action
get_detail(id) → ActionDetail
list(filters?) → Vec<Action>
get_from_db(filters?) → Vec<DbAction>
accept_proposed(id) → Action
reject_proposed(id) → ()
get_proposed() → Vec<ProposedAction>
update_priority(id, priority) → ()
```

**Does NOT touch:**
- Action extraction from emails (that's `processor/email_actions.rs`)
- Action extraction from prep (that's `prepare/actions.rs`)
- Workflow today action rollup (that's `workflow/today.rs`)

---

### IntelligenceService

**Owns:** Quality assessment, staleness detection, enrichment orchestration, prompt building

**Public methods:**
```
get_executive() → ExecutiveIntelligence
enrich_account(id) → ()                      // queues AI enrichment
enrich_person(id) → ()
enrich_project(id) → ()
update_field(entity_id, field, value) → ()   // user edits
update_stakeholders(entity_id, stakeholders) → ()
create_person_from_stakeholder(stakeholder) → Person
get_hygiene_status() → IntelligenceHygieneStatus
generate_risk_briefing() → RiskBriefing
get_risk_briefing() → Option<RiskBriefing>
save_risk_briefing(briefing) → ()
```

**Does NOT touch:**
- Signal bus (that's SignalService)
- Meeting prep generation (that's MeetingService)
- Background intel queue loop (that stays in `intel_queue.rs`)

---

### SignalService

**Owns:** Signal emission, propagation, fusion, callout generation

**Already partially exists** in `signals/` module. The contract formalizes the boundary.

**Public methods:**
```
emit(entity_id, signal_type, confidence, source, payload) → ()
get_for_entity(entity_id) → Vec<SignalEvent>
get_callouts(entity_id) → Vec<Callout>
run_propagation(entity_id) → ()              // fire cross-entity rules
fuse_signals(entity_id) → FusionResult       // Bayesian fusion
apply_decay() → ()                           // time-based decay pass
record_feedback(signal_id, feedback) → ()    // user correction
invalidate_preps(entity_id) → ()             // queue affected meeting preps
```

**Does NOT touch:**
- Entity CRUD (that's Account/Person/ProjectService)
- Intelligence prompts (that's IntelligenceService)
- Email classification (that's the prepare pipeline)

---

## State Decomposition

**Current:** `AppState` has 28 fields. Every subsystem reaches in for its dependencies.

**Target:** Split into domain-specific containers. AppState becomes a facade.

```rust
// Target structure (P2 priority)
pub struct AppState {
    pub db: DbState,              // db connection, backup state
    pub workflow: WorkflowState,  // status, history, last_scheduled_run
    pub calendar: CalendarState,  // events, week_cache
    pub capture: CaptureState,    // dismissed, captured, transcript_processed
    pub hygiene: HygieneState,    // report, scan_running, budget, timestamps
    pub integrations: IntegrationState, // clay_wake, quill_wake, linear_wake, gravatar
    pub signals: SignalState,     // signal_engine, entity_resolution_wake, prep_invalidation_queue
    pub config: RwLock<Option<Config>>,
    pub intel_queue: Arc<IntelligenceQueue>,
    pub embedding_model: Arc<EmbeddingModel>,
    pub embedding_queue: Arc<EmbeddingQueue>,
    pub active_preset: RwLock<Option<RolePreset>>,
    pub pre_dev_workspace: Mutex<Option<String>>,
}
```

**Migration:** Mechanical refactoring. Change `state.field` to `state.domain.field` in all call sites. No logic changes.

---

## DB Module Split

**Current:** `db/mod.rs` contains ~9,700 lines of SQL operations. Domain modules (`db/actions.rs`, `db/accounts.rs`, etc.) exist but `mod.rs` still holds the bulk.

**Target:** Each domain module owns its queries completely. `mod.rs` retains only:
- `ActionDb` struct definition + `open()` + `conn_ref()`
- `with_transaction()` helper
- Shared `const` column lists (e.g., `ACTIONS_SELECT`, `ACCOUNTS_SELECT`)
- Re-exports from domain modules

**Domain modules:**
| Module | Owns | Est. lines |
|--------|------|------------|
| `db/actions.rs` | 41 action queries, DbAction mapper | ~1,500 |
| `db/accounts.rs` | 36 account queries, DbAccount mapper | ~1,500 |
| `db/meetings.rs` | Meeting history, attendees, entities | ~1,200 |
| `db/people.rs` | People CRUD, merge, duplicates | ~1,200 |
| `db/projects.rs` | Project CRUD | ~800 |
| `db/content.rs` | Content files, embeddings | ~600 |
| `db/signals.rs` | Signal events, propagation state | ~500 |
| `db/emails.rs` | Email signals, threads, dismissals | ~800 |
| `db/mod.rs` | Connection, transaction, column consts | ~600 |
| `db/types.rs` | Row structs (already extracted) | existing |

---

## Migration Path

Extract one service at a time. commands.rs shrinks incrementally.

### Phase 1: Complete existing extractions
1. Finish `services/actions.rs` — move all action business logic from commands.rs
2. Finish `services/accounts.rs` — move account business logic from commands.rs
3. Finish `services/people.rs` — move people business logic from commands.rs
4. Finish `services/meetings.rs` — move meeting business logic from commands.rs

### Phase 2: New services
5. Create `services/intelligence.rs` — extract from commands.rs + consolidate intelligence/ callers
6. Formalize `signals/` as SignalService (already well-modularized)

### Phase 3: DB and state
7. Continue migrating queries from `db/mod.rs` into domain modules
8. Extract `const` column lists to eliminate duplication
9. Split AppState into domain containers

### Per-service extraction pattern:
```
1. Create services/foo.rs with public methods
2. Move business logic from commands.rs into service methods
3. commands.rs handler becomes: parse args → call service → serialize response
4. Update scheduler.rs / other callers to use service instead of direct DB
5. Verify: cargo test, cargo clippy -- -D warnings
```

---

## Invariants (Do NOT Change)

These architectural elements are well-designed and must be preserved:

- **Frontend architecture** — hooks, components, design tokens, editorial aesthetic. The frontend is the best-architected layer.
- **Workflow pipeline** — Prepare → Deliver → Enrich three-phase design. Solid.
- **Signal bus design** — append-only event log, propagation engine, Bayesian fusion. Well-architected.
- **Migration framework** — Sequential SQL migrations (ADR-0071). 33 migrations, versioned, backed up.
- **prepare/, processor/, signals/ module boundaries** — Clean separation. Keep as-is.
- **PtyManager** — Good abstraction for Claude Code subprocess.
- **Hook-per-domain frontend pattern** — No global store needed. Backend is source of truth.
- **Design token system** — `design-tokens.css` as single source of truth. Universal compliance.

---

## Cross-References

- **Architecture map:** [ARCHITECTURE-MAP.md](./ARCHITECTURE-MAP.md) — module boundaries, data flow, async tasks
- **Full diagnostic:** [../plans/architectural-assessment.md](../plans/architectural-assessment.md) — assessment with severity ratings
- **Design system:** [DESIGN-SYSTEM.md](./DESIGN-SYSTEM.md) — frontend design rules (invariant)
