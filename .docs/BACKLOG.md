# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** v1.0.0 GA shipped (2026-03-16). **v1.0.1 shipping today:** email intelligence for CS + post-GA hardening. Roadmap reorganized 2026-03-21: v1.0.2 (fix & reconnect) → v1.0.3 (the meeting record) → v1.1.0 (lifecycle intelligence) → v1.2.0 (actions & loops) → v1.3.0 (report engine) → v1.4.0 (publication + portfolio). See version briefs in `.docs/plans/`.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I620** | Actions pipeline audit & repair — 6 root causes, end-to-end fix | P0 | Backend + Frontend / Actions |
| **I621** | Meeting transcript output redesign — rendering completeness + report layout | P1 | Frontend / Meeting Detail |
| **I622** | Fix past-meeting opacity — outcomes full brightness, prep faded | P1 | Frontend / Meeting Detail |
| **I56** | Onboarding: educational redesign — demo data, guided tour | P0 | Onboarding |
| **I57** | Onboarding: add accounts/projects + user domain configuration | P0 | Onboarding |
| **I199** | Archived account recovery UX — restore + relink | P2 | Entity |
| **I258** | Report Mode — export account detail as leadership-ready deck/PDF | P2 | UX |
| **I302** | Shareable PDF export for intelligence reports (editorial-styled) | P2 | UX |
| **I347** | SWOT report type — account analysis from existing intelligence | P2 | Intelligence / Reports |
| **I348** | Email digest push — DailyOS intelligence summaries via scheduled email | P2 | Distribution |
| **I350** | In-app notifications — release announcements, what's new, system status | P1 | UX / Infra |
| **I365** | Inbox-anchored email fetch — `in:inbox` replaces `is:unread newer_than:1d` | P0 | Backend / Gmail API |
| **I366** | Inbox reconciliation — vanished emails removed from DailyOS on each poll | P0 | Backend / Pipeline |
| **I367** | Mandatory email enrichment — every email AI-processed, retry on failure | P0 | Backend / Pipeline |
| **I368** | Persist email metadata to SQLite — DB as source of truth, not JSON files | P1 | Backend / DB |
| **I369** | Contextual email synthesis — entity-aware smart summaries (ADR-0085) | P1 | Backend / Intelligence |
| **I370** | Thread position refresh — detect user replies between polls | P1 | Backend / Gmail API |
| **I371** | Meeting email context rendering — wire recentEmailSignals to meeting detail UI | P1 | Frontend / UX |
| **I372** | Email-entity signal compounding — email signals flow into entity intelligence | P1 | Backend / Signals |
| **I373** | Email sync status indicator — show last fetch time, stage, error state | P2 | Frontend / UX |
| **I374** | Email dismissal learning — dismissed senders/types adjust future classification | P2 | Backend / Intelligence |
| **I375** | Refresh button audit — design continuity with add buttons + action/surface alignment | P2 | UX / Code Quality |
| **I376** | AI enrichment site audit — map every PTY/AI call site, verify ADR-0086 compliance | P1 | Code Quality / Architecture |
| **I377** | Signal system completeness — emitter/propagation/consumer map, remove dead rules | P1 | Code Quality / Signals |
| **I378** | Intelligence schema alignment — intelligence.json ↔ entity_intel ↔ frontend types | P1 | Code Quality / Architecture |
| **I379** | Vector DB audit — map embedding writes vs. queries, disable orphaned paths | P2 | Code Quality / Architecture |
| **I382** | Partner entity type — `partner` account type, badge, partner-appropriate prompt shape | P1 | Backend / Entity |
| **I383** | AccountsPage three-group layout — Your Book / Your Team / Your Partners | P1 | Frontend / UX |
| **I384** | Parent account portfolio intelligence — two-layer intelligence.json (portfolio synthesis + own signals) | P1 | Backend / Intelligence |
| **I385** | Bidirectional entity hierarchy signal propagation — upward accumulation, downward fan-out | P1 | Backend / Signals |
| **I386** | Calendar lifecycle gaps — future meeting cancellation detection, rescheduling sync, continuous future polling | P1 | Backend / Calendar |
| **I388** | Project hierarchy intelligence — two-layer intelligence.json + bidirectional propagation for project entities | P1 | Backend / Intelligence |
| **I389** | Entity-mode-aware surface ordering — nav/primary surface adapts to preset's entityModeDefault | P2 | Frontend / UX |
| **I390** | Person relationship graph — `person_relationships` table, typed edges, confidence scoring, context scoping | P1 | Backend / Entity |
| **I391** | People network intelligence — two-layer intelligence.json + network section + person→person signal propagation | P1 | Backend / Intelligence |
| **I392** | Relationship cluster view on person detail — Network chapter, cluster summary, risks/opportunities, preset vocabulary | P1 | Frontend / UX |
| **I393** | Parent account detail page — portfolio surface (hotspots, cross-BU patterns, portfolio narrative) | P1 | Frontend / UX |
| **I394** | Week page past meeting duration shows NaN — `formatDurationFromIso` doesn't guard against NaN from invalid date strings | P1 | Frontend / Bug |
| **I395** | Email relevance scoring — signal-driven surfacing using entity linkage, Bayesian fusion, embedding relevance, keyword matching | P0 | Backend / Intelligence + Frontend / UX |
| **I396** | intelligence.json report fields — health_score, health_trend, value_delivered, success_metrics, open_commitments, relationship_depth | P1 | Backend / Intelligence |
| **I397** | Report infrastructure — `reports` table, intel_hash invalidation, ReportShell renderer, PDF export, SWOT format | P1 | Backend / Frontend |
| **I398** | Risk report migration — move existing risk briefing to ADR-0086 architecture (entity_intel DB input, reports table output) | P1 | Backend |
| **I399** | Account Health Review report type — internal CS report: health score/trend, risks, stakeholder coverage, open commitments | P1 | Backend / Reports |
| **I400** | EBR/QBR report type — flagship CS customer-facing quarterly review: value delivered, success metrics, asks | P1 | Backend / Reports |
| **I401** | Show internal attendees for internal meetings — hydrate_attendee_context filters out all internal people, leaving "The Room" empty for team syncs | P2 | Backend / UX |
| **I403** | SignalService formalization — formalize signals/ module boundary as a service with clear public API | P2 | Code Quality / Refactor |
| **I404** | AppState decomposition Phase 1 — split 28-field AppState into domain containers (Db, Workflow, Calendar, Capture, Hygiene) | P1 | Code Quality / Refactor |
| **I405** | AppState decomposition Phase 2 — split integration and signal fields into IntegrationState and SignalState containers | P2 | Code Quality / Refactor |
| **I406** | Entity quality scoring — Beta distribution quality model per entity (`entity_quality` table); replaces binary has/missing classification | P1 | Backend / Intelligence |
| **I407** | Semantic coherence validation — embedding distance check between intelligence text and linked meeting corpus; flags topic drift (Jefferies/Adobe Fonts case) | P1 | Backend / Intelligence |
| **I408** | Enrichment trigger function — continuous score (meeting imminence + staleness + importance + signal delta) replaces hardcoded 14-day threshold | P1 | Backend / Hygiene |
| **I409** | Feedback closure — user corrections update entity quality score + enrichment source reliability via Thompson Sampling | P1 | Backend / Signals |
| **I410** | Hygiene event-driven triggers — coherence check and quality re-evaluation fire on signal arrival; 4-hour scan becomes catch-all sweep | P1 | Backend / Hygiene |
| **I411** | User entity — first-class user entity with declared professional context (value_proposition, success_definition, current_priorities, product_context, playbooks) | P1 | Backend / Entity |
| **I412** | User context in enrichment prompts — every entity intelligence prompt includes user context block; enables risk→opportunity reframe (ADR-0089) | P1 | Backend / Intelligence |
| **I413** | User context document attachment — attach product decks, playbooks, case studies to user entity; ingested via existing file processor + embedding pipeline | P2 | Backend / Intelligence |
| **I414** | User-context-weighted signal scoring — signal relevance multiplied by alignment with user's current_priorities; extends email relevance scoring (I395) to entity signals | P1 | Backend / Signals |
| **I415** | User entity page — dedicated `/me` route, six-section professional context surface, dropbox, context entries UI (ADR-0090) | P1 | Frontend / Entity |
| **I416** | User entity navigation — dedicated nav item, identity fields moved out of Settings | P1 | Frontend / Navigation |
| **I417** | Context entries — professional knowledge as embedded intelligence input, retrieved in enrichment by semantic similarity | P1 | Backend / Intelligence |
| **I418** | Weekly Impact Report — personal, user-entity-scoped operational look-back: priorities advanced, signals moved, meetings, commitments. Auto-generates Monday. Lives on `/me`. | P1 | Backend / Reports |
| **I419** | Monthly Wrapped — celebratory narrative impact report: top wins, progress against annual/quarterly priorities, the honest miss, volume stats. Auto-generates 1st of month. Shareable PDF. | P1 | Backend / Reports |
| **I420** | Stakeholder–Person reconciliation — canonical name injection in enrichment prompt + post-enrichment fuzzy matching with confidence-tiered linking (auto-link ≥0.8, suggest 0.5–0.8) | P1 | Backend / Intelligence + Frontend / Entity |
| **I421** | Connector rename — "Integrations" → "Connectors" throughout Settings UI, nav, and user-facing copy | P2 | Frontend / UX |
| **I422** | Clay production validation — configure in production DB, verify signal emission path, remove legacy `emit_signal` path | P1 | Backend / Clay |
| **I423** | Gravatar writeback + propagation — write `photo_url` back to `people` table; upgrade `emit_signal` → `emit_signal_and_propagate` | P1 | Backend / Gravatar |
| **I424** | Granola hardening — fix DB mutex hold during AI pipeline; add wake signal from calendar poller | P1 | Backend / Granola |
| **I425** | Linear signal wiring — emit signals on issue sync; surface blocked/overdue issues in meeting prep and briefing attention | P1 | Backend / Linear + Intelligence |
| **I426** | Google Drive connector — OAuth scope, folder/doc selection, doc import to `_inbox/` or entity context, markdown conversion, entity linking | P1 | Backend / Integrations + Frontend |
| **I432** | IntelligenceProvider abstraction — multi-LLM backend trait replacing direct PtyManager calls in intel_queue | P1 | Backend / Architecture |
| **I433** | Ollama provider — local LLM support; nothing leaves the device when Ollama is selected | P1 | Backend / Intelligence |
| **I434** | OpenAI API provider — GPT-4o with user-supplied key; shares HTTP client with Ollama (OpenAI-compatible) | P2 | Backend / Intelligence |
| **I437** | Empty state redesign — every surface guides action rather than reporting emptiness; role-preset-aware copy | P1 | Frontend / UX |
| **I439** | Personality expanded in UI — completion messages, generating states, toasts, error copy; never touches AI prompts (role presets handle intelligence framing) | P1 | Frontend / UX |
| **I440** | Meeting prep preset persona — remove hardcoded "Customer Success Manager"; use active preset role name and vocabulary | P1 | Backend / Intelligence |
| **I441** | Personality coverage + useActivePreset cache — fill 3 missing empty state keys; shared reactive context replacing per-page IPC calls | P1 | Frontend |
| **I442** | stakeholder_roles wired — relationship type dropdowns on person-to-account linking pull from active preset | P1 | Backend / Frontend |
| **I443** | internal_team_roles wired — account team role selectors pull from active preset | P1 | Backend / Frontend |
| **I444** | lifecycle_events wired — lifecycle stage pickers use preset events; stage injected into entity intelligence prompts | P1 | Backend / Frontend |
| **I445** | prioritization wired — account list default sort and weekly forecast ranking use preset's primary_signal and urgency_drivers | P1 | Backend / Frontend |
| **I446** | User entity page × role preset — section prominence, vocabulary/placeholders, named playbook sections for all 9 presets | P1 | Frontend / Entity |
| **I455** | 1:1 meeting prep focuses on person entity intelligence, not account | P1 | Backend / Intelligence |
| **I456** | In-app markdown reader for entity documents — view .md files from account/project/person Documents/ without leaving app | P2 | Frontend / UX |
| **I457** | Background task throttling — ActivityMonitor, HeavyWorkSemaphore, adaptive polling intervals | P1 | Backend / Performance |
| **I477** | Meeting entity switch should hot-swap briefing content — stale disk fallback guard + single mutation-and-refresh service | P1 | Backend / Meeting + Frontend / UX |
| **I478** | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |
| **I482** | Role-aware Glean query optimization — preset vocabulary shapes search queries, lifecycle-stage filtering, dedup | P1 | Backend / Connectors + Intelligence |
| **I483** | Theme infrastructure + shipped presets — `data-theme` token layering, typography scale controls, three shipped themes (Warm/Dark/Cool), Settings picker, custom theme docs | P1 | Frontend / Tokens + Settings |
| **I489** | VP Account Review report type — leadership-facing strategic assessment, risk/opportunity matrix, VP-level actions | P1 | Backend / Reports |
| **I490** | Renewal Readiness report type (absorbs I458) — 90-day renewal risk assessment, readiness rating, champion alignment | P1 | Backend / Reports |
| **I491** | Portfolio Health Summary report type — cross-account VP synthesis, exceptions, renewal pipeline, portfolio narrative | P1 | Backend / Reports |
| **I492** | Portfolio Health page — editorial aggregate view, health heatmap, exception list, renewal timeline | P1 | Frontend / Pages |
| **I494** | Glean account discovery flow — import CRM accounts via Glean, one-click add with pre-populated context | P1 | Backend / Connectors + Frontend |
| **I495** | Ephemeral account query — "tell me about X" transient briefing via Glean, no persistent entity | P1 | Backend / Connectors + Frontend |
| **I496** | Stakeholder Map report type (absorbs I459) — coverage grid, influence network, engagement assessment | P1 | Backend / Reports |
| **I497** | Success Plan report type (absorbs I460) — shared objectives, progress, customer-presentable | P1 | Backend / Reports |
| **I498** | Coaching Patterns report type (absorbs I461) — meeting cadence, engagement patterns, book-level norms, coaching recommendations | P2 | Backend / Reports |
| **I531** | Glean-powered proactive self-healing — hygiene detects gaps, searches Glean, fills intelligence. Signal emission fixes. Spec: `.docs/issues/i531.md` | P1 | Backend / Self-Healing / Connectors |
| **I532** | Intelligence surfacing threshold model — significance scoring, surfacing budgets, fatigue prevention, feedback-driven learning. Spec: `.docs/issues/i532.md` | P1 | Backend / Signals + Frontend / Briefing |
| **I533** | Publication engine — Google Drive output layer. Reports published as PDF/Google Doc to Shared Drive. Auto-indexed by Glean. Spec: `.docs/issues/i533.md` | P1 | Backend / Publication |
| **I534** | Portfolio reader — read published intelligence from Shared Drive for cross-IC portfolio synthesis. JSON sidecar parsing. Spec: `.docs/issues/i534.md` | P1 | Backend + Frontend |
| **I535** | Glean Agent integration — call purpose-built Glean Agents via REST API for org-level analysis during enrichment. Spec: `.docs/issues/i535.md` — Steps 1-8 implemented, blocked on testing | P1 | Backend / Connectors |
| **I563** | shellConfig render loop on AccountDetailEditorial — `useRegisterMagazineShell` re-register cycle causes max update depth warning | P2 | Frontend / Architecture |
| **I604** | Report engine foundation — schema (3 cols + 3 tables), assembly engine, enrichment extensions, animation primitives, dead code cleanup. Spec: `.docs/issues/i604.md` | P0 | Backend + Frontend / Reports |
| **I605** | Account reports → display-only — Account Health, SWOT, EBR/QBR assembly from entity_assessment. Spec: `.docs/issues/i605.md` | P0 | Backend / Reports |
| **I606** | Book of Business rebuild — portfolio_assessment background task, mechanical + stored narrative, re-enable. Spec: `.docs/issues/i606.md` | P1 | Backend + Frontend / Reports |
| **I607** | Weekly Impact v2 — user_weekly_summary rollover, comparative framing, animation polish. Spec: `.docs/issues/i607.md` | P1 | Backend + Frontend / Reports |
| **I608** | Monthly Wrapped v2 — user_monthly_summary rollover, archetypes, achievements, visual transformation. Spec: `.docs/issues/i608.md` | P1 | Backend + Frontend / Reports |

