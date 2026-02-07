# DailyOS Roadmap

> What's built, what's next, and how we get to ship.

---

## Where We Are (2026-02-07)

**The app works.** Briefing generates automatically, dashboard renders schedule/actions/emails, meeting prep provides deep context for all meeting types, post-meeting capture processes transcripts into outcomes, inbox processes files, archive cleans up at midnight. No terminal required.

### What's Built

| Capability | Status | Key ADRs |
|------------|--------|----------|
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
| Sidebar nav, profile-aware UI | Working | 0038 |
| Feature toggles (per-operation, profile-conditional defaults) | Working | 0039 |
| Standalone email refresh | Working | 0030 |
| FYI email classification (bulk senders, noreply, headers) | Working | — |
| Density-aware briefing narrative | Working | — |
| Processing history page | Working | — |
| Google API credential caching (per-process) | Working | — |
| macOS chrome (overlay titlebar, tray icon, app icon) | Working | — |

**155 Rust tests + 37 Python tests passing.** Core data pipeline is solid.

### What's Untested End-to-End

Everything above works in James's `~/Documents/VIP/` workspace. Nothing has been validated against a clean workspace, a first-time user, or a machine without pre-existing config. The gap is not "what to build" but **"does it work for someone who isn't the developer?"**

---

## Sprint Plan

Goal: get from working prototype to shippable product. Each sprint has a concrete, testable "done" milestone.

### Sprint 1: "First Run to Working Briefing"

**Milestone:** A fresh `~/Documents/test-workspace/` goes from app launch → onboarding → first briefing → rendered dashboard. No hand-editing config files. All three entity modes work. Both Google-authed and no-auth paths work.

**Code audit findings (2026-02-07):** The app has no workspace initialization, no config auto-creation, and Google auth is buried in Settings. The Python pipeline already handles missing Google auth gracefully (returns empty data). The real gaps are frontend signaling and first-run infrastructure.

**ADR-0046 integration:** Onboarding asks "How do you organize your work?" (account-based / project-based / both) instead of choosing a profile. Config gains `entity_mode` field; `profile` kept for backend compat (derived from entity mode). Workspace scaffolding creates `Accounts/` only for account/both modes. Sidebar renders Accounts and Projects as peers.

#### Phase A: Foundation (prerequisites for onboarding)

| Issue | What | Notes |
|-------|------|-------|
| — | Shared infrastructure: `create_or_update_config` helper + `entity_mode` config field | Unlocks all config-writing commands. Handles "no config exists yet" case. |
| I48 | Workspace scaffolding — create dirs when workspace path is set | Entity-mode-aware: `Accounts/` only for account/both modes. |
| I49 | No-auth graceful degradation — dashboard shows "Connect Google" CTA | Python already handles no-auth. Frontend needs auth status in DashboardResult. |
| I7 | Settings: workspace path picker (directory dialog + validation) | Small scope. Calls workspace scaffolding on change. |
| I15 | Settings: entity-mode switcher (account / project / both) | Replaces profile switcher per ADR-0046. |
| I16 | Settings: schedule editing (time picker, writes cron, hides syntax) | Small scope. |

All Phase A issues run in **parallel** (independent features on shared infrastructure).

#### Phase B: Onboarding (depends on Phase A)

| Issue | What | Notes |
|-------|------|-------|
| I13 | Onboarding wizard: entity mode → workspace → Google auth → first briefing | Depends on all Phase A. Uses `set_entity_mode` + `set_workspace_path` + existing auth flow. |

**Three test paths:**
1. **Account-based + Google auth:** Onboarding → select account-based → set workspace → connect Google → generate briefing → full dashboard
2. **Project-based + no Google:** Onboarding → select project-based → set workspace → skip Google → dashboard with "Connect Google" CTA
3. **Both mode:** Onboarding → select both → workspace gets `Accounts/` + `Projects/` → sidebar shows both sections

#### Phase C: Polish (fills gaps while B is in progress)

| Issue | What |
|-------|------|
| I25 | Meeting badge/status unification (MeetingDisplayState refactor) |
| I19 | AI enrichment failure indicator (quiet badge, Principle 9) |

#### Design decisions resolved

1. **Default workspace path** — `~/Documents/DailyOS/` (user can change during onboarding)
2. **Directory structure** — Pipeline dirs (`_today/`, `_inbox/`, `_archive/`) always. `Projects/` always (core PARA). `Accounts/` conditional on entity mode.
3. **Google auth** — Optional with degraded experience. Dashboard shows clear "Connect Google" CTA.
4. **Entity mode replaces profile** — ADR-0046. `profile` field derived from entity mode for backend compat during Sprint 1.

#### Open design question

5. **First briefing content** — What does a briefing look like with no historical data? Graceful "welcome" state vs. minimal real data?

**Done when:** All three test paths work end-to-end in `~/Documents/test-workspace/`. Onboarding completes without manual config editing. Dashboard renders meaningfully in both authed and unauthed states.

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

| Issue | What |
|-------|------|
| I8 | Distribution mechanism (DMG + notarization, or GitHub Releases) |
| I9 | Focus/Week stubs (make non-embarrassing — "coming soon" > broken stub) |
| — | 7-day crash-free validation on test workspace |
| — | README / landing page for first external users |

**Done when:** A DMG installs cleanly, onboarding completes, briefing works for 7 consecutive days on a clean machine.

---

## Parking Lot

These are decided (ADRs exist) but not scheduled. Entity-mode architecture (ADR-0046) replaces the profile/extension model with entity modes + integrations + domain overlays.

### Entity-Mode Architecture (I27 umbrella)

| Issue | What | Blocked by |
|-------|------|------------|
| I27 | Entity-mode architecture umbrella | — (Phase gate) |
| I50 | Projects overlay table + project entity support | I27 |
| I51 | People sub-entity table + relationships | I27 |
| I52 | Meeting-entity many-to-many (replaces account_id FK) | I50 |
| I53 | Entity-mode config, onboarding, UI adaptation | I50, I52 |
| I54 | MCP client integration framework (Gong, Salesforce, Linear) | I27 |
| I40 | CS domain overlay — account-mode vocabulary + schemas | I27 |
| I35 | ProDev domain overlay — personal impact, career narrative | I27 |
| I29 | Structured document schemas | I27 |
| I28 | MCP server + client (I54 covers client side) | — (Phase gate) |

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
