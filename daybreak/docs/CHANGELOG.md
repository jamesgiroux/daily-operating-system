# Changelog

Closed issues organized by sprint. For active work, see [BACKLOG.md](./BACKLOG.md).

---

## Sprint 16 — Meeting Permanence + Identity Hardening

*Prelaunch refactor for durable meeting records, stable identity, and unified current/historical detail access.*

### Lifecycle + Durability
- **ADR-0065 completion:** User-authored prep fields (`userAgenda`, `userNotes`) are now DB-authoritative in `meetings_history` and protected by freeze/past editability rules.
- **Archive freeze ordering:** archive executor now persists and freezes meetings before `_today/data` cleanup, preventing prep loss.
- **Immutable snapshots:** archive path writes one frozen prep snapshot per meeting to entity `Meeting-Notes/` (fallback `_archive/meetings/YYYY/MM/`), with hash/path/frozen metadata persisted in SQLite.

### Identity + Contract
- **ADR-0066 completion:** Added unified `get_meeting_intelligence(meeting_id)` backend contract and converted `get_meeting_prep` into compatibility wrapper mode.
- **Meeting identity normalization:** event ID (sanitized) is now the canonical meeting key across poller, DB persistence, reviewed state, and dependent references (`captures`, `meeting_entities`, `meeting_attendees`, transcript actions).
- **Single meeting route behavior:** frontend now resolves historical/current meeting detail through canonical `/meeting/$meetingId`; `/meeting/history/$meetingId` now redirects.

### Focus/Capacity
- **I178:** Closed. Focus available blocks now compute from live calendar events with schedule `startIso` fallback only when live events are unavailable (ADR-0062 completion).
- **I179:** Closed. Focus now ranks pending/waiting actions deterministically with urgency and feasibility scoring, surfaces top 3 recommendations, and flags at-risk actions.

### Security/Auth Hardening
- **I158:** OAuth hardening completed with PKCE (`S256`) + state validation, macOS Keychain token storage (legacy `~/.dailyos/google/token.json` one-time migration + removal), and secretless default token exchange/refresh paths with compatibility fallback for legacy clients.

### Outcomes
- Outcomes retrieval no longer requires transcript-record file state; DB transcript metadata + captures/actions now drive outcomes durability.

---

## Sprint 15 — Meeting Intelligence Report

*Report-grade prep UX and semantic cleanup built on Sprint 14 foundation.*

### Meeting Prep Experience
<a name="i187"></a>- **I187:** Prep page three-tier layout delivered on `MeetingDetailPage` with executive brief hero, agenda-first flow, deep-context appendix, and report-style visual hierarchy.
<a name="i189"></a>- **I189:** Meeting prep editability shipped: user agenda + notes persisted and editable from prep detail with future-meeting guardrails.
<a name="i191"></a>- **I191:** Card-detail unification landed: prep/outcomes flow moved toward a unified meeting record presentation.
<a name="i194"></a>- **I194:** User agenda and notes edits persist immediately to meeting prep JSON via new Tauri commands (`update_meeting_user_agenda`/`update_meeting_user_notes`), and the UI surfaces inline edit controls plus save/failure states (`ADR-0065`).
<a name="i195"></a>- **I195:** Meeting outcomes now render inside the prep/outcomes section (depending on `MeetingOutcomes`/`MeetingDetailPage` wiring) so outcomes surface at the top even when post-meeting captures arrive earlier (`ADR-0066`).

### Prep Semantics
- **I196:** Prep agenda/wins semantic split + source governance completed. `recentWins` and `recentWinSources` added as first-class prep fields (additive/backward compatible with `talkingPoints`). Enrichment parser now supports distinct `AGENDA` and `WINS` blocks, strips inline `source:` tails from display text, and persists source provenance structurally. Mechanical agenda generation now prioritizes open items/risks/questions and only falls back to wins when needed. Added one-time migration command `backfill_prep_semantics(dry_run)` to upgrade `_today/data/preps/*.json` and `meetings_history.prep_context_json`.

### Backlog & ADR Alignment
- **I95:** Week proactive suggestions scope split into three executable tracks in `BACKLOG.md`: `I200` (Week UI rendering from week artifact), `I201` (live proactive suggestions via ADR-0062 query boundary), and `I202` (prep prefill/draft agenda actions aligned with ADR-0065 additive edit model). ADR-0052 now includes a dated alignment note recording shipped vs remaining Phase 3 scope.

