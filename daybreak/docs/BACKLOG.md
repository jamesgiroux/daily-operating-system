# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

---

## Issues

<!-- Sprint-oriented grouping (2026-02-08):

  TEST BED: ~/Documents/test-workspace/ — clean workspace for end-to-end validation.
  Every sprint milestone is tested here, not in VIP/. See ROADMAP.md for full sprint plan.

  COMPLETED:
    Sprint 1: "First Run to Working Briefing" — I48, I49, I7, I15, I16, I13. 155 tests.
    Sprint 2: "Make it Smarter" — I42, I43, I41, I31. 168 tests.
    Sprint 3: "Make it Reliable" — I39, I18, I20, I21, I37, I6. 176 tests.
    Sprint 4a: "Entity Intelligence" — I51 (people), I72+I73 (account dashboards),
      I59 (script paths), I56 (onboarding 80%), demo data expansion. 189 tests.
    Sprint 5: "Complete the App" — 199 tests.
      Track A (Onboarding): I56 finish, I57, I78, I79, I58 — all done.
      Track B (Security): I60, I62, I63 — done. I64, I65 — ~90% done (gaps carry to S6).
      Track C (Meeting Intelligence): I80, I81, I82 — done (pulled forward from S7).
    Sprint 6+7: "Harden & Enrich" (combined) — 224 tests.
      Track A (Safety): I61, I64, I65, I66, I67, I69, I70, I71 — all done.
      Track B (Polish): I19, I25 — done.
      Track C (Intelligence): I74 — done.
      Track D (Durability): I75, I76, I77 — done.
      Track E (Performance): I68 — done.

  ═══════════════════════════════════════════════════════════════════
  SPRINT 8: "Kill Python" — Eliminate Python runtime dependency
  ═══════════════════════════════════════════════════════════════════

    I83 (Rust-native Google API client — reqwest + OAuth2)
    I84 (Port Phase 1 operations — classification, email priority, etc.)
    I85 (Port orchestrators + delete Python scripts/)

    Done when: `scripts/` deleted, `run_python_script()` removed from pty.rs,
    no Python on $PATH required, all 224+ Rust tests still pass,
    onboarding no longer checks for Python.

  ═══════════════════════════════════════════════════════════════════
  SPRINT 9: "Distribute" — Ship to colleagues
  ═══════════════════════════════════════════════════════════════════

    I8 (DMG build + GitHub Actions CI + GitHub Releases)
    7-day crash-free validation on clean machine
    README for colleague installs

    Done when: DMG installs cleanly on a clean Mac (arm64),
    onboarding → first real briefing works end-to-end.
    No signing/notarization (no Apple Developer account — Gatekeeper
    bypass instructions in README). No updater (zero users, premature).

  ═══════════════════════════════════════════════════════════════════
  PARKING LOT (post-ship, needs real usage data)
  ═══════════════════════════════════════════════════════════════════

    Entity-mode architecture (ADR-0046, I27 umbrella):
      I50 (projects table), I52 (meeting-entity M2M), I53 (entity-mode config)
      I54 (MCP integration framework), I28 (MCP server + client)
      ~~I29 (non-entity structured document schemas)~~ — Closed, superseded by I73 + kit issues
    Kits: I40 (CS Kit)
    Intelligence: I35 (ProDev Intelligence), I55 (Executive Intelligence)
    Research: I26 (web search for unknown meetings)
    Low: I3, I10
    I86 (first-party integrations), I87 (in-app notifications),
    I88 (Weekly Wrapped), I89 (personality system), I90 (telemetry)
-->

### Open — Ship Blocker

**I8: DMG build + GitHub Actions CI + GitHub Releases**
Unsigned DMG for colleague distribution. GitHub Actions builds arm64 DMG on push/tag. GitHub Releases hosts the artifact. README with Gatekeeper bypass instructions (`xattr -cr`). No signing/notarization (no Apple Developer account). No updater (zero users, premature). Blocked by I85 (Python elimination must land first so DMG is self-contained).

