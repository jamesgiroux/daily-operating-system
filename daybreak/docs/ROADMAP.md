# DailyOS Roadmap

> What's built, what's next, and how we get to ship.

---

## Where We Are (2026-02-08)

**The app works end-to-end from first launch.** Onboarding teaches the philosophy (9-chapter educational flow with demo data, inbox training, Claude Code validation), workspace population gives the system entities to link against, and the briefing generates automatically. Dashboard renders schedule/actions/emails, meeting prep provides AI-synthesized agendas with people dynamics, post-meeting capture processes transcripts into outcomes, inbox processes files, archive cleans up at midnight. Security hardening (atomic writes, path validation, timeout enforcement) covers the critical paths. No terminal required, no manual config editing.

### What's Built

| Capability | Status | Key ADRs |
|------------|--------|----------|
| First-run onboarding (9-chapter educational flow, demo data, inbox training, Claude Code validation) | Working | 0046 |
| Entity-mode architecture (account / project / both) | Working | 0046 |
| Workspace scaffolding + population (entity-mode-aware) | Working | 0046 |
| Settings: workspace picker, entity-mode switcher, schedule editor | Working | — |
| No-auth graceful degradation (Connect Google CTA) | Working | — |
| Daily briefing pipeline (prepare → enrich → deliver) | Working | 0006, 0042 |
| Per-operation Rust-native delivery | Working | 0042 |
| AI enrichment (emails, briefing narrative) with user profile context | Working, fault-tolerant | 0042 |
| Meeting prep with proposed agenda + people dynamics | Working | 0043, 0046 |
| Copy-to-clipboard for meeting prep (full + per-section) | Working | — |
| Post-meeting transcript intake + outcome extraction | Working | 0037, 0044 |
| Outcome interaction UI (action completion, capture editing) | Working | 0045 |
| Inbox processing with entity intelligence | Working | 0045 |
| Daily impact rollup | Working | 0041 |
| Archive with reconciliation | Working | 0040 |
| Entity abstraction (profile-agnostic) | Working | 0045 |
| People sub-entity (universal tracking, signals, entity links, file I/O) | Working | 0048 |
| Account dashboards (two-file JSON+md pattern, list + detail views) | Working | 0047 |
| Executive intelligence (decisions, delegations, portfolio alerts) | Working | 0043 |
| Stakeholder context in meeting prep (frequency, temperature, trend) | Working | 0043 |
| Reactive meeting prep from calendar polling | Working | — |
| Transcript-aware inbox enrichment (richer summaries) | Working | — |
| Cross-briefing action dedup (3 layers) | Working | — |
| Sidebar nav, entity-mode-aware UI | Working | 0038, 0046 |
| Feature toggles (per-operation, profile-conditional defaults) | Working | 0039 |
| Standalone email refresh | Working | 0030 |
| FYI email classification (bulk senders, noreply, headers) | Working | — |
| Density-aware briefing narrative | Working | — |
| Processing history page | Working | — |
| Google API credential caching (per-process) | Working | — |
| Security hardening (path traversal guards, atomic writes, script timeouts) | Working | — |
| Account enrichment via Claude Code websearch (on-demand company research) | Working | 0047 |
| SQLite backup + rebuild-from-filesystem | Working | 0048 |
| External edit detection for accounts (file watcher + sync) | Working | 0047, 0048 |
| RwLock for read-heavy AppState fields (config, calendar, status) | Working | — |
| Edge hardening (atomic preps, safe router, TOCTOU fix, scheduler widen) | Working | — |
| macOS chrome (overlay titlebar, tray icon, app icon) | Working | — |

**224 Rust tests + 37 Python tests passing.** Sprints 1–7 complete. Next: Sprint 8 (kill Python), then Sprint 9 (distribute).

---

## Sprint Plan

Goal: get from working prototype to shippable product. Each sprint has a concrete, testable "done" milestone.

### Sprint 1: "First Run to Working Briefing" — COMPLETE

**Milestone:** A fresh workspace goes from app launch → onboarding → first briefing → rendered dashboard. No hand-editing config files. All three entity modes work. Both Google-authed and no-auth paths work.

| Issue | What | Status |
|-------|------|--------|
| — | Shared infrastructure: `create_or_update_config` helper + `entity_mode` config field | Done — `state.rs`, handles "no config" case |
| I48 | Workspace scaffolding — entity-mode-aware dir creation | Done — `initialize_workspace()`, 4 tests |
| I49 | No-auth graceful degradation — dashboard "Connect Google" CTA | Done — `google_auth` in DashboardResult, DashboardEmpty CTA |
| I7 | Settings: workspace path picker (directory dialog + validation) | Done — `set_workspace_path` command, WorkspaceCard |
| I15 | Settings: entity-mode switcher (account / project / both) | Done — `set_entity_mode` command, EntityModeCard |
| I16 | Settings: schedule editing (human-readable time display) | Done — `set_schedule` command, `cronToHumanTime()` |
| I13 | Onboarding wizard: entity mode → workspace → Google → first briefing | Done — `OnboardingWizard.tsx`, replaces ProfileSelector |

