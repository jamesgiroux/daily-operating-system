# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

**Current state:** 439 Rust tests passing. Sprints 1-8 complete. Python runtime eliminated. Entity intelligence architecture complete (ADR-0057, I130-I138). Active work: Sprint 9 (entity relationship graph) + ship-path bugs.

---

## Issues

### Sprint 9 — Entity Relationship Graph (Ship Blocker)

Entities exist but live in isolation. Meetings don't know which accounts they serve. People can't be linked to accounts from the UI. Projects don't exist as entities. The user is doing the synthesis work that the system should be doing — violates P6 (AI-Native) and P7 (Consumption Over Production). Without M2M, meeting intelligence can't connect to entity intelligence, and the core promise ("your day is ready") falls short.

**Scope:** Build the relationship graph between meetings, people, accounts, and projects. Three issues, in dependency order:

**I50: Projects overlay table and project entity support**
Projects as first-class entities alongside accounts. `projects` overlay table, CRUD commands, ProjectsPage + ProjectDetailPage (mirror account patterns). Content indexing + intelligence enrichment reuse entity_intel.rs shared module. Unblocks I52 (meetings need entities to link to).

**I52: Meeting-entity many-to-many association**
Junction table linking meetings to entities (accounts, projects, people). Auto-association from attendee domains + manual override. Meeting prep consumes all linked entity intelligence. Schedule view shows entity badges on meeting cards. This is the architectural keystone — once meetings know their entities, every surface gets smarter.

**I129: People entity editability — name, account link, and user contributions**
Already on ship path. People are auto-created from calendar attendees but key fields (name, account link) have no edit path. Entity picker for account linking, editable names, manual person creation, promoted notes. The "last mile" for people as useful nodes in the relationship graph.

**Sequence:** I50 (projects exist) → I52 (M2M links everything) → I129 (people become editable nodes). I129 can start in parallel with I50 since it doesn't depend on projects.

**Not in scope for Sprint 9:** Entity-mode config UI (I53), MCP integrations (I54), kits (I40), intelligence layers (I35/I55). Those build on top of the relationship graph but don't block it.

---

### Planned — Ship Path

**I8: DMG build + GitHub Actions CI + GitHub Releases**
Unsigned DMG for colleague distribution. GitHub Actions builds arm64 DMG on push/tag. GitHub Releases hosts the artifact. README with Gatekeeper bypass instructions (`xattr -cr`). No signing/notarization (no Apple Developer account). No updater (zero users, premature).

**I119: Gmail API returns empty metadata fields**
`gmail.rs` fetches messages but `from`, `from_email`, `subject`, and `date` are all empty strings in the directive. Snippets are populated so the API call works — header extraction (`From`, `Subject`, `Date`) is failing silently. Likely cause: `format=metadata` or `metadataHeaders` parameter issue, or response parsing doesn't walk `payload.headers[]` correctly.

**I123: Production Google OAuth credentials**
App requires users to supply their own `credentials.json` from Google Cloud Console. For distribution, DailyOS needs its own registered OAuth client. Client ID/secret embedded in Rust binary (compile-time `include_str!` or build-time env var). Steps: (1) Create Google Cloud project. (2) Configure OAuth consent screen (external, limited scope: Calendar read, Gmail read). (3) Create Desktop app OAuth client. (4) Embed in `google_api/auth.rs`. (5) Remove `credentials.json` file requirement. Dev override via env var for local testing.


*I129 moved to Sprint 9 (Entity Relationship Graph).*

### Planned — Intelligence Quality

**I139: File summary indexing + enrichment payload optimization**
Intelligence enrichment times out on accounts with 15+ content files because the prompt sends up to 50K chars of raw file text plus a web search instruction. Three changes:
(a) **Sort files by recency.** `get_entity_files` returns `modified_at DESC` so newest files get priority within the char budget. 90-day recency filter already applied (I138 hotfix).
(b) **Mechanical file summaries.** On content index sync, extract a summary per file (headings + first paragraph, ~500 chars). Store in the existing `content_index.summary` column (already in schema, always NULL). Zero AI cost — purely mechanical text extraction.
(c) **Enrichment sends summaries, not full text.** Prompt includes file summaries (50 files in 3K chars) instead of raw contents (3 files in 12K chars). Dramatically better signal density per token. Full text available on demand if a future multi-turn enrichment needs it.
Also: removed web search from initial enrichment prompt, reduced char caps (50K→12K initial, 20K→8K incremental), bumped PTY timeout to 180s. These hotfixes are already shipped; I139 completes the architectural fix.

### Open

