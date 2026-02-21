# Architecture Map

**Purpose:** Structural reference for "what exists where." Facts, not analysis.
**Last verified:** 2026-02-20
**Source:** Codebase inspection + architectural assessment

---

## Backend Module Tree

```
src-tauri/src/
  lib.rs ............... Entry point, plugin setup, 15 background task spawns
  commands.rs .......... ~11,500 lines  IPC handlers (260+ commands registered)
  state.rs ............. ~940 lines     AppState: 28 fields, config, DB helpers
  executor.rs .......... ~1,500 lines   Workflow orchestration, scheduler receiver
  error.rs ............. Central error types

  db/ .................. Data access layer (split from monolithic db.rs)
    mod.rs ............. ActionDb struct, connection, shared helpers (~9,700 lines)
    types.rs ........... DB row structs (DbAction, DbAccount, DbPerson, etc.)
    actions.rs ......... Action CRUD queries
    accounts.rs ........ Account CRUD queries
    people.rs .......... People CRUD queries
    meetings.rs ........ Meeting history queries
    projects.rs ........ Project CRUD queries
    content.rs ......... Content index queries
    entities.rs ........ Cross-entity queries
    signals.rs ......... Signal event queries

  services/ ............ Emerging service layer (partial extraction)
    mod.rs
    entities.rs ........ Entity signal prose formatting
    actions.rs ......... Action business logic
    accounts.rs ........ Account business logic
    people.rs .......... People business logic
    meetings.rs ........ Meeting business logic

  prepare/ ............. Daily briefing preparation pipeline (~3,650 lines)
    orchestrate.rs ..... Pipeline coordinator (fetch → classify → resolve → write)
    meeting_context.rs . Meeting prep generation
    entity_resolver.rs . Entity resolution (account/project matching)
    email_classify.rs .. Email classification and routing
    actions.rs ......... Action extraction from prep data
    gaps.rs ............ Intelligence gap detection
    constants.rs ....... Shared constants

  processor/ ........... Inbox file processing & AI enrichment (~2,400 lines)
    mod.rs ............. Entry point, file routing
    router.rs .......... File type routing
    classifier.rs ...... Content classification
    extract.rs ......... Data extraction
    enrich.rs .......... AI enrichment
    embeddings.rs ...... Background embedding generation
    transcript.rs ...... Transcript processing
    email_actions.rs ... Email-to-action extraction
    metadata.rs ........ File metadata parsing
    hooks.rs ........... Post-processing hooks

  signals/ ............. Signal bus system (~2,000 lines, 17 files)
    bus.rs ............. Core signal emission (append-only event log)
    propagation.rs ..... Cross-entity signal derivation engine
    rules.rs ........... Propagation rule definitions
    fusion.rs .......... Bayesian log-odds signal fusion
    decay.rs ........... Time-based confidence decay
    relevance.rs ....... Signal relevance scoring
    callouts.rs ........ Signal-to-callout generation
    cadence.rs ......... Communication cadence tracking
    email_bridge.rs .... Email → signal bridge
    email_context.rs ... Email context extraction
    feedback.rs ........ User feedback signals
    invalidation.rs .... Prep invalidation on signal change
    patterns.rs ........ Signal pattern detection
    post_meeting.rs .... Post-meeting signal emission
    sampling.rs ........ Signal sampling
    event_trigger.rs ... Event-driven entity resolution trigger

  workflow/ ............ Three-phase pipeline execution (~7,500 lines)
    mod.rs ............. Workflow registry
    today.rs ........... Daily briefing workflow (Prepare → Deliver → Enrich)
    week.rs ............ Weekly planning workflow
    deliver.rs ......... JSON delivery phase
    operations.rs ...... Shared workflow operations
    reconcile.rs ....... Data reconciliation
    impact_rollup.rs ... Impact score rollups
    archive.rs ......... Archive management

  intelligence/ ........ Intelligence computation (refactored from monolithic files)
    mod.rs
    compute.rs ......... Intelligence scoring algorithms
    lifecycle.rs ....... Intelligence lifecycle management
    prompts.rs ......... AI prompt building
    io.rs .............. Intelligence I/O operations

  Integrations (~4,900 lines total):
    google.rs .......... Calendar poller, OAuth helpers
    google_api/ ........ Google Calendar + Gmail API clients
    clay/ .............. Clay enrichment integration
    granola/ ........... Granola transcript poller
    quill/ ............. Quill transcript poller
    linear/ ............ Linear issue sync
    gravatar/ .......... Gravatar avatar fetcher

  Infrastructure:
    migrations/ ........ 33 sequential SQL migrations (ADR-0071)
    presets/ ........... Role preset definitions + loader
    proactive/ ......... Proactive suggestion engine
    mcp/ ............... MCP sidecar (Claude Desktop integration)
    queries/ ........... Specialized DB query modules (~650 lines)

  Other modules:
    accounts.rs ........ Account workspace sync
    people.rs .......... People workspace sync
    projects.rs ........ Project workspace sync
    entity.rs .......... Entity type definitions
    entity_io.rs ....... Entity I/O operations
    types.rs ........... Shared Rust types (Config, CalendarEvent, etc.)
    helpers.rs ......... normalize_key(), normalize_domains(), build_entity_hints()
    util.rs ............ Utility functions (slugify, atomic_write, etc.)
    json_loader.rs ..... JSON file reading (schedule.json, actions.json, etc.)
    hygiene.rs ......... Data hygiene scanner loop
    intel_queue.rs ..... Background intelligence enrichment queue
    capture.rs ......... Post-meeting capture detection loop
    scheduler.rs ....... Background task scheduling (cron-like)
    pty.rs ............. Claude Code subprocess management
    notification.rs .... macOS notification helpers
    parser.rs .......... Inbox file parsing
    calendar_merge.rs .. Calendar event merging
    backfill_meetings.rs Historical meeting backfill
    risk_briefing.rs ... Risk briefing generation
    audit.rs ........... Audit logging
    db_backup.rs ....... Database backup/restore
    devtools.rs ........ Developer tools (scenarios, mock data)
    embeddings.rs ...... Embedding model runtime
    focus_capacity.rs .. Focus capacity scoring
    focus_prioritization.rs Focus prioritization logic
    latency.rs ......... Latency tracking
    watcher.rs ......... Inbox file system watcher
```