Phase C (I25 badge unification, I19 enrichment badge) deferred — low priority polish, can land in any sprint.

**Design decisions resolved:** Default workspace `~/Documents/DailyOS/`, entity mode replaces profile (ADR-0046), Google auth optional with clear CTA, `Accounts/` conditional on entity mode.

---

### Sprint 2: "Make it Smarter" — COMPLETE

**Milestone:** The briefing surfaces executive intelligence and stakeholder context. Meeting prep triggers reactively from calendar changes.

| Issue | What | Status |
|-------|------|--------|
| I42 | CoS executive intelligence layer (decisions, delegations, portfolio alerts, cancelable meetings) | Done — `intelligence.rs`, `IntelligenceCard.tsx`, 13 tests |
| I43 | Stakeholder context in meeting prep (frequency, temperature, trend from SQLite) | Done — `db.rs` signals, `RelationshipContext` in prep detail, 5 tests |
| I41 | Reactive meeting:prep wiring (calendar polling → lightweight prep generation) | Done — `google.rs` prep generation from SQLite, `prep-ready` event, 8 tests |
| I31 | Inbox transcript summarization (richer enrichment with discussion highlights) | Done — `enrich.rs` transcript detection + rich prompts, 12 tests |

All 168 Rust tests passing.

---

### Sprint 3: "Make it Reliable" — COMPLETE

**Milestone:** Pipeline handles partial failures gracefully. Users can refresh individual data sources. System communicates what's stale.

| Issue | What | Status |
|-------|------|--------|
| I39 | Feature toggle runtime (config + orchestrator checks + Settings UI) | Done — `is_feature_enabled()` priority chain, FeaturesCard in Settings, 7 tests |
| I18 | Google API credential caching (per-process cache for concurrent callers) | Done — `_cached_credentials` + `_cached_services` in config.py |
| I20 | Standalone email refresh (thin orchestrator for email_fetch) | Done — `refresh_emails.py`, executor + command + UI refresh button |
| I21 | FYI email classification (expand low-priority signals) | Done — bulk domains, noreply, List-Unsubscribe/Precedence headers, 16 tests |
| I37 | Density-aware dashboard overview (enrichment prompt with meeting count) | Done — `classify_meeting_density()`, prompt injection, 4 tests |
| I6 | Processing history page (table exists, needs command + UI) | Done — `get_processing_history` command, HistoryPage.tsx, sidebar nav |

All 155 Rust + 37 Python tests passing.

---

### Sprint 4a: "Entity Intelligence" — COMPLETE

**Milestone:** People and account dashboards are first-class entities with full CRUD, file I/O, and UI.

| Issue | What | Status |
|-------|------|--------|
| I51 | People sub-entity (tables, signals, file I/O, UI) | Done — `people.rs`, PeoplePage, PersonDetailPage |
| I72+I73 | Account dashboards (two-file JSON+md pattern, list + detail) | Done — `accounts.rs`, AccountsPage, AccountDetailPage |
| I59 | Runtime script path resolution for release builds | Done — `resolve_scripts_dir()` with Tauri resource resolver |
| I56 | Onboarding redesign (80% — educational flow, demo data, dashboard tour) | Done — `OnboardingFlow.tsx`, 7 chapters |

189 Rust tests passing.

---

### Sprint 5: "Complete the App" — COMPLETE

**Milestone:** Onboarding teaches the full paradigm (inbox behavior, Claude Code dependency). Security hardening covers critical paths. Meeting prep gains AI-synthesized agendas and people dynamics.

| Issue | What | Status |
|-------|------|--------|
| I56 | Onboarding finish (wire PopulateWorkspace to Tauri commands) | Done |
| I57 | Populate workspace (accounts/projects + userDomain) | Done — `populate_workspace` command |
| I78 | Inbox-first behavior training (onboarding chapter) | Done — `InboxTraining.tsx` |
| I79 | Claude Code validation/installation step | Done — `ClaudeCode.tsx`, `check_claude_status` command |
| I58 | User profile context in enrichment prompts | Done — `UserContext` struct, injected into enrich ops |
| I60 | Path traversal guards | Done — `validate_inbox_path()`, `validate_entity_name()` in util.rs |
| I62 | .unwrap() panic elimination in JSON mutation paths | Done — all production paths use safe alternatives |
| I63 | Script timeout enforcement | Done — `run_python_script()` uses spawn + recv_timeout |
| I64 | Atomic file writes | ~90% — `atomic_write()` helper used widely; `write_json()` in deliver.rs gap carries to S6 |
| I65 | Impact log append race | ~50% — fixed in transcript.rs; commands.rs `append_to_impact_log()` gap carries to S6 |
| I80 | Proposed agenda in meeting prep (pulled forward from S7) | Done — `generate_mechanical_agenda()`, AI refinement via `enrich_preps()` |
| I81 | People dynamics in meeting prep UI (pulled forward from S7) | Done — "People in the Room" component with temperature badges |
| I82 | Copy-to-clipboard for meeting prep | Done — full + per-section copy, `useCopyToClipboard` hook |

