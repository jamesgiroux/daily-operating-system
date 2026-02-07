# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

---

## Issues

<!-- Sprint-oriented grouping (2026-02-07 PM analysis, revised after code audit):

  TEST BED: ~/Documents/test-workspace/ — clean workspace for end-to-end validation.
  Every sprint milestone is tested here, not in VIP/. See ROADMAP.md for full sprint plan.

  SPRINT 1: "First Run to Working Briefing" — two phases + polish
    Phase A (foundation, parallel):
      I48 (workspace scaffolding — NEW), I49 (no-auth graceful degradation — NEW)
      I7 (workspace path), I15 (profile switch), I16 (schedule UI)
    Phase B (sequential, depends on Phase A):
      I13 (onboarding) — depends on I7, I48, I49. Design decisions during impl.
    Phase C (polish, fills gaps):
      I25 (badge unification), I19 (enrichment failure indicator)
    Test paths: with-Google-auth AND without-Google-auth
    Done when: Both paths work e2e in test-workspace, no manual config editing

  SPRINT 2: "Make it Smarter" — COMPLETE
    I42 (executive intelligence), I43 (stakeholder context), I41 (reactive prep), I31 (transcript enrichment)
    168 Rust tests passing.

  SPRINT 3: "Make it Reliable" — COMPLETE
    I39 (feature toggles), I18 (API caching), I20 (email refresh), I21 (FYI classification),
    I37 (density-aware overview), I6 (processing history). 155 Rust + 37 Python tests passing.

  SPRINT 4: "Ship It"
    I8 (distribution — DMG, notarization)
    I9 (focus/week stubs — non-embarrassing)
    7-day crash-free validation on test-workspace
    Done when:       DMG installs cleanly, onboarding→briefing works 7 days on clean machine

  PARKING LOT (post-ship, entity-mode architecture):
    I27 (umbrella) → I50, I51, I52, I53, I54 (entity-mode foundation)
    I40 (CS overlay), I35 (ProDev overlay), I29 (doc schemas)
    I28 (MCP — now integration protocol per ADR-0046)
    Deferred: I26 | I2, I3, I4, I10
    Revisit after Sprint 4 ships with real usage data. ADR-0046 accepted.
-->

### Open — High Priority (Sprint 1 prerequisites)

**I48: Workspace scaffolding on initialization**
App never creates workspace directories. When a workspace path is set (via onboarding or Settings), the app must create `_today/`, `_inbox/`, `_archive/` if they don't exist. Currently: `_today/data/` is created on-demand by `deliver_today.py` (line 94), but `_inbox/` and `_archive/` are never created — inbox batch and archive workflow fail silently on a fresh workspace. Needs: a Rust `initialize_workspace(path)` function called when workspace path is set, with validation that the parent directory exists and is writable. Design decision: whether to also create `Projects/`, `Accounts/`, or other PARA dirs depends on the onboarding workspace strategy (see I13).

**I49: Graceful degradation without Google authentication**
Pipeline behavior when Google isn't authenticated is undefined. `prepare_today.py` calls Google Calendar and Gmail APIs — if no token exists, it may crash or return partial data. The app must handle the no-auth path cleanly: skip calendar/email API calls, generate a briefing with empty schedule/email sections, show a clear "Connect Google for calendar and email" prompt on the dashboard. This is a thin slice of I39 (feature toggles) — just the auth check, not the full toggle UI. Needed before onboarding (I13) can offer a "skip Google for now" path. Check: `scripts/prepare_today.py` Google API call sites, `src-tauri/src/google.rs` token detection, dashboard empty states in `src/components/dashboard/`.

### Open — Medium Priority

**I7: Settings page can't change workspace path**
Displays as read-only. Needs Tauri `dialog::FileDialogBuilder` for directory picker, a `set_workspace_path(path)` command, and validation. Small scope. When workspace path is set, should call workspace scaffolding (I48).