---

## Version Planning

### 0.8.2 through 0.9.2 — CLOSED

All issues resolved. See CHANGELOG.

---

### 0.10.0 — Signal Intelligence — CLOSED

All issues (I305, I306, I307, I308, I334, I335, I336, I339, I260, I262) closed in v0.10.0. Bayesian signal fusion, Thompson Sampling correction learning, cross-entity propagation, event-driven processing, entity-generic data model, proactive surfacing. See CHANGELOG.

---

### 0.10.1 — User Feedback & Onboarding Polish — CLOSED

All issues (I344, I345, I346) closed in v0.10.1. Gmail teammate suggestions, Linear data layer, onboarding back-nav fix. See CHANGELOG.

---

### 0.11.0 — Role Presets & Entity Architecture — CLOSED

All issues (I309, I310, I311, I312, I313, I314, I315, I316, I143a, I143b, I352) closed in v0.11.0. 9 embedded role presets, preset-driven vitals, metadata storage migration, n-level nesting, shared entity hooks. Remaining work on actions standardization (I351) and prompt field injection (I359) carried forward. See CHANGELOG.

---

### 0.12.0 — Email Intelligence — CLOSED

All issues (I317, I318, I319, I320, I321, I322, I323, I324, I337, I338, I353, I354) closed in v0.12.0. Meeting-aware email digest, thread position tracking, entity cadence monitoring, hybrid classification, commitment extraction, briefing narrative, auto-archive, 1:1 relationship intelligence, self-healing hygiene. See CHANGELOG.