**I110: Portfolio alerts on accounts sidebar/list**
IntelligenceCard removed from dashboard (ADR-0055). Portfolio alerts (renewal approaching within 60d, stale contact 30d+) have no home. `intelligence.rs` computation still exists. Surface as warning indicators on Accounts sidebar (badge count) and/or AccountsPage list rows. CS-profile gated. Rust layer done — purely frontend wiring.

**I115: Multi-line action extraction in inbox processor**
`extract_and_sync_actions()` only parses single-line actions (`- [ ]` checkbox lines). Structured multi-line formats (metadata on indented sub-lines like `- Account:`, `- Due:`) are silently ignored. Fix: after matching checkbox line, look ahead for indented `- Key: Value` sub-lines and merge into `ActionMetadata`.

**I122: Sunday briefing fetches Monday calendar labeled as "today"**
Running daily briefing on Sunday produces a directive with Monday's date and meetings labeled "today". Prepare phase likely targets next business day. If intentional, UI should say "Tomorrow" or "Monday." If not, fetch actual current day's calendar.




**I127: Manual action creation with entity connection**
No path to create an action from scratch — every action enters via briefing pipeline, post-meeting capture, inbox processing, or transcript parsing. Violates P3 (Buttons, Not Commands). New `create_action` Tauri command + inline creation row on ActionsPage + "Add action" on entity detail pages. `person_id` FK on actions table. Does NOT include field editing after creation (see I128).

**I128: Action field editing after creation** — Blocked by I127
Most action fields are immutable after creation. Only priority and status can be changed. Title, due_date, context, source_label, entity connections are write-once. New `update_action` command accepting partial field set. Editable fields inline on ActionDetailPage. Guard: warn before editing briefing-generated actions (would be overwritten on next briefing run).

**I26: Web search for unknown external meetings**
When a meeting involves people/companies not in the workspace, prep is thin. Pattern exists: I74 does websearch for known accounts. Extend to unknown meeting attendees: detect unrecognized domains, research company + attendee context, inject into prep. Not blocked by I27.

**I94: Week page Phase 2 — AI enrichment + briefing layout** — ADR-0052
`weekNarrative` + `topPriority` AI fields. Layout restructure from dashboard feel to briefing feel. Remove Card wrappers, full-width narrative prose, taper visual density. Research: `docs/research/2026-02-08-weekly-briefing-ux-research.md`.

**I95: Week page Phase 3 — proactive suggestions** — ADR-0052, blocked by I94
Draft agenda requests, pre-fill preps, suggest tasks for open blocks. Time blocking proactivity setting. Only "suggestions" ships initially.