**I8: No app update/distribution mechanism**
Options: Tauri's built-in updater, GitHub Releases + Sparkle, manual DMG, Mac App Store. Needs Apple Developer ID for notarization. Not blocking MVP — can ship as manual DMG.

**I9: Focus page and Week priorities are disconnected stubs**
`focus.json` returns "not yet implemented." Weekly priorities from `week-overview.json` don't flow into daily focus. `/week` should set weekly priorities; `/today` should derive daily focus from those + today's schedule.

**I13: No onboarding flow**
First-time user hits dead end after profile selection. Depends on I7 (workspace path picker), I48 (workspace scaffolding), I49 (no-auth graceful degradation). Onboarding flow must handle two paths: (1) with Google auth → full briefing, (2) without Google → degraded but functional dashboard with "Connect Google" prompt. Design decisions needed during implementation: workspace strategy (create fresh dir vs map to existing working directory vs let user choose), directory structure (minimum pipeline dirs vs full PARA), default workspace path, first-briefing content with no historical data, Google auth as optional vs required. Current state: profile selector exists and works (router.tsx:47-60), config auto-creation does not exist, workspace dirs are never scaffolded, Google auth is only accessible via Settings page. Design constraint: Principle 4 (Opinionated Defaults) — should work out-of-box with sensible choices, escapable for power users.

**I15: Profile switching unavailable in Settings**
Profile selector at first launch says "You can change this later in Settings" but Settings has no switcher. Needs: dropdown/radio in Settings, writes to config.json, triggers reload.

**I16: Schedule editing requires manual config.json editing**
Settings shows raw cron expressions. Needs: time picker ("Briefing time: 6:00 AM"), writes cron to config, hides syntax. Power users can still edit JSON directly.

**I40: CS domain overlay — account-mode vocabulary and schemas**

**I40: CS domain overlay — account-mode vocabulary and schemas**
ADR-0046 replaces the CS extension concept with a CS domain overlay. What remains CS-specific after ADR-0043 narrowed extensions: CS-specific account fields (ARR, renewal dates, health scores, ring classification), account dashboard template/generation, success plan templates, Google Sheets sync (Last Engagement Date writeback). CRM data sources (Clay, Gainsight, Salesforce) are now integrations (I54), not overlay responsibilities. The existing `accounts` table IS the CS overlay — it carries CS-specific fields on top of the universal `entities` table. Remaining work: formalize overlay registration, schema contribution mechanism, template system. Blocked by I27 umbrella. Reference: `~/Documents/VIP/.claude/skills/daily-csm/`.

**I20: No standalone email refresh**
Emails only update with full briefing. ADR-0030 decomposition makes this more feasible — `ops/email_fetch.py` is now a standalone callable operation. Remaining work: a thin orchestrator or Rust command that invokes `email_fetch` independently and writes `emails.json`. Still raises partial-refresh semantics questions; ADR-0006 determinism boundary still applies.

**I21: FYI emails may never appear due to classification defaults**
`classify_email_priority()` in `ops/email_fetch.py` defaults to "medium." Only newsletters, automated senders, and GitHub notifications trigger "low." If a user's inbox is mostly customer + internal emails, the FYI section is permanently empty — not wrong, but means the three-tier promise (ADR-0029) is invisible. Consider: expanding low signals (marketing domains, bulk senders), or showing an explicit "0 FYI" indicator so users know the tier exists.

**I25: Unify meeting badge/status rendering**
MeetingCard has 5 independent status signals (isCurrent, hasPrep, isPast, overlayStatus, type) each with their own conditional. Consolidate into a computed MeetingDisplayState. Relates to ADR-0033.

**I26: Web search for unknown external meetings not implemented**
ADR-0022 specifies proactive research via local archive + web for unknown meetings. Local archive search works in `ops/meeting_prep.py`. Web search does not exist. Likely a Phase 2 task — Claude can invoke web search during enrichment (Phase 2). Low urgency since archive search provides some coverage.

