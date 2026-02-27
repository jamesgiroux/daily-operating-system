# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** v0.14.3 shipped (Google Drive connector). v0.15.0 in progress (CS reports + personal impact reports). 0.15.1 planned (security hardening — SQLCipher, injection fixes, app lock, inbox matching — ADRs 0092/0093). 0.15.2 implemented (audit log + enterprise observability + dual-mode context — ADRs 0094/0095/0096). 0.16.0 planned (onboarding + first-run). 0.16.1 planned (beta hardening + search + offline). 0.16.2 planned (UI finesse). 1.0.0 = ship to beta users on tag. 1.0.1 planned (CS report suite completion; Glean moved to v0.15.2). 1.1.0 planned (local-first AI — ADR-0091).

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I56** | Onboarding: educational redesign — demo data, guided tour | P0 | Onboarding |
| **I57** | Onboarding: add accounts/projects + user domain configuration | P0 | Onboarding |
| **I88** | Monthly Book Intelligence — portfolio report | P2 | Intelligence |
| **I90** | Product telemetry + analytics infrastructure | P2 | Infra |
| **I115** | Multi-line action extraction | P2 | Data |
| **I141** | AI content tagging during enrichment | P2 | Data |
| **I142** | Account Plan artifact | P2 | Entity |
| **I198** | Account merge + transcript reassignment | P2 | Entity |
| **I199** | Archived account recovery UX — restore + relink | P2 | Entity |
| **I225** | Gong integration — sales call intelligence + transcripts | P1 | Integrations |
| **I227** | Gainsight integration — CS platform data sync | P2 | Integrations |
| **I230** | Claude Cowork integration — project/task sync | P2 | Integrations |
| **I258** | Report Mode — export account detail as leadership-ready deck/PDF | P2 | UX |
| **I277** | Marketplace repo for community preset discoverability | P3 | Integrations |
| **I280** | Beta hardening umbrella — dependency, DB, token, DRY audit | P1 | Code Quality |
| **I302** | Shareable PDF export for intelligence reports (editorial-styled) | P2 | UX |
| ~~I340~~ | ~~Glean integration~~ — superseded by I479-I481 in v0.15.2 | — | — |
| **I347** | SWOT report type — account analysis from existing intelligence | P2 | Intelligence / Reports |
| **I348** | Email digest push — DailyOS intelligence summaries via scheduled email | P2 | Distribution |
| **I350** | In-app notifications — release announcements, what's new, system status | P1 | UX / Infra |
| ~~I357~~ | ~~Semantic email reclassification~~ — absorbed by I367 (mandatory enrichment) | — | — |
| **I359** | Vocabulary-driven prompts — inject all 7 role fields into enrichment + briefing prompts | P2 | Intelligence |
| **I360** | Community preset import UI — frontend caller for existing import backend | P2 | UX |
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
| **I380** | commands.rs service extraction Phase 1 — complete services/ per SERVICE-CONTRACTS.md | P1 | Code Quality / Refactor |
| **I381** | db/mod.rs domain migration — move queries into domain modules per SERVICE-CONTRACTS.md | P2 | Code Quality / Refactor |
| **I382** | Partner entity type — `partner` account type, badge, partner-appropriate prompt shape | P1 | Backend / Entity |
| **I383** | AccountsPage three-group layout — Your Book / Your Team / Your Partners | P1 | Frontend / UX |
| **I384** | Parent account portfolio intelligence — two-layer intelligence.json (portfolio synthesis + own signals) | P1 | Backend / Intelligence |
| **I385** | Bidirectional entity hierarchy signal propagation — upward accumulation, downward fan-out | P1 | Backend / Signals |
| **I386** | Calendar lifecycle gaps — future meeting cancellation detection, rescheduling sync, continuous future polling | P1 | Backend / Calendar |
| **I387** | Multi-entity signal extraction from parent-level meetings — content-level entity resolution in transcript processor | P3 | Backend / Pipeline |
| **I388** | Project hierarchy intelligence — two-layer intelligence.json + bidirectional propagation for project entities | P1 | Backend / Intelligence |
| **I389** | Entity-mode-aware surface ordering — nav/primary surface adapts to preset's entityModeDefault | P2 | Frontend / UX |
| **I390** | Person relationship graph — `person_relationships` table, typed edges, confidence scoring, context scoping | P1 | Backend / Entity |
| **I391** | People network intelligence — two-layer intelligence.json + network section + person→person signal propagation | P1 | Backend / Intelligence |
| **I392** | Relationship cluster view on person detail — Network chapter, cluster summary, risks/opportunities, preset vocabulary | P1 | Frontend / UX |
| **I393** | Parent account detail page — portfolio surface (hotspots, cross-BU patterns, portfolio narrative) | P1 | Frontend / UX |
| **I394** | Week page past meeting duration shows NaN — `formatDurationFromIso` doesn't guard against NaN from invalid date strings | P1 | Frontend / Bug |
| **I395** | Email relevance scoring — signal-driven surfacing using entity linkage, Bayesian fusion, embedding relevance, keyword matching | P0 | Backend / Intelligence + Frontend / UX |
| ~~I258~~ | ~~Report Mode~~ — superseded by I397 (report infrastructure) | — | — |
| ~~I302~~ | ~~Shareable PDF export~~ — absorbed into I397 (report infrastructure) | — | — |
| ~~I347~~ | ~~SWOT report type~~ — absorbed into I397 (report infrastructure, bundled format) | — | — |
| ~~I348~~ | ~~Email digest push~~ — removed; creates feedback loop into email processing pipeline | — | — |
| **I396** | intelligence.json report fields — health_score, health_trend, value_delivered, success_metrics, open_commitments, relationship_depth | P1 | Backend / Intelligence |
| **I397** | Report infrastructure — `reports` table, intel_hash invalidation, ReportShell renderer, PDF export, SWOT format | P1 | Backend / Frontend |
| **I398** | Risk report migration — move existing risk briefing to ADR-0086 architecture (entity_intel DB input, reports table output) | P1 | Backend |
| **I399** | Account Health Review report type — internal CS report: health score/trend, risks, stakeholder coverage, open commitments | P1 | Backend / Reports |
| **I400** | EBR/QBR report type — flagship CS customer-facing quarterly review: value delivered, success metrics, asks | P1 | Backend / Reports |
| **I401** | Show internal attendees for internal meetings — hydrate_attendee_context filters out all internal people, leaving "The Room" empty for team syncs | P2 | Backend / UX |
| **I402** | IntelligenceService extraction — move intelligence/enrichment business logic from commands.rs to services/intelligence.rs | P1 | Code Quality / Refactor |
| **I403** | SignalService formalization — formalize signals/ module boundary as a service with clear public API | P2 | Code Quality / Refactor |
| **I450** | ProjectService extraction — move 6 thick project handlers to services/projects.rs | P1 | Code Quality / Refactor |
| **I451** | EmailService expansion — move email mutation handlers to services/emails.rs | P1 | Code Quality / Refactor |
| **I452** | AccountService expansion — move create_internal_organization + child account handlers | P1 | Code Quality / Refactor |
| **I453** | MeetingService expansion — move refresh_meeting_preps + attach_meeting_transcript | P1 | Code Quality / Refactor |
| **I454** | SettingsService extraction — create services/settings.rs, move 7 settings handlers | P1 | Code Quality / Refactor |
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
| **I427** | Full-text search — Cmd+K finds entities, meetings, actions, contacts using SQLite FTS5; results in < 300ms | P1 | Frontend + Backend |
| **I428** | Offline/degraded mode — serve cached intelligence gracefully when APIs unavailable; system status indicator | P1 | Backend |
| **I429** | Data export — JSON ZIP export of entities, signals, intelligence; portability guarantee | P1 | Backend |
| **I430** | Privacy clarity — Settings section explaining what's stored, how long, clear intelligence + delete all data options | P1 | Frontend |
| **I431** | Cost visibility — Claude call tracking, estimated weekly cost breakdown in Settings | P2 | Backend + Frontend |
| **I432** | IntelligenceProvider abstraction — multi-LLM backend trait replacing direct PtyManager calls in intel_queue | P1 | Backend / Architecture |
| **I433** | Ollama provider — local LLM support; nothing leaves the device when Ollama is selected | P1 | Backend / Intelligence |
| **I434** | OpenAI API provider — GPT-4o with user-supplied key; shares HTTP client with Ollama (OpenAI-compatible) | P2 | Backend / Intelligence |
| **I435** | Token optimization — audit ModelTier usage; Haiku for email enrichment; quality-gated entity enrichment | P1 | Backend |
| **I436** | Workspace file deprecation — DB as sole source of truth; remove sync_people_from_workspace; retire _today/data/ files | P2 | Backend / Architecture |
| **I437** | Empty state redesign — every surface guides action rather than reporting emptiness; role-preset-aware copy | P1 | Frontend / UX |
| **I438** | Onboarding: Prime DailyOS — first content ingestion step; manual (drop transcript/doc) or connector (Quill/Granola/Drive); teaches feeding habit before automation takes over | P0 | Frontend / Onboarding |
| **I439** | Personality expanded in UI — completion messages, generating states, toasts, error copy; never touches AI prompts (role presets handle intelligence framing) | P1 | Frontend / UX |
| **I440** | Meeting prep preset persona — remove hardcoded "Customer Success Manager"; use active preset role name and vocabulary | P1 | Backend / Intelligence |
| **I441** | Personality coverage + useActivePreset cache — fill 3 missing empty state keys; shared reactive context replacing per-page IPC calls | P1 | Frontend |
| **I442** | stakeholder_roles wired — relationship type dropdowns on person-to-account linking pull from active preset | P1 | Backend / Frontend |
| **I443** | internal_team_roles wired — account team role selectors pull from active preset | P1 | Backend / Frontend |
| **I444** | lifecycle_events wired — lifecycle stage pickers use preset events; stage injected into entity intelligence prompts | P1 | Backend / Frontend |
| **I445** | prioritization wired — account list default sort and weekly forecast ranking use preset's primary_signal and urgency_drivers | P1 | Backend / Frontend |
| **I446** | User entity page × role preset — section prominence, vocabulary/placeholders, named playbook sections for all 9 presets | P1 | Frontend / Entity |
| **I447** | Design token audit — formalise opacity tokens, fix phantom token (`eucalyptus`), replace all rgba() violations, unify max-width | P1 | Frontend / Tokens |
| **I448** | ActionsPage editorial rebuild — CSS module, margin grid, ChapterHeadings for groups, correct max-width, unconditional FinisMarker | P1 | Frontend / Actions |
| **I449** | WeekPage + EmailsPage CSS module polish — TimelineDayGroup module, stat line tokens, EditorialLoading/EditorialError, FinisMarker | P1 | Frontend |
| **I450** | Portfolio chapter extraction — shared CSS module for Account + Project Detail portfolio; conclusion-before-evidence editorial order | P1 | Frontend / Entity |
| **I451** | MeetingDetailPage polish — Recent Correspondence editorial treatment; avatar tint tokens; FinisMarker unconditional | P2 | Frontend / Meeting |
| **I452** | Settings page editorial audit — inline style cleanup, vocabulary compliance, section rules, FinisMarker | P2 | Frontend / Settings |
| **I453** | Onboarding pages editorial standards — v0.16.0 wizard/demo/tour built to editorial spec; no inline styles | P1 | Frontend / Onboarding |
| **I454** | Vocabulary pass — replace all remaining user-visible system terms per ADR-0083 | P1 | Frontend / Copy |
| **I455** | 1:1 meeting prep focuses on person entity intelligence, not account | P1 | Backend / Intelligence |
| **I456** | In-app markdown reader for entity documents — view .md files from account/project/person Documents/ without leaving app | P2 | Frontend / UX |
| **I457** | Background task throttling — ActivityMonitor, HeavyWorkSemaphore, adaptive polling intervals | P1 | Backend / Performance |
| **I475** | Inbox entity-gating follow-ups — transcript NeedsEntity path, onAssignEntity result check, enrich.rs redundant DB, action account validation | P2 | Backend / Pipeline + Frontend / UX |
| **I477** | Meeting entity switch should hot-swap briefing content — stale disk fallback guard + single mutation-and-refresh service | P1 | Backend / Meeting + Frontend / UX |
| **I478** | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |
| ~~I479~~ | ~~ContextProvider trait + LocalContextProvider — pure refactor~~ — done in v0.15.2 | P1 | Backend / Architecture |
| ~~I480~~ | ~~GleanContextProvider + cache + migration~~ — done in v0.15.2 | P1 | Backend / Connectors + Intelligence |
| ~~I481~~ | ~~Connector gating + mode switching + Settings UI~~ — done in v0.15.2 | P1 | Backend / Connectors + Frontend / Settings |
| **I458** | Renewal Readiness report type — CS report for accounts renewing in the next 90 days; risk rating, champion alignment, recommended actions | P1 | Backend / Reports |
| **I459** | Stakeholder Map report type — relationship network, coverage gaps, engagement levels; uses people relationship network (v0.13.5) | P1 | Backend / Reports |
| **I460** | Success Plan report type — mutual objectives, success metrics, responsibilities; requires user entity context (v0.14.0) to be meaningful | P1 | Backend / Reports |
| **I461** | Coaching Patterns — pattern recognition across 3+ months of Monthly Wrapped; factual not prescriptive | P2 | Backend / Reports |
| **I482** | Role-aware Glean query optimization — preset vocabulary shapes search queries, lifecycle-stage filtering, dedup | P1 | Backend / Connectors + Intelligence |

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

