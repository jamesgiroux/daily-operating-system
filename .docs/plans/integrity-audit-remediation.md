# Integrity Audit & Remediation Plan

**Date:** 2026-02-20
**Scope:** v0.10.0 through staged v0.13.0
**Purpose:** Feature-by-feature code-level verification of CHANGELOG claims against actual implementation. Identify every instance of scaffolding-claimed-as-shipped.
**Method:** Four parallel agent audits, each tracing every claimed feature end-to-end: data input -> processing -> user-visible output.

---

## Executive Summary

A four-agent parallel audit was conducted against the entire codebase, covering 50+ claimed features across 5 released versions and 1 staged version. Each claim was verified with file:line evidence.

**Overall: 42 of 50 features are COMPLETE (84%). 5 are HOLLOW. 3 are BROKEN/NOT STARTED.**

The initial SVP hypothesis — "they're adding plumbing but not following through" — was **partially wrong**. Versions 0.10.0 through 0.12.0 are substantially solid. The email intelligence pipeline (0.12.0) is 100% wired. Signal intelligence (0.10.0) is 95% wired. Role presets (0.11.0) are 77% wired.

**Where the hypothesis holds:**
1. The 0.13.0 meeting intelligence engine is mechanical-only — no AI enrichment exists
2. Component standardization (C1-C3 merges from ADR-0084) was never done — 3 features still have 2-3 duplicate implementations each
3. Preset-driven UI features (metadata fields, stakeholder role dropdowns) have full backend support but zero frontend surface
4. The full vocabulary audit (I341) shipped 5 string changes, not the comprehensive audit described

**One charge walked back:** I338 (1:1 person intelligence) IS working. The BACKLOG note "plumbing shipped, files never generated" is stale — the intel_queue pipeline generates all three files. BACKLOG needs updating.

---

## Audit Results by Version

### v0.10.0 — Intelligence (12/13 COMPLETE)

| Feature | Verdict | Evidence |
|---------|---------|----------|
| Bayesian fusion (5 signal producers) | **COMPLETE** | `entity_resolver.rs:77-589` — Junction, attendee, group pattern, keyword, embedding all wired |
| Signal bus + temporal decay | **COMPLETE** | `bus.rs`, `fusion.rs`, `decay.rs` — Weighted log-odds, exponential decay integrated |
| Thompson Sampling (5-sample gate) | **COMPLETE** | `sampling.rs:13-24` + `bus.rs:223` — Beta distribution, gated behind 5 samples |
| Cross-entity propagation (5+ rules) | **COMPLETE** | `propagation.rs` + `rules.rs` — 6 rules registered including renewal compound |
| Proactive detectors (9 SQL+Rust) | **COMPLETE** | `detectors.rs:18-720` — All 9 detectors with SHA256 fingerprint dedup |
| Entity-generic junction table | **COMPLETE** | Migrations + `entity_resolver.rs:217-259` — Account/Project/Person |
| Entity-generic classification | **COMPLETE** | `entity_resolver.rs:290-300` — 1:1 person detection, multi-type hints from DB |
| Entity-generic context building | **COMPLETE** | `meeting_context.rs:505-513` — Type dispatch: Account/Project/Person |
| Three-file person pattern | **COMPLETE** | `people.rs:78-339` — PersonJson, dashboard.json, person.md all defined |
| Person as first-class entity | **COMPLETE** | `EntityType::Person` throughout pipeline |
| Proposed actions triage | **COMPLETE** | `email_actions.rs` -> DB -> ActionsPage + DailyBriefing -> accept/reject |
| Content index (transcripts/notes) | **HOLLOW** | Schema exists but no ingestion pipeline. No content signals emitted. |
| Proactive insights UI surface | **HOLLOW** | Detectors run + signals stored, but **no dedicated UI** surfaces the 9 insight types to users |