---

## Frontend Structure

```
src/
  pages/ ................. 16 route-level page components
    MeetingDetailPage.tsx .. ~2,100 lines (largest — prep, transcript, agenda, outcomes)
    InboxPage.tsx .......... ~1,200 lines
    WeekPage.tsx ........... ~1,050 lines
    AccountsPage.tsx
    AccountDetailEditorial.tsx
    ActionsPage.tsx
    ActionDetailPage.tsx
    PeoplePage.tsx
    PersonDetailEditorial.tsx
    ProjectsPage.tsx
    ProjectDetailEditorial.tsx
    EmailsPage.tsx
    HistoryPage.tsx
    MeetingHistoryDetailPage.tsx
    RiskBriefingPage.tsx
    SettingsPage.tsx

  hooks/ .................. 25 custom hooks (one per data domain)
    useDashboardData.ts .... Main dashboard data fetching
    useCalendar.ts ......... Calendar events
    useActions.ts .......... Action CRUD
    useProposedActions.ts .. Proposed action management
    useInbox.ts ............ Inbox file operations
    useAccountDetail.ts .... Account detail fetching
    useAccountFields.ts .... Account field updates
    usePersonDetail.ts ..... Person detail fetching
    useProjectDetail.ts .... Project detail fetching
    useWorkflow.ts ......... Workflow execution
    useGoogleAuth.ts ....... Google OAuth flow
    useMeetingOutcomes.ts .. Meeting outcome tracking
    usePostMeetingCapture.ts Post-meeting capture
    useExecutiveIntelligence.ts Executive intelligence
    useTeamManagement.ts ... Team member management
    useIntelligenceFieldUpdate.ts Intelligence field editing
    useClaudeStatus.ts ..... Claude Code status
    useActivePreset.ts ..... Role preset
    useNotifications.ts .... System notifications
    useTauriEvent.ts ....... Tauri event listener utility
    useMagazineShell.ts .... Magazine layout shell
    useChapterObserver.ts .. Chapter scroll observer
    useRevealObserver.ts ... Reveal animation observer
    useCopyToClipboard.ts .. Clipboard utility
    use-mobile.ts .......... Mobile detection

  components/ ............. ~90 components across 5 subdirectories
    editorial/ ............ Magazine-style components (sections, heroes, callouts)
    ui/ ................... Generic UI primitives (buttons, badges, modals)
    entity/ ............... Entity-specific components (account cards, people lists)
    dashboard/ ............ Dashboard-specific widgets
    layout/ ............... Layout components (shell, sidebar, nav)

  types/index.ts .......... ~1,600 lines (comprehensive, aligned with Rust structs)
  styles/design-tokens.css . ~140 lines (single source of truth for all design values)
```

