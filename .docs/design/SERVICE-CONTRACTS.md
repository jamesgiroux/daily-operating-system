# Service Contracts

> **SUPERSEDED 2026-03-02.** Replaced by `.docs/architecture/COMMAND-REFERENCE.md` + `.docs/architecture/MODULE-MAP.md`. Retained for historical reference (documents the v0.13.6 service layer extraction).

**Purpose:** Actionable contracts for the target service layer extraction.
**Status:** Phase 2 complete (v0.13.6). 11 service files, commands.rs reduced to ~7,000 lines. SignalService formalization (I403) deferred.
**Source:** Architectural assessment Section 10 + codebase inspection (2026-02-20). Updated 2026-02-22 after v0.13.6 maximum extraction.

---

## Current State

```
Frontend hook → invoke() → commands.rs → services/*.rs → db/ methods
```

**Post v0.13.6:** `commands.rs` (~6,992 lines, down from 8,389) is a thin IPC dispatch layer. All thick command handlers have been extracted to domain services. Command handlers parse Tauri args, acquire locks, call service methods, and serialize responses.

**What exists in `services/`:**
- `services/actions.rs` (466 lines) — Action CRUD, status transitions, signal emission. 11 public methods.
- `services/accounts.rs` (1,083 lines) — Account lifecycle, team, events, merge, archive, internal org creation, child accounts, computed list items. 19+ public methods.
- `services/people.rs` (383 lines) — Person CRUD, merge, delete, entity linking, archive. 10 public methods.
- `services/meetings.rs` (1,517 lines) — Intelligence assembly, prep loading, entity linking, attendee hydration, outcomes, history, search, transcript processing, prep refresh. 17+ public methods.
- `services/projects.rs` (461 lines) — Project CRUD, list assembly, notes, bulk creation, archive. 8 public methods.
- `services/emails.rs` (624 lines) — Email enrichment, entity emails, dismissals, archive, refresh. 8 public methods.
- `services/settings.rs` (303 lines) — Config mutations: entity mode, workspace, AI model, hygiene, schedule, profile, domains. 7 public methods.
- `services/intelligence.rs` (242 lines) — Entity enrichment, intelligence field edits, stakeholder management, risk briefings. 6 public methods.
- `services/entities.rs` (315 lines) — Entity signal prose formatting.
- `services/dashboard.rs` (902 lines) — Dashboard + week result assembly.
- `services/integrations.rs` (267 lines) — Claude Desktop, Quill, connector status.

**Signal emission pattern:** All service methods that modify user-visible data call `emit_signal_and_propagate()` (not bare `emit_signal()`), ensuring propagation rules fire and meeting preps are invalidated in real time. Service methods that need the propagation engine receive either `&AppState` (which has `signal_engine`) or `engine: &PropagationEngine` directly.