### Runtime Reliability
- **I197:** Resume responsiveness hardening completed. Added in-memory command latency rollups (`p50`/`p95`/max, budget violations, degraded counters) via `get_latency_rollups` + devtools panel, expanded instrumentation for startup/resume-sensitive commands, and standardized hot-path DB access with `AppState` helper methods (`with_db_try_read`/`with_db_read`/`with_db_write`) plus staged split-lock migration guidance (ADR-0067).

---

## Sprint 14 — Meeting Intelligence Foundation

*Calendar/plumbing reliability work that unblocked the report redesign.*

### Closed
- **I177:** Email sync reliability completed end-to-end. `emails.json` now carries structured `sync` health metadata, fetch/delivery failures preserve last-known-good email lists, dashboard surfaces persistent email sync state, and manual refresh now returns blocking failures with explicit retry. Email enrichment retries once with synthesis model when extraction model is unavailable.
- **I173:** Enrichment responsiveness fixed with split-lock enrichment path reuse and `nice -n 10` PTY execution support.
- **I185:** Calendar description pipeline completed end-to-end and exposed in prep as `calendarNotes`.
- **I186:** Account snapshot enrichment completed with compact prep snapshot rendering and sanitization.
- **I190:** Meeting route migration completed (`/meeting/$meetingId`) with DB/disk fallback prep loading.
- **I159:** People-aware prep support for internal meeting types added via person-prep eligibility path.

---

## Sprint 13 — Entity Relationships & Domain Intelligence

*Auto-linking people to entities via meetings. Multi-entity MeetingCard. Multi-domain reclassification. Theme toggle fix. Entity archive/unarchive. Strategic programs UI.*

### Entity Relationships (I184)
- **I184:** Person-entity auto-linking via meeting attendance. `cascade_meeting_entity_to_people()` links external attendees to the meeting's entity (idempotent INSERT OR IGNORE). Multi-entity MeetingCard — `add_meeting_entity` / `remove_meeting_entity` commands with full cascade (people, intelligence queue, legacy account_id). Entity chips with X to unlink, EntityPicker always available for adding more. Organization field on PersonDetailPage/PeoplePage replaced with linked account entity names (clickable links to account detail). `account_names` via GROUP_CONCAT subquery on PersonListItem. 6 new Rust tests.

### Domain Intelligence (I171)
- **I171:** Multi-domain user config — tag/chip input UX on SettingsPage (comma/Enter/Tab adds domain, X removes, Backspace deletes last, auto-save). `reclassify_people_for_domains()` re-derives internal/external relationship from email domain. `reclassify_meeting_types_from_attendees()` updates meeting types when attendee relationships change (preserves title-based types like QBR, training, all_hands). Runs on every domain config change.

### Entity Management
- **I176:** Entity archive/unarchive — `archived INTEGER DEFAULT 0` on accounts/projects/people. Archive commands with parent cascade. Archived tabs on list pages. Archive button + unarchive banner on detail pages. DB flag only, filesystem untouched.
- **I163:** Strategic programs edit UI — inline-editable ProgramRow component on AccountDetailPage. Name input, status dropdown (Active/Planning/On Hold/Complete), notes field, delete button. Debounced auto-save via `update_account_programs`.

### UX & Polish
- **I156:** Theme toggle fixed — replaced broken DropdownMenu (radix-ui dual-install issue, ADR-0060) with segmented button group (Light / Dark / System). No more dropdown portal disconnect.

### Architecture
- **I180:** Resolved by ADR-0062 (briefing artifacts vs. live queries). schedule.json stays as briefing document. Time-aware features compute from live layer via `src-tauri/src/queries/`.

---

## Sprint 12 — Meeting Intelligence Persistence

*Enriched prep context persisted to history. Meeting search across entities.*

### Meeting History & Persistence
- **I181:** Persist enriched meeting context as durable record. `prep_context_json TEXT` column in `meetings_history` (auto-migrated). `persist_meetings()` reads prep files during reconciliation, validates substantiveness, stores with COALESCE to avoid overwrites. `PrepContext` struct + `PrepContextCard` component render agenda, talking points, risks, stakeholder insights, and open items on MeetingHistoryDetailPage.
- **I183:** Meeting search — cross-entity historical lookup. `search_meetings` Tauri command with SQL LIKE over title, summary, prep_context_json (LIMIT 50). CommandMenu (Cmd+K) wired with debounced search (250ms, min 2 chars) + navigation to meeting detail. Also fixed CommandMenu nav items to actually route.
- **I182:** Wire daily preps to consume entity intelligence — already delivered in Sprint 9 (I135). `inject_entity_intelligence()` reads intelligence.json, `entityReadiness`, `intelligenceSummary`, `entityRisks`, `stakeholderInsights` flow into enriched preps.