---

## Data Flow

```
                     ┌─────────────┐     ┌─────────────┐
                     │Google Calendar│     │  Google Gmail │
                     └──────┬──────┘     └──────┬──────┘
                            │                    │
                   poll (5min)              poll (on workflow)
                            │                    │
                            ▼                    ▼
                     ┌──────────────────────────────┐
                     │         PREPARE               │
                     │  orchestrate.rs coordinates:   │
                     │  • email_classify (Gmail→categories) │
                     │  • entity_resolver (match→accounts) │
                     │  • meeting_context (prep for each mtg) │
                     │  • actions (extract action items) │
                     └──────────────┬───────────────┘
                                    │
                                    ▼
                     ┌──────────────────────────────┐
                     │         DELIVER               │
                     │  workflow/deliver.rs writes:   │
                     │  • schedule.json (today's data)│
                     │  • actions.json, emails.json   │
                     │  → _today/data/ in workspace   │
                     └──────────────┬───────────────┘
                                    │
                                    ▼
              ┌─────────────────────────────────────────┐
              │              SQLite DB                    │
              │  29 tables, WAL mode, Mutex-guarded      │
              │  accounts, people, actions, meetings,     │
              │  signal_events, entity_intel, etc.        │
              └─────────────┬───────────────────────────┘
                            │
                   commands.rs (IPC dispatch)
                            │
                            ▼
              ┌─────────────────────────────────────────┐
              │           React Frontend                 │
              │  hooks/ → invoke() → commands.rs         │
              │  Tauri events → real-time updates        │
              │  design-tokens.css → editorial styling   │
              └─────────────────────────────────────────┘

  Parallel enrichment flows:
  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐
  │ intel_queue │  │ signals/   │  │ processor/ │  │ hygiene    │
  │ (entity AI) │  │ (bus+prop) │  │ (inbox AI) │  │ (data QA)  │
  └──────┬─────┘  └──────┬─────┘  └──────┬─────┘  └──────┬─────┘
         └───────────────┴───────────────┴───────────────┘
                              │
                        writes to DB
                              │
                     Tauri events notify frontend
```

---

## Async Task Map

15 background tasks spawned in `lib.rs::run()` during app setup:

| # | Task | Function | Purpose |
|---|------|----------|---------|
| 1 | Embedding model init | `embedding_model.initialize()` | Downloads/loads nomic-embed-text-v1.5 (~137MB) |
| 2 | Startup sync | `state::run_startup_sync()` | Sync workspace files → DB (people, accounts, projects, content) |
| 3 | Scheduler | `scheduler::Scheduler::run()` | Cron-like background scheduling (triggers workflows) |
| 4 | Executor | `executor::Executor::run()` | Receives scheduler messages, runs workflow pipelines |
| 5 | File watcher | `watcher::start_watcher()` | FSEvents on _inbox/ directory → process new files |
| 6 | Calendar poller | `google::run_calendar_poller()` | Polls Google Calendar every 5 min |
| 7 | Capture detector | `capture::run_capture_loop()` | Detects meeting endings → prompts for outcomes |
| 8 | Intelligence processor | `intel_queue::run_intel_processor()` | Background AI enrichment for entities |
| 9 | Embedding processor | `processor::embeddings::run_embedding_processor()` | Background vector embedding generation |
| 10 | Hygiene scanner | `hygiene::run_hygiene_loop()` | Periodic data quality scanning (ADR-0058) |
| 11 | Quill poller | `quill::poller::run_quill_poller()` | Polls Quill for meeting transcripts |
| 12 | Granola poller | `granola::poller::run_granola_poller()` | Polls Granola for meeting transcripts |
| 13 | Gravatar fetcher | `gravatar::client::run_gravatar_fetcher()` | Fetches avatar images for contacts |
| 14 | Clay poller | `clay::poller::run_clay_poller()` | Polls Clay for contact/company enrichment |
| 15 | Linear poller | `linear::poller::run_linear_poller()` | Syncs Linear issues for project context |
| — | Entity resolution trigger | `signals::event_trigger::run_entity_resolution_trigger()` | Event-driven entity resolution on signal changes |

