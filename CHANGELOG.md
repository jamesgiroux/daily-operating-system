# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).


## [1.0.2] — 2026-03-21

### Fixed

- **Health scoring recalibrated (I633)** — five formula bugs fixed that caused healthy accounts to score at-risk. Financial proximity formula was inverted (just-renewed accounts scored 5/100), confidence regression was too aggressive (40% pull to neutral at 3-4 dimensions), stakeholder coverage required all 3 archetype roles (denominator always 3), champion inference capped at 55 even with 70%+ attendance, and real-time rescores dropped the org baseline blend. All formulas corrected; bulk recompute command added for migration.
- **Email signals now populate from enrichment** — `email_signals` table was never written to by the enrichment pipeline, causing email engagement to read as zero in health scoring. Enriched emails now write to both `signal_events` and `email_signals`.
- **Stakeholder roles persist across re-enrichment** — user-set roles (champion, technical, executive) were lost when AI reordered the stakeholder array during enrichment. `preserve_user_edits` now matches by person name instead of array index.
- **Actions pipeline end-to-end (I620)** — proposed actions from transcript extraction were invisible. Six root causes fixed: event-driven refresh for proposed actions, auto-switch to Proposed tab on new arrivals, dedup guard now logs at debug level, service layer returns write/skip counts, lifecycle feedback on every extraction.
- **Past-meeting opacity boundary (I622)** — outcomes section now renders at full brightness on past meetings; only pre-meeting prep fades.
- **MCP query_entity reads from DB (I632)** — `query_entity` now reads intelligence from `entity_assessment` table instead of stale disk files.

### Added

- **Health score trend tracking (I633)** — `health_score_history` table records each recompute. Real trends computed from last 3-5 data points, replacing hardcoded "stable". Migration 072.
- **Bulk health recompute command** — `bulk_recompute_health` Tauri command rescores all accounts after formula fixes.
- **Briefing expansion panels restored (I629)** — PrepGrid and MeetingActionChecklist now render in the expansion panel for non-lead meetings on the daily briefing.
- **Transcript filing for person + project entities (I631)** — transcripts route to `~/People/{Name}/Call-Transcripts/` for 1:1 meetings and `~/Projects/{Name}/Call-Transcripts/` for project meetings.
- **WatchListPrograms wired (I630)** — strategic programs component connected to account detail WatchList.

### Changed

- **Transcript output redesign (I621)** — win sub-type badges on individual items, evidence quote borders unified to warm accent, speaker attribution repositioned, champion health section reordered to after commitments.
- **Reconnection audit (I630)** — stale I377 comment on `rule_meeting_frequency_drop` updated to reflect I555 re-activation. All 12 propagation rules verified active.


## [1.0.1] — 2026-03-21

### Added

- **Email triage actions** — archive (syncs to Gmail with undo), open in Gmail (deep link), and pin (sort boost within score band) on every email item. All actions emit signals for the Intelligence Loop.
- **Commitment tracking** — extracted commitments show inline with a "Track" form (title, due date, owner, entity). Promoted commitments persist as tracked Actions visible on reload.
- **Gone Quiet detection** — accounts whose email cadence drops below 2× their historical norm surface in a "GONE QUIET" section with dismiss capability. Emits `email_cadence_drop` signal for briefing callouts with 7-day dedup.
- **Email-meeting linkage** — emails from upcoming meeting attendees show a meeting badge with click-to-navigate. Meeting detail page shows linked correspondence digest.
- **DB growth monitoring** — startup size logging (warn at 300MB, error at 500MB), Settings → Diagnostics storage card, persistent toast at 500MB+, daily age-based purge (180d signals, 30d deactivated email signals, 60d resolved emails). User corrections never purged.
- **RiskBriefingPage in reports framework** — route moved to `/accounts/$accountId/reports/risk_briefing`, loads via `get_report`, all 6 slides and feedback preserved.
- **Migration 071** — `pinned_at`, `commitments`, `questions` columns on emails table.

### Changed

- **AppState cleanup (I609/I610)** — removed sync `Mutex<Option<ActionDb>>`, migrated 100+ callers to `ActionDb::open()`. Consolidated 4 lock fields into single `AppLockState` struct.
- **hygiene.rs decomposed** — 3,463-line monolith split into `hygiene/` directory with 6 sub-modules (detectors, fixers, matcher, narrative, loop_runner, mod). Phase-level `catch_unwind` error isolation.
- **Console.error → toast sweep** — all user-initiated action catch blocks now show toast.error. Background errors annotated. 55+ files updated.
- **InboxPage + AccountsPage inline styles** — migrated to CSS modules with design token variables.
- **Email ranking** — pinned emails sort to top of their score band (not globally). `compare_email_rank` in both Rust and TypeScript.
- **Intelligence feedback cleared on re-enrichment** — old votes no longer stick to new content at the same field position.

### Fixed

- **Archived emails resurrecting** — two reconciliation paths (`services/emails.rs` + `prepare/orchestrate.rs`) were un-resolving user-archived emails. Both fixed: vanished emails still resolved, known emails never un-resolved.
- **Archive not syncing to Gmail** — `archive_email` now removes INBOX label via Gmail API. `unarchive_email` restores it. Gmail failure is non-fatal (warn-only).
- **Cross-page archive propagation** — all email mutations (archive, pin, reply, entity change) now emit `emails-updated` event. Dashboard, Correspondent, and entity pages all refresh.
- **Entity email queries showed archived emails** — added `resolved_at IS NULL` to `get_emails_for_entity` + both fallback paths.
- **Dashboard thread count inflated** — dashboard now calls `collapse_to_latest_thread_emails()` matching Correspondent behavior.
- **`archive_low_priority_emails` DB-blind** — was mutating `emails.json` only. Now also sets `resolved_at` in DB.
- **Read emails in PRIORITY** — added `is_unread` field to Email struct, PRIORITY section filters to unread only.
- **Awaiting reply dropped read threads** — removed `is_unread` requirement from `get_emails_awaiting_reply`.
- **Pre-existing clippy fixes** — removed unnecessary `usize` casts in `db/meetings.rs`, `too_many_arguments` allows on service functions.

## [1.0.0] — 2026-03-16

### Added

- **Health scoring engine** — every account gets a health score powered by 6 algorithmic dimensions (champion health, stakeholder coverage, email engagement, cadence consistency, signal momentum, financial proximity). Scores update as new data arrives. Sparse accounts (one meeting, no email) get a confidence qualifier instead of a misleading number.
- **Intelligence schema redesign** — 6 research-grounded dimensions replace the flat intelligence blob. Sub-struct types (I508a), source-agnostic enrichment prompts (I508b), and multi-query dimension coverage (I508c).
- **Transcript signal fidelity** — wins extracted with 6 sub-types (ADOPTION, EXPANSION, RETENTION, ADVOCACY, MILESTONE, VALUE_REALIZATION). Risks carry urgency tiers (RED, YELLOW, GREEN_WATCH). Champion health assessed per meeting. Verbatim evidence quotes captured. Generic sentiment ("customer seems happy") filtered out.
- **Success Plans** — objectives, milestones, and templates for account lifecycle management. Auto-complete milestones on lifecycle events. AI suggestions from transcript commitments and entity assessment. 4 built-in templates (onboarding, growth, renewal, at-risk).
- **Account detail editorial redesign** — margin label layout, hero with executive assessment, pull quotes, State of Play two-column treatment, scroll-driven reveal. Value & Commitments, Competitive & Strategic Landscape, and Outlook chapters surface previously hidden intelligence.
- **Meeting post-intelligence** — meetings with transcripts show engagement dynamics (talk balance, speaker sentiment), champion health, urgency-sorted outcomes, and role changes.
- **Full-text search** — Cmd+K via SQLite FTS5. Accounts, people, projects, meetings, actions, emails. Results in < 300ms.
- **Offline/degraded mode** — cached intelligence when APIs unavailable. System status indicator. No blank screens.
- **Data export** — JSON ZIP of all entities, signals, intelligence from Settings → Data.
- **Privacy clarity** — Settings explains what's stored, how long. Clear intelligence and delete all data options.
- **Intelligence feedback UI** — hover-triggered thumbs up/down on any intelligence item. Feeds Bayesian source weights. Signal taxonomy: delete = curation (no penalty), edit = correction (source penalized).
- **Glean-first intelligence (ADR-0100)** — Glean MCP `chat` tool as primary enrichment engine when connected. Tiered signal confidence (CRM 0.9, Zendesk 0.85, Gong 0.8, AI 0.7, Slack 0.5). PTY fallback for non-Glean users.
- **Glean onboarding** — 3-connector wizard (Google, Claude, Glean). Account discovery, profile pre-fill from org directory, background enrichment.
- **Automatic connector management** — Additive/Governed strategy removed. Token health monitoring with pre-expiry notifications. In-app re-auth without restart.
- **Welcome screen** — branded asterisk + "DailyOS" appears instantly at window-show before JS executes. Eliminates blank window on cold and warm start.
- **Background task supervisor** — all 13 long-lived background tasks wrapped in restart-on-panic with exponential backoff. No more silent subsystem death.
- **Prompt evaluation suite** — 29 golden fixture tests validating prompt construction, response parsing, transcript extraction quality (sub-types, urgency tiers, champion health), and dimension merge correctness.
- **Service layer smoke tests** — 25 tests across 5 mutation services (mutations, accounts, success_plans, intelligence, reports) verifying DB state + signal emission.

### Changed

- **ServiceLayer is mandatory** — every user-facing mutation goes through `services/`. No direct DB writes from command handlers. 20 domain service modules.
- **DB as sole data source** — app reads zero generated state from filesystem. `intelligence.json`, `dashboard.json`, `schedule.json`, `actions.json`, `preps/*.json` no longer read by app. Workspace dirs and user files untouched.
- **Module decomposition** — `commands.rs` from 4,000+ lines to 80-line dispatcher with 10 domain modules. `db.rs` replaced by 21 domain query modules.
- **Schema migration framework** — fail-hard runner, guaranteed pre-migration backups, schema integrity checks for every version gate. Migration 068 rebuilt with correct DROP+CREATE pattern.
- **Typed resource permits** — single `heavy_work_semaphore` replaced with 5 independent permits (PTY, user-initiated, embeddings, email, orchestration). Background enrichment no longer blocks UI.
- **PTY on blocking threads** — enrichment wrapped in `spawn_blocking`. Async executor never blocked > 1s during enrichment. Beach ball eliminated.
- **Glean token refresh isolated** — network calls on dedicated OS threads, not Tokio workers.
- **Command handlers async-clean** — zero `state.db.lock()` in commands/. All DB access through async `db_service`.
- **Actions pipeline** — Granola actions get correct priority and context. Briefing includes DB actions. 30-day pending auto-archive. Rejection source tracked correctly.
- **Settings UX rebuild** — YouCard split into Identity/Workspace/Preferences. Audit log pagination. Vocabulary fixes. CSS modules throughout.

### Fixed

- Meeting briefing refresh rollback — snapshot existing prep before clearing; restore if enrichment fails. No more blank "Building context" pages.
- Pipeline reliability — retry with backoff, PTY circuit breaker, partial result preservation.
- Actions pipeline — 6 broken paths fixed (Granola metadata loss, briefing blind to DB actions, archive never called, rejection source "unknown", thin summaries, deceptive tooltip).
- Component DRY/SRP — StatusDot defined once (was 3x), EditorialLoading/Empty/Error shared across all pages.
- Inline styles migrated to CSS modules across MeetingDetailPage, Settings, entity detail pages.
- ADR-0083 vocabulary compliance across all user-facing strings.
- Top 20 user-facing error paths now show toast notifications instead of silent console.error.

### Security

- Data governance with source-aware lifecycle (ADR-0098). Purge by source on credential revocation.
- Database recovery UX for migration/integrity failures — startup blocker + Settings recovery controls.

---

## Resolved Issues (moved from backlog, 2026-03-15)

Issues previously tracked in BACKLOG.md that have been completed, archived, superseded, absorbed, or withdrawn.

### Done