---

### 0.12.1 — Product Language & UX Polish — CLOSED

All issues (I341 partial, I342 Phases 1–3, I343 partial, I349, I355, I356 partial, I358 partial) closed in v0.12.1. Vocabulary pass, surface cuts, settings redesign, email intelligence UI. Remaining work on vocabulary (I341), inline editing (I343), surface restructure Phase 4 (I342), and email surface (I356, I358) carried into v0.13.0. See CHANGELOG.

---

### 0.13.0 — Event-Driven Meeting Intelligence + Unified Surfaces — CLOSED

18 issues shipped + pre-ship engineering audit remediation.

| ID | Resolution |
|----|------------|
| I326 | Per-meeting intelligence lifecycle — `meeting_prep_queue` state machine (new → enriching → enriched → archived), background enrichment, real AI replacing mechanical row-count |
| I327 | Advance intelligence generation — weekly pipeline + polling cadence preps meetings ahead of time, not day-of |
| I328 | Classification expansion — all meeting types (1:1, internal, all-hands, team sync) get intelligence via entity-generic prep pipeline |
| I329 | Intelligence quality indicators — hasPrep dot replaced with vocabulary-correct quality badges (Sparse, Developing, Ready) |
| I330 | Week page ±7-day timeline — meeting intelligence timeline renders upcoming and past meetings with quality context |
| I331 | Always-live daily briefing — assembles from pre-computed intelligence without blocking; no empty state |
| I332 | Signal-triggered refresh — calendar polling + email signals mark prep stale and re-queue; prep invalidation wired end-to-end |
| I333 | Meeting intelligence collaboration — share, request input, and draft agenda from meeting detail folio |
| I341 | Product vocabulary — system-term strings removed ("Meeting Intelligence Report" → "Meeting Briefing", "Proposed" → "Suggested", etc.) |
| I342 | Surface restructure Phase 4 — Lead Story removed, Day Frame / Schedule / Attention / Finis structure, meeting detail appendix cuts, Actions page meeting-centric grouping |
| I343 | Inline editing — AccountFieldsDrawer and ProjectFieldsDrawer replaced with EditableText/List; no drawers |
| I351 | Actions chapter on PersonDetailEditorial — actions from 1:1 meetings surface on person detail |
| I356 | Thread position UI — Replies Needed subsection in daily briefing Attention section |
| I358 | Email page — promoted to first-class nav surface with meeting-centric organization |
| I361 | Timeline meeting filtering — personal, focus, and blocked events excluded from weekly forecast timeline |
| I362 | Shared meeting card — MeetingCard.tsx extracted with accent colors, intelligence badges, entity byline, past treatment |
| I363 | Timeline data enrichment — formatted time and duration added to all TimelineMeeting rows |
| I364 | Weekly forecast timeline adoption — MeetingRow variant="timeline" deleted; weekly forecast uses shared MeetingCard |

**Engineering audit remediation (pre-0.13.0 tag):** 2026-02-20 audit of 0.10.0+ found: intelligence lifecycle marking meetings enriched after mechanical row-count with no AI; 3 of 6 propagation rules listening for signals never emitted; prep invalidation fully implemented but never called; frontend types ahead of backend. All gaps remediated before 0.13.0 tagged.

---

### 0.13.1 — Email as Intelligence Input — CLOSED

**Theme:** Email is an intelligence input, not a display surface. Every email AI-processed, entity-resolved, contextually synthesized. DailyOS doesn't show you emails — it tells you what your emails mean. (ADR-0085)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I365 | Inbox-anchored email fetch — `in:inbox` replaces `is:unread newer_than:1d` | P0 | Backend / Gmail API |
| I366 | Inbox reconciliation — vanished emails removed from DailyOS on each poll | P0 | Backend / Pipeline |
| I367 | Mandatory email enrichment — every email AI-processed, retry on failure | P0 | Backend / Pipeline |
| I368 | Persist email metadata to SQLite — DB as source of truth, not JSON files | P1 | Backend / DB |
| I369 | Contextual email synthesis — entity-aware smart summaries | P1 | Backend / Intelligence |
| I370 | Thread position refresh — detect user replies between polls | P1 | Backend / Gmail API |
| I371 | Meeting email context rendering — wire recentEmailSignals to meeting detail UI | P1 | Frontend / UX |
| I372 | Email-entity signal compounding — email signals flow into entity intelligence | P1 | Backend / Signals |
| I373 | Email sync status indicator — show last fetch time, stage, error state | P2 | Frontend / UX |
| I374 | Email dismissal learning — dismissed senders/types adjust future classification | P2 | Backend / Intelligence |
| I375 | Refresh button audit — design continuity with add buttons + action/surface alignment | P2 | UX / Code Quality |
| I386 | Calendar lifecycle gaps — future meeting cancellation detection, rescheduling sync, continuous future polling | P1 | Backend / Calendar |
| I394 | Week page past meeting duration shows NaN — `formatDurationFromIso` doesn't guard against NaN from invalid date strings | P1 | Frontend / Bug |
| I395 | Email relevance scoring — signal-driven surfacing using entity linkage, Bayesian fusion, embedding relevance, keyword matching | P0 | Backend / Intelligence + Frontend / UX |

Note: I357 (semantic email reclassification) is absorbed by I367 — enrichment is mandatory, not opt-in.

---

### 0.13.2 — Structural Clarity — CLOSED

**Theme:** Know what you built before you build the next layer. ADR-0086 defines the intended architecture. This version audits whether reality matches it, documents the full E2E intelligence chain, and does the structural refactoring that makes the system maintainable heading into v0.14.0.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I376 | AI enrichment site audit — map every PTY/AI call site, verify ADR-0086 compliance | P1 | Code Quality / Architecture |
| I377 | Signal system completeness — emitter/propagation/consumer map, remove dead rules | P1 | Code Quality / Signals |
| I378 | Intelligence schema alignment — intelligence.json ↔ entity_intel ↔ frontend types | P1 | Code Quality / Architecture |
| I379 | Vector DB audit — map embedding writes vs. queries, disable orphaned paths | P2 | Code Quality / Architecture |
| I380 | commands.rs service extraction Phase 1 — complete services/ per SERVICE-CONTRACTS.md | P1 | Code Quality / Refactor |
| I381 | db/mod.rs domain migration — move queries into domain modules per SERVICE-CONTRACTS.md | P2 | Code Quality / Refactor |