**I83: Rust-native Google API client** — Sprint 8
New `google_api.rs` module. OAuth2 token storage/refresh using `reqwest` + `serde`. Localhost redirect server for initial auth flow (replace `google_auth.py`). Calendar v3 event listing (replace `calendar_poll.py` API calls). Gmail v1 message listing/fetching (replace `refresh_emails.py` API calls). ADR-0049.

**I84: Port Phase 1 operations to Rust** — Sprint 8, blocked by I83
Port `ops/` Python modules: meeting classification (`classify_meetings()`), email priority classification, action parsing from markdown, meeting prep context gathering. These are pure logic + SQLite + file I/O — Rust already does all of this elsewhere. ~1,200 lines of Python.

**I85: Port orchestrators and delete Python** — Sprint 8, blocked by I84
Port `prepare_today.py`, `prepare_week.py`, `deliver_week.py`, `refresh_emails.py`, `calendar_poll.py`, `prepare_meeting_prep.py` as Rust functions composed from I83 + I84 operations. Delete `scripts/` directory. Remove `run_python_script()` from `pty.rs`. Remove script resources from `tauri.conf.json`. Remove Python check from onboarding. ADR-0049.

### Open — Parking Lot (blocked by I27, post-ship)

**I27: Entity-mode architecture — umbrella issue**
ADR-0046 replaces profile-activated extensions (ADR-0026) with three-layer architecture: Core + Entity Mode + Integrations. Entity mode (account-based, project-based, or both) replaces profile as the organizing principle. Integrations (MCP data sources) are orthogonal to entity mode. Two overlay types: **Kits** (entity-mode-specific: CS Kit, Sales Kit) contribute fields + templates + vocabulary; **Intelligence** (entity-mode-agnostic: Executive, ProDev) contribute analytical perspective via enrichment prompt fragments. Sub-issues: I50 (projects table), I52 (meeting-entity M2M), I53 (entity-mode config/onboarding), I54 (MCP integration framework), I55 (Executive Intelligence). Current state: `entities` table and `accounts` overlay exist (ADR-0045), bridge pattern proven. Post-Sprint 4.

**I40: CS Kit — account-mode fields, templates, and vocabulary** — Blocked by I27
ADR-0046 replaces the CS extension with a CS Kit (entity-mode-specific overlay). Remaining CS-specific items: account fields (ARR, renewal dates, health scores, ring classification), dashboard templates, success plan templates, value driver categories, ring-based cadence thresholds. The existing `accounts` table IS the CS Kit's schema contribution. Kit also contributes enrichment prompt fragments for CS vocabulary. Reference: `~/Documents/VIP/.claude/skills/daily-csm/`.

**I50: Projects overlay table and project entity support** — Blocked by I27
ADR-0046 requires a `projects` overlay table parallel to `accounts`. Fields: id, name, status, milestone, owner, target_date. Bridge pattern: `upsert_project()` auto-mirrors to `entities` table. CRUD commands + Projects page (parallel to Accounts page).

**I52: Meeting-entity many-to-many association** — Blocked by I50
Replace `account_id` FK on `meetings_history`, `actions`, `captures` with `meeting_entities` junction table. Enables meetings to associate with multiple entities (an account AND a project).

**I53: Entity-mode config, onboarding, and UI adaptation** — Blocked by I50, I52
Replace `profile` config field with `entityMode` + `integrations` + `domainOverlay`. Update onboarding, sidebar, dashboard portfolio attention. Migration: `profile: "customer-success"` → `entityMode: "account"`.

**I54: MCP client integration framework** — Blocked by I27
Build MCP client infrastructure in Rust for consuming external data sources per ADR-0046 and ADR-0027. Start with one integration per category: one transcript source (Gong or Granola), one CRM (Salesforce), one task tool (Linear). Evolves I28 (MCP client side).

**I28: MCP server and client not implemented**
ADR-0027 accepts dual-mode MCP (server exposes workspace tools to Claude Desktop, client consumes Clay/Slack/Linear). ADR-0046 elevates MCP client to the integration protocol. No MCP protocol code exists. See I54 for client framework.

**I29: Structured document schemas not implemented** — Closed
Schema pattern delivered by I73 (entity dashboard template system). Remaining non-entity documents (success plans, QBR templates) belong to kit-specific issues (I40 CS Kit, etc.). Superseded.