**Remaining in `commands.rs`:** Thin wrappers, type definitions, and a handful of handlers (processor triggers, dev tools, workflow triggers, capture UI) that are either already thin or low ROI for extraction.

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
- Email classification (that's the prepare pipeline)

**v0.13.2 status:** 16 methods extracted. Signal emissions use `emit_signal_and_propagate()`. Methods that modify data receive `&AppState` for signal engine access.

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

**v0.13.2 status:** 10 methods extracted. Signal emissions use `emit_signal_and_propagate()`. Known gap: `hydrate_attendee_context` filters out internal people, leaving "The Room" empty for internal meetings (I401, 0.13.6).

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

**v0.13.2 status:** 15+ methods extracted. Key changes:
- `generate_intelligence` no longer calls PTY — mechanical quality assessment + MeetingPrepQueue enqueue (ADR-0086)
- `load_meeting_prep_from_sources` loads `prep_frozen_json` before disk files (mechanical assembly takes priority)
- `link/unlink_meeting_entity_with_prep_queue` clears prep and enqueues for mechanical re-assembly
- `hydrate_attendee_context` provides live attendee data from DB (not from prep JSON)

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

**v0.13.2 status:** 11 methods extracted. Signal emissions use `emit_signal_and_propagate()` with `engine: &PropagationEngine` parameter.

---

### IntelligenceService

**Owns:** Entity enrichment, intelligence field edits, stakeholder management, risk briefings.

**File:** `services/intelligence.rs` (242 lines)

**v0.13.6 status:** Extracted. 6 public methods.

**Public methods:**
```
enrich_entity(id, type) → IntelligenceJson    // unified for account/person/project
update_intelligence_field(id, type, path, value) → ()  // user edits + signal
update_stakeholders(id, type, stakeholders) → ()       // bulk replace + signal
generate_risk_briefing(state, account_id) → RiskBriefing  // async PTY
get_risk_briefing(db, state, account_id) → RiskBriefing   // cached read
save_risk_briefing(db, state, account_id, briefing) → ()  // user corrections
```

**Does NOT touch:**
- Signal bus (that's SignalService)
- Meeting prep generation (that's MeetingService)
- Background intel queue loop (that stays in `intel_queue.rs`)

---

### SettingsService

**Owns:** Configuration mutations: entity mode, workspace path, AI model, hygiene config, schedules, user profile, user domains.

**File:** `services/settings.rs` (303 lines)

**v0.13.6 status:** Extracted. 7 public methods.

**Public methods:**
```
set_entity_mode(mode, state) → Config
set_workspace_path(path, state) → Config      // scaffold + entity sync
set_ai_model(tier, model, state) → Config
set_hygiene_config(scan, budget, pre_meeting, state) → Config
set_schedule(workflow, hour, minute, tz, state) → Config
set_user_profile(name, company, title, focus, domain, domains, state) → String
set_user_domains(domains, state) → Config     // reclassify people + meetings
```

**Does NOT touch:**
- Entity CRUD (that's Account/Person/ProjectService)
- Scheduler restart (that stays in scheduler.rs)

---

### ProjectService

**Owns:** Project CRUD, list assembly, workspace files, notes, bulk creation, archive.

**File:** `services/projects.rs` (461 lines)

**v0.13.6 status:** Expanded from 131 lines. 8 public methods.

**Public methods:**
```
get_projects_list(state) → Vec<ProjectListItem>
get_child_projects_list(parent_id, state) → Vec<ProjectListItem>
get_project_detail(id, state) → ProjectDetailResult
create_project(name, parent_id, state) → String
update_project_field(id, field, value, state) → ()
update_project_notes(id, notes, state) → ()
bulk_create_projects(db, workspace, names) → Vec<String>
archive_project(db, id, archived) → ()
```

---

### EmailService

**Owns:** Email enrichment, entity-linked email queries, dismissals, Gmail archive, refresh.

**File:** `services/emails.rs` (624 lines)

**v0.13.6 status:** Expanded from 310 lines. 8 public methods.

**Public methods:**
```
get_emails_enriched(state) → EmailBriefingData
get_entity_emails(db, entity_id, entity_type) → Vec<DbEmail>
update_email_entity(db, email_id, entity_id, entity_type) → ()
dismiss_email_signal(db, signal_id) → ()
dismiss_email_item(db, type, email_id, text, domain, email_type, entity_id) → ()
archive_low_priority_emails(state) → usize    // async Gmail
refresh_emails(state, app_handle) → String     // async spawn
best_account_for_person(db, person_id, context) → Option<String>
```

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

**Status:** Complete (v0.13.2). `db/mod.rs` reduced from ~3,972 to 441 lines.

`mod.rs` retains only:
- `ActionDb` struct definition + `open()` + `conn_ref()`
- `with_transaction()` helper
- `db_path()` with dev-mode isolation
- Startup data-repair methods (`normalize_reviewed_prep_keys`, `backfill_meeting_identity`, `backfill_meeting_user_layer`)
- `pub mod` declarations and re-exports
- Test utilities (`test_db()`)

Tests extracted to `db/mod_tests.rs` (3,526 lines) via `#[cfg(test)] #[path = "mod_tests.rs"]`.

**Domain modules** (all queries live here, not in mod.rs):
| Module | Owns |
|--------|------|
| `db/actions.rs` | Action CRUD queries, DbAction mapper |
| `db/accounts.rs` | Account CRUD queries, DbAccount mapper |
| `db/meetings.rs` | Meeting history, attendees, entities |
| `db/people.rs` | People CRUD, merge, duplicates |
| `db/projects.rs` | Project CRUD |
| `db/content.rs` | Content files, embeddings |
| `db/entities.rs` | Cross-entity queries |
| `db/signals.rs` | Signal events, propagation state |
| `db/emails.rs` | Email signals, threads, dismissals |
| `db/types.rs` | Row structs |

---

## Migration Path

### Phase 1: Complete existing extractions — DONE (v0.13.2)
1. ~~Finish `services/actions.rs`~~ — 11 methods, 466 lines
2. ~~Finish `services/accounts.rs`~~ — 16 methods, 799 lines
3. ~~Finish `services/people.rs`~~ — 10 methods, 382 lines
4. ~~Finish `services/meetings.rs`~~ — 15+ methods, 1,328 lines

`commands.rs`: 11,500 → 8,121 lines. All extracted handlers are thin wrappers (parse args → lock → call service → return).

### Phase 2: New services (planned)
5. Create `services/intelligence.rs` — extract from commands.rs + consolidate intelligence/ callers
6. Formalize `signals/` as SignalService (already well-modularized)

### Phase 3: State decomposition (planned, v0.14.0 candidate)
7. ~~Migrate queries from `db/mod.rs` into domain modules~~ — DONE (v0.13.2)
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