---

### 0.13.3 — Entity Hierarchy Intelligence — CLOSED

**Theme:** Parent accounts are portfolio surfaces, not folders. Partners are a distinct entity type. Signals flow up from BUs and down from the parent. The AccountsPage reflects how users actually think about their work. (ADR-0087)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I382 | Partner entity type — `partner` account type, badge, partner-appropriate prompt shape | P1 | Backend / Entity |
| I383 | AccountsPage three-group layout — Your Book / Your Team / Your Partners | P1 | Frontend / UX |
| I384 | Parent account portfolio intelligence — two-layer intelligence.json | P1 | Backend / Intelligence |
| I385 | Bidirectional entity hierarchy signal propagation — upward accumulation, downward fan-out | P1 | Backend / Signals |
| I393 | Parent account detail page — portfolio surface (hotspots, cross-BU patterns, portfolio narrative) | P1 | Frontend / UX |

Note: I387 (multi-entity signal extraction from parent-level meetings) deferred — P3, not version-locked. Bidirectional propagation (I385) covers the majority of the use case.

---

### 0.13.4 — Project Hierarchy Intelligence — CLOSED

**Theme:** ADR-0087 applied to project entities. Parent projects become portfolio surfaces for Marketing, Product, and Agency users — same two-layer intelligence model, same bidirectional propagation, project-appropriate vocabulary. The ADR-0079 `entityModeDefault: "project"` preset users get the same portfolio capability account-mode users get in v0.13.3.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I388 | Project hierarchy intelligence — two-layer intelligence.json + bidirectional propagation for project entities | P1 | Backend / Intelligence |
| I389 | Entity-mode-aware surface ordering — nav/primary surface adapts to preset's entityModeDefault | P2 | Frontend / UX |

---

### 0.13.5 — People Relationship Network Intelligence — CLOSED

**Theme:** People are not rows — they're nodes in a relationship network. A buying committee, a product team, a marketing cluster — these are graphs of individuals with influence flows, not isolated contacts. This version makes those relationships visible, persistent, and intelligent. (ADR-0088)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I390 | Person relationship graph — `person_relationships` table, typed edges, confidence scoring, context scoping | P1 | Backend / Entity |
| I391 | People network intelligence — two-layer intelligence.json + network section + person→person signal propagation | P1 | Backend / Intelligence |
| I392 | Relationship cluster view on person detail — Network chapter, cluster summary, risks/opportunities, preset vocabulary | P1 | Frontend / UX |

---

### 0.13.6 — Cleanup + Service Extraction Phase 2 — CLOSED

**Theme:** Maximum commands.rs extraction. Six service files created or expanded, 1,400+ lines moved from commands.rs. IntelligenceService (6 methods) and SettingsService (new file) created. SignalService (I403) deferred as standalone follow-up.

| ID | Title | Priority | Status |
|----|-------|----------|--------|
| I401 | Show internal attendees for internal meetings | P2 | Done |
| I402 | IntelligenceService extraction (6 of 9 contract methods) | P1 | Done (partial) |
| I450 | ProjectService extraction | P1 | Done |
| I451 | EmailService expansion | P1 | Done |
| I452 | AccountService expansion | P1 | Done |
| I453 | MeetingService expansion | P1 | Done |
| I454 | SettingsService extraction | P1 | Done |
| I403 | SignalService formalization — deferred to v0.13.8, cross-cutting refactor of 27 call sites | P2 | Deferred → 0.13.8 |

---

### 0.13.7 — Intelligence Self-Healing — CLOSED

**Theme:** The system should know when its intelligence is wrong, not just when it's missing. Entity quality scores replace binary has/missing classification. Semantic coherence validation catches topic drift before it reaches meeting prep (the Jefferies/Adobe Fonts failure mode). An enrichment trigger function replaces the hardcoded 14-day staleness threshold. Feedback closure wires user corrections back to source reliability. See `.docs/research/self-healing-intelligence.md`.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I406 | Entity quality scoring — Beta distribution quality model per entity (`entity_quality` table) | P1 | Backend / Intelligence |
| I407 | Semantic coherence validation — embedding distance check; flags topic drift in AI-generated intelligence | P1 | Backend / Intelligence |
| I408 | Enrichment trigger function — continuous score replaces 14-day hardcoded staleness threshold | P1 | Backend / Hygiene |
| I409 | Feedback closure — user corrections update entity quality score + enrichment source reliability | P1 | Backend / Signals |
| I410 | Hygiene event-driven triggers — quality re-evaluation fires on signal arrival; 4-hour scan becomes catch-all | P1 | Backend / Hygiene |

---

### 0.13.8 — AppState Decomposition — CLOSED

**Theme:** AppState has 28 fields and every subsystem reaches in for its dependencies. This version splits it into domain-specific containers so each subsystem declares what it needs, not reaches into a god struct. Mechanical refactoring — `state.field` becomes `state.domain.field`. No logic changes, no behavior changes. SignalService formalization (deferred from v0.13.6) fits naturally here since AppState decomposition is already restructuring state access patterns. The last structural prerequisite before v0.14.0 feature work.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I403 | SignalService formalization — formalize signals/ module boundary as a service with clear public API; deferred from v0.13.6 | P2 | Code Quality / Refactor |
| I404 | AppState decomposition Phase 1 — split core domain fields (Db, Workflow, Calendar, Capture, Hygiene) into containers | P1 | Code Quality / Refactor |
| I405 | AppState decomposition Phase 2 — split integration and signal fields (IntegrationState, SignalState) into containers | P2 | Code Quality / Refactor |

---

### 0.13.9 — Connectors — CLOSED

**Theme:** Every connector should produce signals. Most don't. Gravatar stores avatars but never writes back to the people table. Linear syncs issues and then stops — nothing reads them. Granola holds the DB mutex across an AI pipeline call. Clay has never run in production. This version hardens every existing connector against the connector signal contract (data source → DB → `emit_signal_and_propagate` → signal_events → entity intelligence) and wires Linear into the briefing.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I421 | Connector rename — "Connections" → "Connectors" throughout Settings UI and user-facing copy | P2 | Frontend / UX |
| I422 | Clay production validation — SSE transport, OAuth flow, verify signal emission, remove legacy path | P1 | Backend / Clay |
| I423 | Gravatar writeback + propagation — write `photo_url` to `people` table; upgrade to `emit_signal_and_propagate` | P1 | Backend / Gravatar |
| I424 | Granola hardening — fix DB mutex hold during AI pipeline; add calendar-poller wake signal | P1 | Backend / Granola |
| I425 | Linear signal wiring — emit signals on sync; surface blocked/overdue issues in meeting prep + briefing | P1 | Backend / Linear |

---

### 0.14.0 — User Entity + Professional Context — CLOSED

**Theme:** DailyOS knows a lot about the world the user operates in. It knows almost nothing about the user themselves. This version gives the user a first-class entity with a dedicated page (`/me`), six-section professional context surface, two-layer priority model, context entries (professional knowledge as embedded intelligence input), and document attachments. User context flows into every entity intelligence prompt and signal scoring. CS-first in v0.14.0; all role presets expanded in v0.14.1. (ADR-0089, ADR-0090)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I350 | In-app notifications — What's New modal, system status toasts, auth expiry alerts | P1 | UX / Infra |
| I411 | User entity — data model, two-layer priorities (strategic + tactical), context entries table, workspace file | P1 | Backend / Entity |
| I412 | User context in enrichment prompts — user context block in every entity intelligence prompt; enables risk→opportunity reframe | P1 | Backend / Intelligence |
| I414 | User-context-weighted signal scoring — signal relevance multiplied by alignment with strategic priorities | P1 | Backend / Signals |
| I415 | User entity page — dedicated `/me` route, six-section professional context surface, dropbox, context entries UI | P1 | Frontend / Entity |
| I416 | User entity navigation — dedicated nav item; identity fields moved out of Settings | P1 | Frontend / Navigation |
| I417 | Context entries — professional knowledge as embedded intelligence input, retrieved in enrichment by semantic similarity | P1 | Backend / Intelligence |
| I396 | intelligence.json report fields — health_score, health_trend, value_delivered, success_metrics, open_commitments, relationship_depth | P1 | Backend / Intelligence |