---

## Sprint 11 — Meeting Identity + Prep Reliability

*Calendar event IDs become the canonical meeting key. Prep detection fixed. People merge shipped.*

### Meeting Identity
- **I165:** Calendar event ID as meeting primary key (ADR-0061). `meeting_primary_id()` prefers Google Calendar event ID, falls back to slug. Prep + schedule use same ID function.
- **I168:** Account resolution fallback — junction table lookup + attendee inference when `guess_account_name` fails.
- **I160:** Calendar-to-people sync — `populate_people_from_events()` records attendance on every poll. Meeting counts, last_seen, temperature/trend signals now work.

### Prep Quality
- **I166:** Empty prep page fix — `is_substantive_prep()` checks for real content, `reconcile_prep_flags()` updates schedule.json. Frontend shows "generating" message instead of blank.

### Operations
- **I174:** Model tiering for AI operations — `ModelTier` enum (Synthesis/Extraction/Mechanical), `AiModelConfig` with serde defaults (sonnet/sonnet/haiku), `PtyManager::for_tier()`, Settings UI.
- **I170:** People merge + delete — full cascade (attendees, entities, actions, intelligence), filesystem cleanup, PersonDetailPage merge/delete UI, AlertDialog component. Phase 1 of merge/dedup.

### Polish
- **I167:** Calendar poller polls immediately on startup (5s auth delay) instead of sleeping first.
- **I169:** People page refresh button spins during fetch, disables to prevent double-clicks.

---

## Sprint 10 — Entity Intelligence Architecture

*Nine-phase intelligence pipeline. Proactive maintenance. Content indexing. ADR-0057/0058.*

### Entity Intelligence Pipeline (ADR-0057)
- **I130:** intelligence.json schema, `entity_intelligence` DB table, TypeScript types. Foundation.
- **I131:** Full enrichment engine — context builder, entity-parameterized prompt (initial + incremental), structured parser, PTY orchestrator.
- **I132:** IntelligenceQueue with priority-based dedup, debounce, background processor. Watcher + inbox pipeline integration.
- **I133:** AccountDetailPage intelligence-first redesign — executive assessment, attention items, meeting readiness, stakeholder intelligence, evidence history.
- **I134:** Shared `format_intelligence_markdown()` in entity_intel.rs. Accounts, projects, and people share markdown generation.
- **I135:** meeting_context.rs reads intelligence.json for prep. Calendar-triggered readiness refresh.
- **I136:** People intelligence enrichment from SQLite signals (meetings, entity connections, captures).
- **I137:** Briefing + weekly enrichment prompts include cached entity intelligence. Brief DB lock pattern (microsecond read, release before PTY).
- **I138:** Project content indexing delegates to shared `sync_content_index_for_entity()`. Watcher integration.

### Proactive Intelligence Maintenance (ADR-0058)
- **I145:** Hygiene scanner — gap detection, mechanical fixes (reclassification, orphan linking, meeting recount, file summary backfill), 4-hour background cycle. 19 tests.
- **I146:** Email display name extraction, auto-link people by domain, AI-budgeted gap filling with daily reset.
- **I147:** Pre-meeting intelligence refresh (2h window, 7d staleness). Overnight batch with expanded AI budget.
- **I148:** `get_hygiene_report` command. System Health card on SettingsPage.

### Content Indexing
- **I124:** `content_index` table, recursive directory scanner, startup sync, `get_entity_files` + `reveal_in_finder` commands, Files card on AccountDetailPage. 409 tests.
- **I125:** `AccountContent` watch source, debounced content change events, "new files detected" banner.
- **I126:** Superseded by I130. Basic `build_file_context()` delivered with I124.

### Entity Pages & CRUD
- **I50:** Projects as first-class entities — overlay table, CRUD, ProjectsPage + ProjectDetailPage.
- **I52:** Meeting-entity M2M junction table. Auto-association from attendee domains. EntityPicker on MeetingCard. Entity directory template (ADR-0059).
- **I129:** People editability — editable names, account linking, manual creation, promoted notes.
- **I127:** `create_action` with full field support. `useActions` hook. Inline "Add action" on ActionsPage + AccountDetailPage.
- **I128:** `update_action` with partial-field updates. ActionDetailPage with click-to-edit fields.