**I27: Entity-mode architecture — umbrella issue**
ADR-0046 replaces profile-activated extensions (ADR-0026) with three-layer architecture: Core + Entity Mode + Integrations. Entity mode (account-based, project-based, or both) replaces profile as the organizing principle. Integrations (MCP data sources) are orthogonal to entity mode. Domain overlays replace extensions as thin vocabulary/schema contributors. Sub-issues: I50 (projects table), I51 (people table), I52 (meeting-entity M2M), I53 (entity-mode config/onboarding), I54 (MCP integration framework). Current state: `entities` table and `accounts` overlay exist (ADR-0045), bridge pattern proven. Post-Sprint 4.

**I28: MCP server and client not implemented**
ADR-0027 accepts dual-mode MCP (server exposes workspace tools to Claude Desktop, client consumes Clay/Slack/Linear). ADR-0046 elevates MCP client to the integration protocol — every external data source (Gong, Salesforce, Linear, etc.) is an MCP server consumed by the app. IPC commands are designed to be MCP-exposable (good foundation from ADR-0025). No MCP protocol code exists. Server side exposes DailyOS tools; client side is the integration layer. See I54 for client framework.

**I29: Structured document schemas not implemented**
ADR-0028 accepts JSON-first schemas for account dashboards, success plans, and structured documents (`dashboard.json` + `dashboard.md` pattern). Briefing JSON pattern exists as a template. Account dashboard UI is a stub. No schema validation system. Less coupled to extensions post-ADR-0046 — core entity schemas are universal, domain overlays contribute additional fields. Blocked by I27 umbrella for overlay-contributed schemas.

**I50: Projects overlay table and project entity support**
ADR-0046 requires a `projects` overlay table parallel to `accounts`. Fields: id, name, status, milestone, owner, target_date. Bridge pattern: `upsert_project()` auto-mirrors to `entities` table (same mechanism as `upsert_account()` → `ensure_entity_for_account()`). CRUD commands: `upsert_project`, `get_project`, `get_projects_by_status`. Frontend: Projects page (parallel to Accounts page), project entity in sidebar for project-based and both modes. Blocked by I27.

**I51: People sub-entity table and entity-people relationships**
ADR-0046 establishes people as universal sub-entities. Create `people` table (id, name, email, organization, role, last_contact) and `entity_people` junction (entity_id, person_id, relationship_type). People are populated from: meeting attendees (automatic), CRM integrations (I54), manual entry. Enriches meeting prep with stakeholder context (interaction history, relationship signals). Population strategy: attendee-seeded on first briefing, CRM-enriched when integrations are connected, user-correctable. Blocked by I27.

**I52: Meeting-entity many-to-many association**
Replace `account_id` FK on `meetings_history`, `actions`, `captures` with `meeting_entities` junction table. Enables meetings to associate with multiple entities (an account AND a project). Deferred explicitly from ADR-0045 to I27. Migration: existing `account_id` values become rows in `meeting_entities`. Association logic: account-based uses domain matching (existing), project-based uses integration links + AI inference + manual correction. Blocked by I50 (projects must exist first).

**I53: Entity-mode config, onboarding, and UI adaptation**
Replace `profile` config field with `entityMode` (account | project | both) + `integrations` + `domainOverlay`. Update onboarding: entity-mode selector ("How do you organize your work?") → integration checklist → optional role shortcut. Update sidebar to render Accounts and/or Projects based on entity mode. Update dashboard portfolio attention to compute signals for active entity types. Migration: `profile: "customer-success"` → `entityMode: "account"` + `domainOverlay: "customer-success"`. `profile: "general"` → `entityMode: "project"`. Blocked by I50, I52.

**I54: MCP client integration framework**
Build MCP client infrastructure in Rust for consuming external data sources per ADR-0046 and ADR-0027. Requirements: auth flow per integration (OAuth where needed), sync cadence configuration, error handling and retry, integration settings in Settings page. Start with one integration per category to prove the pattern: one transcript source (Gong or Granola), one CRM (Salesforce), one task tool (Linear). Each integration is an MCP server the app consumes — community can build new ones without touching core. Evolves I28 (MCP client side). Blocked by I27.