### v1.0.1 — CS Report Suite Completion [PLANNED]

**Theme:** The CS report suite completes with Renewal Readiness, Stakeholder Map, and Success Plan — the three reports a CSM needs beyond EBR/QBR. Coaching Patterns emerges from 3+ months of Monthly Wrapped history.

> **Note:** I340 (Glean integration) has been superseded by I479-I481, shipped in v0.15.2. The dual-mode context architecture (ContextProvider trait, GleanContextProvider, connector gating) now lives there. See ADRs 0095 and 0096.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| ~~I340~~ | ~~Glean integration~~ — superseded by I479-I481 in v0.15.2 | — | — |
| I458 | Renewal Readiness report — 90-day renewal window, risk rating, champion alignment, recommended actions | P1 | Backend / Reports |
| I459 | Stakeholder Map report — relationship network, coverage gaps, engagement levels | P1 | Backend / Reports |
| I460 | Success Plan report — mutual objectives, metrics, responsibilities (requires user entity context) | P1 | Backend / Reports |
| I461 | Coaching Patterns — pattern recognition across 3+ Monthly Wrapped reports; factual not prescriptive | P2 | Backend / Reports |
| I482 | Role-aware Glean query optimization — preset vocabulary shapes search queries, lifecycle-stage filtering, query budget, dedup | P1 | Backend / Connectors + Intelligence |