Note: I413 (document attachment — `_user/docs/` watched folder) is P2, moves to v0.15.0 where it directly enriches report generation.

---

### 0.14.1 — Role Presets + Personality: Actually Work — CLOSED

**Theme:** Both personalisation systems are 30–50% realised. Personality affects only empty-state strings — not a single line of AI output changes tone. Five preset fields are typed, validated, and defined in all 9 presets but consumed by nothing. Meeting prep hardcodes "Customer Success Manager" regardless of who's using the app. This version finishes what was started: personality shapes every AI output, preset fields drive the UI they were built for, and the user entity page gets its full preset-specific vocabulary across all 9 roles.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I439 | Personality expanded in UI — completion messages, generating states, toasts, errors; never touches AI prompts | P1 | Frontend / UX |
| I440 | Meeting prep preset persona — remove hardcoded "Customer Success Manager"; use active preset role name | P1 | Backend / Intelligence |
| I441 | Personality coverage + useActivePreset cache — fill 3 missing empty state keys; shared reactive context | P1 | Frontend |
| I442 | stakeholder_roles wired — relationship type dropdowns use active preset's stakeholder roles | P1 | Backend / Frontend |
| I443 | internal_team_roles wired — account team role selectors use active preset | P1 | Backend / Frontend |
| I444 | lifecycle_events wired — lifecycle pickers use preset events; stage injected into intelligence prompts | P1 | Backend / Frontend |
| I445 | prioritization wired — account list default sort + weekly forecast risk ranking use preset primary_signal | P1 | Backend / Frontend |
| I446 | User entity page × all 9 presets — section prominence, vocabulary/placeholders, named playbooks per preset | P1 | Frontend / Entity |
| I455 | 1:1 meeting prep focuses on person entity intelligence, not account | P1 | Backend / Intelligence |
| I457 | Background task throttling — ActivityMonitor, HeavyWorkSemaphore, adaptive polling | P1 | Backend / Performance |

---

### 0.14.3 — Google Drive Connector — CLOSED

**Theme:** Collaborative documents are where real work context lives — GTM docs, PRDs, playbooks. Google Drive is the first document-import connector: one-time import and continuous watch mode via the Changes API. Deferred from v0.13.9 to give it the space it needs — new OAuth scope, Google Picker in Tauri, a new poller, and watch mode with content change detection.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I426 | Google Drive connector — OAuth, folder/doc selection, import to `_inbox/` or entity context, markdown conversion, watch mode via Changes API | P1 | Backend / Connectors + Frontend |

---

### 0.15.0 — Reports: Outward and Inward [IN PROGRESS]

**Theme:** Two kinds of reports, both made possible by v0.14.0. Outward-facing: CS reports (EBR/QBR, Account Health Review, Risk Report) framed through the user's value narrative — the EBR "Value Delivered" section now references the user's actual story, not a generic summary. Inward-facing: personal impact reports (Weekly Impact, Monthly Wrapped) that answer the question no other tool answers — "what did I actually accomplish, and did it move what matters to me?" The user entity's declared priorities are the lens for both.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I397 | Report infrastructure — `reports` table, intel_hash invalidation, ReportShell, PDF export, SWOT format | P1 | Backend / Frontend |
| I398 | Risk report migration — ADR-0086 alignment for existing risk briefing | P1 | Backend |
| I399 | Account Health Review — internal CS report type | P1 | Backend / Reports |
| I400 | EBR/QBR — flagship CS customer-facing quarterly review | P1 | Backend / Reports |
| I418 | Weekly Impact Report — personal operational look-back: priorities advanced, signals moved, meetings, commitments. Auto-generates Monday. Lives on `/me`. | P1 | Backend / Reports |
| I419 | Monthly Wrapped — celebratory narrative: top wins, progress against annual/quarterly priorities, the honest miss, volume stats. Auto-generates 1st of month. Shareable PDF. | P1 | Backend / Reports |
| I413 | User entity document attachment — product decks, playbooks, case studies ingested via existing file processor | P2 | Backend / Intelligence |
| I447 | One-time config migration: extraction tier sonnet→haiku for users who never explicitly changed the default | P1 | Backend |
| I448 | Fix stale archived emails in briefing — remove JSON fallback, DB is source of truth with resolved_at filtering | P0 | Backend |
| I449 | Fix hallucinated meeting relevance — require entity to have a meeting today + raise similarity threshold | P0 | Backend |

Note: I258, I302, I347 superseded by I397. I348 removed.

---

### ~~v1.0.1 — CS Report Suite Completion~~ [DISSOLVED]

Dissolved 2026-02-28. All CS report types absorbed into v1.1.0 with richer acceptance criteria built on a fixed intelligence foundation. I458 → I490, I459 → I496, I460 → I497, I461 → I498. See `.docs/research/2026-02-28-hook-gap-analysis.md` for rationale.

---

### ~~v1.1.0 — Intelligence Foundation + CS Report Suite~~ [DISSOLVED]

Dissolved 2026-03-02. Intelligence foundation (I508, I499-I507, I509) and CS report suite (I489-I491, I496-I498) absorbed into v1.0.0 rearchitecture. Health surfaces (I502) and account detail (I493) moved to v1.0.0 Phase 3. See ADR-0099.

---

### ~~v1.1.1 — Portfolio Surfaces~~ [DISSOLVED]

Dissolved 2026-03-02. I493 absorbed into v1.0.0 Phase 3. I492 (Portfolio Health page) moved to v1.1.0 (Teams + Portfolio). See ADR-0099.

---

### ~~v1.1.2 — Glean Account Discovery~~ [DISSOLVED]

Dissolved 2026-03-02. I494, I495 moved to v1.1.0 (Teams + Portfolio). See ADR-0099.

---

### v1.0.2 — Fix & Reconnect [PLANNED]

**Theme:** Fix what's broken before building more. Actions pipeline doesn't surface extracted actions. Briefing expansion panels lost prep previews. Past meetings fade the wrong content. Reconnection audit sweeps for other built-but-not-wired components. Coherence work, not feature work. Origin: planning reorganization 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I620 | Actions pipeline audit & repair — 6 root causes, end-to-end fix | P0 | Backend + Frontend / Actions |
| I621 | Meeting transcript output redesign — rendering completeness + report layout | P1 | Frontend / Meeting Detail |
| I622 | Fix past-meeting opacity — outcomes full brightness, prep faded | P1 | Frontend / Meeting Detail |
| I629 | Briefing expansion panel restoration — reconnect PrepGrid + action checklist (regression fix) | P1 | Frontend / Briefing |
| I630 | Reconnection audit — sweep for built-but-not-wired components | P2 | Code Quality |
| I631 | Transcript filing for person + project entities — extend routing beyond accounts | P2 | Backend / Transcripts |
| I632 | MCP query_entity reads intelligence from DB — replace stale intelligence.json reads | P2 | Backend / MCP |

Version brief: `.docs/plans/v1.0.2.md`.

---

### v1.0.3 — The Meeting Record [PLANNED]

**Theme:** The meeting page transforms through 4 temporal stages: upcoming (prep) → in-progress (locked) → just-ended (processing progress) → processed (meeting record). After transcript processing, the page becomes a polished executive brief with prediction scorecard, structured outcomes, and meeting-to-meeting continuity. A consolidated meeting record markdown is generated for MCP consumption. Health scoring accuracy fix ensures scores update post-meeting. Origin: CEO plan review SCOPE EXPANSION 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I633 | Health scoring real-time accuracy — scores must update post-meeting | P0 | Backend / Health Scoring |
| I634 | Meeting page temporal transformation — 4-stage lifecycle | P0 | Frontend / Meeting Detail |
| I635 | Prep prediction scorecard — what we predicted vs what happened | P1 | Backend / Intelligence + Frontend |
| I636 | Consolidated meeting record markdown — structured output for MCP | P1 | Backend / Transcripts |
| I637 | Meeting-to-meeting continuity thread | P1 | Backend + Frontend |