---

## IPC Contract Summary

**`commands.rs`** registers ~260 Tauri IPC commands via `tauri::generate_handler![]`. These are the sole API surface between frontend and backend.

**Call pattern:**
```
Frontend hook (useX.ts) → invoke("command_name", args) → commands.rs handler → state.with_db_read/write() → db/ methods
```

**Event-driven updates:** The backend emits Tauri events that hooks listen to:
- `calendar-updated` — calendar poller refreshed events
- `prep-ready` — meeting prep generation completed
- `entity-updated` — entity intelligence changed
- `workflow-completed` — workflow pipeline finished
- `inbox-updated` — new file processed from inbox

**Command categories** (from `lib.rs` registration):
- Core (dashboard, workflow, config) — ~15 commands
- Google Auth — 3 commands
- Calendar — 3 commands
- Post-Meeting Capture — 4 commands
- Weekly Planning — 3 commands
- Transcript & Outcomes — 4 commands
- Manual Action CRUD — 2 commands
- Entity CRUD (accounts, projects, people) — ~60 commands
- Meeting-Entity linking — 6 commands
- Intelligence & Enrichment — ~10 commands
- Integrations (Quill, Granola, Gravatar, Clay, Linear) — ~30 commands
- Hygiene — 4 commands
- Dev Tools — 7 commands
- Settings & Config — ~20 commands

---

## Database Overview

- **Engine:** SQLite, WAL mode, foreign keys enforced
- **Location:** `~/.dailyos/dailyos.db` (dev mode: `dailyos-dev.db`)
- **Tables:** 29 tables, 30+ indexes, 10 FK constraints
- **Migrations:** 33 sequential SQL files in `src-tauri/src/migrations/` (framework: ADR-0071)
- **Access:** Single `ActionDb` struct wrapping a `rusqlite::Connection`, Mutex-guarded in `AppState.db`
- **Helpers:** `with_db_read()`, `with_db_write()`, `with_db_try_read()` on AppState

Key tables: `actions`, `accounts`, `projects`, `people`, `person_emails`, `meetings`, `meeting_entities`, `meeting_attendees`, `entity_intel`, `signal_events`, `content_files`, `content_embeddings`, `email_signals`, `email_threads`, `enrichment_log`, `quill_sync_state`, `granola_sync_state`, `gravatar_cache`, `linear_issues`

---

## Signal System Overview

The signal bus (`signals/`) implements an append-only event log for cross-entity intelligence:

1. **Emission** (`bus.rs`): Events are appended to `signal_events` table with entity_id, signal_type, confidence score, source, and payload
2. **Propagation** (`propagation.rs` + `rules.rs`): Cross-entity derivation rules fire when signals arrive — e.g., a person signal propagates to their accounts
3. **Fusion** (`fusion.rs`): Bayesian log-odds fusion combines multiple signal sources for entity resolution confidence
4. **Decay** (`decay.rs`): Time-based confidence decay prevents stale signals from dominating
5. **Callouts** (`callouts.rs`): High-confidence signals generate user-visible callout cards
6. **Invalidation** (`invalidation.rs`): Signal changes trigger prep regeneration for affected meetings

---

## Cross-References

- **Design system:** [DESIGN-SYSTEM.md](./DESIGN-SYSTEM.md) — tokens, typography, colors, spacing
- **Components:** [COMPONENT-INVENTORY.md](./COMPONENT-INVENTORY.md) — 90+ shared components
- **Page structure:** [PAGE-ARCHITECTURE.md](./PAGE-ARCHITECTURE.md) — layout patterns per page
- **Full diagnostic:** [../plans/architectural-assessment.md](../plans/architectural-assessment.md) — 355-line assessment with recommendations
- **Service contracts:** [SERVICE-CONTRACTS.md](./SERVICE-CONTRACTS.md) — target service layer extraction plan