See `.docs/research/cs-report-types.md`. Coaching Patterns (I461) only activates when 3+ Monthly Wrapped reports exist.

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
| I475 | Inbox entity-gating follow-ups — transcript NeedsEntity path, onAssignEntity result check, enrich.rs redundant DB, action account validation | P2 | Backend / Pipeline + Frontend / UX |
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
| I438 | Prime DailyOS — first content ingestion step in wizard; manual (drop file) or connector (Quill/Granola/Drive) | P0 | Frontend / Onboarding |
| I478 | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |

---

### 0.16.1 — Beta Hardening + Search + Offline [PLANNED]

**Theme:** The app needs to be trustworthy (I280 hardening), findable (I427 search), graceful (I428 offline mode), honest (I429 export, I430 privacy, I431 cost), and efficient (I435 token optimization). The last pass before the stability window.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I280 | Beta hardening — dependency audit, DB integrity, input validation, security review | P1 | Code Quality |
| I427 | Full-text search — Cmd+K entity/meeting/action search via SQLite FTS5, < 300ms | P1 | Frontend + Backend |
| I428 | Offline/degraded mode — cached intelligence served gracefully; system status indicator | P1 | Backend |
| I429 | Data export — JSON ZIP export of all entities, signals, intelligence | P1 | Backend |
| I430 | Privacy clarity — what's stored, how long, clear intelligence + delete all data | P1 | Frontend |
| I431 | Cost visibility — Claude call tracking, estimated weekly cost in Settings | P2 | Backend + Frontend |
| I435 | Token optimization — ModelTier audit; Haiku for email enrichment; quality-gated enrichment | P1 | Backend |