**I35: ProDev Intelligence — personal impact capture and career narrative** — Blocked by I27
ADR-0046 classifies ProDev as an Intelligence layer (entity-mode-agnostic). Daily reflection, weekly narrative, quarterly rollup. Contributes enrichment prompt fragments. Reference: `/wrap` "Personal Impact" section, `/month`, `/quarter`.

**I55: Executive Intelligence — decision framing, delegation tracking, and strategic analysis** — Blocked by I27
ADR-0046 classifies Executive as an Intelligence layer (entity-mode-agnostic). Decision quality assessment, delegation tracking, time protection, political dynamics, noise filtering. Draws from `/cos`, `strategy-consulting`, `/veep`. Blocked by prompt fragment mechanism.

**I86: First-party integrations — meeting notes + task management** — Blocked by I54
Ship v1 with integrations for two families: (1) Meeting notes/transcripts: Quill Meetings, Granola, Gong (needs API feasibility research). (2) Task management: Linear, Asana, ClickUp — connect to the task tooling people already use rather than managing tasks only inside DailyOS. Each integration needs: auth flow, data mapping to DailyOS entities, sync strategy. Depends on I54 (MCP client framework) for integration protocol. Expands I54's "one per category" target list with concrete tools.

**I87: In-app notifications and feature announcements**
Notification surface for new features, kits, overlays. No design exists. Needs: notification model (triggers), UI surface (placement), persistence (read/unread), content delivery for announcements. Includes push notification strategy. Distinct from Tauri updater (I8) which handles binary updates.

**I88: Weekly Wrapped — Monday morning celebration + personal metrics**
Spotify Wrapped for your work week, delivered Monday morning. Celebrates the prior week: wins landed, outcomes captured, meetings navigated, actions closed. Fuses reflection (the feel-good lens) with personal productivity metrics (patterns, growth signals, trends). Not a dashboard — a moment. The one place where the system says "look what you did" instead of "here's what's next." Pairs with I89 (personality) for voice/tone.

**I89: Personality system — work bestie voice picker** — Supersedes I4
Selectable voice/tone for celebrations, delight moments, and Weekly Wrapped. Personality options: OK Boomer (grumpy dad energy), Millennial, Gen Z, Gen AI (delightfully weird), or user-defined. Private and playful, never public-facing. The product's playground — the one space where we lean into personality over professionalism. Supersedes I4 (motivational quotes) which captured the seed of this idea.

**I90: Product telemetry and analytics**
Usage tracking, feature adoption, and workflow completion metrics for product development. Must reconcile with Principle 5 (local-first). Opt-in, privacy-first design. Distinct from I88 (personal metrics shown to user) — this is about product development insights. Needs: event taxonomy, storage (local aggregate vs. remote), consent model, dashboard or export.

**I26: Web search for unknown external meetings**
When a meeting involves people/companies not in the workspace, prep is thin — violates P2 (Prepared, Not Empty). ADR-0022 specifies proactive research. Pattern exists: I74 does websearch for known accounts via `enrich_account()`. Extend to unknown meeting attendees: detect unrecognized domains in calendar events, research company + attendee context, inject into prep. Not blocked by I27 — can use existing `enrich_preps()` pipeline with a web search step.

### Open — Low Priority