### Other
- **I94:** Week AI enrichment — `weekNarrative` + `topPriority` fields. ADR-0052.
- **I119:** Gmail header extraction works correctly in Rust-native `gmail.rs`.
- **I123:** Production Google OAuth credentials embedded. DailyOS Google Cloud project.
- **I139:** File summary extraction via hygiene scanner backfill.
- **I144:** `archive_emails` via Gmail `batchModify` API. "Archive all" button on FYI section.

---

## Sprint 9 — Entity Relationship Graph

*Accounts, projects, and people become connected entities with M2M relationships.*

- **I50:** Projects as first-class entities.
- **I52:** Meeting-entity M2M junction table + EntityPicker on MeetingCard.
- **I129:** People entity editability.

---

## Sprint 8 — Python Elimination (ADR-0049)

*All Google API calls ported to Rust via `reqwest`. Python runtime removed entirely.*

- **I83:** Rust-native Google API client (`google_api/` module).
- **I84:** Phase 1 operations ported to Rust (`prepare/` module).
- **I85:** Orchestrators ported, `scripts/` directory deleted.
- **I91:** Universal file extraction (`processor/extract.rs`). ADR-0050.

---

## Sprint 7 — UX Redesign

*Schedule-first dashboard (ADR-0055), list page redesign (ADR-0054), focus page.*

### Dashboard
<a name="i97"></a>
- **I97:** Readiness strip (later removed by ADR-0055).
<a name="i98"></a>
- **I98:** Action/email sidebar order flipped.
<a name="i99"></a>
- **I99:** Greeting removed, Focus promoted.
<a name="i100"></a>
- **I100:** ActionList maxVisible 3 → 5.
<a name="i101"></a>
- **I101:** Full-width summary (later superseded by ADR-0055 two-column).
<a name="i109"></a>
- **I109:** Focus page — `get_focus_data` from schedule.json + SQLite + gap analysis.
- **I111:** Dashboard visual rhythm — removed chrome, tapered spacing, breathing room.
- **I112:** Graceful empty state — missing schedule.json returns `Empty` not `Error`.

### List Pages (ADR-0054)
- **I102:** Shared `ListRow` + `ListColumn` primitives.
- **I103:** AccountsPage flat rows with health dot.
- **I104:** PeoplePage flat rows with temperature + trend.
- **I105:** PeoplePage shared component consolidation (SearchInput, TabFilter).
- **I106:** `PersonListItem` struct + batch `get_people_with_signals()` query.
- **I107:** Action detail page at `/actions/$actionId`.

### Week Page (ADR-0052)
- **I93:** Week page mechanical redesign — consumption-first layout.
- **I96:** Week planning wizard retired.

### Entity Hierarchy (ADR-0056)
- **I113:** Workspace transition detection. Auto-scaffold, skip `_`-prefixed folders.
- **I114:** Parent-child accounts — `parent_id` FK, expandable rows, breadcrumb, rollup.
<a name="i116"></a>
- **I116:** ActionsPage account name resolution via `ActionListItem`.
<a name="i117"></a>
- **I117:** `guess_account_name()` discovers child BU directories.
- **I118:** Timezone formatting in `deliver.rs`, `orchestrate.rs`, `calendar_merge.rs`.

---

## Sprint 6 — Account Pages & Enrichment

*Entity dashboard system, enrichment pipeline, account detail pages.*

- **I72:** AccountsPage + AccountDetailPage. 6 Tauri commands.
- **I73:** Entity dashboard template system, two-file pattern, three-way sync. ADR-0047.
- **I74:** Account enrichment via Claude Code websearch.
- **I75:** Entity dashboard external edit detection via watcher.
- **I76:** SQLite backup + rebuild-from-filesystem.
- **I77:** Filesystem writeback audit.
- **I78:** Onboarding: inbox-first behavior chapter.
- **I79:** Onboarding: Claude Code validation chapter.
- **I80:** Proposed Agenda in meeting prep.
- **I81:** People dynamics in meeting prep UI.
- **I82:** Copy-to-clipboard for meeting prep.

---

## Sprint 5 — Onboarding & Security Hardening

*Educational onboarding flow, atomic writes, security audit.*

### Onboarding
- **I56:** Onboarding redesign — 9-chapter educational flow.
- **I57:** Onboarding: populate workspace before first briefing.
- **I58:** User profile context in AI enrichment prompts.