---

### 0.16.2 — UI Finesse Pass [PLANNED]

**Theme:** The app is 60% editorial magazine, 40% database output — highly variable by page. This version closes the gap. Every page is audited against DESIGN-SYSTEM.md and made consistent: tokens instead of rgba(), margin grid instead of flex columns of divs, ChapterHeadings instead of raw div group headers, FinisMarker always visible, no phantom tokens. No new features. Pure presentation quality.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I447 | Design token audit — opacity tokens, phantom token, rgba violations, max-width unification | P1 | Frontend / Tokens |
| I448 | ActionsPage editorial rebuild — CSS module, margin grid, ChapterHeadings, correct max-width | P1 | Frontend / Actions |
| I449 | WeekPage + EmailsPage CSS module polish — TimelineDayGroup, stat lines, loading/error components | P1 | Frontend |
| I450 | Portfolio chapter extraction — CSS module for Account + Project Detail portfolio, editorial prose order | P1 | Frontend / Entity |
| I451 | MeetingDetailPage polish — Recent Correspondence editorial treatment, avatar token colours | P2 | Frontend / Meeting |
| I452 | Settings page editorial audit — inline styles, vocabulary compliance, FinisMarker | P2 | Frontend / Settings |
| I453 | Onboarding pages editorial standards — v0.16.0 wizard and demo pages built to editorial spec | P1 | Frontend / Onboarding |
| I454 | Vocabulary pass — grep all user-visible strings; replace remaining system terms per ADR-0083 | P1 | Frontend / Copy |

