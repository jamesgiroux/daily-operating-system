# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

**Current state:** 439 Rust tests passing. Sprints 1-8 complete. Python runtime eliminated. Entity intelligence architecture complete (ADR-0057, I130-I138). Active work: ship-path bugs + distribution.

---

## Issues

### Planned — Ship Path

**I8: DMG build + GitHub Actions CI + GitHub Releases**
Unsigned DMG for colleague distribution. GitHub Actions builds arm64 DMG on push/tag. GitHub Releases hosts the artifact. README with Gatekeeper bypass instructions (`xattr -cr`). No signing/notarization (no Apple Developer account). No updater (zero users, premature).

**I119: Gmail API returns empty metadata fields**
`gmail.rs` fetches messages but `from`, `from_email`, `subject`, and `date` are all empty strings in the directive. Snippets are populated so the API call works — header extraction (`From`, `Subject`, `Date`) is failing silently. Likely cause: `format=metadata` or `metadataHeaders` parameter issue, or response parsing doesn't walk `payload.headers[]` correctly.

**I123: Production Google OAuth credentials**
App requires users to supply their own `credentials.json` from Google Cloud Console. For distribution, DailyOS needs its own registered OAuth client. Client ID/secret embedded in Rust binary (compile-time `include_str!` or build-time env var). Steps: (1) Create Google Cloud project. (2) Configure OAuth consent screen (external, limited scope: Calendar read, Gmail read). (3) Create Desktop app OAuth client. (4) Embed in `google_api/auth.rs`. (5) Remove `credentials.json` file requirement. Dev override via env var for local testing.


**I129: People entity editability — name, account link, and user contributions**
People are auto-created from calendar attendees, which bootstraps ~80% of the data. The remaining 20% requires user contribution, but key fields have no edit path. The `name` field is derived from email via `name_from_email()` (e.g. `schen@acme.com` → "Schen") and is not in the `update_person_field` whitelist — so bad auto-names are permanent. Account/entity associations are auto-inferred from email domains and can be wrong, with no way to correct them from the UI (linking commands exist but aren't wired to PersonDetailPage). The system does the heavy lift; the user needs to do the finishing work.
Scope:
(a) **Add `name` to the editable whitelist** in `db.rs` `update_person_field()`. Display as click-to-edit on PersonDetailPage header (currently a static `<h1>`). Auto-derived names become the default, user override sticks.
(b) **Entity link management on PersonDetailPage.** Show current linked entities with unlink affordance. Add entity picker (accounts combobox) to link a person to an account. Commands exist (`link_person_entity`, `unlink_person_entity`) — this is frontend wiring.
(c) **Manual person creation.** "Add person" entry point on PeoplePage. Minimum fields: name + email. Auto-derives organization, relationship from email. Pre-links to account if triggered from AccountDetailPage.
(d) **Notes as first-class contribution.** Notes field exists and is editable, but it's buried in edit mode. Promote to always-visible section with inline editing — this is the user's primary way to contribute knowledge (context, preferences, history, working style).
Does NOT include: merge/dedup of duplicate people, bulk import, or contact sync from external sources.

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

**I3: Low-friction web capture to _inbox/**
Reduce friction for feeding external context into the system. Form factor TBD — browser extension, macOS share sheet, bookmarklet, "paste URL" in-app. Inbox already works via drag-and-drop.

### Parking Lot (post-ship, blocked by I27 or needs usage data)

**I27: Entity-mode architecture — umbrella issue**
ADR-0046 three-layer architecture: Core + Entity Mode + Integrations. Sub-issues: I50, I52, I53, I54, I55. Current state: `entities` table and `accounts` overlay exist (ADR-0045), bridge pattern proven.

**I40: CS Kit — account-mode fields, templates, and vocabulary** — Blocked by I27

**I50: Projects overlay table and project entity support** — Blocked by I27

**I52: Meeting-entity many-to-many association** — Blocked by I50

**I53: Entity-mode config, onboarding, and UI adaptation** — Blocked by I50, I52

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