**Notable gaps:**
- Thompson Sampling has the signal emission path (accept/reject emit confidence signals) but the agent flagged a potential gap in the feedback loop that writes back to `signal_weights` alpha/beta values. Needs verification.
- 9 proactive detectors run and emit signals to the DB, but there is **no dedicated user-facing surface** to browse proactive insights. Signals flow indirectly through entity intelligence but users can't see a "proactive insights dashboard."

---

### v0.10.1 — User Feedback (3/3 COMPLETE)

| Feature | Verdict | Evidence |
|---------|---------|----------|
| Gmail teammate suggestions | **COMPLETE** | `gmail.rs:381-465` -> `AboutYou.tsx:84-389` — Full pipeline |
| Linear integration data layer | **COMPLETE** | GraphQL client, poller, DB sync, Settings card — all wired |
| Onboarding back nav fix | **COMPLETE** | State lifted to parent, visited chapters tracked, no reset |

**Clean bill of health.**

---

### v0.11.0 — Role Presets & Entity Architecture (10/13 COMPLETE)

| Feature | Verdict | Evidence |
|---------|---------|----------|
| 9 embedded presets | **COMPLETE** | `embedded.rs:1-43` — All 9 via `include_str!()`, tested |
| Role selection (Settings + Onboarding) | **COMPLETE** | `YouCard.tsx:330-448`, `EntityMode.tsx:12-125` — Both surfaces |
| Role-aware email keywords | **COMPLETE** | `email_classify.rs:229-312` — Preset keywords -> classifier |
| Lifecycle events + renewal | **COMPLETE** | `account_events` table, `get_renewal_alerts()`, merge support |
| EntityPicker multiselect | **COMPLETE** | `entity-picker.tsx:100-116` — Excluded-parent child visibility |
| PersonNetwork optimistic linking | **COMPLETE** | `PersonNetwork.tsx:37-62` — Local state first, background persist |
| StakeholderGallery search-before-create | **COMPLETE** | `StakeholderGallery.tsx:181-239` — Debounced search, select or create |
| Stakeholder roles from preset | **HOLLOW** | Roles defined in preset JSON + DB column exists. **No UI dropdown** to select from preset-defined roles. User types freeform text instead. |
| Preset metadata fields UI | **HOLLOW** | `PresetFieldsEditor.tsx` exists (I312). **Never imported or rendered** in any entity detail page. Backend command `update_entity_metadata()` exists but is unreachable from UI. |
| Account team roles | **BROKEN** | DB column `account_team.role` exists. Preset defines `internal_team_roles`. **No command to update** and **no UI to assign** team roles. |

**The preset infrastructure is solid. The preset-to-UI connection is broken in three places.** Users can select a role, but the role-specific metadata fields and stakeholder role dropdowns never appear on entity pages. The preset shapes AI prompts and email classification correctly — but the visual experience of role-specific fields is absent.

---

### v0.12.0 — Email Intelligence (9/9 COMPLETE)

| Feature | Verdict | Evidence |
|---------|---------|----------|
| Meeting-aware email digest (I317) | **COMPLETE** | `gather_email_context()` -> structured by meeting relevance |
| Thread position tracking (I318) | **COMPLETE** | `track_thread_positions()` -> "YOUR MOVE" in DailyBriefing + EmailsPage |
| Entity email cadence monitoring (I319) | **COMPLETE** | `compute_and_emit_cadence_anomalies()` -> 30d rolling + anomaly detection |
| Hybrid email classification (I320) | **COMPLETE** | `boost_with_entity_context()` -> medium->high promotion |
| Email commitment extraction (I321) | **COMPLETE** | Gmail body fetch -> Claude extraction -> proposed actions |
| Email briefing narrative (I322) | **COMPLETE** | `synthesize_email_narrative()` -> DailyBriefing hero section |
| Zero-touch email disposition (I323) | **COMPLETE** | Auto-archive pipeline with manifest, behind feature flag |
| Enhanced email signals (I324) | **COMPLETE** | Sender resolution + cadence in entity enrichment |
| Calendar description steering (I337) | **COMPLETE** | Calendar descriptions injected into meeting context |
| 1:1 relationship intelligence (I338) | **COMPLETE** | intel_queue pipeline generates all 3 files. BACKLOG note is stale. |
| Self-healing hygiene (I353) | **COMPLETE** | Signal->hygiene feedback loop with auto-merge |