---

### 1.0.0 — General Availability

Requires v0.16.0 + v0.16.1 + v0.16.2 (visual consistency). All GA checklist items in `.docs/plans/v1.0.0.md` must be met. Ship to beta users on tag — no stability window.

---

### 1.1.0 — Local-First AI [PLANNED]

**Theme:** The local-first story that is true for data becomes true for AI. IntelligenceProvider abstraction makes Claude Code, Ollama, and OpenAI interchangeable. Nothing leaves the device when Ollama is selected. (ADR-0091)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I432 | IntelligenceProvider abstraction — multi-LLM backend trait replacing PtyManager in intel_queue | P1 | Backend / Architecture |
| I433 | Ollama provider — local LLM inference, nothing leaves the device | P1 | Backend / Intelligence |
| I434 | OpenAI API provider — GPT-4o with user-supplied key | P2 | Backend / Intelligence |
| I436 | Workspace file deprecation — DB as sole source of truth; retire `_today/data/` and workspace sync | P2 | Backend / Architecture |

---

## Integrations Queue

Not version-locked. Pulled in when capacity allows.

| ID | Title | Priority | Dependency |
|----|-------|----------|------------|
| I225 | Gong integration — sales call intelligence + transcripts | P1 | Gong API access |
| I227 | Gainsight integration — CS platform data sync | P2 | Gainsight API |
| I230 | Claude Cowork integration — project/task sync | P2 | API TBD |
| ~~I340~~ | ~~Glean integration~~ — superseded by I479-I481 in v0.15.2 | — | — |
| I277 | Marketplace repo — preset community discoverability | P3 | — |

---

## Parking Lot

Acknowledged, not scheduled.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I88 | Monthly Book Intelligence — portfolio report | P2 | Intelligence |
| I90 | Product telemetry + analytics infrastructure | P2 | Infra |
| I115 | Multi-line action extraction | P2 | Data |
| I141 | AI content tagging during enrichment | P2 | Data |
| I142 | Account Plan artifact | P2 | Entity |
| I198 | Account merge + transcript reassignment | P2 | Entity |
| I199 | Archived account recovery UX — restore + relink | P2 | Entity |
| I359 | Vocabulary-driven prompts — inject all 7 role fields into enrichment + briefing prompts | P2 | Intelligence |
| I360 | Community preset import UI — frontend caller for existing import backend | P2 | UX |