Version brief: `.docs/plans/v1.0.3.md`. Umbrella spec: `.docs/issues/i634.md`.

---

### v1.1.0 — Lifecycle Intelligence + Briefing Depth [PLANNED]

**Theme:** AI-native lifecycle management. The system detects lifecycle transitions from signals and acts automatically — then reports in the daily briefing. No management UI, no pipeline, no workbench. "It should just know." Products discovered by Glean/AI, not managed. Provenance shows data sources. Origin: CEO plan review + product philosophy challenge 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I623 | Lifecycle intelligence engine — automatic signal-driven transitions + briefing reporting | P0 | Backend / Services + Frontend / Briefing |
| I624 | Product intelligence — AI-discovered products from Glean + transcript/email inference (no CRUD) | P1 | Backend / DB + Frontend |
| I625 | Provenance inline markers — editorial attribution scoped to Glean sub-sources | P1 | Frontend / Components |
| I628 | Success plan milestone auto-completion — lifecycle events auto-complete matching milestones | P2 | Backend / Success Plans |
| I642 | Token usage observability + efficiency foundations — metering, daily budget, transcript dedup, email batching | P1 | Backend / Intelligence / PTY |

Killed: ~~I626 (breadcrumb bar — management artifact)~~, ~~I627 (briefing quick actions — replaced by automatic detection)~~.

Version brief: `.docs/plans/v1.1.0.md`. Umbrella spec: `.docs/issues/i623.md`.

---

### v1.2.0 — Actions & Success Plans: Closing the Loop [PLANNED]

**Theme:** Close broken loops. Actions feed health scoring. Milestones advance when work is done. Value persists. Stale actions age out (zero-guilt). Proposed actions learn from rejection. Origin: moved from v1.0.2 during planning reorganization 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I597 | Action & success plan mock data expansion | P0 | Backend / Mock Data |
| I583 | Action aging & zero-guilt cleanup | P0 | Backend / Actions |
| I584 | Action completion feeds health scoring | P0 | Backend / Health Scoring |
| I585 | Persist value delivered | P0 | Backend / Intelligence |
| I586 | Action completion → milestone advancement | P1 | Backend / Success Plans |
| I587 | Action context in meeting prep | P1 | Backend / Intelligence |
| I588 | Recommended actions from intelligence | P1 | Backend / Intelligence |
| I589 | Completion momentum surface | P1 | Frontend / Actions |
| I590 | Waiting-on as first-class status | P1 | Backend + Frontend |
| I591 | Unify AI objectives with user objectives | P2 | Backend / Intelligence |
| I592 | Auto-link actions to objectives | P2 | Backend / Intelligence |
| I593 | Captured commitments → milestone candidates | P2 | Backend / Intelligence |
| I594 | Decision-requiring actions | P2 | Backend / Intelligence |
| I595 | Rejection learning for proposed actions | P2 | Backend / Intelligence |

Version brief: `.docs/plans/v1.2.0.md`.

---

### v1.3.0 — Report Engine Rebuild [PLANNED]

**Theme:** Reports become assembly surfaces that render pre-computed intelligence, not independent AI targets. Display-only by default. No generation step, no spinner. Origin: moved from v1.0.3 during planning reorganization 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I604 | Schema + assembly engine (foundation) | P0 | Backend / Reports |
| I605 | Account reports display-only | P1 | Backend + Frontend / Reports |
| I606 | Book of Business rebuild | P1 | Backend + Frontend / Reports |
| I607 | Weekly Impact with delight | P1 | Backend + Frontend / Reports |
| I608 | Monthly Wrapped with delight | P1 | Backend + Frontend / Reports |

Version brief: `.docs/plans/v1.3.0.md`.

---

### v1.4.0 — Publication + Portfolio + Intelligence Quality [PLANNED]

**Theme:** Intelligence flows upward via governed publication. ICs publish curated narrative to Google Drive. VPs consume via portfolio page. Surfacing model learns when to interrupt vs stay quiet. Origin: moved from v1.2.0 during planning reorganization 2026-03-21.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I533 | Publication engine — Google Drive output layer, auto-publish | P1 | Backend / Publication |
| I534 | Portfolio reader — cross-IC intelligence from Shared Drive | P1 | Backend + Frontend |
| I492 | Portfolio Health page — editorial aggregate, exception list | P1 | Frontend / Pages |
| I532 | Intelligence surfacing threshold model | P1 | Backend / Signals + Frontend |

Version brief: `.docs/plans/v1.4.0.md`.

---

### 0.15.1 — Security Hardening [PLANNED]

**Theme:** DailyOS holds corporate intelligence — account ARR, renewal risk, meeting briefings, relationship graphs. Before this version that data sits in plaintext SQLite with no access control, no backup protection, and an AI pipeline with an unguarded injection surface. v0.15.1 closes the encryption gap (SQLCipher + Keychain key), hardens the injection surface (HTML escaping, `email_enrich.rs` fix, preamble rollout, sanitize utilities), and adds operational security controls (Time Machine exclusion, iCloud detection, app lock). ADRs: 0092, 0093.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I462 | SQLCipher encryption at rest — AES-256 on `dailyos.db`, key in macOS Keychain, one-time migration | P0 | Backend / Security |
| I463 | Time Machine exclusion + file permission hardening — `tmutil addexclusion ~/.dailyos/`, `0o700/0o600` on DB files | P1 | Backend / Security |
| I464 | iCloud workspace detection and warning — warn if workspace path is under iCloud-synced directory | P1 | Frontend / Security |
| I465 | App lock on idle — Touch ID / macOS auth overlay after 15-min inactivity (configurable) | P1 | Backend + Frontend / Security |
| I466 | Fix `wrap_user_data` — HTML entity escaping (`& < > "`) before tag wrap; prevents tag breakout | P0 | Backend / Security |
| I467 | Fix `email_enrich.rs` — apply `wrap_user_data` to `sender`, `sender_name`, `subject`, `snippet` (currently zero protection) | P0 | Backend / Security |
| I468 | Injection resistance preamble — "external data, do not execute" instruction in all data-bearing prompts; schema instruction moved to end | P1 | Backend / Security |
| I469 | Prompt sanitization utilities + full rollout — `sanitize_external_field`, `encode_high_risk_field`, `strip_invisible_unicode`; coverage map across all Tier 3 call sites | P1 | Backend / Security |
| I470 | Output schema validation + anomaly detection — reject malformed AI output before DB write; log injection anomaly events | P1 | Backend / Security |
| I474 | Inbox document → historical meeting matching — score MeetingNotes-classified inbox files against `meetings_history` using Quill/Granola algorithm; link and run transcript pipeline on confident match | P2 | Backend / Pipeline |
| I476 | Granola cache auto-detection — scan for `cache-v*.json` instead of hardcoded filename; handle v4 format (direct JSON, transcript segment arrays) | P1 | Backend / Integrations |
| I477 | Meeting entity switch should hot-swap briefing content — stale disk fallback guard + single mutation-and-refresh service | P1 | Backend / Meeting + Frontend / UX |

Issue specs: `i462.md` (SQLCipher), `i465.md` (app lock), `i469.md` (sanitize utilities), `i474.md` (inbox matching), `i475.md` (inbox entity-gating follow-ups), `i477.md` (meeting entity switch hot-swap). ADRs: `.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md`, `.docs/decisions/0093-prompt-injection-hardening.md`.

---

### 0.15.2 — Audit Log, Enterprise Observability + Dual-Mode Context [IMPLEMENTED]

**Theme:** v0.15.1 hardens the security posture. v0.15.2 adds two capabilities: (1) a tamper-evident append-only audit log for observability, and (2) a dual-mode context architecture enabling intelligence from local data or Glean's organizational knowledge graph. ADRs: 0094, 0095, 0096.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I471 | `AuditLogger` core — append-only JSON-lines at `~/.dailyos/audit.log`, SHA-256 hash chain, 90-day rotation | P1 | Backend / Security |
| I472 | Instrument all pipeline events — security, data access, AI operation, anomaly, and config events wired to `AuditLogger` | P1 | Backend / Security |
| I473 | Settings → Data → Activity Log UI — last 100 records, category filter, anomaly highlighting, export, chain verification | P2 | Frontend / Security |
| I479 | `ContextProvider` trait + `LocalContextProvider` — pure refactor, zero behavior change | P1 | Backend / Architecture |
| I480 | `GleanContextProvider` + cache + migration — Glean MCP client, two-phase gather, graceful fallback | P1 | Backend / Connectors + Intelligence |
| I481 | Connector gating + mode switching + Settings UI — Gmail/Drive/Clay disabled in Governed mode, ContextSourceSection.tsx | P1 | Backend / Connectors + Frontend / Settings |