**I35: ProDev domain overlay — personal impact and career narrative**
ADR-0046 replaces ProDev extension with a domain overlay. ADR-0041 establishes that Personal Impact capture is ProDev territory: daily end-of-day reflection prompt ("What did you move forward today?"), weekly narrative summary, monthly/quarterly rollup for performance reviews. Distinct from CS outcomes (which are captured via transcripts and post-meeting prompts). Works with any entity mode (account-based, project-based, or both) — personal impact is orthogonal to how work is organized. Blocked by overlay registration mechanism (I27). `/wrap`'s "Personal Impact" section is the reference implementation.

### Open — Low Priority

**I2: Compact meetings.md format for dashboard dropdowns**
Archive contains a compact format with structured prep summaries. Could be useful for quick-glance meeting cards. Post-MVP.

**I3: Browser extension for web page capture to _inbox/**
Chromium extension for page capture to markdown in `_inbox/`. Aligns with "system does the work." Post-Phase 2 when inbox processing is stable.

**I4: Motivational quotes as personality layer**
Viable placements: overview greeting (daily rotating), empty states ("you crushed it"). Rejected approach: welcome interstitial (adds required click, violates Principle 2).

**I6: Processing history page**
`processing_log` table exists in SQLite with `get_processing_log()`. Missing: Tauri command + UI to render history. Supports Principle 9 (Show the Work).

**I10: No shared glossary of app terms**
Overlapping terms (briefing, workflow, capture, focus, etc.) used inconsistently. Needs shared definitions in DEVELOPMENT.md or a GLOSSARY.md.

**I19: AI enrichment failure not communicated to user**
When Phase 2 fails, briefing renders thin with no indication. Recommended: quiet "AI-enriched" badge (absence = not enriched). Fits Principle 9.

**I37: Dashboard overview text should adapt to day density**
Busy days (8+ meetings) and light days (0-2 meetings) get the same generic overview. The AI-generated overview text should be density-aware: busy day → "Packed day — your 9 AM Acme call is the priority." Light day → "Open afternoon — good day to tackle that overdue Globex proposal." Near-term: tweak Phase 2 enrichment prompt with meeting count context. Long-term: adaptive layout or between-meetings HUD (separate feature). Demoted from ADR-0034 — not an architectural decision.

### Closed

**I1: Config directory naming** — Resolved. Renamed `.daybreak` → `.dailyos`.

**I5: Orphaned pages (Focus, Week, Emails)** — Resolved. All three now have defined roles: Focus = drill-down from dashboard, Week = sidebar item (Phase 2+), Emails = drill-down from dashboard. See ADR-0010.

**I11: Phase 2 email enrichment not fed to JSON** — Resolved. `deliver_today.py` gained `parse_email_enrichment()` which reads `83-email-summary.md` and merges into `emails.json`.

**I12: Email page missing AI context** — Resolved. Email page shows summary, recommended action, conversation arc per priority tier. Removed fake "Scan emails" button.

**I14: Dashboard meeting cards don't link to detail page** — Resolved. MeetingCard renders "View Prep" button linking to `/meeting/$prepFile` when prep exists. Added in Phase 1.5.

**I17: Post-meeting capture outcomes don't resurface in briefings** — Resolved (actions side). Non-briefing actions (post-meeting, inbox) now merge into dashboard via `get_non_briefing_pending_actions()` with title-based dedup. Wins/risks resurfacing split to I33.

**I22: Action completion doesn't write back to source markdown** — Resolved. `sync_completion_to_markdown()` in `hooks.rs` runs during post-enrichment hooks. Queries recently completed actions with `source_label`, writes `[x]` markers back to source files. Lazy writeback is acceptable — SQLite is working store, markdown is archive.

**I24: schedule.json meeting IDs are local slugs, not Google Calendar event IDs** — Resolved. Added `calendarEventId` field alongside the local slug `id` in both `schedule.json` and `preps/*.json`. Local slug preserved for routing/filenames; calendar event ID available for cross-source matching (ADR-0032, ADR-0033).

**I30: Inbox action extraction lacks rich metadata** — Resolved. Added `processor/metadata.rs` with regex-based extraction of priority (`P1`/`P2`/`P3`), `@Account`, `due: YYYY-MM-DD`, `#context`, and waiting/blocked status. Both inbox (Path A) and AI enrichment (Path B) paths now populate all `DbAction` fields. AI prompt enhanced with metadata token guidance. Title-based dedup widened to prevent duplicate pending actions. Waiting actions now visible in dashboard query.

**I34: Archive workflow lacks end-of-day reconciliation** — Resolved. Added `workflow/reconcile.rs` with mechanical reconciliation that runs before archive: reads schedule.json to identify completed meetings, checks transcript status in `Accounts/` and `_inbox/`, computes action stats from SQLite, writes `day-summary.json` to archive directory and `next-morning-flags.json` to `_today/` for tomorrow's briefing. Pure Rust, no AI (ADR-0040).

**I23: No cross-briefing action deduplication** — Resolved. Three layers: (1) `action_parse.py` SQLite pre-check (`_load_existing_titles()`) skips known titles during Phase 1 parsing. (2) `deliver_today.py` `_make_id()` uses category-agnostic `action-` prefix so the same action gets the same ID regardless of overdue/today/week bucket, plus within-briefing dedup by ID, plus its own SQLite pre-check before JSON generation. (3) Rust-side `upsert_action_if_not_completed()` title-based dedup as final guard.

**I33: Captured wins/risks don't resurface in meeting preps** — Resolved. ADR-0030 `meeting_prep.py` queries `captures` table via `_get_captures_for_account()` for recent wins/risks by account_id (14-day lookback). Also queries open actions and meeting history per account. Rust `db.rs` gained `get_captures_for_account()` method with test.

**I38: Deliver script decomposition** — Resolved. ADR-0042 Chunk 1 replaces deliver_today.py with Rust-native per-operation delivery (`workflow/deliver.rs`). Chunk 3 adds AI enrichment ops: `deliver_emails()` (mechanical email mapping), `enrich_emails()` (PTY-spawned Claude for summaries/actions/arcs per high-priority email), `enrich_briefing()` (PTY-spawned Claude for 2-3 sentence day narrative patched into schedule.json). All AI ops are fault-tolerant — if Claude fails, mechanical data renders fine. Pipeline: Phase 1 Python → mechanical delivery (instant) → partial manifest → AI enrichment (progressive) → final manifest. Week delivery (deliver_week.py) remains monolithic (ADR-0042 Chunk 6).

**I36: Daily impact rollup for CS extension** — Resolved. Added `workflow/impact_rollup.rs` with `rollup_daily_impact()` that queries today's captures from SQLite, groups wins/risks by account, and appends to `Weekly-Impact/{YYYY}-W{WW}-impact-capture.md`. Runs in archive workflow between reconciliation and file moves, profile-gated to `customer-success`, non-fatal. Idempotent (day header check prevents double-writes). Creates file with template if missing. `db.rs` gained `get_captures_for_date()`. 9 new tests.

**I45: Post-transcript outcome interaction UI** — Resolved. `MeetingOutcomes.tsx` renders AI-extracted summary, wins, risks, decisions, and actions inside MeetingCard's collapsible area. Actions: checkbox completion (`complete_action`/`reopen_action`), priority cycling via `update_action_priority` command. Wins/risks/decisions: inline text editing via `update_capture` command. All changes write to SQLite (working store). Markdown writeback for actions already exists via `sync_completion_to_markdown` hook; non-action capture edits stay SQLite-only (consistent with ADR-0018 — SQLite is disposable cache, originals from transcript processing are the markdown source of truth).

**I44: Meeting-scoped transcript intake from dashboard** — Resolved. ADR-0044. Both surfaces have transcript attachment: `PostMeetingPrompt` file picker and `MeetingCard` attach affordance. `processor/transcript.rs` handles full pipeline — frontmatter stamping, AI enrichment via Claude (`--print`), extraction of summary/actions/wins/risks/decisions, routing to account location. Immutability enforced via `transcript_processed` state map. Frontend: `MeetingOutcomes.tsx` + `useMeetingOutcomes.ts`. Meeting card is now a lifecycle view: prep → current → outcomes.

**I32: Inbox processor doesn't update account intelligence** — Resolved. AI enrichment prompt extracts WINS/RISKS sections. Post-enrichment `entity_intelligence` hook writes captures (with synthetic `inbox-{filename}` meeting IDs) and touches `accounts.updated_at` as last-contact signal. Read side (`get_captures_for_account`) + write side both wired.

**I47: Profile-agnostic entity abstraction** — Resolved. Introduced `entities` table and `EntityType` enum (ADR-0045). Bridge pattern: `upsert_account()` auto-mirrors to entities table, backfill migration populates from existing accounts on DB open. `entity_intelligence()` hook replaces profile-gated `cs_account_intelligence()` — now runs for all profiles as core behavior (ADR-0043). `account_id` FK migration deferred to I27.

**I42: CoS executive intelligence layer** — Resolved. New `intelligence.rs` module computes five signal types from existing SQLite data + schedule: decisions due (AI-flagged `needs_decision` actions ≤72h), stale delegations (waiting actions >3 days), portfolio alerts (renewals ≤60d, stale contacts >30d, CS-only), cancelable meetings (internal + no prep), skip-today (AI enrichment). New `IntelligenceCard.tsx` renders signal counts as badges with expandable detail sections. Schema migration adds `needs_decision` column. 13 new tests.

**I43: Stakeholder context in meeting prep** — Resolved. `db.rs` gained `get_stakeholder_signals()` which computes meeting frequency (30d/90d), last contact, relationship temperature (hot/warm/cool/cold), and trend (increasing/stable/decreasing) from `meetings_history` and `accounts` tables. Signals computed live at prep load time in `get_meeting_prep` command (always fresh from SQLite, not baked into prep files). `RelationshipContext` component in `MeetingDetailPage.tsx` shows four-metric grid. 5 new tests.

**I41: Reactive meeting:prep wiring** — Resolved. `google.rs` calendar poller now generates lightweight prep JSON for new prep-eligible meetings (customer/qbr/partnership) after each poll cycle. Checks both meeting ID and calendar event ID to avoid duplicates. Enriches preps from SQLite account data (Ring, ARR, Health, Renewal, open actions). Emits `prep-ready` event; `useDashboardData` listens for silent refresh. Rust-native (ADR-0025), no Python subprocess. 8 new tests.

**I31: Inbox transcript summarization** — Resolved. `enrich.rs` gained `detect_transcript()` heuristic (filename keywords, speaker label ratio >40%, timestamp ratio >20%, minimum 10 lines) and richer enrichment prompt for transcripts: 2-3 sentence executive summary + discussion highlights block. Parser handles `DISCUSSION:` / `END_DISCUSSION` markers. Non-transcript files unchanged (backward compatible). 12 enrich tests.

**I46: Meeting prep context limited to customer/QBR/training meetings** — Resolved. `meeting_prep.py` only gathered rich context (SQLite history, captures, open actions) for customer meetings with account-based queries. Internal syncs, 1:1s, and partnership meetings got at most a single archive ref. Per ADR-0043 (meeting intelligence is core), expanded with title-based SQLite queries (`_get_meeting_history_by_title`, `_get_captures_by_meeting_title`, `_get_all_pending_actions`) so all non-personal/non-all-hands types get meeting history, captures, and actions context. 1:1s get deeper lookback (60-day history, 3 archive refs). Partnership meetings try account match first, fall back to title-based. No schema or orchestrator changes.

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |

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