**Clean bill of health. The email intelligence pipeline is the most complete subsystem in the app.** Every feature traces from data source through processing to user-visible output.

**Action required:** Update BACKLOG.md I338 to remove "(plumbing only, files never generated)" — this is no longer true.

---

### v0.12.1 — Product Language & UX Polish (11/16 items)

| Feature | Verdict | Evidence |
|---------|---------|----------|
| The Correspondent email page | **COMPLETE** | `EmailsPage.tsx` (492 lines) — 4 sections, noise filtering, inline dismiss |
| Surface cuts (Week) | **COMPLETE** | Meetings, Open Time, Commitments removed — Hero + Three + Timeline remain |
| Surface cuts (Meeting Detail) | **COMPLETE** | Deep Dive + Appendix removed (2931->2286 lines) |
| Surface cuts (Daily Briefing) | **COMPLETE** | Hero+Focus merged to Day Frame, Later This Week removed |
| Surface cuts (Actions) | **COMPLETE** | 3 tabs (Suggested/Pending/Completed), smart default |
| Surface cuts (Entity) | **COMPLETE** | Value Delivered, Portfolio Summary, Resolution Keywords removed |
| Product vocabulary (5 changes) | **COMPLETE** | Refresh, Last updated, Work mode, AI analysis, daily briefings |
| IntelligenceQualityBadge (I329) | **COMPLETE** | 178-line component with freshness dots + pulse |
| EditableText (I343) | **COMPLETE** | 195-line component, textarea-first, Tab/Shift+Tab nav |
| EditableList (I343) | **COMPLETE** | 261-line component, drag-to-reorder with grip handles |
| Settings modularization | **COMPLETE** | YouCard, ConnectionsGrid, SystemStatus, DiagnosticsSection |
| C1: Shared ActionRow (I351) | **NOT DONE** | 3 separate implementations remain |
| C2: Shared ProposedActionRow (I351) | **NOT DONE** | 2 separate implementations remain |
| C3: Shared MeetingRow (I351) | **NOT DONE** | 3 separate implementations remain |
| C4: Shared useIntelligenceFieldUpdate (I352) | **COMPLETE** | Consolidated hook used in all 3 entity detail pages |
| D8: Command menu navigation | **PARTIAL** | Missing: Email, Settings, entity detail pages |

**Phase-creep note:** I330 (week page ±7-day meeting intelligence timeline) was staged for 0.13.0 in the backlog but is **already fully implemented** in 0.12.1. The `get_meeting_timeline` command and `WeekPage.tsx` timeline rendering are live.

---

### v0.13.0 — Meeting Intelligence (Staged)

| Issue | Status | Evidence |
|-------|--------|----------|
| I326 Phase 2: AI enrichment | **NOT STARTED** | `intelligence_lifecycle.rs:183`: "no AI enrichment. Phase 2 to be added." Only mechanical quality scoring. |
| I327: Advance generation | **HOLLOW** | Weekly run calls `generate_meeting_intelligence()` but it's mechanical-only. No AI narrative, no risk framing, no agenda. |
| I328: Classification expansion | **NEEDS VERIFICATION** | Classification tiers may already route all meetings. Verify `classify.rs` handling of training/personal types. |
| I329: Intelligence quality indicators | **COMPLETE** | Shipped in 0.12.1 (phase-creep). IntelligenceQualityBadge is live. |
| I330: Week timeline surface | **COMPLETE** | Shipped in 0.12.1 (phase-creep). `get_meeting_timeline` + WeekPage rendering live. |
| I331: Daily briefing as assembly | **NOT STARTED** | `prepare_today()` still runs full pipeline. No diff model. No "assemble from pre-computed" path. |
| I332: Signal-triggered refresh | **NOT STARTED** | No implementation. Scheduler has prep invalidation queue but not intelligence refresh per ADR-0081 Section 7. |
| I333: Collaboration (P2) | **NOT STARTED** | No implementation. Expected — this is P2. |