Issue specs: `i471.md`, `i479-done.md`, `i480-done.md`, `i481-done.md`. ADRs: 0094, 0095, 0096.

---

### 0.16.0 — Onboarding + First-Run [PLANNED]

**Theme:** The first minute determines whether someone becomes a user. Demo data shows what "ready" looks like before anything is connected. A 5-step wizard (Google → role → domain → first account → user entity) gets new users to a live briefing in under 5 minutes. Every empty state guides rather than reports emptiness.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I56 | Educational redesign — demo data mode, guided surface tour on first visit | P0 | Frontend / Onboarding |
| I57 | First-run wizard — 6-step setup: Claude Code, Google, role preset, user domains, first account, user entity basics | P0 | Frontend / Onboarding |
| I437 | Empty state redesign — every surface guides action; role-preset-aware copy | P1 | Frontend / UX |
| I478 | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |

---

### ~~0.16.1 — Beta Hardening + Search + Offline~~ [DISSOLVED]

Dissolved 2026-03-04. All issues absorbed into v1.0.0 (done) or archived. See CHANGELOG.

---

### ~~0.16.2 — UI Finesse Pass~~ [DISSOLVED]

Dissolved 2026-03-03. Editorial polish absorbed into v1.0.0 Phase 3: I447-I454. Theming (I483) deferred to post-1.0. Archived brief: `.docs/plans/_archive/v0.16.2-dissolved.md`.

---

### 1.0.0 — Local Rearchitecture + Intelligence Foundation + GA [PLANNED]

**Theme:** The GA release. Structural cleanup of the local architecture. Workspace files eliminated (DB as sole local data layer). Mandatory ServiceLayer for all mutations. Schema decomposed (meetings_history→3 tables, entity_intelligence→2 tables, entity_people+account_team→1). God modules broken up. Pipeline reliability hardened. Intelligence foundation (6-dimension schema, health scoring, relationships) ships on the clean schema. CS report suite completes. Full-text search, offline mode, data export, privacy, and editorial polish bring the app to GA quality. DailyOS stays local-first. ADR-0099 withdrawn; governance and team views solved via output-layer publication + Glean Agents, not server sync. See `.docs/research/2026-03-03-architecture-first-principles-review.md`.

Requires v0.16.0 first. Full version brief: `.docs/plans/v1.0.0.md`.

**Phase 1 — Schema + ServiceLayer + Workspace File Elimination (DONE):** All issues resolved. See CHANGELOG.

**Phase 2 — Intelligence Foundation (DONE):** All issues resolved. See CHANGELOG.

**Phase 2a — Dev Tools Mock Data Migration (DONE):** All issues resolved. See CHANGELOG.

**Phase 3 — Structural Cleanup + Surfaces + GA Readiness:**

Execution model: umbrella branch `codex/v1-phase3` + short-lived issue branches, with mandatory mock+production parity gate before merge to `main`. Tracker: `.docs/plans/phase-3-execution-tracker.md`.

**Phase 3 — Structural Cleanup + Surfaces + GA Readiness (DONE):** All issues resolved. I543/I546 moved to Phase 6. See CHANGELOG.

**Phase 4 — Reports + Success Plans + Editorial (DONE):** I547 stashed, I550 pass 1 done / pass 2 suspended. All others resolved. See v1.0.0.md.

**Phase 5 — Glean-First Intelligence:**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I535 | Glean MCP chat intelligence provider — Steps 1-8 implemented, blocked on testing. Spec: `.docs/issues/i535.md` | P0 | Backend / Connectors |
| I531 | Proactive self-healing with Glean. Spec: `.docs/issues/i531.md` | P1 | Backend / Self-Healing |
| I494 | Glean account discovery. Spec: `.docs/issues/i494.md` | P1 | Backend / Connectors + Frontend |
| I495 | Ephemeral account query. Spec: `.docs/issues/i495.md` | P1 | Backend / Connectors + Frontend |
| I560 | Glean connector optimization + token lifecycle. Spec: `.docs/issues/i560.md` | P1 | Backend / Connectors |
| I561 | Onboarding for Glean-connected users. Spec: `.docs/issues/i561.md` | P1 | Frontend / Onboarding |
| I562 | Glean chat PTY equivalence test suite. Spec: `.docs/issues/i562.md` | P2 | QA |
| I574 | Parallel dimension enrichment. Spec: `.docs/issues/i574.md` | P1 | Backend / Performance |
| I575 | Progressive enrichment everywhere. Spec: `.docs/issues/i575.md` | P1 | Backend + Frontend |
| I576 | Source-aware intelligence reconciliation. Spec: `.docs/issues/i576.md` | P1 | Backend / Intelligence |

**Phase 6 — GA Hardening:**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I543 | GA design documentation. Spec: `.docs/issues/i543.md` | P1 | Documentation |
| I546 | Interaction/data/navigation design docs. Spec: `.docs/issues/i546.md` | P2 | Documentation |
| **I563** | shellConfig render loop | P2 | Frontend / Architecture |
| **I564** | PTY enrichment blocks async runtime | P0 | Backend / Performance |
| **I565** | Split `heavy_work_semaphore` into typed resource permits — PTY, embedding, pipeline each get own capacity | P0 | Backend / Performance |
| **I566** | User-facing semaphore timeout + cancellation — manual refresh times out after 10s, shows message instead of beach ball | P0 | Backend + Frontend / Performance |
| **I567** | Narrow `refresh_emails` semaphore scope — hold permit only during PTY calls, not entire fetch+classify pipeline | P1 | Backend / Performance |
| **I568** | Migrate remaining `state.db.lock()` calls to async `db_service` — 6 legacy Mutex locks in command handlers contend with background tasks | P1 | Backend / Performance |
| **I569** | Replace `block_in_place`+`block_on` in Glean context provider with `spawn_blocking` — network calls block Tokio threads | P1 | Backend / Performance |
| **I570** | Move Keychain retry/backoff off Tokio threads — `std::thread::sleep` in token stores blocks async runtime | P2 | Backend / Performance |
| **I571** | Surface background work status to user — toast/indicator when semaphore blocks a user action | P1 | Frontend / UX |
| **I572** | Audit log coverage for OAuth token lifecycle — token save, refresh, delete, expiry events not tracked | P2 | Backend / Security |
| **I573** | Mutex poisoning recovery for critical state — re-create audit_log/config/db resources instead of permanent session failure | P2 | Backend / Stability |
| **I574** | Parallel dimension enrichment — decompose ALL monolithic AI calls (PTY + Glean) into parallel focused calls. 12 call sites audited, no single call > 30s | P0 | Backend / Performance / Intelligence |
| **I576** | Source-aware intelligence reconciliation — Glean chat as reconciliation engine, local context injected with source tags + confidence, source-attributed output items | P0 | Backend / Intelligence / Architecture |
| **I598** | Reactive health recomputation on signal arrival — close the Signals→Health Scoring loop arrow. Spec: `.docs/issues/i598.md` | P0 | Backend / Intelligence Loop / Health Scoring |
| **I575** | Progressive enrichment everywhere — write partial results as they complete, emit frontend events, show data filling in incrementally across all surfaces | P1 | Backend + Frontend / UX / Intelligence |
| I560 | Glean mode connector optimization + token lifecycle + data governance — remove Additive/Governed, auto-connector management, token health monitoring with pre-expiry notifications, in-app re-auth without restart, purge gap fixes (entity_assessment/emails/gravatar on revocation), data residency clarity. Spec: `.docs/issues/i560.md` | P0 | Backend / Connectors / Auth / Data Governance |
| I561 | Onboarding flow for Glean-connected users — wizard branches: Glean auth → auto-discover accounts → confirm → background enrichment → "Your book is ready." Profile pre-fill from org directory. No Claude Code needed. Spec: `.docs/issues/i561.md` | P0 | Frontend + Backend / Onboarding |
| I562 | Glean `chat` PTY equivalence test suite — systematic test of every PTY prompt via Glean chat. 12 call sites, 8 testable. Replace vs augment per call site. Extends I559 validation. Spec: `.docs/issues/i562.md` | P1 | Backend / Exploration |

