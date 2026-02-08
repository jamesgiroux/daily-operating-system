# DailyOS Roadmap

> What's built, what's next, and how we get to ship.

---

## Where We Are (2026-02-07)

**The app works end-to-end from first launch.** Onboarding wizard guides new users through entity mode selection, workspace setup, and optional Google auth. Briefing generates automatically, dashboard renders schedule/actions/emails, meeting prep provides deep context for all meeting types, post-meeting capture processes transcripts into outcomes, inbox processes files, archive cleans up at midnight. No terminal required, no manual config editing.

### What's Built

| Capability | Status | Key ADRs |
|------------|--------|----------|
| **First-run onboarding wizard** | **Working** | **0046** |
| **Entity-mode architecture (account / project / both)** | **Working** | **0046** |
| **Workspace scaffolding (entity-mode-aware)** | **Working** | **0046** |
| **Settings: workspace picker, entity-mode switcher, schedule editor** | **Working** | — |
| **No-auth graceful degradation (Connect Google CTA)** | **Working** | — |
| Daily briefing pipeline (prepare → enrich → deliver) | Working | 0006, 0042 |
| Per-operation Rust-native delivery | Working | 0042 |
| AI enrichment (emails, briefing narrative) | Working, fault-tolerant | 0042 |
| Meeting prep for all meeting types | Working | 0043, 0046 |
| Post-meeting transcript intake + outcome extraction | Working | 0037, 0044 |
| Outcome interaction UI (action completion, capture editing) | Working | 0045 |
| Inbox processing with entity intelligence | Working | 0045 |
| Daily impact rollup | Working | 0041 |
| Archive with reconciliation | Working | 0040 |
| Entity abstraction (profile-agnostic) | Working | 0045 |
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
| macOS chrome (overlay titlebar, tray icon, app icon) | Working | — |
| Devtools demo data (8 meeting states, 3 preps, week overview, calendar overlay, transcript outcomes) | Working | — |

**176 Rust tests + 37 Python tests passing.** Sprints 1–3 complete. Next: Sprint 4 (distribution).

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

### Sprint 4: "Ship It"

**Milestone:** Someone outside the dev team can download, install, and use DailyOS.

| Issue | What | Status |
|-------|------|--------|
| I8 | Distribution mechanism (DMG + notarization, or GitHub Releases) | |
| I9 | Focus/Week stubs (make non-embarrassing — "coming soon" > broken stub) | |
| I56 | Onboarding redesign (educational flow, demo data, dashboard tour) | In progress — OnboardingFlow.tsx + demo data fixtures complete, wiring remaining |
| I57 | Onboarding: add accounts/projects + user domain (populate workspace for first briefing) | |
| — | 7-day crash-free validation on test workspace | |
| — | README / landing page for first external users | |

**Done when:** A DMG installs cleanly, onboarding completes, first real briefing has meeting-entity associations on a clean machine.

---

## Parking Lot

These are decided (ADRs exist) but not scheduled. Entity-mode architecture (ADR-0046) replaces the profile/extension model with entity modes + Kits + Intelligence + integrations.

### Entity-Mode Architecture (I27 umbrella)

| Issue | What | Type | Blocked by |
|-------|------|------|------------|
| I27 | Entity-mode architecture umbrella | — | — (Phase gate) |
| I50 | Projects overlay table + project entity support | Foundation | I27 |
| I51 | People sub-entity table + relationships | Foundation | I27 |
| I52 | Meeting-entity many-to-many (replaces account_id FK) | Foundation | I50 |
| I53 | Entity-mode config, onboarding, UI adaptation | Foundation | I50, I52 |
| I54 | MCP client integration framework (Gong, Salesforce, Linear) | Integration | I27 |
| I40 | CS Kit — account-mode fields, templates, vocabulary | Kit | I27 |
| I55 | Executive Intelligence — decision framing, delegation, strategy | Intelligence | I27 |
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

**When to revisit:** After Sprint 4 ships and we have real usage data. ADR-0046 accepted — architecture designed, implementation sequencing TBD based on user demand.

---

## Risks and Dependencies

Tracked in [BACKLOG.md](BACKLOG.md) (R1-R4, D1-D3). Not duplicated here.

---

*Decisions: [docs/decisions/](decisions/README.md). Issues: [BACKLOG.md](BACKLOG.md). Code is the proof of what's done.*