**I140: Branded Google OAuth success page**
After Google authentication completes, the browser lands on a localhost callback that currently has no UI. Replace with an on-brand landing page that (a) confirms auth succeeded, (b) reinforces the DailyOS value prop, and (c) teaches the user what happens next (e.g. "Your calendar and email are now connected — open DailyOS and your day will be ready"). Design tokens from UI-SPEC.md (cream bg, charcoal text, gold accent). Page is static HTML served by the Tauri localhost callback handler — no React, no build step. Inspiration: [Google Antigravity auth-success](https://antigravity.google/auth-success).

**I141: AI content tagging during entity enrichment**
When entity intelligence runs, the enrichment prompt already reads file summaries but discards its assessment of them. Add an optional output field where the AI rates which files it found most relevant and classifies them (e.g. "people-oriented", "meeting-oriented", "commercial", "technical"). Store tags in the existing `content_index` table (new `tags` TEXT column, JSON array). Zero extra AI cost — piggybacks on the enrichment call that already reads the files. Enables future filtering/surfacing of high-signal files on entity detail pages.

**I142: Account Plan — leadership-facing generated artifact**
Intelligence assessments serve the CSM's daily situational awareness. Leadership needs a different artifact: a structured Account Plan with executive summary (verdict + top risk + biggest opportunity), 90-day focus areas, customer context, risk mitigation table, and products/adoption status. Separate generated document that consumes `intelligence.json` + `dashboard.json` and produces a templated output. Not a refinement of enrichment — a distinct artifact with its own generation command. Scope: (a) Account Plan template definition (structured sections, not prose). (b) Generation command that reads intelligence + dashboard + captures and fills the template. (c) UI entry point on AccountDetailPage ("Generate Account Plan"). (d) Output as markdown in account directory (consumable by Claude Desktop, exportable). Data gaps: Products & Adoption and Consumption metrics require CRM/product telemetry (I54) — initially those sections render as "Not available."

**I143: Renewal lifecycle tracking — auto-rollover, churn/expansion markers**
Renewal date is currently a static field with no concept of outcome. When a renewal passes, the stat bar shows "Xd overdue" indefinitely — even if the renewal was successful. Three parts:
(a) **Auto-rollover.** When a renewal date passes and the account hasn't churned, automatically advance the renewal date by 1 year (or contract term length if known). The "overdue" state should be temporary, not permanent.
(b) **Lifecycle event markers.** Capture churn, downsell, expansion, and renewal as historical events on an account — not current-state fields. Likely a new `account_events` table (event_type, date, notes, arr_delta). These are the "what happened" record, distinct from the account's current health/lifecycle fields.
(c) **UI for recording events.** Entry point on AccountDetailPage to log a renewal outcome, expansion, downsell, or churn. Events surface in Evidence & History section. Churn event sets lifecycle to "churned" and suppresses renewal countdown.
Design question: where do historical events live in the filesystem? Appended to dashboard.json? Separate events.json? Needs an ADR if the pattern is reusable beyond renewals.

**I3: Low-friction web capture to _inbox/**
Reduce friction for feeding external context into the system. Form factor TBD — browser extension, macOS share sheet, bookmarklet, "paste URL" in-app. Inbox already works via drag-and-drop.

### Parking Lot (post-ship, blocked by I27 or needs usage data)

**I27: Entity-mode architecture — umbrella issue**
ADR-0046 three-layer architecture: Core + Entity Mode + Integrations. Sub-issues: I53, I54, I55. I50 and I52 promoted to Sprint 9 (ship blocker). Current state: `entities` table and `accounts` overlay exist (ADR-0045), bridge pattern proven. Remaining scope: entity-mode config (I53), MCP integration framework (I54), intelligence layers (I35/I55), entity mention extraction, cross-entity content linking.

**I40: CS Kit — account-mode fields, templates, and vocabulary** — Blocked by I27

**I53: Entity-mode config, onboarding, and UI adaptation** — Blocked by I52 (Sprint 9)

**I54: MCP client integration framework** — Blocked by I27

**I28: MCP server and client** — Blocked by I27

**I35: ProDev Intelligence** — Blocked by I27

**I55: Executive Intelligence** — Blocked by I27

**I86: First-party integrations** — Blocked by I54

**I87: In-app notifications and feature announcements**

**I88: Weekly Wrapped — Monday morning celebration + personal metrics**

**I89: Personality system — work bestie voice picker** — Supersedes I4

**I90: Product telemetry and analytics**

**I92: User-configurable metadata fields** — Blocked by I27, ADR-0051

### Closed

**I130:** Resolved. intelligence.json schema, entity_intelligence DB table, CRUD in db.rs, TypeScript EntityIntelligence type. CompanyOverview migration. Foundation for ADR-0057.

**I131:** Resolved. Full intelligence enrichment engine — context builder (meetings, actions, captures, people, file contents), entity-parameterized prompt (initial + incremental modes), structured response parser, PTY orchestrator. Web search on initial, delta-only on incremental.

**I132:** Resolved. IntelligenceQueue with priority-based dedup and debounce. Background processor in lib.rs. Watcher enqueues ContentChange on account/project file changes. Inbox pipeline enqueues after capture ingestion.

**I133:** Resolved. AccountDetailPage intelligence-first redesign — executive assessment, attention items (risks/wins/unknowns), meeting readiness, stakeholder intelligence, evidence history. Graceful degradation when no intelligence exists.

**I134:** Resolved. Shared `format_intelligence_markdown()` in entity_intel.rs generates intelligence sections for dashboard.md. Used by accounts, projects, and people. Company Overview skipped when intelligence.json has company_context.

**I135:** Resolved. meeting_context.rs reads intelligence.json for entity prep. deliver.rs includes intelligence summary + risks + readiness in prep files. Calendar-triggered readiness refresh queued after schedule delivery.

**I136:** Resolved. People intelligence enrichment from SQLite signals (meetings, entity connections, captures). `enrich_person` command. PersonDetailResult includes intelligence. write_person_markdown includes intelligence sections.

**I137:** Resolved. Daily and weekly briefing enrichment prompts include cached entity intelligence for accounts with meetings. Brief DB lock pattern (microsecond read, release before PTY). Cross-entity synthesis instructions in weekly prompt.

**I138:** Resolved. Project content indexing (sync_content_index_for_project delegates to shared sync_content_index_for_entity). ProjectContent watcher variant with intel queue integration. sync_all_content_indexes covers both accounts and projects.

**I1:** Resolved. Config directory renamed `.daybreak` → `.dailyos`.

**I2:** Closed, redundant. JSON-first meeting cards render at multiple fidelity levels.

**I4:** Superseded by I89 (personality system).

**I5:** Resolved. Focus, Week, Emails all have defined roles. ADR-0010.

**I6:** Resolved. Processing history page + `get_processing_history` command.

**I7:** Resolved. Settings workspace path change with directory picker.

**I9:** Resolved. Focus and Week pages fully implemented.

**I10:** Closed, won't do. Types are the data model.

**I11:** Resolved. Email enrichment parsed and merged into `emails.json`.

**I12:** Resolved. Email page shows AI context per priority tier.

**I13:** Resolved. Onboarding wizard with 5-step flow.

**I14:** Resolved. MeetingCard "View Prep" button.

**I15:** Resolved. Entity-mode switcher in Settings.

**I16:** Resolved. Schedule editing UI with human-readable cron.

**I17:** Resolved. Non-briefing actions merge into dashboard.

**I18:** Resolved. Google API credential caching.

**I19:** Resolved. "Limited prep" badge for AI enrichment failures.

**I20:** Resolved. Standalone email refresh.

**I21:** Resolved. Expanded FYI email classification.

**I22:** Resolved. Action completion writeback to source markdown.

**I23:** Resolved. Three-layer cross-briefing action deduplication.

**I24:** Resolved. `calendarEventId` field alongside local slug.

**I25:** Resolved. `computeMeetingDisplayState()` unified badge rendering.

**I29:** Closed. Superseded by I73 template system + kit issues.

**I30:** Resolved. Inbox action extraction with rich metadata (`processor/metadata.rs`).

**I31:** Resolved. Inbox transcript summarization with `detect_transcript()` heuristic.

**I32:** Resolved. Inbox processor updates account intelligence via WINS/RISKS extraction.

**I33:** Resolved. Wins/risks resurface in meeting preps via 14-day lookback.

**I34:** Resolved. Archive reconciliation (`workflow/reconcile.rs`).

**I36:** Resolved. Daily impact rollup (`workflow/impact_rollup.rs`).

**I37:** Resolved. Density-aware dashboard overview.

**I38:** Resolved. ADR-0042. Rust-native delivery + AI enrichment ops.

**I39:** Resolved. Feature toggle runtime with `is_feature_enabled()`.

**I41:** Resolved. Reactive meeting:prep wiring via calendar poller.

**I42:** Resolved. CoS executive intelligence layer (`intelligence.rs`).

**I43:** Resolved. Stakeholder context in meeting prep.

**I44:** Resolved. ADR-0044. Meeting-scoped transcript intake.

**I45:** Resolved. Post-transcript outcome interaction UI.

**I46:** Resolved. Meeting prep context expanded beyond customer/QBR/training. ADR-0043.

**I47:** Resolved. ADR-0045. Entity abstraction with `entities` table + bridge pattern.

**I48:** Resolved. Workspace scaffolding on initialization.

**I49:** Resolved. Graceful degradation without Google auth.

**I51:** Resolved. People sub-entity — universal person tracking. 3 tables, 8 commands.

**I56:** Resolved. Onboarding redesign — 9-chapter educational flow.

**I57:** Resolved. Onboarding: populate workspace before first briefing.

**I58:** Resolved. User profile context in AI enrichment prompts.

**I59:** Resolved. Script path resolution for production builds.

**I60:** Resolved. Path traversal validation in inbox/workspace.

**I61:** Resolved. TOCTOU sentinel for transcript immutability.

**I62:** Resolved. `.unwrap()` panics replaced with graceful handling.

**I63:** Resolved. Python script timeout handling.

**I64:** Resolved. Atomic file writes via `atomic_write_str()`.

**I65:** Resolved. Impact log append safety.

**I66:** Resolved. Safe prep delivery (write-first, then remove stale).

**I67:** Resolved. Scheduler boundary widened 60 → 120 seconds.

**I68:** Resolved. `Mutex<T>` → `RwLock<T>` for read-heavy AppState.

**I69:** Resolved. File router duplicate destination handling.

**I70:** Resolved. `sanitize_for_filesystem()` strips unsafe characters.

**I71:** Resolved. Low-severity edge hardening (9 items).

**I72:** Resolved. AccountsPage + AccountDetailPage. 6 Tauri commands.

**I73:** Resolved. ADR-0047. Entity dashboard template system, two-file pattern, three-way sync.

**I74:** Resolved. Account enrichment via Claude Code websearch.

**I75:** Resolved. Entity dashboard external edit detection via watcher.

**I76:** Resolved. SQLite backup + rebuild-from-filesystem.

**I77:** Resolved. Filesystem writeback audit.

**I78:** Resolved. Onboarding: inbox-first behavior chapter.

**I79:** Resolved. Onboarding: Claude Code validation chapter.

**I80:** Resolved. Proposed Agenda in meeting prep.

**I81:** Resolved. People dynamics in meeting prep UI.

**I82:** Resolved. Copy-to-clipboard for meeting prep.

**I83:** Resolved. Sprint 8. Rust-native Google API client (`google_api/` module).

**I84:** Resolved. Sprint 8. Phase 1 operations ported to Rust (`prepare/` module).

**I85:** Resolved. Sprint 8. Orchestrators ported, `scripts/` deleted, Python eliminated. ADR-0049.

**I91:** Resolved. Universal file extraction (`processor/extract.rs`). ADR-0050.

**I93:** Resolved. ADR-0052. Week page mechanical redesign — consumption-first layout.

**I96:** Resolved. ADR-0052. Week planning wizard retired.

**I97:** Resolved. ADR-0053. Dashboard readiness strip (later removed by ADR-0055).

**I98:** Resolved. ADR-0053. Action/email sidebar order flipped.

**I99:** Resolved. ADR-0053. Greeting removed, Focus promoted.

**I100:** Resolved. ADR-0053. ActionList maxVisible 3 → 5.

**I101:** Resolved. ADR-0053. Full-width summary (later superseded by ADR-0055 two-column).

**I102:** Resolved. ADR-0054. Shared `ListRow` + `ListColumn` primitives.

**I103:** Resolved. ADR-0054. AccountsPage flat rows with health dot.

**I104:** Resolved. ADR-0054. PeoplePage flat rows with temperature + trend.

**I105:** Resolved. PeoplePage shared component consolidation (SearchInput, TabFilter).

**I106:** Resolved. `PersonListItem` struct + batch `get_people_with_signals()` query.

**I107:** Resolved. Action detail page at `/actions/$actionId`. Context card, source meeting, account link.

**I109:** Resolved. ADR-0055. Focus page — `get_focus_data` assembles from schedule.json + SQLite + gap analysis.

**I111:** Resolved. ADR-0055. Dashboard visual rhythm — removed chrome, tapered spacing, breathing room.

**I112:** Resolved. Graceful empty state — `load_schedule_json()` missing file returns `Empty` not `Error`.

**I113:** Resolved. Workspace transition detection. Auto-scaffold, skip `_`-prefixed folders.

**I114:** Resolved. ADR-0056. Parent-child accounts — `parent_id` FK, expandable rows, breadcrumb, aggregate rollup.

**I116:** Resolved. ADR-0056 downstream. ActionsPage account name resolution via `ActionListItem`.

**I117:** Resolved. ADR-0056 downstream. `guess_account_name()` discovers child BU directories.

**I120:** Closed, won't fix. Legacy action import from VIP workspace. Starting clean — manual action creation (I127) replaces bulk import approach.

**I121:** Closed, won't fix. Legacy prep generation against pre-existing workspace data. Clean start means preps build from fresh Google Calendar + account data.

**I118:** Resolved. `format_time_display_tz()` in `deliver.rs` accepts optional `Tz` and converts with `with_timezone()`. `orchestrate.rs` converts to `chrono::Local`. `calendar_merge.rs` takes `Tz` param. All three call sites handle timezone correctly.

**I124:** Resolved. `content_index` table, recursive directory scanner (respects child account boundaries), startup sync, `get_entity_files` + `index_entity_files` + `reveal_in_finder` commands, Files card on AccountDetailPage with watcher integration. 409 tests. Foundation for ADR-0057.

**I125:** Resolved. `AccountContent` watch source variant, debounced content change events, `content-changed` event emission, frontend listener with "new files detected" banner. Delivered with I124.

**I126:** Superseded by I130. Basic `build_file_context()` delivered with I124. ADR-0057 replaces with full intelligence pipeline (I130-I138).

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |
| R5 | Open format = no switching cost. Moat is archive quality, not format lock-in. | High | Medium | Enrichment quality is the lock-in. | Open |
| R6 | N=1 validation. All architecture designed from one user/role. | High | High | Recruit beta users across roles before I27. | Open |
| R7 | Org cascade needs adoption density. | Medium | High | Ship individual product first. | Open |
| R8 | AI reliability gap. Bad briefing erodes trust faster than no briefing. | High | Medium | Quality metrics, confidence signals, editable outputs. | Open |
| R9 | Composability untested at scale. Kit + Intelligence composition is theoretical. | Medium | Medium | Build one Kit + one Intelligence first. | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | Partial | Onboarding checks (I79) |
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

*Migrated from RAIDD.md on 2026-02-06. Decisions tracked in [docs/decisions/](decisions/README.md).*