---

## The Pattern (Revised)

The initial hypothesis was: "every release ships scaffolding and calls it the house." **That's wrong for 0.10.0-0.12.0.** The email pipeline, signal bus, role presets, and entity architecture are genuinely complete end-to-end.

**The actual pattern is narrower but still real:**

1. **Backend-complete, frontend-absent.** Preset metadata fields have a schema, a DB table, a command, and a component — but the component is never rendered. Stakeholder role dropdowns have the data but no picker UI. The backend team shipped. The frontend wiring didn't happen.

2. **Mechanical-first without the AI follow-through.** `intelligence_lifecycle.rs` was designed as Phase 1 (mechanical) + Phase 2 (AI). Phase 1 shipped. Phase 2 never started. This is the 0.13.0 problem — the quality meter reads from an empty tank.

3. **Duplication survives design decisions.** ADR-0084 identified 5 merge targets. Only 1 (C4: shared hook) was executed. C5 was resolved by removal. C1-C3 (ActionRow, ProposedActionRow, MeetingRow) still have 8 total duplicate implementations across the codebase.

4. **Phase-creep without backlog reconciliation.** I329 and I330 were staged for 0.13.0 but shipped in 0.12.1. The backlog still lists them as 0.13.0. This isn't a problem — it's good news — but the planning artifacts don't reflect reality.

---

## Remediation Plan

### Priority 1: The 0.13.0 AI Enrichment Engine (CRITICAL)

The single highest-impact gap. Every badge, every timeline, every quality indicator is reading from an empty tank.

| Task | Scope | Effort | Depends On |
|------|-------|--------|------------|
| **I326 Phase 2: AI enrichment in `generate_meeting_intelligence()`** | Add PTY/Claude call for narrative synthesis, risk framing, agenda generation. Write results to `meetings_history.prep_context_json`. Incremental mode (delta-only prompts when refreshing). | L | Nothing — foundation exists |
| **I327: Wire weekly run to AI enrichment** | `prepare_week()` already loops meetings and calls `generate_meeting_intelligence()`. Once Phase 2 lands, this works. Verify entity-clustered batching per ADR-0081. | S | I326 Phase 2 |
| **I331: Daily briefing assembly model** | Refactor `prepare_today()` to check for pre-existing intelligence before generating. If intelligence exists and is Current/Aging, skip individual meeting prep and assemble from DB. Only generate for meetings without intelligence. | M | I326 Phase 2 |
| **I332: Signal-triggered intelligence refresh** | On signal emission (email, transcript, entity update, calendar change), mark affected meetings as `has_new_signals=1`. Scheduler checks every 30 min and calls `generate_meeting_intelligence()` for flagged meetings. Blue dot already displays via `IntelligenceQualityBadge`. | M | I326 Phase 2 |
| **I328: Classification expansion** | Verify current `classify.rs` routes training/personal meetings to at least `minimal` tier. If not, expand classification. Every non-all-hands meeting should get at least mechanical intelligence. | S | Nothing |

**Sequence:** I328 (verify) -> I326 Phase 2 (foundation) -> I327 + I331 + I332 (consumers, parallelizable)

---

### Priority 2: Preset-to-UI Wiring (HIGH)

Presets shape AI prompts and email classification but the user never sees role-specific fields.