199 Rust tests passing.

---

### Sprint 6+7: "Harden & Enrich" (combined) — COMPLETE

**Milestone:** Zero crash paths, all writes safe, account enrichment, SQLite durability, RwLock performance. Sprints combined because S6 was smaller than projected.

| Track | Issues | What | Status |
|-------|--------|------|--------|
| Safety | I61, I64, I65, I66, I67, I69, I70, I71 | Atomic writes, safe preps, TOCTOU fix, scheduler widen, router dedup, sanitize, edge hardening | Done |
| Polish | I19, I25 | MeetingCard badge unification + "Limited prep" indicator | Done |
| Intelligence | I74 | Account enrichment via Claude Code websearch + Enrich button | Done |
| Durability | I75, I76, I77 | External edit detection, SQLite backup/rebuild, writeback audit | Done |
| Performance | I68 | Mutex → RwLock for config/calendar/status/last_scheduled_run | Done |

224 Rust tests passing. I55 (Executive Intelligence) deferred to parking lot — needs prompt fragment mechanism (I27).

---

### Sprint 8: "Kill Python"

**Milestone:** Python runtime is eliminated. The app is a single Rust binary with no external language dependencies (Claude Code CLI remains the only external tool).

| Issue | What | Blocked by |
|-------|------|------------|
| I83 | Rust-native Google API client (`google_api.rs` — reqwest + OAuth2) | — |
| I84 | Port Phase 1 operations (classification, email priority, action parsing, prep context) | I83 |
| I85 | Port orchestrators + delete `scripts/` + remove `run_python_script()` | I84 |

**Done when:** `scripts/` directory deleted, `run_python_script()` removed from `pty.rs`, no Python on `$PATH` required, all Rust tests pass, onboarding no longer checks for Python. ADR-0049.

---

### Sprint 9: "Distribute"

**Milestone:** Colleagues can download, install, and use DailyOS.

| Issue | What |
|-------|------|
| I8 | DMG build + GitHub Actions CI + GitHub Releases |
| — | 7-day crash-free validation on clean machine |
| — | README for colleague installs (Gatekeeper bypass, Google OAuth setup) |

**Done when:** Unsigned arm64 DMG installs cleanly on a clean Mac, onboarding → first real briefing works end-to-end. No signing/notarization (no Apple Developer account). No updater (zero users, premature).

---

## Parking Lot

These are decided (ADRs exist) but not scheduled. Entity-mode architecture (ADR-0046) replaces the profile/extension model with entity modes + Kits + Intelligence + integrations.

### Entity-Mode Architecture (I27 umbrella)

| Issue | What | Type | Blocked by |
|-------|------|------|------------|
| I27 | Entity-mode architecture umbrella | — | — (Phase gate) |
| I50 | Projects overlay table + project entity support | Foundation | I27 |
| I52 | Meeting-entity many-to-many (replaces account_id FK) | Foundation | I50 |
| I53 | Entity-mode config, onboarding, UI adaptation | Foundation | I50, I52 |
| I54 | MCP client integration framework (Gong, Salesforce, Linear) | Integration | I27 |
| I40 | CS Kit — account-mode fields, templates, vocabulary | Kit | I27 |
| I35 | ProDev Intelligence — personal impact, career narrative | Intelligence | I27 |
| I29 | Structured document schemas | Foundation | I27 |
| I28 | MCP server + client (I54 covers client side) | Integration | — (Phase gate) |

### Deferred

| Issue | What |
|-------|------|
| I26 | Web search for unknown external meetings |
| I2 | Compact meetings format |
| I3 | Browser extension for web capture |
| I4 | Motivational quotes |
| I10 | Shared glossary of app terms |

**When to revisit:** After Sprint 9 ships and we have real usage data. ADR-0046 accepted — architecture designed, implementation sequencing TBD based on user demand.

---

## Risks and Dependencies

Tracked in [BACKLOG.md](BACKLOG.md) (R1-R4, D1-D3). Not duplicated here.

---

*Decisions: [docs/decisions/](decisions/README.md). Issues: [BACKLOG.md](BACKLOG.md). Code is the proof of what's done.*