**I3: Low-friction web capture to _inbox/**
Job: reduce friction for feeding external context into the system (P7: Consumption Over Production). User reads something relevant, wants it in DailyOS without leaving their flow. Form factor TBD — browser extension is one option but heavy (separate tech stack, Chrome Web Store). Alternatives: macOS share sheet, bookmarklet, "paste URL" in-app with fetch+convert. Inbox already works via drag-and-drop. Solve the job, not the specific form factor.

### Closed

**I1: Config directory naming** — Resolved. Renamed `.daybreak` → `.dailyos`.

**I2: Compact meetings.md format for dashboard dropdowns** — Closed, redundant. JSON-first meeting cards (schedule.json, preps/*.json) with MeetingCard component already render at multiple fidelity levels. The job this was solving is done by the shipped meeting prep system.

**I4: Motivational quotes as personality layer** — Superseded by I89 (personality system).

**I5: Orphaned pages (Focus, Week, Emails)** — Resolved. All three now have defined roles: Focus = drill-down from dashboard, Week = sidebar item (Phase 2+), Emails = drill-down from dashboard. See ADR-0010.

**I6: Processing history page** — Resolved. `get_processing_history` Tauri command (reads `processing_log` table, default limit 50). `HistoryPage.tsx` with table rendering. Route at `/history`, sidebar nav item under Workspace group.

**I7: Settings page can change workspace path** — Resolved. `set_workspace_path` Tauri command with directory picker via `@tauri-apps/plugin-dialog`. Validates path, calls `initialize_workspace()`, updates config. `WorkspaceCard` component in SettingsPage.

**I9: Focus page and Week priorities are disconnected stubs** — Resolved. Both `FocusPage.tsx` and `WeekPage.tsx` fully implemented with data loading, workflow execution, progress tracking, meeting cards, time blocks, action summaries. Closed Sprint 4a.

**I10: No shared glossary of app terms** — Closed, won't do. Types are the data model (types.rs, types/index.ts). Per CLAUDE.md documentation discipline: "Don't document what the code already says." If terms are confusing, rename in code — don't maintain a separate glossary that drifts.

**I11: Phase 2 email enrichment not fed to JSON** — Resolved. `deliver_today.py` gained `parse_email_enrichment()` which reads `83-email-summary.md` and merges into `emails.json`.

**I12: Email page missing AI context** — Resolved. Email page shows summary, recommended action, conversation arc per priority tier. Removed fake "Scan emails" button.

**I13: Onboarding wizard** — Resolved. `OnboardingWizard.tsx` with 5-step flow: Welcome → Entity Mode → Workspace → Google Auth (skippable) → Generate First Briefing. Replaces `ProfileSelector`. All three entity modes + both auth paths work end-to-end.

**I14: Dashboard meeting cards don't link to detail page** — Resolved. MeetingCard renders "View Prep" button linking to `/meeting/$prepFile` when prep exists. Added in Phase 1.5.

**I15: Entity-mode switcher in Settings** — Resolved. `set_entity_mode` Tauri command validates mode, sets `entity_mode` + derives `profile` for backend compat. `EntityModeCard` component in SettingsPage. Supersedes profile switching per ADR-0046.

**I16: Schedule editing UI** — Resolved. `set_schedule` Tauri command generates cron from hour/minute/timezone. `cronToHumanTime()` helper replaces raw cron display with "6:00 AM" format.

**I17: Post-meeting capture outcomes don't resurface in briefings** — Resolved (actions side). Non-briefing actions (post-meeting, inbox) now merge into dashboard via `get_non_briefing_pending_actions()` with title-based dedup. Wins/risks resurfacing split to I33.

**I18: Google API credential caching** — Resolved. Module-level `_cached_credentials` and `_cached_services` dict in `ops/config.py`. Per-process only. Eliminates double token refresh within `prepare_today.py`.

**I19: AI enrichment failure not communicated to user** — Resolved. "Limited prep" badge shown in MeetingCard when prep exists but enrichment fields are empty. Folded into I25 badge unification. Closed Sprint 6+7.

**I20: Standalone email refresh** — Resolved. `refresh_emails.py` thin orchestrator. `execute_email_refresh()` in executor.rs spawns script, calls `deliver_emails()` + optional `enrich_emails()`. Refresh button in EmailList.tsx.

**I21: FYI email classification expansion** — Resolved. Expanded `LOW_PRIORITY_SIGNALS` with marketing/promo/noreply terms. Added `BULK_SENDER_DOMAINS`, `NOREPLY_LOCAL_PARTS`. Enhanced classification: List-Unsubscribe header, Precedence bulk/list, bulk sender domain, noreply local part. 16 new Python tests.

**I22: Action completion doesn't write back to source markdown** — Resolved. `sync_completion_to_markdown()` in `hooks.rs` runs during post-enrichment hooks. Lazy writeback is acceptable — SQLite is working store, markdown is archive.

**I23: No cross-briefing action deduplication** — Resolved. Three layers: (1) `action_parse.py` SQLite pre-check, (2) `deliver_today.py` category-agnostic IDs + within-briefing dedup, (3) Rust-side `upsert_action_if_not_completed()` title-based dedup as final guard.

**I24: schedule.json meeting IDs are local slugs, not Google Calendar event IDs** — Resolved. Added `calendarEventId` field alongside the local slug `id` in both `schedule.json` and `preps/*.json`.

**I25: Unify meeting badge/status rendering** — Resolved. Consolidated into `computeMeetingDisplayState()` pure function with badge array output. Added "Limited prep" badge when prep exists but enrichment fields are empty. Closed Sprint 6+7.

**I30: Inbox action extraction lacks rich metadata** — Resolved. Added `processor/metadata.rs` with regex-based extraction of priority, `@Account`, `due: YYYY-MM-DD`, `#context`, and waiting/blocked status.

**I31: Inbox transcript summarization** — Resolved. `enrich.rs` gained `detect_transcript()` heuristic and richer enrichment prompt for transcripts. Parser handles `DISCUSSION:` / `END_DISCUSSION` markers. 12 enrich tests.

**I32: Inbox processor doesn't update account intelligence** — Resolved. AI enrichment prompt extracts WINS/RISKS sections. Post-enrichment `entity_intelligence` hook writes captures and touches `accounts.updated_at` as last-contact signal.

**I33: Captured wins/risks don't resurface in meeting preps** — Resolved. `meeting_prep.py` queries `captures` table via `_get_captures_for_account()` for recent wins/risks by account_id (14-day lookback).

**I34: Archive workflow lacks end-of-day reconciliation** — Resolved. Added `workflow/reconcile.rs` with mechanical reconciliation: reads schedule.json, checks transcript status, computes action stats, writes `day-summary.json` + `next-morning-flags.json`. Pure Rust, no AI (ADR-0040).

**I36: Daily impact rollup for CS extension** — Resolved. `workflow/impact_rollup.rs` with `rollup_daily_impact()`. Groups wins/risks by account, appends to `Weekly-Impact/{YYYY}-W{WW}-impact-capture.md`. Profile-gated, non-fatal, idempotent. 9 new tests.

**I37: Density-aware dashboard overview** — Resolved. `classify_meeting_density()` in `deliver.rs` categorizes day as light/moderate/busy/packed. Density guidance injected into `enrich_briefing()` prompt. 4 new tests.

**I38: Deliver script decomposition** — Resolved. ADR-0042 Chunk 1 replaces deliver_today.py with Rust-native per-operation delivery (`workflow/deliver.rs`). Chunk 3 adds AI enrichment ops. All AI ops are fault-tolerant — if Claude fails, mechanical data renders fine.

**I39: Feature toggle runtime** — Resolved. `features: HashMap<String, bool>` on Config. `is_feature_enabled()` priority chain: explicit override → profile default → true. Executor gates + Settings UI. 7 new tests.

**I41: Reactive meeting:prep wiring** — Resolved. `google.rs` calendar poller generates lightweight prep JSON for new prep-eligible meetings. Enriches from SQLite account data. Emits `prep-ready` event. Rust-native, no Python subprocess. 8 new tests.

**I42: CoS executive intelligence layer** — Resolved. `intelligence.rs` computes five signal types from SQLite + schedule: decisions due, stale delegations, portfolio alerts, cancelable meetings, skip-today. `IntelligenceCard.tsx` renders signal counts as badges. 13 new tests.

**I43: Stakeholder context in meeting prep** — Resolved. `db.rs` gained `get_stakeholder_signals()` — meeting frequency, last contact, relationship temperature, trend. `RelationshipContext` component in `MeetingDetailPage.tsx`. 5 new tests.

**I44: Meeting-scoped transcript intake from dashboard** — Resolved. ADR-0044. `processor/transcript.rs` handles full pipeline — frontmatter, AI enrichment, extraction, routing. Immutability enforced via `transcript_processed` state map. Frontend: `MeetingOutcomes.tsx` + `useMeetingOutcomes.ts`.

**I45: Post-transcript outcome interaction UI** — Resolved. `MeetingOutcomes.tsx` renders AI-extracted summary, wins, risks, decisions, and actions inside MeetingCard. Action completion, priority cycling, capture inline editing. All changes write to SQLite.

**I46: Meeting prep context limited to customer/QBR/training meetings** — Resolved. Expanded per ADR-0043 with title-based SQLite queries so all non-personal/non-all-hands types get meeting history, captures, and actions context.

**I47: Profile-agnostic entity abstraction** — Resolved. Introduced `entities` table and `EntityType` enum (ADR-0045). Bridge pattern: `upsert_account()` auto-mirrors to entities table. `entity_intelligence()` hook replaces profile-gated `cs_account_intelligence()`.

**I48: Workspace scaffolding on initialization** — Resolved. `initialize_workspace()` in `state.rs` creates dirs conditional on entity mode. Idempotent. 4 new tests.

**I49: Graceful degradation without Google authentication** — Resolved. `DashboardResult` includes `google_auth` status. `DashboardEmpty` shows "Connect Google" CTA when unauthenticated.

**I51: People sub-entity** — Resolved. Universal person tracking with ADR-0048 compliance. 3 new tables, ~15 DB functions, `people.rs` file I/O, 8 Tauri commands, calendar auto-population, file watcher, startup sync, person signals. Frontend: PeoplePage, PersonDetailPage. 189 Rust tests.

**I56: Onboarding redesign — teach the philosophy** — Resolved. `OnboardingFlow.tsx` with 9-chapter educational flow replacing config wizard. Demo data fixtures, dashboard tour, meeting deep dive mock. All Tauri commands wired. Closed Sprint 5.

**I57: Onboarding: populate workspace before first briefing** — Resolved. `populate_workspace` command creates folders + upserts accounts. `set_user_profile` saves userDomain. PopulateWorkspace.tsx chapter wired. Closed Sprint 5.

**I58: Feed user profile context into AI enrichment prompts** — Resolved. `UserContext` struct injected into `enrich_emails()`, `enrich_briefing()`, and meeting prep directives. Profile fields from config.json. Closed Sprint 5.

**I59: `CARGO_MANIFEST_DIR` runtime resolution** — Resolved. `resolve_scripts_dir()` uses Tauri resource resolver in release builds, falls back to `CARGO_MANIFEST_DIR` in debug. Scripts bundled via `tauri.conf.json` resources array.

**I60: Path traversal in inbox processing and workspace population** — Resolved. `validate_inbox_path()` and `validate_entity_name()` added in `util.rs`. Applied to `process_inbox_file`, `enrich_inbox_file`, and `populate_workspace`. Closed Sprint 5.

**I61: TOCTOU race in transcript immutability check** — Resolved. Sentinel `TranscriptRecord` (with `processed_at: "processing"`) inserted before dropping lock. Concurrent calls blocked by sentinel check. Closed Sprint 6+7.

**I62: `.unwrap()` panics in JSON mutation paths** — Resolved. All production `as_object_mut().unwrap()` calls replaced with `if let Some(obj)` or `.ok_or()` with graceful skip + warning log. Closed Sprint 5.

**I63: `run_python_script` ignores `timeout_secs` parameter** — Resolved. Uses `spawn()` + `recv_timeout` pattern matching PTY manager's timeout handling. Closed Sprint 5.

**I64: Non-atomic file writes risk corruption on crash** — Resolved. All critical writes use `crate::util::atomic_write_str()` (write .tmp then rename). Config, impact log, deliver.rs `write_json()` all covered. Closed Sprint 6+7.

**I65: Impact log append uses read-modify-write instead of atomic append** — Resolved. `append_to_impact_log()` in commands.rs uses `OpenOptions::append()`. `transcript.rs` also uses append-safe writes. Closed Sprint 6+7.

**I66: `deliver_preps` clears existing preps before writing new ones** — Resolved. Reversed order: new preps written first (filenames tracked in HashSet), then old files NOT in the set removed. Uses `atomic_write_str`. Closed Sprint 6+7.

**I67: Scheduler `should_run_now` window can miss jobs near boundary** — Resolved. Widened time check window from 60 → 120 seconds for sleep/wake recovery. Closed Sprint 6+7.

**I68: `Mutex` contention on read-heavy `AppState` fields** — Resolved. `config`, `workflow_status`, `calendar_events`, `last_scheduled_run` changed from `Mutex<T>` to `RwLock<T>`. All callers updated. Closed Sprint 6+7.

**I69: File router silently overwrites duplicate destinations** — Resolved. `unique_destination()` helper in `router.rs` appends `-1`, `-2` etc. suffix before extension when destination exists. Closed Sprint 6+7.

**I70: `sanitize_account_dir` doesn't strip filesystem-unsafe characters** — Resolved. `sanitize_for_filesystem()` in `util.rs` — strips `:*?"<>|`, replaces `/\` with `-`, trims dots/spaces, falls back to "unnamed". Closed Sprint 6+7.

**I71: Assorted low-severity edge hardening** — Resolved. All 9 items audited and confirmed: empty meeting list, missing config, missing prep dir, no-title meetings, no-subject emails, JSON parse failures, no-extension files, empty AI briefing — all handled. Closed Sprint 6+7.

**I72: Entity dashboard pages** — Resolved. Account list page (`AccountsPage.tsx`) with sortable table + account detail page (`AccountDetailPage.tsx`) with card-based layout. 6 Tauri commands. Route at `/accounts` and `/accounts/$accountId`.

**I73: Entity dashboard template system** — Resolved. ADR-0047 two-file pattern: `dashboard.json` (canonical) + `dashboard.md` (generated). `accounts.rs` with `AccountJson` schema, read/write/sync functions, markdown generation from JSON + SQLite live data. Three-way sync. File watching via mtime comparison.

**I74: Account enrichment via Claude Code websearch** — Resolved. `enrich_account()` in `accounts.rs` spawns Claude Code with structured websearch prompt. Parses `ENRICHMENT...END_ENRICHMENT` block into `CompanyOverview`. Updates JSON+DB+markdown. "Enrich"/"Refresh" button on AccountDetailPage. 3 new parser tests. Closed Sprint 6+7.

**I75: Entity dashboard external edit detection** — Resolved. Extended `watcher.rs` to watch `Accounts/` directory for `dashboard.json` changes. On change: debounce 500ms, read JSON, upsert to SQLite, regenerate markdown, emit `accounts-updated` frontend event. Closed Sprint 6+7.

**I76: SQLite durability — backup and rebuild-from-filesystem** — Resolved. `db_backup.rs`: `backup_database()` uses `rusqlite::backup::Backup` API for hot copy. `rebuild_from_filesystem()` scans `Accounts/` and `People/` workspace dirs. Two Tauri commands registered. 2 new tests. Closed Sprint 6+7.

**I77: Filesystem writeback audit** — Resolved. Full audit completed. Priority writeback: markdown has no priority field (not applicable). Decision writeback: intentionally excluded. Account markdown staleness: already handled by `update_account_field`. All paths verified. Closed Sprint 6+7.

**I78: Onboarding: teach inbox-first behavior** — Resolved. `InboxTraining.tsx` chapter teaches the paradigm shift: drop files in, intelligence comes out. Guided first inbox drop with visual progress. Closed Sprint 5.

**I79: Onboarding: Claude Code validation step** — Resolved. `ClaudeCode.tsx` chapter with `check_claude_status` command. Detects installation + auth. Framed as "Connect your AI" — parallel to Google Connect. Skippable with warning. Closed Sprint 5.

**I80: Proposed Agenda in meeting prep** — Resolved. `generate_mechanical_agenda()` assembles structured agenda from prep data (overdue items, risks, talking points, questions, capped at 7). AI enrichment via `enrich_preps()` refines ordering and adds rationale. "Proposed Agenda" card renders as first card on prep page. 199 tests.

**I81: People dynamics in meeting prep UI** — Resolved. "People in the Room" component in `MeetingDetailPage.tsx` with temperature badges, meeting count, last seen, organization, notes, "New contact" flags, cold-contact warnings, and person links. Pure frontend.

**I82: Copy-to-clipboard for meeting prep page** — Resolved. "Copy All" button exports full prep as markdown. Per-section `<CopyButton>` for individual cards. Reusable `useCopyToClipboard` hook and `CopyButton` component.

**I83: Rust-native Google API client** — Resolved. `google_api/` module: auth.rs (OAuth2 browser flow), calendar.rs (Calendar API v3), classify.rs (10-rule meeting classification), gmail.rs (Gmail API). Token compat with Python format. Sprint 8.

**I84: Port Phase 1 operations to Rust** — Resolved. `prepare/` module: constants.rs, email_classify.rs (3-tier), actions.rs (markdown parse + SQLite dedup), gaps.rs (calendar gap analysis), meeting_context.rs (rich meeting context), orchestrate.rs (4 orchestrators). Sprint 8.

**I85: Port orchestrators and delete Python** — Resolved. Wired Rust orchestrators into executor.rs, deleted `scripts/` (16 Python files), removed `run_python_script()` and related code. No Python on $PATH required. ADR-0049. Sprint 8. 324 tests.

**I91: Universal file extraction for inbox pipeline** — Resolved. `processor/extract.rs` module with format-aware text extraction (PDF, DOCX, XLSX, PPTX, HTML, RTF, plaintext). Companion .md pattern: original binary + extracted .md travel together. Classifier strips all known extensions. Inbox preview shows extracted text. ADR-0050. 346 tests.

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |
| R5 | **Open format = no switching cost.** Markdown portability means users can leave as easily as they arrive. The moat (archive quality) only works if DailyOS maintains the archive better than users could themselves — and better than a competitor wrapping the same open files. | High | Medium | Archive must be demonstrably better than DIY. Enrichment quality is the lock-in, not format. | Open |
| R6 | **N=1 validation.** All architecture designed from one user in one role (CS leader). Entity modes, Kits, Intelligence untested with actual project-based, sales, or engineering users. Assumptions about "how work is organized" may not survive contact with diverse roles. | High | High | Recruit 3-5 beta users across different roles before implementing I27. Validate entity-mode assumptions with real workflows. | Open |
| R7 | **Org cascade needs adoption density.** Organizational intelligence (Thursday Updates, cascading contributions) requires multiple DailyOS users on the same team. Single-user value must stand alone — org features are years away from being testable. | Medium | High | Ship individual product first. Don't invest in org features until adoption density exists. Keep it in Vision, not Roadmap. | Open |
| R8 | **AI reliability gap.** "Zero discipline" promise depends on AI enrichment being consistently good. Current fault-tolerant design (mechanical data survives AI failure) mitigates data loss but not quality — a bad briefing erodes trust faster than no briefing. | High | Medium | Invest in enrichment quality metrics. Surface confidence signals to users. Make AI outputs editable/correctable. | Open |
| R9 | **Composability untested at scale.** Kit + Intelligence + Integration composition is designed on paper (ADR-0046) but never built. Enrichment prompt fragment ordering, conflicts between multiple Intelligence layers, and "both" entity mode UX are all theoretical. | Medium | Medium | Build one Kit (CS) + one Intelligence (Executive) first. Validate composition with two overlays before designing more. | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | No | Need onboarding check (I13) |
| A2 | Workspace follows PARA structure | No | Should handle variations gracefully |
| A3 | `_today/` files use expected markdown format | Partial | Parser handles basic cases |
| A4 | Users have Google Workspace (Calendar + Gmail) | No | Personal Gmail, Outlook, iCloud not supported in MVP |

---

## Dependencies

| ID | Dependency | Type | Status | Notes |
|----|------------|------|--------|-------|
| D1 | Claude Code CLI | Runtime | Available | Requires user subscription |
| D2 | Tauri 2.x | Build | Stable | Using latest stable |
| D3 | Google Calendar API | Runtime | Optional | For calendar features |

---

*Migrated from RAIDD.md on 2026-02-06. Decisions are now tracked in [docs/decisions/](decisions/README.md).*