### Security & Robustness
- **I59:** Script path resolution for production builds.
- **I60:** Path traversal validation in inbox/workspace.
- **I61:** TOCTOU sentinel for transcript immutability.
- **I62:** `.unwrap()` panics replaced with graceful handling.
- **I63:** Python script timeout handling.
- **I64:** Atomic file writes via `atomic_write_str()`.
- **I65:** Impact log append safety.
- **I66:** Safe prep delivery (write-first, then remove stale).
- **I67:** Scheduler boundary widened 60 → 120 seconds.
- **I68:** `Mutex<T>` → `RwLock<T>` for read-heavy AppState.
- **I69:** File router duplicate destination handling.
- **I70:** `sanitize_for_filesystem()` strips unsafe characters.
- **I71:** Low-severity edge hardening (9 items).

---

## Sprint 4 — Workflow & Intelligence

*Archive reconciliation, executive intelligence, entity abstraction.*

- **I34:** Archive reconciliation (`workflow/reconcile.rs`).
- **I36:** Daily impact rollup (`workflow/impact_rollup.rs`).
<a name="i37"></a>
- **I37:** Density-aware dashboard overview.
<a name="i38"></a>
- **I38:** Rust-native delivery + AI enrichment ops. ADR-0042.
- **I39:** Feature toggle runtime with `is_feature_enabled()`.
<a name="i41"></a>
- **I41:** Reactive meeting:prep wiring via calendar poller.
<a name="i42"></a>
- **I42:** CoS executive intelligence layer (`intelligence.rs`).
<a name="i43"></a>
- **I43:** Stakeholder context in meeting prep.
<a name="i44"></a>
- **I44:** Meeting-scoped transcript intake. ADR-0044.
<a name="i45"></a>
- **I45:** Post-transcript outcome interaction UI.
- **I46:** Meeting prep context expanded beyond customer/QBR/training. ADR-0043.
- **I47:** Entity abstraction with `entities` table + bridge pattern. ADR-0045.
- **I48:** Workspace scaffolding on initialization.
- **I49:** Graceful degradation without Google auth.
- **I51:** People sub-entity — universal person tracking. 3 tables, 8 commands.

---

## Sprint 3 — Inbox Pipeline

*File processing, transcript extraction, account intelligence updates.*

<a name="i30"></a>
- **I30:** Inbox action extraction with rich metadata (`processor/metadata.rs`).
<a name="i31"></a>
- **I31:** Inbox transcript summarization with `detect_transcript()` heuristic.
- **I32:** Inbox processor updates account intelligence via WINS/RISKS extraction.
<a name="i33"></a>
- **I33:** Wins/risks resurface in meeting preps via 14-day lookback.

---

## Sprints 1-2 — Foundation

*Core app shell, data loading, pages, CI.*

- **I1:** Config directory renamed `.daybreak` → `.dailyos`.
- **I2:** JSON-first meeting cards render at multiple fidelity levels.
- **I4:** Superseded by I89 (personality system).
- **I5:** Focus, Week, Emails all have defined roles. ADR-0010.
- **I6:** Processing history page + `get_processing_history` command.
- **I7:** Settings workspace path change with directory picker.
- **I8:** GitHub Actions CI — unsigned arm64 DMG on tag push. Product website (daily-os.com).
- **I9:** Focus and Week pages fully implemented.
- **I10:** Closed, won't do. Types are the data model.
- **I11:** Email enrichment parsed and merged into `emails.json`.
- **I12:** Email page shows AI context per priority tier.
- **I13:** Onboarding wizard with 5-step flow.
<a name="i14"></a>
- **I14:** MeetingCard "View Prep" button.
- **I15:** Entity-mode switcher in Settings.
- **I16:** Schedule editing UI with human-readable cron.
<a name="i17"></a>
- **I17:** Non-briefing actions merge into dashboard.
<a name="i18"></a>
- **I18:** Google API credential caching.
- **I19:** "Limited prep" badge for AI enrichment failures.
- **I20:** Standalone email refresh.
- **I21:** Expanded FYI email classification.
- **I22:** Action completion writeback to source markdown.
- **I23:** Three-layer cross-briefing action deduplication.
<a name="i24"></a>
- **I24:** `calendarEventId` field alongside local slug.
- **I25:** `computeMeetingDisplayState()` unified badge rendering.
<a name="i29"></a>
- **I29:** Superseded by I73 template system + kit issues.
- **I120:** Closed, won't fix. Legacy action import — starting clean.
- **I121:** Closed, won't fix. Legacy prep generation — clean start.