**Phase 6 — GA Readiness (Performance, Security, Stability):**

Final pass before tagging v1.0.0. No beach balls, no silent failures, no security gaps.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I564 | PTY enrichment on blocking threads — wrap `run_enrichment()` in `spawn_blocking` in intel_queue | P0 | Backend / Performance |
| I565 | Typed resource permits — replace single `heavy_work_semaphore(1)` with PTY/embedding/pipeline permits | P0 | Backend / Performance |
| I566 | User-facing semaphore timeout — manual refresh times out after 10s with message, no infinite await | P0 | Backend + Frontend / Performance |
| I567 | Narrow `refresh_emails` semaphore scope — only PTY calls hold the permit | P1 | Backend / Performance |
| I568 | Migrate legacy `state.db.lock()` to async `db_service` | P1 | Backend / Performance |
| I569 | Glean context provider async bridging — `spawn_blocking` instead of `block_in_place`+`block_on` | P1 | Backend / Performance |
| I570 | Keychain retry off Tokio threads | P2 | Backend / Performance |
| I563 | shellConfig render loop on AccountDetailEditorial | P2 | Frontend / Architecture |
| I571 | Surface background work status to user | P1 | Frontend / UX |
| I599 | Welcome screen — eliminate blank window on startup, branded static HTML + React crossfade | P0 | Frontend + Backend / Startup UX |
| I572 | Audit log for OAuth token lifecycle | P2 | Backend / Security |
| I573 | Mutex poisoning recovery for critical state | P2 | Backend / Stability |

**v1.0.1 — Email Intelligence + Post-GA Hardening (The Correspondent enhancements):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I615 | Migration schema integrity checks + 068 rebuild | P0 | Backend / Migrations |
| I616 | Background task restart wrapper with backoff | P0 | Backend / Infrastructure |
| I617 | Top 20 console.error → toast.error for user-facing actions | P1 | Frontend / UX |
| I618 | Smoke tests for top 5 mutation services | P1 | Backend / Testing |
| I619 | Prompt evaluation suite — golden fixture tests | P1 | Backend / Testing / Intelligence |
| I596 | Email mock data foundation — seed 15-20 emails, signals, cadence, relevance scores for dev/verification | P0 | Backend / Dev Tools |
| I577 | Reply debt surface — unanswered customer emails as first-class signal | P1 | Backend + Frontend / Email |
| I578 | Render `repliesNeeded` from existing `EmailBriefingData` | P1 | Frontend / Email |
| I579 | Per-email triage actions — archive, open in Gmail, pin | P1 | Frontend + Backend / Email |
| I580 | Commitment → Action promotion from email extraction | P1 | Backend + Frontend / Email |
| I581 | Email cadence awareness — silence detection for tracked accounts | P1 | Backend + Frontend / Email |
| I582 | Email-meeting linkage — surface `pre_meeting_context` on Correspondent + Meeting Detail | P1 | Frontend / Email |
| I600 | Migrate RiskBriefingPage to reports framework | P2 | Frontend / Reports |
| I609 | Retire sync DB path from AppState | P2 | Code Quality / Backend |
| I610 | Consolidate AppLockState into single struct | P2 | Code Quality / Backend |
| I611 | Full console.error → toast sweep (~136 remaining) | P1 | Frontend / UX |
| I612 | InboxPage + AccountsPage inline style migration | P2 | Frontend / Code Quality |
| I613 | hygiene.rs decomposition into sub-modules | P2 | Code Quality / Backend |
| I614 | DB growth monitoring + age-based purge scheduler | P2 | Backend / Infrastructure |

**v1.0.2 — Actions & Success Plans: Closing the Loop:**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I597 | Action & success plan mock data expansion — 30+ actions across all states, value_delivered, recommendations, rejection patterns | P0 | Backend / Dev Tools |
| I583 | Action aging & zero-guilt cleanup — wire stale archive to scheduler, aging prompts, auto-archive proposed | P0 | Backend + Frontend / Actions |
| I584 | Action completion feeds health scoring — propagation rules, velocity metrics, engagement + momentum dimensions | P0 | Backend / Intelligence / Actions |
| I585 | Persist value delivered — store in entity_assessment, user edits survive re-enrichment, feed meeting prep + health scoring | P0 | Backend + Frontend / Intelligence |
| I586 | Action completion → milestone advancement — action-milestone linking, auto-complete cascade | P1 | Backend / Actions + Success Plans |
| I587 | Action context in meeting prep — overdue/waiting/completions/objective progress in prep context | P1 | Backend / Intelligence + Frontend / Meeting Detail |
| I588 | Recommended actions from intelligence — fill empty `recommended_actions` field, account detail + briefing surface | P1 | Backend / Intelligence |
| I589 | Completion momentum surface — velocity trends, streaks, account breadth, personality-aware copy | P1 | Frontend + Backend / Actions |
| I590 | Waiting-on as first-class status — Delegated view, aging, nudge/complete/reclaim, reply debt cross-ref | P1 | Frontend + Backend / Actions |
| I591 | Unify AI objectives with user objectives — adopt model, evidence accumulation, deduplication | P2 | Backend / Intelligence + Success Plans |
| I592 | Auto-link actions to objectives — fuzzy matching on creation, user confirmation, Bayesian correction | P2 | Backend / Actions + Success Plans |
| I593 | Captured commitments → milestone candidates — transcript commitment matching, milestone suggestions | P2 | Backend / Intelligence + Success Plans |
| I594 | Decision-requiring actions — pattern + LLM flagging, dedicated view, briefing callouts | P2 | Backend / Intelligence + Frontend / Actions |
| I595 | Rejection learning for proposed actions — pattern suppression, source fatigue, Bayesian feedback | P2 | Backend / Intelligence |

**CS Report Suite (after Phase 2):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I489 | VP Account Review report | P1 | Backend / Reports |
| I490 | Renewal Readiness report | P1 | Backend / Reports |
| I491 | Portfolio Health Summary report | P1 | Backend / Reports |
| I496 | Stakeholder Map report | P1 | Backend / Reports |
| I497 | Success Plan report | P1 | Backend / Reports |
| I498 | Coaching Patterns report | P2 | Backend / Reports |

---

### 2.1.0 — Local-First AI [DEFERRED]

**Theme:** The local-first story that is true for data becomes true for AI. IntelligenceProvider abstraction makes Claude Code, Ollama, and OpenAI interchangeable. Nothing leaves the device when Ollama is selected. (ADR-0091)

**Deprioritized from v1.1.0 (2026-02-28).** Note: I436 (workspace file deprecation) absorbed by I513 in v1.0.0 — workspace files eliminated entirely as part of remote-first rearchitecture.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I432 | IntelligenceProvider abstraction — multi-LLM backend trait replacing PtyManager in intel_queue | P1 | Backend / Architecture |
| I433 | Ollama provider — local LLM inference, nothing leaves the device | P1 | Backend / Intelligence |
| I434 | OpenAI API provider — GPT-4o with user-supplied key | P2 | Backend / Intelligence |

---

### 2.2.0 — Document Intelligence [DEFERRED]

**Theme:** Documents as first-class context sources. In-app markdown reader, document search, entity linking in documents.

**Deprioritized from v1.2.0 (2026-02-28).** See `.docs/research/2026-02-28-hook-gap-analysis.md`.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I456 | In-app markdown reader for entity documents — view .md files from account/project/person Documents/ without leaving app | P2 | Frontend / UX |

---

## Parking Lot

Acknowledged, not scheduled.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I199 | Archived account recovery UX — restore + relink | P2 | Entity |