| ID | Resolution |
|----|------------|
| I225 | Gong integration — done (Gong transcripts via Glean) |
| I230 | Claude Cowork integration — done (obsoleted by product changes) |
| I359 | Vocabulary-driven prompts — done (all 7 fields injected) |
| I427 | Full-text search — Cmd+K finds entities, meetings, actions, contacts using SQLite FTS5; results in < 300ms — done |
| I428 | Offline/degraded mode — serve cached intelligence gracefully when APIs unavailable; system status indicator — done |
| I429 | Data export — JSON ZIP export of entities, signals, intelligence; portability guarantee — done |
| I430 | Privacy clarity — Settings section explaining what's stored, how long, clear intelligence + delete all data options — done |
| I438 | Onboarding: Prime DailyOS — first content ingestion step; manual (drop transcript/doc) or connector (Quill/Granola/Drive); teaches feeding habit before automation takes over — done |
| I447 | Design token audit — formalise opacity tokens, fix phantom token (`eucalyptus`), replace all rgba() violations, unify max-width — done |
| I448 | ActionsPage editorial rebuild — CSS module, margin grid, ChapterHeadings for groups, correct max-width, unconditional FinisMarker — done |
| I449 | WeekPage + EmailsPage CSS module polish — TimelineDayGroup module, stat line tokens, EditorialLoading/EditorialError, FinisMarker — done |
| I450 | Portfolio chapter extraction — shared CSS module for Account + Project Detail portfolio; conclusion-before-evidence editorial order — done |
| I451 | MeetingDetailPage polish — Recent Correspondence editorial treatment; avatar tint tokens; FinisMarker unconditional — done |
| I452 | Settings page editorial audit — inline style cleanup, vocabulary compliance, section rules, FinisMarker — done |
| I453 | Onboarding pages editorial standards — v0.16.0 wizard/demo/tour built to editorial spec; no inline styles — done |
| I454 | SettingsService extraction — create services/settings.rs, move 7 settings handlers — done |
| I479 | ContextProvider trait + LocalContextProvider — pure refactor — done in v0.15.2 |
| I480 | GleanContextProvider + cache + migration — done in v0.15.2 |
| I481 | Connector gating + mode switching + Settings UI — done in v0.15.2 |
| I487 | Glean signal emission — new-only document signals, ADR-0098 purge compliance (person signals → I505) — done |
| I493 | Account detail enriched intelligence surface — Glean-sourced titles, coverage gaps, reports chapter (health rendering owned by I502) — done |
| I499 | Health scoring engine — 6 algorithmic relationship dimensions, lifecycle weighting, sparse data handling — done |
| I500 | Glean org-score parsing — extract structured health data from Glean results as baseline — done |
| I502 | Health surfaces — render health band, dimensions, divergence across all app pages — done |
| I503 | intelligence.json health schema evolution — AccountHealth struct, RelationshipDimensions, migration — done |
| I504 | AI-inferred relationship extraction — fix prompt schema, call extraction function, persist to person_relationships — done |
| I505 | Glean stakeholder intelligence — contact discovery, profile enrichment, entity linkage, manager relationships, team sync (absorbs I486) — done |
| I506 | Co-attendance relationship inference — algorithmic collaborator/peer edges from meeting frequency — done |
| I507 | Source-attributed correction feedback — close feedback loop for Glean, Clay, email sources — done |
| I508 | Intelligence schema redesign for multi-source enrichment — 6 research-grounded dimensions, gap detection, source-agnostic prompt — done |
| I509 | Transcript personal interpretation + sentiment — personal priority impact, relationship trajectory, sentiment as local signal; org-level dynamics deferred to Glean; absorbs I501 (← I508) — done |
| I511 | Local schema decomposition + migration safety hardening (backend-only) — fail-hard runner, guaranteed backups, atomic decomposition migration, schema integrity checks — done |
| I512 | ServiceLayer — mandatory mutation path + signal emission — spec: `.docs/issues/i512.md` (absorbs I380, I402) — done |
| I513 | DB as sole source of truth for app-generated state — spec: `.docs/issues/i513.md` (absorbs I436). Workspace dirs + user files stay; app stops reading intelligence.json, dashboard.json, _today/data/*.json as data sources — done |
| I514 | Module decomposition — commands.rs → domain files, db.rs → re-export hub. Spec: `.docs/issues/i514.md` — done |
| I515 | Pipeline reliability — retry with backoff, circuit breaker, partial result preservation, pipeline_failures table. Spec: `.docs/issues/i515.md` — done |
| I521 | Frontend structural cleanup + production-data parity gate — remove ghost components, consolidate duplicate patterns, lock command/field contracts, enforce mock+production fixture parity. Spec: `.docs/issues/i521.md` — done |
| I527 | Intelligence consistency guardrails — deterministic contradiction checks, balanced repair retry, and corrected/flagged trust surfacing for intelligence output. Spec: `.docs/issues/i527.md` — done |
| I528 | ADR-0098 data lifecycle infrastructure — DataSource enum, purge_source(), data_lifecycle.rs. Prerequisite for I487 + I505 purge ACs. Spec: `.docs/issues/i528.md` — done |
| I529 | Intelligence quality feedback UI — thumbs up/down on hover for any intelligence item. Feeds Bayesian source weights. Spec: `.docs/issues/i529.md` — done |
| I530 | Signal taxonomy: curation vs correction — delete = no source penalty, edit = correction, thumbs down = correction. Spec: `.docs/issues/i530.md` — done |
| I536 | Dev tools mock data migration — rewrite seed data for v1.0.0 schema (I511 tables), 6-dimension intelligence (I508), health scores (I499), signal/feedback variety (I529/I530). Consolidate scenarios 6→4. Eliminate workspace file writes. `mock-` prefix IDs. Spec: `.docs/issues/i536.md` — done |
| I537 | Gate role presets behind feature flag — hide preset selection UI (onboarding + settings), hard-default to CS. Preset infrastructure stays. Spec: `.docs/issues/i537.md` — done |
| I538 | Meeting briefing refresh — rollback on failure. Snapshot existing prep before clearing, restore if enrichment fails. Spec: `.docs/issues/i538.md` — done |
| I539 | Database recovery UX for migration/DB failure — startup blocker + Settings/Data recovery controls; scope extracted from I511. Spec: `.docs/issues/i539.md` — done |
| I540 | Actions pipeline integrity + lifecycle — 6 broken paths: Granola metadata loss, briefing blind to DB actions, archive never called, rejection source "unknown", thin free-tier summaries, deceptive tooltip. 30-day pending archive, Granola enrichment, briefing integration. Spec: `.docs/issues/i540.md` — done |
| I541 | Settings page UX rebuild — IA reorg (YouCard split into Identity/Workspace/Preferences), full inline style migration to CSS modules, audit log pagination, vocabulary fixes, StatusDot consolidation. Supersedes I452. Spec: `.docs/issues/i541.md` — done |
| I542 | MeetingDetailPage style migration + vocabulary — migrate 51 inline styles to CSS module, replace hardcoded colors with tokens, fix 3 ADR-0083 violations. Supersedes I451. Spec: `.docs/issues/i542.md` — done |
| I544 | Component DRY/SRP reconciliation — app-wide duplicate detection (StatusDot 3x, per-page empty/loading states), shared component extraction, dead code removal, SRP violations. Spec: `.docs/issues/i544.md` — done |
| I545 | Entity detail pages style migration — 105 inline styles across AccountDetailEditorial (51), ProjectDetailEditorial (39), PersonDetailEditorial (15) + 7 hardcoded rgba values. Shared CSS module extraction. Spec: `.docs/issues/i545.md` — done |
| I547 | Book of Business Review report — stashed (2026-03-14) |
| I549 | Composable report slide templates + report mockups — done (2026-03-14) |
| I550 | Account detail editorial redesign: margin label layout + visual storytelling — pass 1 done, pass 2 suspended (2026-03-14) |
| I551 | Success Plan data model + backend — done (2026-03-14) |
| I552 | Success Plan frontend — done (2026-03-14) |
| I553 | Success Plan templates + starter lifecycle collection — done (2026-03-14) |
| I554 | Transcript extraction signal fidelity — CS-grounded prompt definitions: 6 win sub-types, Red/Yellow/Green risk urgency, value delivered quantification, 3-level champion health (MEDDPICC), COMMITMENTS extraction, successPlanSignals schema. Absorbs I551 PTY changes. Spec: `.docs/issues/i554.md` — done |
| I555 | Captures metadata + interaction dynamics persistence + architecture integration — urgency/sub_type/impact/evidence_quote columns on captures, interaction dynamics + champion health + role changes tables, captured_commitments table (dual-write to captures). Signal bus emissions (champion → person-level → propagation → callouts), reactivates `rule_meeting_frequency_drop`, upgrades 3 health scoring dimensions to behavioral, adds dynamics/commitments to intel + prep context. Absorbs I551 schema. Spec: `.docs/issues/i555.md` — done |
| I556 | Report content pipeline — meeting summaries + captures for Weekly Impact/Monthly Wrapped (currently title-only), customer quote pipeline for EBR/QBR, urgency-enriched captures for Account Health + BoB. Spec: `.docs/issues/i556.md` — done |
| I557 | Surface hidden intelligence on Account Detail — renders ~15 computed-but-invisible fields (valueDelivered, successMetrics, openCommitments, relationshipDepth, competitiveContext, strategicPriorities, expansionSignals, renewalOutlook, organizationalChanges, blockers). 3 new chapters. Spec: `.docs/issues/i557.md` — done |
| I558 | Meeting Detail intelligence expansion — post-meeting intelligence section (engagement dynamics, champion health, categorized outcomes, role changes, sentiment), surfaces unused FullMeetingPrep fields. Spec: `.docs/issues/i558.md` — done |
| I559 | Glean Agent validation spike — resolve 6 open questions (auth, rate limits, connectors, JSON output, latency, MCP tool discovery). Exploration only, no production code. GATE for I535. Spec: `.docs/issues/i559.md` — done |

### Archived

| ID | Resolution |
|----|------------|
| I90 | Product telemetry + analytics infrastructure — archived (partially absorbed by audit log in v0.15.2) |
| I198 | Account merge + transcript reassignment — archived (parked, rare use case) |
| I227 | Gainsight integration — archived (won't do, no clear path) |
| I277 | Marketplace repo for community preset discoverability — archived (won't do) |
| I280 | Beta hardening umbrella — archived (scope absorbed by individual issues in v0.15.1/v0.16.1) |
| I360 | Community preset import UI — archived (won't do) |
| I387 | Multi-entity signal extraction from parent-level meetings — archived (deferred per ADR-0087, bidirectional propagation covers the need) |
| I475 | Inbox entity-gating follow-ups — archived (re-raise if bugs surface) |

### Superseded

| ID | Resolution |
|----|------------|
| I88 | Monthly Book Intelligence — superseded by I491/I492 portfolio reports |
| I115 | Multi-line action extraction — superseded by transcript pipeline improvements |
| I141 | AI content tagging during enrichment — superseded by intelligence schema (I508) |
| I142 | Account Plan artifact — superseded by reports suite |
| I258 | Report Mode — superseded by I397 (report infrastructure) |
| I340 | Glean integration — superseded by I479-I481 in v0.15.2 |
| I451 | MeetingDetailPage polish — superseded by I542 (full style migration + vocabulary) |
| I452 | Settings page editorial audit — superseded by I541 (full UX rebuild: IA reorg, style migration, pagination, vocabulary) |
| I484 | Health score always-on — superseded by I499–I503 per ADR-0097 |
| I485 | Store inferred relationships from enrichment — superseded by I504–I506 |

### Absorbed / Pulled Forward

| ID | Resolution |
|----|------------|
| I302 | Shareable PDF export — absorbed into I397 (report infrastructure) |
| I347 | SWOT report type — absorbed into I397 (report infrastructure, bundled format) |
| I357 | Semantic email reclassification — absorbed by I367 (mandatory enrichment) |
| I380 | commands.rs service extraction Phase 1 — absorbed by I512 (ServiceLayer) + I514 (module decomp) in v1.0.0 |
| I381 | db/mod.rs domain migration — absorbed by I511 (schema decomposition) in v1.0.0 |
| I402 | IntelligenceService extraction — absorbed by I512 (ServiceLayer) in v1.0.0 |
| I436 | Workspace file deprecation — absorbed by I513 (workspace file elimination) in v1.0.0 |
| I458 | Renewal Readiness report type — absorbed by I490 in v1.1.0 |
| I459 | Stakeholder Map report type — absorbed by I496 in v1.1.0 |
| I460 | Success Plan report type — absorbed by I497 in v1.1.0 |
| I461 | Coaching Patterns — absorbed by I498 in v1.1.0 |
| I486 | Glean structured person data writeback — absorbed by I505 |
| I488 | Semantic gap queries sent to Glean — absorbed by I508 (intelligence schema redesign) |
| I501 | Transcript sentiment extraction — absorbed by I509 (interaction dynamics + sentiment) |

### Withdrawn (ADR-0099)

| ID | Resolution |
|----|------------|
| I510 | Supabase project provisioning — withdrawn with ADR-0099 (2026-03-03) |
| I516 | Sync engine — withdrawn with ADR-0099 (2026-03-03) |
| I517 | Supabase Auth — withdrawn with ADR-0099 (2026-03-03) |
| I518 | Organization + territory model — withdrawn with ADR-0099 (2026-03-03) |
| I519 | RLS policy design — withdrawn with ADR-0099 (2026-03-03) |
| I520 | Auth-first onboarding — withdrawn with ADR-0099 (2026-03-03) |
| I522 | Server-side embedding pipeline — withdrawn with ADR-0099 (2026-03-03) |
| I523 | Admin panel — withdrawn with ADR-0099 (2026-03-03) |
| I524 | Conflict resolution — withdrawn with ADR-0099 (2026-03-03) |
| I525 | Offline mode redesign — withdrawn with ADR-0099 (2026-03-03). I428 restored. |
| I526 | Online/offline detection — withdrawn with ADR-0099 (2026-03-03) |

### Removed

| ID | Resolution |
|----|------------|
| I348 | Email digest push — removed; creates feedback loop into email processing pipeline |

---

## [0.16.0] - 2026-03-02

First-run experience: demo mode, onboarding wizard, and empty state redesign.

### Added

- **First-run wizard** — 7-step guided setup on first launch: verify Claude Code, connect Google, set role preset, enter work domain, add first account, configure user context, and prime with a first piece of context. Claude Code step is non-skippable — without it the app cannot build briefings. All other steps have a skip option. Wizard progress persists across app restarts. Completing the wizard lands on the daily briefing, not an empty page.
- **Demo data mode** — "Try with demo data" option on the empty dashboard. Loads a pre-populated workspace with sample accounts, meetings, actions, and email context. Demo badge visible in folio bar throughout. "Connect real data" exits demo mode, clears all demo data, and starts the wizard.
- **Guided tour** — 4–6 contextual callouts on first real launch (non-demo). One per surface, dismissible individually or all-at-once. Does not block interaction. Does not reappear after dismissal.
- **Claude Code status in Settings** — System chapter shows installation and auth status with green/amber/red indicator. Banner shown in Settings when Claude Code isn't ready after wizard completion. "Sign in to Claude" opens claude.ai/login in browser; "Download Claude Code" opens claude.ai/download. "Check again" re-polls immediately.

### Changed

- **Empty states redesigned** — Every surface that previously showed a blank page now shows a purpose-specific message with a direct action button and benefit statement. Accounts, People, Actions, Email, and Daily Briefing all updated.
- **Advanced Settings simplified** — Feature toggle section removed. AI Models, Hygiene, Capture, Data Management, and Lock Timeout remain.

### Fixed

- **Claude auth check** — Replaced `claude --print hello` (LLM API call that timed out at 3s and always returned unauthenticated) with macOS Keychain lookup for `Claude Code-credentials` entry. Auth status now reflects actual credential presence, instantly.
- **Onboarding wizard stuck on role step** — Role step now advances correctly after selection.
- **Stakeholder contact creation navigation** — Creating a contact from meeting detail now navigates to the correct page.
- **Raw ISO timestamps on demo dashboard** — Demo data timestamps now format correctly.
- **Toast text selectable** — Toast notifications now allow text selection and copy.

---

## [0.15.2] - 2026-02-27

Audit log and enterprise observability release. DailyOS now records what it does in a tamper-evident log and supports dual-mode context gathering from local data or Glean's knowledge graph.

### Added

- **Tamper-evident audit log** — Append-only JSON-lines log at `~/.dailyos/audit.log` with SHA-256 hash chain, 90-day rotation, and `0o600` permissions. Every security, data access, AI operation, anomaly, and config event is recorded with structured detail fields. No PII stored — only IDs, counts, categories.
- **Activity Log UI** — Settings → Data → Activity Log. Last 100 records grouped by day, category filter chips (Security / Data / AI / Anomalies / Config), anomaly highlighting, collapsible detail view. Export to JSON-lines. Verify integrity button validates the hash chain.
- **Event instrumentation** — 18 audit event types covering: `db_key_generated`, `db_key_accessed`, `db_migration_started/completed`, `oauth_connected/revoked`, `app_unlock_succeeded/failed`, `google_calendar_sync` (with `events_fetched`), `gmail_sync` (with `emails_fetched`), `clay_enrichment`, `gravatar_lookup`, `entity_enrichment_completed` (with `duration_ms`), `entity_enrichment_failed` (with `error_category`), `email_enrichment_batch`, `injection_tag_escape_detected`, `injection_instruction_in_output`, `schema_validation_failed`, `workspace_path_changed` (with path `category`).
- **ContextProvider trait** — `context_provider/mod.rs` with `gather_entity_context()` and `mode()`. `LocalContextProvider` wraps existing context gathering. All intelligence and report generators call through the trait.
- **GleanContextProvider** — Glean MCP client with DashMap + SQLite cache (docs 1h, profiles 24h, org graph 4h). Two-phase gather: Phase A (local, ms) + Phase B (Glean network, 200-2000ms). Graceful fallback to local on Glean outage.
- **Dual-mode context switching** — Local mode (default) or Glean mode (Additive/Governed strategy). Gmail/Drive pollers disabled in Governed mode, Clay/Gravatar disabled in any Glean mode. `set_context_mode` command triggers full re-enrichment. Settings UI: ContextSourceSection with mode selector, endpoint, token, strategy.
- **Glean MCP spec discovery** — Dynamic Client Registration via MCP spec for Glean endpoint authentication.
- **Native Touch ID** — Replaced JXA-based LocalAuthentication with native `LAContext` via objc2 FFI. Eliminates osascript stalls.

### Changed

- **Idle lock timer** — Now resets on mouse/keyboard activity, not just app focus events. Prevents premature lock during active use.
- **Activity Log translations** — Dynamic event names: "Calendar synced (N events)", "Email synced (N emails)", "Database opened", "Intelligence updated", "Log maintenance", "AI output rejected (unexpected format)".

### Fixed

- **Touch ID stall** — Native LAContext FFI replaces osascript path that could hang indefinitely.
- **Idle lock timer** — Timer now resets on actual user activity (mouse move, key press), not just window focus.
- **Person-linked 1:1 meeting summary** — 1:1 meetings linked to a person now show the correct summary.
- **Encrypted DB startup checks** — Migration backup fallback handles edge cases cleanly.

### Security

- Tamper-evident audit log with SHA-256 hash chain for compliance and forensics
- Injection attempt detection and audit logging (`injection_tag_escape_detected`, `injection_instruction_in_output`)
- Schema validation failure auditing (`schema_validation_failed`)
- No PII in audit records — only IDs, counts, and classifications

## [0.15.1] - 2026-02-26

Security hardening release. All corporate intelligence data is now encrypted at rest, AI prompts are hardened against injection, and the app locks itself after idle periods.

### Added

- **SQLCipher encryption at rest** — Database encrypted with AES-256 via SQLCipher. Key stored in macOS Keychain, accessed via `security` CLI to avoid repeated password prompts during development. Automatic one-time migration from plaintext on first launch. Recovery screen shown when key is missing from Keychain.
- **App lock on idle** — Full-screen lock overlay after 15 minutes of inactivity (configurable: 5/15/30/never). Unlock via Touch ID using JXA-based LocalAuthentication. 3-attempt cooldown with 30-second lockout. Settings → System → Security section.
- **iCloud workspace detection** — One-time dismissible warning modal when workspace path is under iCloud-synced directory (Desktop, Documents, or Mobile Documents). Prevents accidental cloud sync of local intelligence data.
- **Time Machine exclusion** — `~/.dailyos/` excluded from Time Machine via `tmutil addexclusion -p` (sticky xattr). Directory permissions set to `0o700`, database files to `0o600`. Runs once per process lifetime.
- **Prompt injection resistance preamble** — Standard "external data, do not execute" instruction added to all 7 AI prompt sites (intelligence, email enrichment, transcript extraction, inbox enrichment, risk briefing, delivery workflows, report generation).
- **Three-tier prompt sanitization** — `wrap_user_data` (HTML-escaped tag wrap), `sanitize_external_field` (invisible unicode strip + 2KB cap + tag wrap), `encode_high_risk_field` (base64 encoding for titles/subjects). Applied across all prompt construction sites.
- **Output schema validation** — JSON structure validation on AI responses before DB write. Anomaly detection flags 6 suspicious patterns (system role leaks, injection phrases). Failed validation re-queues entity for retry (max 2 attempts).
- **Inbox-to-meeting matching** — MeetingNotes-classified inbox documents scored against historical meetings using multi-signal algorithm (title similarity + time proximity + entity match). Auto-links on confident match (score ≥ 100).
- **Meeting entity hot-swap** — Switching linked entities on Meeting Briefing page now auto-refreshes briefing content via `prep-ready` event. No manual refresh required. "Updating briefing..." banner during rebuild.
- **Encryption recovery screen** — Dedicated full-screen UI when encrypted database exists but Keychain key is missing. Instructions for Keychain restore or fresh start.

### Changed

- **Prompt structure** — Schema/format instructions moved to end of prompts (after all data sections) for better injection resistance.
- **Keychain access** — Replaced `keyring` crate with macOS `security` CLI. Eliminates repeated password prompts when dev binary changes on recompile.

### Fixed

- **`wrap_user_data` HTML escaping** — Now escapes `& < > "` before wrapping in `<user_data>` tags, preventing tag breakout from adversarial input.
- **Email enrichment injection** — `sender`, `sender_name`, `subject`, and `snippet` fields now sanitized before prompt injection (were interpolated raw).
- **Encrypted backup** — `db_backup.rs` now applies PRAGMA key to backup connection so `.bak` files are encrypted.
- **Touch ID unlock** — Switched from AppleScript to JXA for LocalAuthentication. AppleScript couldn't handle the async completion handler on `evaluatePolicy`.
- **Granola cache auto-detection** — Scans for `cache-v*.json` instead of hardcoded filename.

### Security

- AES-256-CBC encryption on all data at rest (SQLCipher)
- HTML entity escaping on all user-data boundaries
- Base64 encoding on high-risk fields (email subjects, calendar titles)
- Invisible unicode stripping on all external text fields
- Anomaly detection for prompt injection artifacts in AI output
- File permissions hardened (`0o700` directory, `0o600` files)
- Time Machine exclusion prevents backup of sensitive data

## [0.15.0] - 2026-02-25

Reports become meaningful when the system knows both sides of the equation. DailyOS generates two categories of reports: outward-facing reports (Account Health Review, EBR/QBR, SWOT) that reference the user's actual narrative, and inward-facing personal impact reports (Weekly Impact, Monthly Wrapped) that answer "what did I actually accomplish?" All stored in DB, invalidated when intelligence updates, exportable as PDF.

### Added

- **Report infrastructure** — `reports` table with `entity_id`, `entity_type`, `report_type`, `content_json`, `intel_hash`, `is_stale`. `generate_report` and `get_report` Tauri commands. Intel hash invalidation marks reports stale when underlying intelligence updates. ReportShell renderer with inline field editing (draft only, not persisted). PDF export via `@react-pdf/renderer` with editorial design system styling.
- **SWOT report type** — Four-quadrant strategic analysis populated from entity intelligence signals. Items reference real signal types and meeting history.
- **Account Health Review** — 5-slide internal briefing: health summary, score & trend, key risks, stakeholder coverage, engagement cadence, open commitments, renewal outlook. Accessible from account detail page via Reports button in folio bar.
- **EBR/QBR report type** — 7-slide customer-facing quarterly review: partnership overview, goals recap, value delivered, success metrics, challenges & resolutions, strategic roadmap, customer asks, next period priorities. Value Delivered section cites specific real events with source references. Customer-presentable PDF export.
- **Risk report migration** — Risk briefing reads from and writes to `reports` table instead of disk files. Fallback reads existing `risk-briefing.json` for migration grace period.
- **Weekly Impact Report** — 5-slide personal operational look-back covering the prior 7 days. Priorities moved, wins, what you did, what to watch, what carries forward. Auto-generates every Monday. Renders on `/me` page under "My Impact" section.
- **Monthly Wrapped** — Celebratory narrative impact report for the prior calendar month. Top wins, priority progress, honest miss, personality type, month-over-month comparison. Auto-generates on the 1st of each month. Bold editorial design with warm tone.
- **Preset-aware report language** — All 9 role presets produce reports with role-specific vocabulary, framing, and emphasis. CS reports reference renewals and health; Sales reports reference pipeline and deal stage; Leadership reports reference portfolio and ARR.
- **Entity context entries** — Structured knowledge entries (title + content + date + embeddings) replace static notes textareas on account, person, and project pages. Signal emission on create/update/delete. Semantic retrieval for intelligence prompt injection. Shared `ContextEntryList` component extracted from MePage.
- **Entity context in intelligence prompts** — Entity-specific context entries injected as "User Notes About This Entity" section in enrichment prompts. All entries for the entity included (no semantic threshold — entity-scoped entries are always relevant).
- **Legacy notes migration** — One-time startup migration converts existing people notes to entity context entries. Idempotent.

### Changed

- **Report access pattern** — Reports accessible from entity detail pages via "Reports" dropdown in folio bar. Personal reports on `/me` page. No separate reports page.
- **Notes → Context on entity pages** — Account, person, and project appendix sections renamed from "Notes" to "Context" with structured entry UI replacing free-text textarea.
- **Preset descriptions cleaned** — Removed vocabulary violations from preset description copy.
- **Nav island icon weight** — `/me` page nav icon strokeWidth corrected from 1.5 to 1.8 to match global nav.
- **Service layer async refactor** — State lifetime annotations and async service extraction for cleaner command handler patterns.

### Fixed

- **Stale email narrative in briefing** — Removed JSON fallback path; DB `resolved_at IS NULL` filtering is the only correct email source. Archived emails no longer appear in briefing.
- **Hallucinated meeting relevance** — Entity-aware check ensures email's entity must have a meeting today. Embedding similarity threshold raised from 0.05 to 0.15.
- **Meeting detail beachball** — Fixed blocking operation on meeting detail page load.
- **Extraction tier default** — One-time config migration from sonnet to haiku for extraction tier on app boot when user never explicitly changed it.
- **Transcript metadata persistence** — Transcript metadata now always persists with proper signal emission on attach.
- **Transcript sync button gating** — Sync Transcript button gated on Quill/Granola enabled state.
- **Monthly Wrapped + Weekly Impact polish** — Layout, typography, and content quality improvements across personal report slides.
- **Entity context hook crash** — Fixed `useEntityContextEntries` hook placement after early returns that crashed entity pages.

## [0.14.3] - 2026-02-24

Google Drive becomes a first-class connector. Import documents, spreadsheets, and presentations from Google Drive into entity intelligence — one-time or with ongoing sync via the Changes API.

### Added

- **Google Drive Connector** — Import files from Google Drive into entity Documents/ folders via Google Picker UI. Multi-select, entity linking, folder browsing. Files are converted to markdown: Docs via text/markdown export, Sheets via CSV in code blocks, Slides via text/plain export.
- **Import Once vs Watch Mode** — Choose between one-time import (file downloaded, no ongoing sync) and watch mode (Drive Changes API polls for updates on 60-minute adaptive interval).
- **Drive Settings UI** — Google connector section in Settings shows watched document count, last sync timestamp, Sync Now button, and per-source remove controls.

### Fixed

- **Update Banner Position** — Banner now renders below the FolioBar instead of behind it. Dismiss X button moved into flex flow (was overlapping Install & Restart).
- **HTML in Meeting Descriptions** — Calendar descriptions containing HTML markup (from Google Calendar) now display as clean text on the daily briefing page.
- **Tauri Command Serialization** — Fixed parameter names for all Google Drive commands.

## [0.14.2] - 2026-02-23

Role preset expansion + performance. Every role preset field now drives UI. Meeting prep speaks your role's language. Background tasks no longer fight each other for CPU.

### Added

- **Personality copy expansion (I439)** — 12 new personality-driven copy keys: action completed/dismissed/archived, generating briefing, building context, processing transcript, saved, connected, setup complete, sync/connection/enrichment errors. Toast messages, loading states, and errors now reflect your personality setting (professional, friendly, playful).
- **Preset-driven stakeholder roles (I442)** — Relationship type on stakeholder cards uses a badge dropdown sourced from the active preset's `stakeholderRoles`. CS preset shows Champion, Executive Sponsor, Decision Maker, etc. Sales shows Economic Buyer, Coach, Blocker.
- **Preset-driven team roles (I443)** — Internal team member roles sourced from preset's `internalTeamRoles` via the same badge pattern.
- **Lifecycle in intelligence prompts (I444)** — Account lifecycle stage injected as a prominent `## Current Lifecycle Stage` section in entity intelligence prompts. Previously buried in facts block.
- **Preset-driven account sorting (I445)** — Accounts page has a sort selector defaulting to the preset's `primarySignal` (CS: renewal proximity, Sales: deal stage, Leadership: ARR). User-chosen sort takes precedence.
- **Preset-specific /me playbooks (I446)** — All 9 presets have 3 named playbook sections on the `/me` page (CS: At-Risk Accounts, Renewal Approach, EBR/QBR Preparation; Sales: Deal Review, Territory Planning, Competitive Response; etc.). Preset-specific placeholder text on all "What I Deliver" fields.
- **1:1 meeting person focus (I455)** — 1:1 meetings resolve the non-user attendee as primary person context. Person intelligence, relationship history, and open actions surface in the briefing instead of generic account context. Works both when an entity is linked (person promoted over account) and when no entity is linked (person resolved from attendees).
- **Background task throttling (I457)** — Three-layer system: ActivityMonitor tracks user presence (Active/Idle/Background), HeavyWorkSemaphore prevents PTY and embeddings from competing for CPU simultaneously, adaptive polling backs queue processors from 5s to 30s during active use. 83% reduction in idle wakeups, 50% reduction in peak CPU.
- **Background status diagnostics** — `get_background_status` command returns activity level, queue depths, and semaphore state for dev tools.
- **RoleBadge component** — Reusable badge-style dropdown for stakeholder and team role selection, matching the EngagementSelector visual pattern.
- **useActivitySignal hook** — Frontend signals window focus/blur and debounced interaction to backend for activity-aware throttling.

### Changed

- **Meeting prep persona (I440)** — Removed hardcoded "Customer Success Manager" from all prep prompts. Now uses the active preset's role name and injects `briefing_emphasis` for role-specific framing.
- **useActivePreset** — Rewritten from standalone hook to React context provider with `preset-changed` Tauri event reactivity. Single IPC call at app root instead of per-page.
- **EngagementSelector** — Cleaned up: removed role-like options (Champion, Exec Sponsor) that belong on the role badge, replaced with actual engagement levels (Advocate, Active, Responsive, Passive, Disengaged, Blocker).
- **Email scoring split-lock** — Email relevance scoring no longer holds the main DB mutex during embedding inference. Opens a separate DB connection for scoring, preventing UI freezes when navigating to the emails page.
- **Silent background refreshes** — All event-driven data refreshes (dashboard, week, calendar, inbox, emails, executive intelligence) wrapped in `React.startTransition` to eliminate content blink when background data arrives.
- **Toast deduplication** — Same milestone toast type within 30 seconds is suppressed to prevent stacking.
- **Meeting detail refresh** — Folio bar refresh button now shows loading state and toast feedback. Subsequent data reloads preserve scroll position instead of flashing the loading skeleton.

### Fixed

- **ADR-0083 vocabulary violation** — Replaced "Informing all entity intelligence and signal ranking" on `/me` page with user-facing copy.
- **1:1 resolution on unlinked meetings** — `resolve_1on1_person` and `is_two_person_meeting` helper functions were lost during worktree merge. Restored and added to the "no entity resolved" fallback path.
- **`list_dismissed_email_items` blocking** — Switched from blocking `lock()` to non-blocking `try_lock()` with graceful degradation, preventing UI freeze when email poller holds DB lock.
- **Empty state personality keys** — `projects-empty` wired to ProjectsPage initial empty state (was using `projects-no-matches`). `accounts-empty` wired to AccountsPage.

## [0.14.1] - 2026-02-23

Retag of v0.14.0 for release pipeline.

## [0.14.0] - 2026-02-23

User entity + professional context. The app now knows about you — your role, what you deliver, your priorities, and your knowledge. This context shapes all entity intelligence, signal ranking, and meeting prep. Every account/person/project intelligence output now includes your perspective.

### Added

- **User Entity (`/me` page)** — Six-section editorial interface for professional context:
  - About Me: Name, title, company, company bio, role description, measurement criteria
  - What I Deliver: Value proposition, success definition, product context, pricing model, differentiators, objections, competitive context
  - My Priorities: Two-layer model — annual priorities (year-level bets) and quarterly priorities (current focus). Both persist until manually removed (zero-guilt). Link to accounts or projects for context.
  - My Playbooks: CS-first with 3 sections ("Win Strategy", "Renewal Strategy", "Escalation Protocol"); others get 1 generic section
  - Context Entries: Knowledge base with semantic search — user-created content retrieved during enrichment when relevant
  - Attachments: Drag-and-drop upload for PDFs, documents, etc. Content embedded and retrieved via cosine similarity (threshold 0.70)
- **Signal Weighting (I414)** — User-context-aware signal ranking. Signals linked to annual/quarterly priorities receive 1.5–2.0x score multiplier. Baseline: 1.0x.
- **File Attachment Embeddings (I413)** — PDF/document uploads from `/me` page attachments section stored with `source='user_context'`. Content retrieved with `search_user_attachments()` during enrichment. Embedding collection labels both context entries and file content for unified semantic search.
- **Intelligence Fields (I396)** — 6 new fields on entity_intelligence:
  - `health_score` (REAL) — Account/relationship health metric (0–100)
  - `health_trend` (TEXT) — Trending direction (improving, stable, declining)
  - `value_delivered` (TEXT) — User-facing value narrative (enriched from prior intelligence)
  - `success_metrics` (TEXT) — How success is measured for this entity
  - `open_commitments` (TEXT) — What's owed/pending (account-specific)
  - `relationship_depth` (TEXT) — Relationship maturity/history (people-specific)
- **User Context in Intelligence Prompts (I412)** — Entity enrichment pulls user entity fields + top-2 semantic context matches (entries + attachments). Injected as "Your Professional Context" section in intelligence prompts. Includes name, title, company, value proposition, annual/quarterly priorities, playbooks, and knowledge base matches.
- **Auto-Show What's New Modal** — App detects version upgrade and auto-shows release notes (What's New modal) on first launch after update. Stores `last_seen_version` in localStorage.
- **Update Check & Banner** — Settings includes "Check for Updates" button. When update available, persistent banner appears at top of app with "What's New" and "Install & Restart" options. Dismissible with X button.
- **Editorial Design System for Notifications** — Update banner and What's New modal redesigned to match magazine aesthetic: Newsreader serif headlines, generous padding, minimal chrome, eucalyptus accent colors, type hierarchy doing structural work.
- **Me Navigation Item** — Floating nav island includes new "Me" item (Larkspur accent). Shows content dot indicator when user entity is empty (prompt to fill in).
- **Migration 044–047** — User entity schema, intelligence fields, embedding support, user relevance scoring.

### Changed

- **Settings Page (YouCard)** — Removed "About You" section with identity fields (name, company, title, focus). All user professional context now lives on `/me` page only. Settings now shows: Domains, Role Preset, Workspace, Day Start, Personality, Connectors.
- **Navigation** — FloatingNavIsland includes "Me" item; activity dot shows when user entity is empty.
- **Intelligence Output** — All entity intelligence enrichment now includes "Your Professional Context" section when user entity has content. Prioritizes user-defined context over generic patterns.

### Removed

- **Identity Fields from Config** — User name, company, title, focus migrate from `~/.dailyos/config.json` to `user_entity` SQLite table on first app launch. Config fields deprecated post-migration.

## [0.13.9] - 2026-02-23

Connectors hardening. Every external data source now produces signals through the signal bus. Clay runs in production via Smithery Connect, Gravatar writes back to people profiles, Granola no longer hangs the app, and Linear issues surface in meeting prep. The enrichment pipeline is unified — one write path, one background processor, one source priority system.

### Added

- **Unified enrichment pipeline** — `db/people.rs: update_person_profile()` is the single write function for all enrichment sources (Clay, Gravatar, user, AI). Handles source priority (User 4 > Clay 3 > Gravatar 2 > AI 1), provenance tracking via `enrichment_sources` JSON, and `enrichment_log` audit entries. Replaces scattered per-connector write logic.
- **Unified enrichment processor** (`enrichment.rs`) — Single background task replaces separate Clay poller and Gravatar fetcher. Drain-until-empty loop (keeps sweeping while queue has work), budget by attempts not successes, wakes on `enrichment_wake` signal.
- **Clay via Smithery Connect (I422)** — Clay's direct MCP endpoint has no public OAuth. Rewired to use Smithery as managed proxy (`https://api.smithery.ai/connect/{ns}/{conn}/mcp`). Auto-detects Smithery CLI config and Clay connection ID. Keychain storage for Smithery API key.
- **Linear signal wiring (I425)** — Signals emitted on issue sync: `linear_issue_completed`, `linear_issue_blocked`, `linear_issue_overdue`. New `linear_entity_links` table (migration 041) maps Linear projects to DailyOS entities. Meeting prep includes related Linear issues for linked entities.
- **Linear entity link picker** — Searchable picker in Linear connector card for manually linking Linear projects to DailyOS accounts/projects. Auto-detect button for name matching.
- **Gravatar writeback (I423)** — Gravatar data (photo, bio, company, title) flows through `update_person_profile("gravatar")` with source priority. `emit_signal_and_propagate` for `profile_discovered` signals.
- **Granola wake signal (I424)** — Calendar poller wakes Granola poller immediately when meetings end via `granola_poller_wake`. No more waiting for the full poll interval.
- **Person profile rendering** — PersonHero shows bio (below intelligence lede), phone number, social link tooltips with full URLs. Avatar component accepts `photoUrl` prop for Clay/Gravatar photos.
- **People list avatars** — People entity list shows avatar photos (or initial fallback) with relationship-colored rings: turmeric for external contacts, larkspur for internal team.
- **People list grouping** — "All" tab groups people into sections matching accounts: "Your Contacts" (external), "Your Team" (internal), "Unclassified" (unknown) with ChapterHeading dividers.
- **Logger backend** — `env_logger` initialized at startup. All `log::info!`/`log::warn!` calls now output to stderr. Default filter: `dailyos_lib=info,warn`. Override with `RUST_LOG`.

### Changed

- **"Connections" → "Connectors" (I421)** — All user-facing labels, component files, directory names, types, and routes renamed throughout the Settings UI.
- **Granola DB mutex fix (I424)** — `process_granola_document` rewritten with three-phase lock pattern (read → drop → AI pipeline → re-acquire → write) matching Quill's approach. No more DB lock held across AI calls.
- **Clay enricher simplified** — 500 → 170 lines. Builds a `ProfileUpdate` struct and calls `db.update_person_profile("clay")` instead of inline merge logic.
- **Avatar loading** — `get_person_avatar` returns base64 data URL instead of local file path. No more dependency on Tauri asset protocol or `convertFileSrc`.
- **EntityRow component** — Gains optional `avatar` prop that replaces the accent dot. Existing account/project list rendering unchanged.
- **Active issues filter** — Linear connector "Recent Issues" section filters out completed/cancelled issues, sorted by priority.

### Removed

- **`clay/poller.rs`** — Replaced by unified `enrichment.rs`.
- **Legacy `enrich_person_from_clay`** — Replaced by `enrich_person_from_clay_with_client` using `emit_signal_and_propagate`.
- **`writeback_photo_url`** — Standalone Gravatar writeback function replaced by `update_person_profile("gravatar")`.
- **Clay direct SSE/stdio transport** — Replaced by Smithery Connect HTTP transport.

## [0.13.8] - 2026-02-22

AppState decomposition and SignalService formalization. The 30-field god struct is now a facade over 6 domain containers (14 top-level fields). The signal bus has a service-layer API so consumers no longer reach into bus internals. Purely mechanical refactoring — no logic changes, no behavior changes, no schema migrations.

### Added

- **SignalService facade (I403)** — `services/signals.rs` with 8 public functions: `emit`, `emit_and_propagate`, `emit_propagate_and_evaluate`, `get_for_entity`, `get_by_type`, `get_callouts`, `run_propagation`, `invalidate_preps`. All service-layer and infrastructure callers (13 files, ~40 call sites) migrated from direct `crate::signals::bus::*` imports. Internal `signals/` module callers stay direct. Six documented exceptions (prepare/, processor/, gravatar/) retain raw `db` handle access.

### Changed

- **AppState decomposition Phase 1 (I404)** — Extracted 4 sub-structs: `HygieneState` (6 fields: report, scan_running, last_scan_at, next_scan_at, budget, full_orphan_scan_done), `CaptureState` (3 fields: dismissed, captured, transcript_processed), `CalendarState` (3 fields: google_auth, events, week_cache), `WorkflowState` (3 fields: status, history, last_scheduled_run). All wrapper methods delegate to sub-struct fields.
- **AppState decomposition Phase 2 (I405)** — Extracted 2 sub-structs: `IntegrationState` (4 poller wake signals: clay, quill, linear, email), `SignalState` (3 fields: engine, entity_resolution_wake, prep_invalidation_queue). Constructor preserves init order: prep queue built before signal engine, then wired via `set_prep_queue`/`set_intel_queue`.
- **AppState reduced from 30 to 14 top-level fields** — All original fields accessible through domain containers. No field removed, only regrouped.

## [0.13.7] - 2026-02-22

Intelligence self-healing. The hygiene system now knows when its intelligence is wrong, not just when it's missing. Four new capabilities detect quality degradation, validate semantic coherence against meeting history, replace the hardcoded 14-day enrichment threshold with a continuous priority function, and wire user corrections back to source reliability. A circuit breaker prevents thrashing on persistently incoherent entities.

### Added

- **Entity quality scoring (I406)** — Beta distribution quality model per entity (`entity_quality` table, migration 040). Quality starts at 0.5 (uniform prior), increments alpha on enrichment success, increments beta on user correction. Low-quality entities (< 0.45) surface in the hygiene report and rank higher in the enrichment queue.
- **Semantic coherence validation (I407)** — Post-enrichment embedding check compares entity intelligence text against the entity's linked meeting corpus (last 90 days). Cosine similarity below 0.30 flags the intelligence and re-enqueues for enrichment. Emits `entity_coherence_flagged` signal for downstream consumers. Gracefully skips when embedding model is unavailable or entity has fewer than 2 meetings.
- **Enrichment trigger function (I408)** — Continuous priority score replaces the binary 14-day threshold: `imminence × 0.35 + staleness × 0.25 + quality_deficit × 0.20 + importance × 0.10 + signal_delta × 0.10`. Entities with imminent meetings and low quality scores rank highest. Trigger scores logged at DEBUG level for inspection.
- **Feedback closure (I409)** — User corrections to intelligence fields decrement the entity's quality score and penalize the enrichment source (`intel_queue`) in the Thompson Sampling weight system. Editing Clay-enrichable fields on people, accounts, and projects penalizes the `clay` source. Successful enrichments increment alpha immediately.
- **Circuit breaker (I410)** — Prevents infinite re-enrichment loops. Three coherence failures within 24 hours trips the breaker; auto-expires after 72 hours. Manual "Refresh Intelligence" bypasses the breaker and resets retry state. Blocked entities surface in the hygiene report.
- **Event-driven signal evaluation (I410)** — `emit_signal_propagate_and_evaluate` wrapper in signal bus triggers enrichment re-evaluation on signal arrival (trigger score > 0.7) without waiting for the 4-hour scan. Wired in account, person, and project field update paths.
- **`self_healing/` module** — 1,100+ lines across 6 files (quality.rs, detector.rs, remediation.rs, feedback.rs, scheduler.rs, mod.rs). Clean separation from hygiene.rs which gained only ~17 net lines for integration hooks.

### Changed

- **Hygiene Phase 3 uses `evaluate_portfolio()`** — Replaces `enqueue_ai_enrichments()` with self-healing-aware portfolio evaluation that respects quality scores, circuit breakers, and continuous trigger priorities.
- **`check_upcoming_meeting_readiness` uses trigger score** — Pre-meeting refresh now uses the continuous trigger function instead of the hardcoded `PRE_MEETING_STALE_DAYS` constant. Imminent meetings (< 24h) always trigger refresh regardless of enrichment age.
- **New entity creation initializes quality row** — `create_account`, `create_project`, and `create_person` service functions insert an `entity_quality` row at Beta(1,1) so quality-aware code paths work immediately.
- **`HygieneReport` expanded** — Two new fields: `lowQualityEntities` (score < 0.45) and `coherenceBlockedEntities` (circuit breaker tripped).

## [0.13.6] - 2026-02-22

Maximum commands.rs extraction. Moved 1,405 lines of business logic from command handlers into the service layer, creating two new service files (settings, intelligence) and expanding four existing ones. commands.rs is now a thin IPC dispatch layer at ~7,000 lines. One UX fix ensures internal meetings show their attendees instead of an empty room.

### Added

- **SettingsService** (`services/settings.rs`, 303 lines) — 7 config mutation methods extracted: entity mode, workspace path, AI model tier, hygiene config, workflow schedules, user profile, multi-domain management with people/meeting reclassification
- **IntelligenceService** (`services/intelligence.rs`, 242 lines) — unified `enrich_entity` method replacing 3 identical enrich handlers, plus intelligence field edits, stakeholder bulk-replace with propagating signals, and risk briefing CRUD

### Changed

- **Internal meeting attendees visible (I401)** — `hydrate_attendee_context` no longer filters out internal people for `team_sync`, `internal`, and `one_on_one` meetings. External meetings still show only external attendees.
- **ProjectService expanded** (`services/projects.rs`, 131 → 461 lines) — 7 handlers extracted: list, child list, create, update field, update notes, bulk create, archive
- **EmailService expanded** (`services/emails.rs`, 310 → 624 lines) — 6 handlers extracted: entity email lookup (3-strategy: direct → person sender → account people), entity reassignment, signal/item dismissal, Gmail archive, async refresh
- **AccountService expanded** (`services/accounts.rs`, 819 → 1,083 lines) — 3 handlers extracted: internal organization creation (transactional with filesystem best-effort), child account creation with intel queue enqueue, internal meeting backfill
- **MeetingService expanded** (`services/meetings.rs`, 1,331 → 1,517 lines) — 2 handlers extracted: refresh all future meeting preps (clear + re-enqueue), attach transcript with TOCTOU guard and async processing

## [0.13.5] - 2026-02-22

People are the most under-represented entities in DailyOS. A user managing a buying committee sees five individuals by name but has no way to express that Rachel manages Amy, that two engineers are peers who collaborate on the same project, or that a mentor introduced them to a key contact. This version adds typed person-to-person relationship edges with manual CRUD, directional label resolution, signal propagation across the network, and AI enrichment that synthesizes network intelligence from relationship context.

### Added

- **Person relationship graph** — `person_relationships` table (migration 038) with 7 person-to-person types: peer, manager, mentor, collaborator, ally, partner, introduced_by. Context-scoped edges (optional entity association). Confidence decay on inferred edges (90-day half-life), no decay on user-confirmed edges.
- **Migration 039** — recreates `person_relationships` table with corrected CHECK constraint, maps any legacy account-context types (champion, executive_sponsor, etc.) to person-to-person equivalents
- **Tauri CRUD commands** — `upsert_person_relationship`, `delete_person_relationship`, `get_person_relationships` with computed effective confidence and joined person/entity names
- **Person-to-person signal propagation** — `rule_person_network` propagates signals across relationship edges with type-sensitive multipliers (manager/partner 1.0x, mentor 0.8x, collaborator 0.8x gated at 0.7 confidence, peer/ally 0.7x, introduced_by 0.5x). Loop prevention via source tag. Confidence gate at 0.65.
- **Network intelligence in enrichment** — person enrichment prompt includes relationship edges and neighbor signal summaries. AI produces `network` field: cluster_summary, key_relationships, risks, opportunities, influence_radius, health assessment
- **"Their Network" chapter** — new chapter on person detail pages with manual add/delete connections, editorial Select dropdown with directional choices (Manager/Direct Report, Mentor/Mentee), confidence tooltip for inferred edges, empty state with add CTA
- **Directional label resolution** — asymmetric types flip labels based on viewing direction (manager displays as "Direct Report" on the other person's page, mentor as "Mentee")
- **Relationship vocabulary on presets** — `relationshipVocabulary` field on `RolePreset` for preset-sensitive edge labels

### Changed

- **"The Network" renamed to "Their Orbit"** — person detail Chapter 3 (linked accounts/projects) renamed; "Their Network" is the new Chapter 4 (person-to-person relationships)
- **Chapter spacing** — Both "Their Orbit" and "Their Network" use proper design system spacing (`padding-top: var(--space-2xl)`)
- **PersonNetwork and PersonRelationships** — removed duplicate `sectionId` prop (parent wrapper handles scroll anchoring)

## [0.13.4] - 2026-02-22

v0.13.3 proved that parent accounts become portfolio surfaces when two-layer intelligence and bidirectional signal propagation are wired correctly. This version applies the identical architecture to project entities: parent projects become portfolio surfaces for users in project-mode roles (Marketing, Product, Agency), child project signals propagate upward to feed portfolio intelligence, and parent-level signals cascade down to direct children. Navigation now adapts to the user's role — project-mode users see Projects before Accounts.

### Added

- **Project hierarchy** — `parent_id` column on projects table (migration 037), recursive ancestor/descendant queries, parent aggregate stats (active/on-hold/completed counts, nearest target date), circular-reference prevention on parent assignment
- **Project portfolio intelligence** — `build_project_portfolio_children_context` gathers each child project's intelligence.json and active signals into the parent's enrichment prompt with project-appropriate vocabulary (campaign, workstream, milestone, program health — not account health, renewal, spend)
- **Bidirectional signal propagation for projects** — `rule_hierarchy_up` and `rule_hierarchy_down` extended to handle project entity type with same confidence attenuation as accounts (0.6× upward, 0.5× downward, 0.7 confidence gate for fan-out)
- **Parent project enqueue on child update** — intel_queue enqueues the parent project for portfolio refresh whenever a child project's intelligence is written, matching the account pattern from I384
- **Portfolio chapter on parent project detail** — portfolio narrative (serif epigraph), hotspots (child projects needing attention with one-line reason, linking to child detail), cross-project patterns (hidden when empty), condensed child list with status indicators and action counts
- **Ancestor breadcrumbs on project detail** — nested projects show a `Projects / Parent / Current` breadcrumb trail via `get_project_ancestors` recursive CTE
- **Sub-project creation** — "+ Sub-Project" button on parent project detail pages opens a creation dialog that sets `parent_id` on the new project
- **Expandable project tree on projects list** — parent projects show expand chevron with child count; clicking loads children via `get_child_projects_list`; recursive tree rendering with indentation; search matches against child project names
- **Entity-mode-aware navigation ordering** — FloatingNavIsland accepts `entityMode` prop; project-mode role presets (Marketing, Product) show Projects before Accounts in the nav; account-mode presets (CS, Sales) show the reverse; switching presets in Settings updates nav immediately without app restart

### Changed

- **`create_project` accepts optional `parent_id`** — existing callers unaffected (Tauri deserializes absent field as None)
- **Archive cascade for projects** — archiving a parent project now cascades to all child projects, matching the account archive behavior
- **`ProjectListItem` and `ProjectDetailResult`** — both structs now include `parent_id`, `parent_name`, `child_count`, `is_parent`, `children`, and `parent_aggregate` fields
- **MagazinePageLayout re-fetches entity mode on navigation** — ensures preset changes in Settings take effect on the next page transition without a full app restart

## [0.13.3] - 2026-02-22

Parent accounts are portfolio surfaces, not folders. A user managing Salesforce's 10 BUs under one parent needs a surface that shows the whole picture: which BUs are healthy, which need attention, what patterns emerge across the portfolio. This version makes parent accounts into that surface, adds partner as a first-class entity type, regroups the accounts page to match, and wires bidirectional signal propagation so BU signals accumulate at the parent without the user specifying which children are affected.

### Added

- **Partner entity type** — `account_type` column (customer/internal/partner), migration 036, clickable type badge on account detail with inline dropdown selector, type badges in entity picker, AccountsPage three-group layout (Your Book / Your Team / Your Partners)
- **Partner meeting classification** — meetings with partner-account attendees classify as `partnership` type with Entity intelligence tier, distinct from customer meetings
- **Portfolio chapter on parent accounts** — health summary (bold serif epigraph), portfolio narrative, hotspots (child accounts needing attention), cross-BU patterns, condensed child list with health indicators and type badges
- **Bidirectional signal propagation** — `rule_hierarchy_up` propagates child signals to parent with 48-hour sibling accumulation via weighted log-odds fusion; `rule_hierarchy_down` fans out high-confidence (≥ 0.7) parent signals to direct children at 0.5× attenuation
- **Signal-driven intel enrichment** — propagated signals targeting a different entity than the source automatically enqueue the target at ProactiveHygiene priority in the intelligence queue
- **Child account search** — searching on the accounts page now matches against child account names in the cache; parent rows auto-expand when a child matches
- **`accountType` on `AccountChildSummary`** — child accounts in the portfolio chapter display a type badge for non-customer types (Partner/Internal)
- **`account_type` on `EntityHint`** — meeting classification entity resolution now carries account type for partner detection
- **`healthSummary` in portfolio rendering** — parent account portfolio chapter renders the health summary field above the narrative when present
- **`get_recent_signals_by_type` DB helper** — queries signal_events within a time window by entity and type, excluding hierarchy-propagated signals, used by accumulation logic

### Fixed

- **Watcher clobbering account_type** — `read_account_json` hardcoded `AccountType::Customer`; every field update regenerated `dashboard.json`, triggering the watcher to upsert the account with `Customer` type, immediately overwriting user-set partner/internal types. Watcher now preserves `account_type`, `archived`, and `name` from the existing DB record
- **Sync paths clobbering DB-only fields** — `sync_accounts_from_workspace` merge (both parent and child) and `populate_workspace` now preserve `account_type`, `archived`, and `name` from existing DB records instead of overwriting with hardcoded defaults
- **StakeholderGallery TypeScript error** — `title` prop on Lucide `LinkIcon` replaced with `aria-label` for Lucide React compatibility

## [0.13.2] - 2026-02-21

Know what you built before you build the next layer. Every AI enrichment call site audited and classified against ADR-0086. Every signal propagation rule confirmed to have a live emitter. Every `intelligence.json` field mapped as live, write-only, or dead — dead fields removed. The vector DB confirmed useful with four live consumers. `commands.rs` reduced by service extraction to 8,121 lines. `db/mod.rs` reduced to 441 lines via domain module migration. No user-visible changes.

### Changed

- **Service extraction** — action, account, people, and meeting business logic extracted from `commands.rs` into dedicated `services/` modules. Command handlers are now parse → delegate → serialize with no embedded logic. `commands.rs` reduced from 11,500 to 8,121 lines
- **DB domain migration** — all domain SQL moved from `db/mod.rs` into `db/accounts.rs`, `db/actions.rs`, `db/meetings.rs`, `db/people.rs`, `db/projects.rs`, `db/signals.rs`, `db/emails.rs`, `db/entities.rs`. `db/mod.rs` reduced from ~9,700 to 441 lines; retains only struct definition, `open()`, `conn_ref()`, `with_transaction()`, shared constants, and re-exports
- **Signal chain completed** — all 15 user field edit handlers (accounts, people, projects, stakeholders, intelligence fields) now call `emit_signal_and_propagate` after each DB write, closing the loop from user edit → signal → intel_queue → intelligence refresh → UI update without manual refresh
- **Entity relinking is instant** — changing which account/project/person is linked to a meeting now triggers immediate prep re-assembly via `MeetingPrepQueue` with no AI call and no page reload. Meeting card updates within 2 seconds
- **Intelligence schema aligned** — `valueDelivered` field removed from AI prompts, `entity_intel` schema, and TypeScript types. `entity_intel` table columns and TypeScript `EntityIntelligence` type are now structurally consistent with no undocumented divergence
- **Entity list pages use FinisMarker** — `EntityListEndMark` now renders the canonical three-asterisk `FinisMarker` component instead of a custom italic text ending

### Fixed

- Removed dead `person_departed` signal path from `rule_departure_renewal` — no emitter exists for this signal type; Clay's `company_change` covers the real-world case and remains
- Removed `person_departed` from meeting invalidation watch list for the same reason

## [0.13.1] - 2026-02-21

Email is an intelligence input, not a display surface. Every email in your inbox is AI-processed, resolved to an entity, and synthesised with existing intelligence to produce contextual understanding you can act on. The email page shows you what your emails mean. The daily briefing's "Worth Your Attention" section surfaces the emails that actually matter, scored by entity relevance, signal activity, meeting proximity, and urgency — not by mechanical priority tier. Calendar polling now covers the full week so cancellations, rescheduling, and new events appear without a page visit.

### Added

- **Email relevance scoring** — every enriched email receives a 0.0–1.0 relevance score combining entity linkage, active signal weight, Bayesian urgency fusion, keyword matching, and temporal decay. Score reason is stored alongside the score for transparency
- **"Worth Your Attention" on the daily briefing** — replaces the mechanical "Replies Needed" email section. Top scored emails (threshold 0.15) surface with contextual summaries and entity names; calendar notifications, noreply senders, and newsletters score near zero and never appear here
- **Email page scored layout** — emails sorted by relevance score into bands (Priority / Monitoring / Other), replacing the old high/medium/low tier display
- **Entity badges on emails** — each email shows which account, person, or project it is linked to, with an editable picker for corrections
- **Contextual email synthesis** — AI enrichment produces summaries that reference the entity's current intelligence and recent meeting history, not just the email content in isolation. "Jack is confirming the Acme EBR agenda for Thursday — this relates to the renewal discussion from Tuesday" rather than "Email from Jack about EBR"
- **Emails persisted to SQLite** — `emails` table is the source of truth; the daily JSON file is generated from DB, not the other way around. Email history survives across days and is queryable by entity
- **Inbox-anchored fetch** — Gmail query changed from `is:unread newer_than:1d` to `in:inbox` with no date window. Read emails that are still in the inbox remain visible; archived emails are reconciled out on the next poll
- **Inbox reconciliation** — emails removed from the Gmail inbox are marked resolved in the DB on each poll cycle; their historical signals are retained but they no longer appear in the UI
- **Thread position refresh** — a separate `in:sent` query detects user replies between polls so "Replies Needed" clears when you reply from Gmail without waiting for the other party
- **Recent Correspondence on meeting detail** — meetings show contextual email summaries from attendees, drawn from `recentEmailSignals` on the meeting prep object
- **Email sync status indicator** — email page shows last successful fetch time, enrichment progress, and stale-fallback state
- **Email dismissal learning** — repeatedly dismissing emails from a domain lowers future classification for that domain; learning is additive, not a hard override; reset available in Settings
- **Email-entity signal compounding** — enriched emails emit sentiment, urgency, and commitment signals to their linked entities. Person email signals propagate to linked accounts via `entity_people`, so account-level intelligence reflects email activity
- **Calendar polling expanded to ±7 days** — poller fetches today through the next 7 days on every cycle; new meetings, cancellations, and reschedules appear within one poll interval without requiring a page visit
- **Toast feedback on meeting briefing refresh** — clicking refresh now shows a confirmation toast so the action is never silent

### Fixed

- Future meetings no longer leak into the daily briefing — the calendar merge date filter was missing a timezone-aware guard; any live event not occurring today in the user's timezone is now excluded
- Meeting prep queue no longer locks out AI-generated content — mechanical stubs were setting `prep_frozen_at`, which blocked the AI workflow from updating via `freeze_meeting_prep_snapshot`. The prep queue now writes mechanical context without claiming the frozen-at timestamp
- `generate_meeting_intelligence` no longer destroys `prep_context_json` — a missing field initialisation was overwriting the context column with null on force-refresh
- Intelligence enrichment no longer produces a false "enriched" state from a mechanical row count — enrichment state is only set to enriched after AI output is confirmed
- "No prep" badge shown incorrectly when intelligence existed — badge logic now checks the correct quality field
- Week page date grouping used UTC instead of local timezone — meetings near midnight could appear under the wrong day heading
- Past meeting duration showing NaN on the weekly forecast — `formatDurationFromIso` now guards against invalid or missing timestamps before computing duration
- Quill transcript sync stuck in "already in progress" — force-resync now correctly resets stuck in-progress states back to pending

## [0.13.0] - 2026-02-21

Every meeting gets intelligence before you need it. One meeting, one visual identity, everywhere. The app never shows an empty state — intelligence arrives automatically, refreshes when signals change, and renders the same way whether you're looking at the daily briefing, weekly forecast, or a meeting detail page.

### Meeting Intelligence Lifecycle

- Every meeting gets intelligence — not just external or customer meetings. 1:1s, team syncs, all-hands, and internal meetings all receive contextual preparation
- Background meeting prep queue processes meetings by priority: manual refresh (immediate), calendar changes (high), and background sweeps (low)
- Intelligence regenerates automatically when entity data changes — update an account's intelligence and linked meeting briefings requeue for regeneration
- Signal-triggered prep invalidation: email signals, entity changes, and calendar updates mark affected preps stale and requeue them
- Advance generation: weekly pipeline and calendar polling ensure meetings have intelligence days before they happen

### Unified Meeting Card

- New shared MeetingCard component renders meetings identically across all surfaces
- Meeting type accent colors: turmeric for customer/external, larkspur for 1:1s, linen for internal/team
- Intelligence quality badges appear on every meeting card — daily briefing, weekly forecast, meeting detail
- Entity context (account or project name) shown as a byline below every meeting title
- Past meetings render with muted treatment, future meetings show time-to-meeting and prep status
- All styling via CSS module — no inline style props

### Weekly Forecast Redesign

- Single data source architecture: timeline powered directly by get_meeting_timeline, not a separate WeekOverview pipeline
- Compact header with density map (The Shape) showing meeting load per day
- Meeting intelligence timeline replaces the old MeetingRow list — same visual language as the daily briefing
- Removed workflow phase stepper, waiting messages, and unused AI enrichment fields (weekNarrative, topPriority)
- Past meetings show outcomes and follow-up counts; future meetings show intelligence quality and days until
- Personal events, focus blocks, and blocked time filtered from the timeline

### Daily Briefing — Always Live

- No empty state: briefing displays intelligence as it becomes available, without waiting for a full workflow run
- Day Frame section merges Hero and Focus into a single opening: narrative line, capacity, focus directive
- Schedule section shows today's meetings with inline expansion for the next upcoming meeting
- Attention section: meeting-relevant actions, high-signal proposed actions, urgent emails, replies needed
- Replies Needed subsection shows threads where the last message is from the other party (ball in your court)
- Removed Lead Story, Review, Priorities, Later This Week, Key People, Prep Grid, and entity chips

### Actions — Meeting-Centric

- Primary view groups actions by upcoming meetings: "Acme QBR · Thursday → 3 actions"
- Actions linked to accounts with upcoming meetings float to the top; everything else appears below
- Correlated subqueries on the backend find the next meeting per action's account within 3 days
- Sort by priority within each group, overdue actions pinned to top
- Auto-expiry: proposed actions older than 7 days are automatically archived

### Surface Restructure

- Meeting detail: Deep Dive and Appendix removed — keeps Brief, Risks, Room, Plan, Finis only
- Entity detail pages: removed meeting readiness callouts, resolution keywords, Value Delivered, Portfolio Summary
- Person detail: The Work chapter suppressed when empty (no actions or upcoming meetings)
- Account/Project detail: removed unused intelligence prop from TheWork component
- HorizonChapter (projects): removed Meeting Readiness section — meeting detail owns readiness

### Backend Infrastructure

- Architectural refactoring: split monolithic files, added service layer modules (services/actions, accounts, people, meetings, entities)
- Background email poller runs every 15 minutes during work hours with live frontend refresh via Tauri events
- Work hours gate removed from calendar and email pollers — they run whenever the app is open
- Keychain retry logic for OAuth token refresh on transient Keychain errors
- Fonts bundled locally to eliminate flash of unstyled text on page reload
- Meeting detail page CSS module migration (moved inline styles to editorial pattern)
- Quill sync force-reset fix for in-progress transcript states
- Fixed transcript outcomes being silently discarded (wins, risks, decisions, actions)

### Changed

- "Sync Transcript" button restored to meeting folio bar
- Refresh button on week page requeues meeting prep generation instead of rerunning AI enrichment
- Calendar description and attendees now stored in meetings_history on sync
- Entity-aware meeting classification resolves entities for all meeting types, not just external
- Signal bus wired to all user gestures and system events
- Thompson Sampling connected to four previously unlinked signal plumbing paths

### Fixed

- First-run meeting gap where meetings appeared only after second briefing run
- Schedule staleness: stale data on disk no longer persists across workflow runs
- Week page blank render when reveal observer didn't trigger on timeline data
- Attendee count using prep stakeholders instead of actual calendar invitees
- Attendees stripped before directive write (lean_events removing data too aggressively)
- Timestamps using bare datetime('now') instead of RFC3339 UTC
- Email refresh button showing literal unicode escape instead of ellipsis character

## [0.12.1] - 2026-02-19

The first release that subtracts. Every surface asked "does this earn its keep?" — what failed got cut, system jargon got replaced with product language, and 0.12.0 email intelligence got an editorial UI.

### The Correspondent — Email Intelligence Page

- Email page redesigned as "The Correspondent" — an editorial dispatch, not an email client
- 76px narrative headline synthesized from inbox signals (replies waiting, meeting-linked threads, cadence anomalies)
- Four margin-grid sections: Your Move (replies needed), Commitments (extracted promises), Open Questions (with account/sender context), Signals (per-entity prose assessments)
- Entity-scoped relevance filtering — only emails linked to tracked accounts/projects surface intelligence
- Noise filtering excludes support tickets, notifications, marketing, and billing emails automatically
- Inline dismiss on every item with SQLite persistence for future relevance learning
- Enrichment prompts now request contextual prose ("Sarah Chen committed to delivering the revised SOW by Friday") instead of terse fragments

### Surface Cuts

- Week page: removed Meetings, Open Time, and Commitments chapters — keeps The Three and The Shape only
- Meeting detail: removed Deep Dive zone and Appendix (2931 → 2061 lines) — keeps Brief, Risks, Room, Plan, Finis
- Daily briefing: merged Hero and Focus into single Day Frame section, cut Later This Week action group
- Actions page: three tabs only (proposed, pending, completed) with smart default
- Entity pages: removed Value Delivered, Portfolio Summary, Resolution Keywords, meeting readiness callouts
- Deleted 5 unused components (ActionItem, ActionList, EmailList, WatchItem, AppSidebar)
- Daily briefing: removed deep work block count from Day Frame, kept available minutes
- Meeting detail: removed transcript sync button from folio bar for past meetings

### Product Vocabulary

- "Build Intelligence" → "Refresh" across all entity heroes
- "Account Intelligence" → "Last updated" with timestamp
- "Entity mode" → "Work mode" in settings
- "AI enrichment" → "AI analysis" in status messages and onboarding
- "intelligence layer" → "daily briefings" in settings
- "Generate Briefing" → "Prepare my day" on empty dashboard
- "Read full intelligence →" → "Read full briefing →" on meeting cards and daily briefing
- "Meeting Intelligence Report" → "Meeting Briefing" on meeting detail page
- "Prep not ready yet" → "Not ready yet", "Prep is being generated" → "Briefing is being generated"
- "AI Suggested" → "Suggested", action tabs show human labels (Suggested/Pending/Completed)
- "Reject" → "Dismiss" on all action buttons

### Intelligence Quality Indicators

- New IntelligenceQualityBadge component with freshness dots (green < 24h, amber < 48h, gray > 48h, transparent = none)
- Labels: Fresh, Building, Sparse, No data — shown in tooltips alongside enrichment timestamp
- Integrated into all entity heroes (accounts, people, projects)
- Schedule row prep dots replaced with quality badges

### Inline Editing

- EditableText rewritten: textarea-first default, Tauri event emission on commit, Tab/Shift+Tab keyboard navigation, Escape cancels
- New EditableList component with HTML5 drag-to-reorder and grip handles
- Account and project field drawers replaced with inline editable fields in hero sections
- New CyclingPill component for select-style fields (Health, Lifecycle, Status) — click to cycle through options
- Fields auto-persist on blur with debounced save, no explicit Save button

### Email Intelligence Backend

- Email enrichment groups by thread_id for thread-level context before AI analysis
- Commitments, questions, and sentiment extracted per email and persisted to emails.json
- Semantic email reclassification: opt-in AI re-scoring of medium-priority emails (behind semanticEmailReclass feature flag)
- Entity thread signal summaries upgraded from mechanical counts to editorial prose

### Navigation

- Dropbox added to nav island (above Actions, after separator) for document/file inbox
- Mail nav item for email intelligence page
- InboxPage folio label updated to "Dropbox"

### Settings

- Settings page refactored into component modules (YouCard, ConnectionsGrid, SystemStatus, DiagnosticsSection)
- Day start time picker for morning briefing schedule

### Changed

- Email narrative headline capped at 12 words for 76px readability
- Extracted commitments and questions render in primary text color with per-item source context (entity, sender, subject)
- Entity signal summaries are editorial prose instead of "2 risks, 1 expansion" counts

### Stats

- 915 Rust tests passing, 0 clippy warnings, 29 frontend tests passing
- Net -1,333 lines across 71 files

## [0.12.0] - 2026-02-19

The chief of staff reads your email. Signals, not summaries. Briefing, not inbox. Built on the 0.10.0 signal bus.

### Email Intelligence

- Meeting-aware email digest: high-priority emails organized by meeting relevance instead of raw excerpts, surfaced in meeting prep context
- Thread position tracking: "ball in your court" detection identifies which threads await your reply vs. waiting on others
- Entity-level email cadence monitoring: weekly volume per entity with 30-day rolling average, anomaly detection flags "gone quiet" and "activity spike" patterns
- Hybrid email classification: medium-priority emails from senders linked to entities with active signals get promoted to high priority automatically
- Email commitment extraction: fetches full email bodies for high-priority messages, runs through Claude to identify commitments, requests, and deadlines — creates proposed actions automatically
- Email briefing narrative: daily briefing integrates a synthesized narrative covering reply urgency, entity correlations, and cadence anomalies
- Zero-touch email disposition: auto-archive pipeline for low-priority emails during daily prep, with disposition manifest and correction feedback. Surfaced as "Auto-Archive Email" toggle in Settings (off by default, since it modifies Gmail)
- Enhanced email signals in entity enrichment: sender name/role resolution, relative timestamps, cadence summary with trend analysis, AI prompt interpretation guidance, dynamic signal limit (20 for entities with upcoming meetings)

### Intelligence

- Calendar description steering: meeting calendar descriptions now steer intelligence narrative, giving the AI context about meeting purpose and agenda
- 1:1 relationship intelligence: person entity resolution for 1:1 meetings with three-file intelligence pattern (dashboard.json, intelligence.json, context.md)
- Self-healing hygiene: signal→hygiene feedback loop with auto-merge duplicates, calendar name resolution, co-attendance linking
- Person actions, week entities display, and vocabulary injection for role-preset-aware AI prompts

### Changed

- Email signal text and AI summaries render in primary text color for better readability of the most valuable content
- Email commitment extraction enabled by default — no feature flag needed

### Fixed

- Closed gaps in signal emission, cadence computation, and narrative generation across the email intelligence pipeline
- DB mutex not held across async/PTY calls in email commitment extraction (two-phase pattern: async fetch, sync extraction)

## [0.11.0] - 2026-02-19

Role presets, entity architecture, and industry-aligned terminology. The system speaks your language now.

### Role Presets

- 9 embedded presets (CS, Sales, Marketing, Partnerships, Agency, Consulting, Product, Leadership, The Desk) with role-specific vocabulary, email keywords, metadata fields, and AI prompt framing
- Role selection in Settings and onboarding — the system adapts its entire vocabulary to your function
- Role-aware email classification keywords boost domain-specific signals

### Entity Architecture

- Lifecycle events: renewal metadata, lifecycle event tracking, proactive detectors, account merge support
- EntityPicker supports multiselect mode with excluded-parent child visibility
- PersonNetwork supports optimistic multi-select entity linking without page reload
- StakeholderGallery searches existing people before creating new entries

### Changed

- Meeting card key people sourced from calendar attendees instead of entity stakeholders
- Back button uses browser history on all detail pages

### Fixed

- Quill transcript sync hang — release DB mutex during AI pipeline to prevent deadlock
- Internal account propagation and recursive account tree with add-child on all accounts
- Email signal fan-out with confidence filtering, prep invalidation queue consumer

## [0.10.1] - 2026-02-19

User feedback and onboarding polish. First real user session surfaced friction — fixed fast.

### Added

- Gmail teammate suggestions: onboarding "About You" chapter suggests closest teammates from Gmail frequent correspondents (scans sent mail, filters to same domain, returns top 10 by frequency). Clickable chips above manual entry field.
- Linear integration (data layer): Settings card with API key + test connection, background poller syncing assigned issues and team projects via GraphQL API

### Fixed

- Onboarding back navigation no longer loses entered state — form data lifted to parent component so back navigation preserves everything you've typed

## [0.10.0] - 2026-02-18

The intelligence release. The system that learns from you. Signals compound, corrections teach, events drive action.

### Signal Intelligence

- Intelligent meeting-entity resolution: Bayesian fusion of 5 signal producers (junction table, attendee inference, group patterns, keyword match, embedding similarity) with three-tier confidence thresholds
- Signal bus foundation: event log, weighted log-odds Bayesian fusion, temporal decay, email-calendar bridge
- Correction learning: Thompson Sampling with Beta distribution for per-source reliability weights, gated behind 5-sample minimum — your corrections make the system smarter
- Event-driven signal processing: cross-entity propagation engine with 5 rules (job change, frequency drop, overdue actions, champion sentiment, departure+renewal risk)
- Proactive surfacing: 8 pure SQL+Rust detectors (renewal gap, relationship drift, email volume spike, meeting load forecast, stale champion, action cluster, prep coverage gap, no-contact accounts) with fingerprint dedup

### Entity Architecture

- Entity-generic data model: `meeting_entities` junction table replaces account-only meeting linking — meetings can now relate to accounts, projects, and people
- Entity-generic classification: entity hints from DB, 1:1 person detection, multi-type resolution
- Entity-generic context building: type-dispatched intelligence injection — accounts get dashboard/stakeholders/captures, projects get status/milestones, people get relationship signals
- 1:1 relationship intelligence: three-file pattern for people entities with relationship-specific enrichment prompts
- Person as first-class entity type with dedicated icon, color, and `/people` routing
- Content index populated with transcripts and notes as timeline sources for entity intelligence enrichment

### Actions

- Proposed actions triage: accept/reject flow on Actions page and Daily Briefing — transcript-sourced actions default to "proposed" status with auto-archive hygiene for stale proposals

### Fixed

- Migration blocked by foreign key constraints — resolved with `PRAGMA foreign_keys = OFF`
- Stale column reference in meeting context SQL after schema migration

## [0.9.1] - 2026-02-18

Hotfix for MCP integrations failing when app is launched from Finder/Applications.

### Fixed

- Quill, Clay, and Gravatar MCP clients fail with "connection failed" when launched from Finder — macOS GUI apps don't inherit shell PATH. Added intelligent binary resolution that scans nvm versions, Homebrew, and system paths with process-lifetime caching.

## [0.9.0] - 2026-02-18

The integrations release. Four new data integrations, a plugin marketplace, and UI polish.

### Integrations

- Granola integration: background poller syncs meeting transcripts from Granola's local cache, matches to calendar events by time window and attendee overlap, writes to entity Meeting-Notes directories
- Gravatar integration: MCP-based avatar and profile enrichment with local image caching, background poller for stale email refresh
- Clay integration: MCP client for contact and company enrichment — title, company, LinkedIn, Twitter, phone, bio, industry, HQ, company size. Signal detection for job changes, funding rounds, and leadership transitions. Background poller with bulk enrich wake signal
- Plugin Marketplace: two Claude Code plugins (`dailyos` with 9 commands + 9 skills, `dailyos-writer` with 4 commands + 11 skills) bundled as installable zips with Settings UI for export
- Person schema extended with enrichment fields: LinkedIn URL, Twitter handle, phone, photo URL, bio, title history, company industry/size/HQ
- Avatar component for person images with Gravatar cache lookup and initials fallback
- Settings UI sections for Clay, Gravatar, and Granola configuration

### Fixed

- Unicode escape sequences rendering as literal text in JSX — replaced with actual Unicode characters across 16 frontend files
- Gravatar images showing as broken blue boxes — CSP updated for Tauri's asset protocol
- Avatar component falls back to initials on image load error
- Clay "Enrich All" button now wakes poller immediately instead of waiting for next 24-hour cycle

### Changed

- Person detail pages show LinkedIn and Twitter external links with arrow indicators

## [0.8.4] - 2026-02-17

Hotfix for Claude Desktop MCP integration.

### Fixed

- MCP server stdout pollution: native library output during embedding model init was corrupting the JSON-RPC stream. Fixed by redirecting stdout to stderr during init.

## [0.8.3] - 2026-02-17

Cleanup and hardening. Type safety, migration resilience, input validation, and AI prompt robustness.

### Fixed

- Entity type narrowed at source — removes band-aid cast, fixes entity picker for projects
- Transcript action extraction resolves `@Tag` to real account ID via case-insensitive lookup — fixes silent FK violations that dropped actions
- Path traversal guard added to prep path resolution
- Stale agenda overwrite when hiding attendees — agenda parameter now optional

### Changed

- Migrations hardened with `IF NOT EXISTS` for crash-recovery safety
- Input bounds on user agenda layer: max 50 items per list, 500 chars per string, UTF-8-safe truncation
- Transcript prompt handles null title/account gracefully instead of producing malformed prompts
- Folio bar transcript button shows spinner and `not-allowed` cursor when processing

## [0.8.2] - 2026-02-17

Polish sprint. Meeting intelligence redesigned as editorial briefing, audit trail for AI-generated data, person deduplication, and print-ready PDF export.

### Added

- Audit trail module for AI-generated data — tracks provenance through the enrichment pipeline
- Person email aliasing and cross-domain deduplication — merges duplicate contacts across domains
- Meeting Intelligence Report redesigned as a full editorial briefing with outcomes pinned to top
- Transcript attach button added to folio bar on all meetings
- Print styles for clean briefing PDF output — `Cmd+P` produces a readable document
- Claude Code skill templates distributed to user workspaces for slash-command workflows
- "+ Business Unit" button on account detail folio bar
- Attendee RSVP status carried through the full calendar pipeline

### Changed

- Schedule cards show QuickContext instead of PrepGrid, with internal stakeholders filtered out
- Risk briefing Regenerate button moved to folio bar; byline is now click-to-edit
- Featured meeting remains visible in the schedule list
- Prep summaries hydrated from entity intelligence fields for richer meeting context
- Meeting entity chips use optimistic local state for instant feedback

### Fixed

- MCP sidecar binary missing executable permission after build
- Meeting card padding and prep summary hydration from prep files

## [0.8.1] - 2026-02-16

Hardening release. Security, database integrity, token optimization, and proposed actions workflow.

### Security

- Prompt injection hardening: all 7 PTY enrichment sites now wrap untrusted data in `<user_data>` XML blocks
- Output size limits: capped all parsed AI arrays (20 risks, 50 actions, 10 wins, 20 stakeholders, 10 value items) to prevent unbounded growth

### Database

- Foreign key constraints added to actions, account_team, and account_domains via table recreation migration with FK enforcement at connection level
- Fixed panic in focus capacity during DST spring-forward gaps — new timezone-aware datetime resolver handles all chrono edge cases

### Token Optimization

- Entity intelligence prompts filtered by vector search relevance — context budget capped at 10KB (down from ~25KB), mandatory files always included
- Entity intelligence output switched from pipe-delimited to JSON format with backwards-compatible fallback parser

### Actions

- Proposed actions workflow: AI-extracted actions now insert as "proposed" status with accept/reject UX, "AI Suggested" badge, and 7-day auto-archive via scheduler

### Performance

- Intelligence queue memory pruned every 60s to prevent unbounded growth
- Dashboard DB reads consolidated into single lock acquisition, reducing lock contention

### Stats

- 688 tests passing, 0 clippy warnings

## [0.8.0] - 2026-02-16

The editorial release. Every page redesigned as a magazine-style document you read top to bottom. New typography, new color system, new layout engine. Plus semantic search, MCP integration, and security hardening.

### Editorial Design

- Complete visual overhaul: every page now renders as a magazine-style editorial document with chapter-based navigation
- New typography: Newsreader (serif body) and Montserrat (sans headings) replace the previous system fonts
- New color palette: 14 material-named colors across four families (Paper, Desk, Spice, Garden) replace generic tokens
- Magazine shell layout with atmosphere layer, floating navigation island, and folio bar replaces the sidebar
- Daily briefing reimagined: hero headline, focus block, featured meeting with full prep, schedule rows, tapering priorities — read top to bottom, then you're briefed
- Briefing refresh button shows live workflow progress (Preparing / AI Processing / Delivering) instead of silent wait
- Email visibility: briefing falls through to medium-priority emails when no high-priority exist, with contextual section labels
- Account, project, and person detail pages rebuilt as 7-chapter editorial narratives with shared layout template
- Meeting detail page redesigned with editorial treatment
- Action detail page redesigned with editorial treatment
- Emails, Inbox, History, and Settings pages moved into magazine shell
- Focus capacity and action prioritization folded directly into the daily briefing
- Week page editorial polish with folio bar integration
- Shared editorial components: ChapterHeading, FinisMarker, PullQuote, StateBlock, TimelineEntry, WatchItem, EditableText
- Asterisk brand mark integrated into navigation

### Risk Briefing

- Executive risk briefing redesigned as a 6-slide presentation (Cover, Bottom Line, What Happened, The Stakes, The Plan, The Ask) — each slide fills the viewport with scroll-snap navigation
- Keyboard shortcuts: keys 1-6 jump to slides, arrow keys navigate
- All text fields are click-to-edit — fix names, titles, or facts before presenting, changes auto-save silently to disk
- Tighter AI output: hard word limits prevent verbose slides, health arc rendered as color-coded timeline bars

### Semantic Search

- Local embedding model (nomic-embed-text-v1.5) for semantic vector search over entity content — downloads automatically on first launch, works offline after that
- Background embedding processor: entity files are chunked and embedded automatically as they change
- Hybrid search combining vector similarity (70%) and keyword matching (30%) for best-of-both retrieval
- Semantic search integrated into entity intelligence enrichment — AI now finds relevant historical content instead of relying on recency alone

### MCP Server

- Chat tools for querying entities, searching content, and retrieving briefings via external AI assistants (Claude Desktop via MCP)
- Semantic search tool (`search_content`) exposes hybrid vector+keyword search to Claude Desktop — ask about specific details in workspace files
- Chat session persistence — conversations are remembered across sessions
- Managed CLAUDE.md and settings written to workspace for Claude Desktop discovery

### Security

- Content Security Policy (CSP) enforced on the webview — restricts script, style, image, and connection sources to the app itself
- `reveal_in_finder` command validates paths against workspace and config directories before opening Finder — prevents arbitrary filesystem traversal
- `copy_to_inbox` command restricts source paths to Documents, Desktop, and Downloads — prevents copying from arbitrary filesystem locations

### Reliability

- Database renamed from `actions.db` to `dailyos.db` with automatic migration and WAL checkpoint
- Embedding model initializes asynchronously in the background — app window appears immediately instead of blocking during the 137MB model download
- Database migration framework tolerates duplicate-column errors for safe re-application
- Database indexes added for meeting-entity lookups, calendar event deduplication, and action filtering — faster page loads as data grows
- Removed unused frontend dependencies (lighter install, smaller attack surface)
- Dev database isolation: pattern-based purge, config backup, no Keychain writes in dev mode
- Apple notarization re-enabled in CI release pipeline

## [0.7.5] - 2026-02-14

### Fixed

- All AI enrichment calls (email, briefing, prep, week, entity intelligence, transcript, inbox) hardened against PTY output corruption: TERM=dumb suppresses escape codes, 4096-column width prevents hard line wrapping, ANSI stripping as safety net
- Debug logging of raw Claude output for all enrichment calls — parse failures now include the first 500 bytes for diagnosis
- Email enrichment "No enrichments parsed" caused by ANSI escape codes corrupting structured markers

## [0.7.4] - 2026-02-14

### Fixed

- Claude Code CLI not found when app is launched from Finder — the app now resolves the binary from common install locations (`~/.local/bin`, `/usr/local/bin`, `/opt/homebrew/bin`) instead of relying on shell PATH
- Email retry clearing the error banner without verifying enrichment succeeded — the banner now stays visible if enrichment fails during a retry, instead of falsely reporting success

## [0.7.3] - 2026-02-13

647 Rust tests. 71 Architecture Decision Records. First release with auto-updater.

### Proactive Intelligence

- Weekly briefing with AI narrative, priority synthesis, and readiness assessment
- Live proactive suggestions during workflow execution with progress stepper
- Email signal extraction: timeline events, risks, expansion signals, escalations linked to entities
- Email signals displayed on entity detail pages with signal-type badges and relative dates
- Agenda draft dialog for pre-meeting preparation with AI-generated starter content

### Entity Management

- Internal team setup: create your organization with root account, team, colleagues, and domain auto-linking
- Parent-child account hierarchy with directory scaffolding and domain inheritance
- Account team management: link people to accounts with roles (CSM, TAM, executive sponsor, etc.)
- Bulk person creation form for onboarding flows
- Entity picker filters archived entities from queries
- Account domains tracked in dedicated junction table with N+1 query elimination (single JOIN)

### Onboarding

- Internal Team Setup chapter: configure company, team, colleagues, and domains during onboarding
- Prime Briefing chapter: trigger first briefing from onboarding wizard
- Onboarding flow enhanced with demo data and educational content

### Personality System

- Configurable personality for AI copy across empty states and notifications
- PersonalityProvider context with 5 personality options (Professional, Friendly, Playful, Zen, Direct)
- SectionEmpty and InlineEmpty shared components replace ad-hoc empty states across all pages
- PersonalityCard and UserProfileCard in Settings

### Settings & Security

- Settings tabs with deep-link support (`/settings?tab=...`) for Profile, Integrations, Workflows, Intelligence, Hygiene, and Diagnostics
- Intelligence Hygiene status API + manual scan with gap-specific actions
- OAuth failure event (`google-auth-failed`) surfaces real auth errors without hanging
- OAuth hardened with PKCE S256 challenge + state parameter validation
- Removed hardcoded Google OAuth `client_secret` from source; loaded via `option_env!`
- CI guard to fail builds when committed OAuth secret patterns are detected

### Reliability

- Schema migration framework: numbered SQL migrations, pre-migration backup, forward-compat guard, bootstrap for existing databases
- Transaction wrapper on `create_internal_organization` — atomic multi-record creation with rollback on failure
- Race guard on WeekPage polling — prevents overlapping IPC calls during workflow execution
- Email validation on person creation
- WebKit date compatibility: `parseDate` utility handles Safari's strict timestamp parsing
- PTY subprocess strips Claude Code env vars to prevent nested session detection
- Stale capacity warning suppressed when briefing schedule is from today
- Transcript attachment error visibility improved
- Workflow delivery history tracks explicit failure phase and retry metadata

### Auto-Updater

- Tauri updater plugin with Minisign signature verification
- "Check for Updates" UI in Settings with download + relaunch flow
- Release pipeline generates signed `latest.json` for update endpoint
- Code signing and notarization in CI release workflow

### Infrastructure

- Shared `helpers.rs` module: `normalize_key`, `normalize_domains`, `build_external_account_hints` (DRY consolidation)
- Centralized date formatters in `utils.ts`: `formatRelativeDateLong`, `formatBidirectionalDate`, `formatDayTime`, `formatShortDate`
- Meeting context preparation extracted into dedicated module (`prepare/meeting_context.rs`)
- 5 SQL migrations: baseline, internal_teams, account_team, account_team_role_index, email_signals
- Focus page: isolated refresh command, P1 action cap, agenda anchored to calendar notes
- Proactive intelligence query layer (`queries/proactive.rs`)

### Fixed

- "View All Actions" count now reflects P1 actions only
- Hygiene system: NaN bug, manual scan trigger, detail breakdown
- `latest.json` generation handles multiline Minisign signatures correctly
- Clippy warnings resolved for CI (`-D warnings` enforcement)

## [0.7.2] - 2026-02-12

### Fixed

- OAuth token exchange restored: client_secret was stripped during PKCE migration but Google Desktop App clients still require it — every auth attempt was returning 400 Bad Request
- OAuth callback no longer shows "Authorization successful" before the token exchange completes — browser now waits for the full exchange + Keychain save, and shows the actual error on failure
- Token refresh no longer strips client_secret from saved tokens, preventing refresh failures after the initial hour
- Added diagnostic logging at every step of the OAuth flow for troubleshooting

## [0.7.1] - 2026-02-12

Six sprints of work across meeting intelligence, entity relationships, security hardening, and app responsiveness. 574 Rust tests. 69 Architecture Decision Records.

### Meeting Intelligence

- Meeting prep redesigned as a report: executive brief hero, agenda-first flow, right-rail navigation, appendix-style deep context
- Agenda and Wins are now semantically separate enrichment blocks with structured source provenance (replaces flat talking points)
- User-authored prep fields (`userAgenda`, `userNotes`) are DB-authoritative with freeze/editability rules
- Meeting identity hardened: calendar event ID is canonical primary key across poller, reconcile, and DB
- Unified meeting intelligence contract (`get_meeting_intelligence`) combines prep, outcomes, and transcript metadata in a single backend call
- Enriched prep context persisted to `meetings_history` for durable post-meeting records
- Meeting search across entities via Cmd+K command menu with debounced cross-entity lookup
- Calendar description pipeline extracted and exposed in prep as `calendarNotes`
- Account snapshot enrichment with compact, sanitized prep rendering
- People-aware prep support for internal meeting types
- Immutable prep snapshots written to entity `Meeting-Notes/` during archive

### Entity Relationships

- Person-entity auto-linking via meeting attendance with full cascade
- Multi-entity MeetingCard: add/remove entity associations with people + intelligence queue cascade
- Multi-domain user configuration with tag/chip input UX, auto-reclassification of people and meeting types on domain change
- Entity archive/unarchive with parent cascade (DB flag only, filesystem untouched)
- Strategic programs inline editing on AccountDetailPage with debounced auto-save
- People merge and delete with full cascade across attendees, entities, actions, intelligence, and filesystem

### Focus & Capacity

- Focus page redesigned with live capacity engine computing from calendar events
- Deterministic action prioritization with urgency/impact scoring, top-3 recommendations, risk radar
- Focus capacity computes from live calendar, schedule artifact retained for briefing narrative only

### Security & Auth

- OAuth hardened with PKCE (`S256`) challenge + state parameter validation
- macOS Keychain token storage with one-time legacy file migration and removal
- Secretless token exchange and refresh with compatibility fallback for legacy clients
- IPC input validation DTOs for action create/update with centralized validators
- CI gates: `cargo clippy -D warnings`, `cargo audit`, `pnpm audit` enforced on every build

### Email

- Email sync status tracking with structured health metadata on `emails.json`
- Sticky sync banner with retry affordance when fetch or delivery fails
- Model fallback: email enrichment retries with synthesis model when extraction model unavailable
- Last-known-good email preservation on delivery failures

### Reliability & Performance

- App responsiveness: `check_claude_status` moved to async with `spawn_blocking`, background tasks open own SQLite connections instead of competing for shared Mutex
- Google API retry policy with exponential backoff wired into auth, calendar, and Gmail
- Resume latency instrumentation with p50/p95/max rollups and budget violation tracking
- Split-lock enrichment pattern with `nice -n 10` PTY execution for background AI operations
- Archive lifecycle reordered: reconciliation and prep freezing happen before `_today/data` cleanup
- Claude auth check timeout reduced from 8s to 3s, focus debounce intervals increased

### AI Operations

- Model tiering for AI operations: Synthesis/Extraction/Mechanical tiers with configurable model names per tier
- Prep enrichment contract splits Agenda and Wins parsing with separate blocks and source governance
- One-time migration command `backfill_prep_semantics` for upgrading existing prep files

### UX & Polish

- Frontend meeting routes consolidated to canonical `/meeting/$meetingId` with history route as redirect
- Theme toggle fixed: replaced broken dropdown (Radix dual-install issue) with segmented button group
- Radix UI components migrated to explicit standalone packages, resolving dual-install portal bug
- Calendar poller polls immediately on startup (5s auth delay) instead of sleeping first
- Empty prep page shows "generating" message instead of blank
- Binary size and bundle measurement scripts for repeatable performance tracking

## [0.7.0] - 2026-02-09

### Added

- Native desktop app (Tauri v2) -- complete rewrite from CLI
- Daily briefing with AI-enriched meeting prep
- Account intelligence -- executive assessments, risks, wins, stakeholder insights
- Project intelligence -- status tracking, content indexing
- People tracking -- relationship history, meeting patterns, auto-created from calendar
- Meeting-entity relationship graph with manual reassignment
- Email triage with three-tier AI priority classification
- Action tracking from briefings, transcripts, inbox, and manual creation
- Transcript processing with outcome extraction (actions, captures, decisions)
- Entity directory template (Call-Transcripts, Meeting-Notes, Documents)
- Proactive intelligence maintenance (hygiene scanner, pre-meeting refresh)
- Week page with AI narrative and priority synthesis
- Focus page with gap analysis
- Inbox processing with file classification and routing
- Onboarding wizard with Google OAuth integration
- Production Google OAuth credentials (no user-supplied credentials.json needed)
- Background scheduling (daily briefing, archive reconciliation, intelligence refresh)
- 500 Rust backend tests
- 59 Architecture Decision Records

### Changed

- CLI archived to `_archive/dailyos/`
- Python runtime eliminated -- all operations now in Rust
- Config directory: `~/.dailyos/` (was `~/.daybreak/`)

### Removed

- Python Phase 1/Phase 3 scripts (replaced by Rust-native Google API client)
- CLI commands (/today, /week, /wrap) -- replaced by app UI