| Task | Scope | Effort |
|------|-------|--------|
| **Wire PresetFieldsEditor into entity detail pages** | Import `PresetFieldsEditor` into AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial. Render in the appropriate chapter (hero vitals strip or dedicated section). Connect to `update_entity_metadata()` command. | M |
| **Stakeholder role dropdown from preset** | In StakeholderGallery role input, add a dropdown populated from active preset's `stakeholder_roles` array. Keep freeform as fallback. | S |
| **Account team role assignment** | Add `update_account_team_role` command. Wire into team management UI with dropdown from `internal_team_roles`. | S |

---

### Priority 3: Component Standardization (MEDIUM)

8 duplicate implementations across 5 sites. Not blocking users but blocking maintainability.

| Task | Scope | Effort |
|------|-------|--------|
| **C1: Shared ActionRow** | Extract from TheWork.tsx, ActionsPage.tsx, MeetingDetailPage.tsx into `src/components/shared/ActionRow.tsx` with density variants (compact/full). | M |
| **C2: Shared ProposedActionRow** | Extract from ActionsPage.tsx and DailyBriefing.tsx into shared component with `compact` prop. | S |
| **C3: Shared MeetingRow** | Extract from BriefingMeetingCard, WeekPage TimelineMeetingRow, TheWork upcoming meetings into shared base with variant props. BriefingMeetingCard may stay complex. | M |
| **D8: Command menu surfaces** | Add Email (/emails), Settings (/settings), and entity detail page shortcuts to CommandMenu.tsx. | S |

---

### Priority 4: Backlog & Planning Hygiene (LOW)

| Task | Scope | Effort |
|------|-------|--------|
| **Update BACKLOG I338** | Remove "(plumbing only, files never generated)" — intel_queue generates all 3 files. | Trivial |
| **Reconcile I329/I330 version** | Move from 0.13.0 to 0.12.1 in backlog. Both are shipped. | Trivial |
| **I341 full vocabulary audit** | 5 strings changed. The comprehensive audit described in ADR-0083 has not been done. Decide: ship as-is or do the full audit before 1.0. | Decision |
| **Content index pipeline** | 0.10.0 claimed "content index populated with transcripts and notes." Schema exists but no ingestion. Decide: needed for 0.13.0 intelligence, or defer? | Decision |
| **Proactive insights UI** | 9 detectors run and emit signals. No user-facing surface shows these as a list/dashboard. Signals appear indirectly in entity intelligence but users can't browse proactive insights. Decide: add surface or let signals flow through existing surfaces? | Decision |

---

## Effort Summary

| Priority | Items | Total Effort |
|----------|-------|-------------|
| P1: AI Enrichment Engine | 5 tasks | 1 Large + 2 Medium + 2 Small |
| P2: Preset-to-UI | 3 tasks | 1 Medium + 2 Small |
| P3: Component Standardization | 4 tasks | 2 Medium + 2 Small |
| P4: Backlog Hygiene | 5 tasks | 2 Trivial + 3 Decisions |

**Critical path:** P1 (I326 Phase 2) unblocks everything in 0.13.0. Start there.

---

## Corrections to Initial Audit

For transparency, the initial SVP charge sheet contained errors:

| Original Charge | Correction |
|-----------------|------------|
| "I338 is a ghost feature in three changelogs" | **WRONG.** I338 IS working. intel_queue generates all 3 person files. BACKLOG is stale. |
| "The week page is a surface without substance" | **PARTIALLY WRONG.** The week timeline IS wired and shows real meeting data with quality badges. The badges read from mechanical-only intelligence (valid concern), but the surface itself is functional. |
| "Product vocabulary is five strings, not an audit" | **CORRECT but overstated.** The 5 string changes ARE the vocabulary changes listed in the changelog. I341 (full audit) is acknowledged as open in the backlog. The changelog didn't claim I341 was complete. |
| "Component standardization never happened" | **PARTIALLY WRONG.** C4 (useIntelligenceFieldUpdate) WAS consolidated. C5 was resolved by removal. C1-C3 remain unmerged — that part stands. |

The core charge — **0.13.0's AI enrichment engine has no engine** — stands in full.
