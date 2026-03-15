# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** v0.15.2 shipped. v0.16.0 mostly done (onboarding). v0.16.2 dissolved; v0.16.1 remains tracked as a hardening addendum for trust-critical work (I527). **v1.0.0 is the GA release (rescoped 2026-03-03):** local rearchitecture (schema decomp, ServiceLayer, workspace elimination, module decomp, pipeline reliability) + intelligence foundation (6-dimension schema, health scoring, relationships) + CS report suite + GA readiness (search, offline, data export, privacy, editorial polish, vocabulary). ADR-0099 (remote-first) withdrawn. v1.1.0: publication + portfolio + Glean Agents. See `.docs/research/2026-03-03-architecture-first-principles-review.md`.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I56** | Onboarding: educational redesign — demo data, guided tour | P0 | Onboarding |
| **I57** | Onboarding: add accounts/projects + user domain configuration | P0 | Onboarding |
| ~~I88~~ | ~~Monthly Book Intelligence~~ — superseded by I491/I492 portfolio reports | — | — |
| ~~I90~~ | ~~Product telemetry + analytics infrastructure~~ — archived (partially absorbed by audit log in v0.15.2) | — | — |
| ~~I115~~ | ~~Multi-line action extraction~~ — superseded by transcript pipeline improvements | — | — |
| ~~I141~~ | ~~AI content tagging during enrichment~~ — superseded by intelligence schema (I508) | — | — |
| ~~I142~~ | ~~Account Plan artifact~~ — superseded by reports suite | — | — |
| **I198** | Account merge + transcript reassignment | P2 | Entity |
| **I199** | Archived account recovery UX — restore + relink | P2 | Entity |
| ~~I225~~ | ~~Gong integration~~ — done (Gong transcripts via Glean) | — | — |
| ~~I227~~ | ~~Gainsight integration~~ — archived (won't do, no clear path) | — | — |
| ~~I230~~ | ~~Claude Cowork integration~~ — done (obsoleted by product changes) | — | — |
| **I258** | Report Mode — export account detail as leadership-ready deck/PDF | P2 | UX |
| ~~I277~~ | ~~Marketplace repo for community preset discoverability~~ — archived (won't do) | — | — |
| ~~I280~~ | ~~Beta hardening umbrella~~ — archived (scope absorbed by individual issues in v0.15.1/v0.16.1) | — | — |
| **I302** | Shareable PDF export for intelligence reports (editorial-styled) | P2 | UX |
| ~~I340~~ | ~~Glean integration~~ — superseded by I479-I481 in v0.15.2 | — | — |
| **I347** | SWOT report type — account analysis from existing intelligence | P2 | Intelligence / Reports |
| **I348** | Email digest push — DailyOS intelligence summaries via scheduled email | P2 | Distribution |
| **I350** | In-app notifications — release announcements, what's new, system status | P1 | UX / Infra |
| ~~I357~~ | ~~Semantic email reclassification~~ — absorbed by I367 (mandatory enrichment) | — | — |
| ~~I359~~ | ~~Vocabulary-driven prompts~~ — done (all 7 fields injected) | — | — |
| ~~I360~~ | ~~Community preset import UI~~ — archived (won't do) | — | — |
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
| ~~I380~~ | ~~commands.rs service extraction Phase 1~~ — absorbed by I512 (ServiceLayer) + I514 (module decomp) in v1.0.0 | — | — |
| ~~I381~~ | ~~db/mod.rs domain migration~~ — absorbed by I511 (schema decomposition) in v1.0.0 | — | — |
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
| ~~I402~~ | ~~IntelligenceService extraction~~ — absorbed by I512 (ServiceLayer) in v1.0.0 | — | — |
| **I403** | SignalService formalization — formalize signals/ module boundary as a service with clear public API | P2 | Code Quality / Refactor |
| ~~I450~~ | ~~ProjectService extraction — move 6 thick project handlers to services/projects.rs~~ — done | — | — |
| ~~I451~~ | ~~EmailService expansion — move email mutation handlers to services/emails.rs~~ — done | — | — |
| ~~I452~~ | ~~AccountService expansion — move create_internal_organization + child account handlers~~ — done | — | — |
| ~~I453~~ | ~~MeetingService expansion — move refresh_meeting_preps + attach_meeting_transcript~~ — done | — | — |
| ~~I454~~ | ~~SettingsService extraction — create services/settings.rs, move 7 settings handlers~~ — done | — | — |
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
| ~~I427~~ | ~~Full-text search — Cmd+K finds entities, meetings, actions, contacts using SQLite FTS5; results in < 300ms~~ — done | — | — |
| ~~I428~~ | ~~Offline/degraded mode — serve cached intelligence gracefully when APIs unavailable; system status indicator~~ — done | — | — |
| ~~I429~~ | ~~Data export — JSON ZIP export of entities, signals, intelligence; portability guarantee~~ — done | — | — |
| ~~I430~~ | ~~Privacy clarity — Settings section explaining what's stored, how long, clear intelligence + delete all data options~~ — done | — | — |
| **I431** | Cost visibility — Claude call tracking, estimated weekly cost breakdown in Settings | P2 | Backend + Frontend |
| **I432** | IntelligenceProvider abstraction — multi-LLM backend trait replacing direct PtyManager calls in intel_queue | P1 | Backend / Architecture |
| **I433** | Ollama provider — local LLM support; nothing leaves the device when Ollama is selected | P1 | Backend / Intelligence |
| **I434** | OpenAI API provider — GPT-4o with user-supplied key; shares HTTP client with Ollama (OpenAI-compatible) | P2 | Backend / Intelligence |
| **I435** | Token optimization — audit ModelTier usage; Haiku for email enrichment; quality-gated entity enrichment | P1 | Backend |
| ~~I436~~ | ~~Workspace file deprecation~~ — absorbed by I513 (workspace file elimination) in v1.0.0 | — | — |
| **I437** | Empty state redesign — every surface guides action rather than reporting emptiness; role-preset-aware copy | P1 | Frontend / UX |
| ~~I438~~ | ~~Onboarding: Prime DailyOS — first content ingestion step; manual (drop transcript/doc) or connector (Quill/Granola/Drive); teaches feeding habit before automation takes over~~ — done | — | — |
| **I439** | Personality expanded in UI — completion messages, generating states, toasts, error copy; never touches AI prompts (role presets handle intelligence framing) | P1 | Frontend / UX |
| **I440** | Meeting prep preset persona — remove hardcoded "Customer Success Manager"; use active preset role name and vocabulary | P1 | Backend / Intelligence |
| **I441** | Personality coverage + useActivePreset cache — fill 3 missing empty state keys; shared reactive context replacing per-page IPC calls | P1 | Frontend |
| **I442** | stakeholder_roles wired — relationship type dropdowns on person-to-account linking pull from active preset | P1 | Backend / Frontend |
| **I443** | internal_team_roles wired — account team role selectors pull from active preset | P1 | Backend / Frontend |
| **I444** | lifecycle_events wired — lifecycle stage pickers use preset events; stage injected into entity intelligence prompts | P1 | Backend / Frontend |
| **I445** | prioritization wired — account list default sort and weekly forecast ranking use preset's primary_signal and urgency_drivers | P1 | Backend / Frontend |
| **I446** | User entity page × role preset — section prominence, vocabulary/placeholders, named playbook sections for all 9 presets | P1 | Frontend / Entity |
| ~~I447~~ | ~~Design token audit — formalise opacity tokens, fix phantom token (`eucalyptus`), replace all rgba() violations, unify max-width~~ — done | — | — |
| ~~I448~~ | ~~ActionsPage editorial rebuild — CSS module, margin grid, ChapterHeadings for groups, correct max-width, unconditional FinisMarker~~ — done | — | — |
| ~~I449~~ | ~~WeekPage + EmailsPage CSS module polish — TimelineDayGroup module, stat line tokens, EditorialLoading/EditorialError, FinisMarker~~ — done | — | — |
| ~~I450~~ | ~~Portfolio chapter extraction — shared CSS module for Account + Project Detail portfolio; conclusion-before-evidence editorial order~~ — done | — | — |
| ~~I451~~ | ~~MeetingDetailPage polish — Recent Correspondence editorial treatment; avatar tint tokens; FinisMarker unconditional~~ — done | — | — |
| ~~I452~~ | ~~Settings page editorial audit — inline style cleanup, vocabulary compliance, section rules, FinisMarker~~ — done | — | — |
| ~~I453~~ | ~~Onboarding pages editorial standards — v0.16.0 wizard/demo/tour built to editorial spec; no inline styles~~ — done | — | — |
| ~~I454~~ | ~~Vocabulary pass — replace all remaining user-visible system terms per ADR-0083~~ — done | — | — |
| **I455** | 1:1 meeting prep focuses on person entity intelligence, not account | P1 | Backend / Intelligence |
| **I456** | In-app markdown reader for entity documents — view .md files from account/project/person Documents/ without leaving app | P2 | Frontend / UX |
| **I457** | Background task throttling — ActivityMonitor, HeavyWorkSemaphore, adaptive polling intervals | P1 | Backend / Performance |
| **I475** | Inbox entity-gating follow-ups — transcript NeedsEntity path, onAssignEntity result check, enrich.rs redundant DB, action account validation | P2 | Backend / Pipeline + Frontend / UX |
| **I477** | Meeting entity switch should hot-swap briefing content — stale disk fallback guard + single mutation-and-refresh service | P1 | Backend / Meeting + Frontend / UX |
| **I478** | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |
| ~~I479~~ | ~~ContextProvider trait + LocalContextProvider — pure refactor~~ — done in v0.15.2 | P1 | Backend / Architecture |
| ~~I480~~ | ~~GleanContextProvider + cache + migration~~ — done in v0.15.2 | P1 | Backend / Connectors + Intelligence |
| ~~I481~~ | ~~Connector gating + mode switching + Settings UI~~ — done in v0.15.2 | P1 | Backend / Connectors + Frontend / Settings |
| ~~I458~~ | ~~Renewal Readiness report type~~ — absorbed by I490 in v1.1.0 | — | — |
| ~~I459~~ | ~~Stakeholder Map report type~~ — absorbed by I496 in v1.1.0 | — | — |
| ~~I460~~ | ~~Success Plan report type~~ — absorbed by I497 in v1.1.0 | — | — |
| ~~I461~~ | ~~Coaching Patterns~~ — absorbed by I498 in v1.1.0 | — | — |
| **I482** | Role-aware Glean query optimization — preset vocabulary shapes search queries, lifecycle-stage filtering, dedup | P1 | Backend / Connectors + Intelligence |
| **I483** | Theme infrastructure + shipped presets — `data-theme` token layering, typography scale controls, three shipped themes (Warm/Dark/Cool), Settings picker, custom theme docs | P1 | Frontend / Tokens + Settings |
| ~~I484~~ | ~~Health score always-on~~ — superseded by I499–I503 per ADR-0097 | — | — |
| ~~I499~~ | ~~Health scoring engine — 6 algorithmic relationship dimensions, lifecycle weighting, sparse data handling~~ — done | — | — |
| ~~I500~~ | ~~Glean org-score parsing — extract structured health data from Glean results as baseline~~ — done | — | — |
| ~~I501~~ | ~~Transcript sentiment extraction~~ — absorbed by I509 (interaction dynamics + sentiment) | — | — |
| ~~I502~~ | ~~Health surfaces — render health band, dimensions, divergence across all app pages~~ — done | — | — |
| ~~I503~~ | ~~intelligence.json health schema evolution — AccountHealth struct, RelationshipDimensions, migration~~ — done | — | — |
| ~~I485~~ | ~~Store inferred relationships from enrichment~~ — superseded by I504–I506 | — | — |
| ~~I486~~ | ~~Glean structured person data writeback~~ — absorbed by I505 | — | — |
| ~~I504~~ | ~~AI-inferred relationship extraction — fix prompt schema, call extraction function, persist to person_relationships~~ — done | — | — |
| ~~I505~~ | ~~Glean stakeholder intelligence — contact discovery, profile enrichment, entity linkage, manager relationships, team sync (absorbs I486)~~ — done | — | — |
| ~~I506~~ | ~~Co-attendance relationship inference — algorithmic collaborator/peer edges from meeting frequency~~ — done | — | — |
| ~~I487~~ | ~~Glean signal emission — new-only document signals, ADR-0098 purge compliance (person signals → I505)~~ — done | — | — |
| ~~I507~~ | ~~Source-attributed correction feedback — close feedback loop for Glean, Clay, email sources~~ — done | — | — |
| ~~I488~~ | ~~Semantic gap queries sent to Glean~~ — absorbed by I508 (intelligence schema redesign) | — | — |
| ~~I508~~ | ~~Intelligence schema redesign for multi-source enrichment — 6 research-grounded dimensions, gap detection, source-agnostic prompt~~ — done | — | — |
| ~~I509~~ | ~~Transcript personal interpretation + sentiment — personal priority impact, relationship trajectory, sentiment as local signal; org-level dynamics deferred to Glean; absorbs I501 (← I508)~~ — done | — | — |
| **I489** | VP Account Review report type — leadership-facing strategic assessment, risk/opportunity matrix, VP-level actions | P1 | Backend / Reports |
| **I490** | Renewal Readiness report type (absorbs I458) — 90-day renewal risk assessment, readiness rating, champion alignment | P1 | Backend / Reports |
| **I491** | Portfolio Health Summary report type — cross-account VP synthesis, exceptions, renewal pipeline, portfolio narrative | P1 | Backend / Reports |
| **I492** | Portfolio Health page — editorial aggregate view, health heatmap, exception list, renewal timeline | P1 | Frontend / Pages |
| ~~I493~~ | ~~Account detail enriched intelligence surface — Glean-sourced titles, coverage gaps, reports chapter (health rendering owned by I502)~~ — done | — | — |
| **I494** | Glean account discovery flow — import CRM accounts via Glean, one-click add with pre-populated context | P1 | Backend / Connectors + Frontend |
| **I495** | Ephemeral account query — "tell me about X" transient briefing via Glean, no persistent entity | P1 | Backend / Connectors + Frontend |
| **I496** | Stakeholder Map report type (absorbs I459) — coverage grid, influence network, engagement assessment | P1 | Backend / Reports |
| **I497** | Success Plan report type (absorbs I460) — shared objectives, progress, customer-presentable | P1 | Backend / Reports |
| **I498** | Coaching Patterns report type (absorbs I461) — meeting cadence, engagement patterns, book-level norms, coaching recommendations | P2 | Backend / Reports |
| ~~I510~~ | ~~Supabase project provisioning~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I511~~ | ~~Local schema decomposition + migration safety hardening (backend-only) — fail-hard runner, guaranteed backups, atomic decomposition migration, schema integrity checks~~ — done | — | — |
| ~~I512~~ | ~~ServiceLayer — every mutation → local DB write + signal emission (mandatory mutation path)~~ — done | — | — |
| ~~I513~~ | ~~Workspace file elimination — DB as sole local data layer, no _today/data/, no intelligence.json on disk~~ — done | — | — |
| ~~I514~~ | ~~Module decomposition — commands.rs→domain files, db.rs→domain modules. Spec: `.docs/issues/i514.md`~~ — done | — | — |
| ~~I515~~ | ~~Pipeline reliability — retry with backoff, circuit breaker, partial result preservation, pipeline_failures table. Spec: `.docs/issues/i515.md`~~ — done | — | — |
| ~~I516~~ | ~~Sync engine~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I517~~ | ~~Supabase Auth~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I518~~ | ~~Organization + territory model~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I519~~ | ~~RLS policy design~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I520~~ | ~~Auth-first onboarding~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I521~~ | ~~Frontend structural cleanup + production-data parity gate — remove ghost components, consolidate duplicate patterns, lock command/field contracts, enforce mock+production fixture parity. Spec: `.docs/issues/i521.md`~~ — done | — | — |
| ~~I522~~ | ~~Server-side embedding pipeline~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I523~~ | ~~Admin panel~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I524~~ | ~~Conflict resolution~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I525~~ | ~~Offline mode redesign~~ — withdrawn with ADR-0099 (2026-03-03). I428 restored. | — | — |
| ~~I526~~ | ~~Online/offline detection~~ — withdrawn with ADR-0099 (2026-03-03) | — | — |
| ~~I527~~ | ~~Intelligence consistency guardrails — deterministic contradiction checks, balanced repair retry, and corrected/flagged trust surfacing for intelligence output. Spec: `.docs/issues/i527.md`~~ — done | — | — |
| ~~I528~~ | ~~ADR-0098 data lifecycle infrastructure — DataSource enum, purge_source(), data_lifecycle.rs. Prerequisite for I487 + I505. Spec: `.docs/issues/i528.md`~~ — done | — | — |
| ~~I529~~ | ~~Intelligence quality feedback UI — thumbs up/down on hover for any intelligence item. Feeds Bayesian source weights. Spec: `.docs/issues/i529.md`~~ — done | — | — |
| ~~I530~~ | ~~Signal taxonomy: curation vs correction — delete = no source penalty, edit = correction, thumbs down = correction. Spec: `.docs/issues/i530.md`~~ — done | — | — |
| **I531** | Glean-powered proactive self-healing — hygiene detects gaps, searches Glean, fills intelligence. Signal emission fixes. Spec: `.docs/issues/i531.md` | P1 | Backend / Self-Healing / Connectors |
| **I532** | Intelligence surfacing threshold model — significance scoring, surfacing budgets, fatigue prevention, feedback-driven learning. Spec: `.docs/issues/i532.md` | P1 | Backend / Signals + Frontend / Briefing |
| **I533** | Publication engine — Google Drive output layer. Reports published as PDF/Google Doc to Shared Drive. Auto-indexed by Glean. Spec: `.docs/issues/i533.md` | P1 | Backend / Publication |
| **I534** | Portfolio reader — read published intelligence from Shared Drive for cross-IC portfolio synthesis. JSON sidecar parsing. Spec: `.docs/issues/i534.md` | P1 | Backend + Frontend |
| **I535** | Glean Agent integration — call purpose-built Glean Agents via REST API for org-level analysis during enrichment. Spec: `.docs/issues/i535.md` — Steps 1-8 implemented, blocked on testing | P1 | Backend / Connectors |
| ~~I536~~ | ~~Dev tools mock data migration — rewrite seed data for v1.0.0 schema, consolidate scenarios (6→4), eliminate workspace file writes, rich 6-dimension intelligence data. Spec: `.docs/issues/i536.md`~~ — done | — | — |
| ~~I537~~ | ~~Gate role presets behind feature flag — hide preset selection UI (onboarding + settings), hard-default to CS. Preset infrastructure stays. Spec: `.docs/issues/i537.md`~~ — done | — | — |
| ~~I538~~ | ~~Meeting briefing refresh — rollback on failure. Snapshot existing prep before clearing, restore if enrichment fails. Spec: `.docs/issues/i538.md`~~ — done | — | — |
| ~~I539~~ | ~~Database recovery UX for migration/DB failure — startup blocker + Settings/Data recovery controls; scope extracted from I511. Spec: `.docs/issues/i539.md`~~ — done | — | — |
| **I563** | shellConfig render loop on AccountDetailEditorial — `useRegisterMagazineShell` re-register cycle causes max update depth warning | P2 | Frontend / Architecture |

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

### v1.1.0 — Publication + Portfolio + Glean Agents [PLANNED]

**Theme:** Intelligence flows upward via governed publication, not shared databases. IC's DailyOS publishes curated outputs (reports, health summaries) to Google Drive Shared folder. VP consumes via Glean or a lightweight portfolio reader. Glean Agents provide org-level analysis (call analysis, account health baselines) that DailyOS synthesizes with personal context. Governance is Google Workspace + Glean, not a custom auth/sync layer. See `.docs/research/2026-03-03-architecture-first-principles-review.md`.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I533 | Publication engine — Google Drive output layer, auto-publish, Glean auto-indexed. Spec: `.docs/issues/i533.md` | P1 | Backend / Publication |
| I534 | Portfolio reader — cross-IC intelligence from Shared Drive, JSON sidecar. Spec: `.docs/issues/i534.md` | P1 | Backend + Frontend |
| ~~I535~~ | ~~Glean Agent integration~~ — **pulled into v1.0.0 Phase 5** (2026-03-14) | — | — |
| I492 | Portfolio Health page — editorial aggregate, exception list, renewal timeline | P1 | Frontend / Pages |
| ~~I494~~ | ~~Glean account discovery~~ — **pulled into v1.0.0 Phase 5** (2026-03-14) | — | — |
| ~~I495~~ | ~~Ephemeral account query~~ — **pulled into v1.0.0 Phase 5** (2026-03-14) | — | — |
| ~~I531~~ | ~~Glean-powered proactive self-healing~~ — **pulled into v1.0.0 Phase 5** (2026-03-14) | — | — |
| I532 | Intelligence surfacing threshold model — when to tap the user on the shoulder vs stay quiet. Spec: `.docs/issues/i532.md` | P1 | Backend / Signals + Frontend / Briefing |

Version brief: `.docs/plans/v1.1.0.md`. Research: `.docs/research/2026-03-01-portfolio-intelligence-architecture.md`, `.docs/research/2026-03-03-architecture-first-principles-review.md`.

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
| I478 | Remove feature toggle section from Advanced Settings — internal dev knobs, not user-facing | P1 | Frontend / Settings + Backend / Config |

---

### ~~0.16.1 — Beta Hardening + Search + Offline~~ [DISSOLVED]

Dissolved 2026-03-04. GA features (I427-I431, I435, I438) absorbed into v1.0.0 Phase 2-3. I527 (consistency guardrails) ships as part of v1.0.0 Phase 2 (intelligence foundation).

| ID | Title | Absorbed Into |
|----|-------|---------------|
| ~~I427~~ | ~~Full-text search~~ — done | v1.0.0 Phase 3b |
| ~~I428~~ | ~~Offline/degraded mode~~ — done | v1.0.0 Phase 3b |
| ~~I429~~ | ~~Data export~~ — done | v1.0.0 Phase 3b |
| ~~I430~~ | ~~Privacy clarity~~ — done | v1.0.0 Phase 3b |
| I431 | Cost visibility | v1.0.0 Phase 3b |
| I435 | Token optimization | v1.0.0 Phase 2 |
| ~~I438~~ | ~~Prime DailyOS~~ — done | v1.0.0 Phase 3b |
| ~~I527~~ | ~~Intelligence consistency guardrails~~ — done | v1.0.0 Phase 2 |

---

### ~~0.16.2 — UI Finesse Pass~~ [DISSOLVED]

Dissolved 2026-03-03. Editorial polish absorbed into v1.0.0 Phase 3: I447-I454. Theming (I483) deferred to post-1.0. Archived brief: `.docs/plans/_archive/v0.16.2-dissolved.md`.

---

### 1.0.0 — Local Rearchitecture + Intelligence Foundation + GA [PLANNED]

**Theme:** The GA release. Structural cleanup of the local architecture. Workspace files eliminated (DB as sole local data layer). Mandatory ServiceLayer for all mutations. Schema decomposed (meetings_history→3 tables, entity_intelligence→2 tables, entity_people+account_team→1). God modules broken up. Pipeline reliability hardened. Intelligence foundation (6-dimension schema, health scoring, relationships) ships on the clean schema. CS report suite completes. Full-text search, offline mode, data export, privacy, and editorial polish bring the app to GA quality. DailyOS stays local-first. ADR-0099 withdrawn; governance and team views solved via output-layer publication + Glean Agents, not server sync. See `.docs/research/2026-03-03-architecture-first-principles-review.md`.

Requires v0.16.0 first. Full version brief: `.docs/plans/v1.0.0.md`.

**Phase 1 — Schema + ServiceLayer + Workspace File Elimination (DONE):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| ~~I511~~ | ~~Local schema decomposition + migration safety hardening (backend-only) — spec: `.docs/issues/i511.md` (absorbs I381)~~ — done | — | — |
| ~~I539~~ | ~~Database recovery UX for migration/DB failure — startup blocker + Settings/Data recovery controls. Scope extracted from I511. Spec: `.docs/issues/i539.md`~~ — done | — | — |
| ~~I512~~ | ~~ServiceLayer — mandatory mutation path + signal emission — spec: `.docs/issues/i512.md` (absorbs I380, I402)~~ — done | — | — |
| ~~I513~~ | ~~DB as sole source of truth for app-generated state — spec: `.docs/issues/i513.md` (absorbs I436). Workspace dirs + user files stay; app stops reading intelligence.json, dashboard.json, _today/data/*.json as data sources~~ — done | — | — |

**Phase 2 — Intelligence Foundation (DONE):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| ~~I508~~ | ~~Intelligence schema redesign — 6 dimensions~~ — done | — | — |
| ~~I499~~ | ~~Health scoring engine~~ — done | — | — |
| ~~I503~~ | ~~Health schema evolution~~ — done | — | — |
| ~~I500~~ | ~~Glean org-score parsing~~ — done | — | — |
| ~~I504~~ | ~~AI-inferred relationship extraction~~ — done | — | — |
| ~~I505~~ | ~~Glean stakeholder intelligence~~ — done | — | — |
| ~~I506~~ | ~~Co-attendance relationship inference~~ — done | — | — |
| ~~I487~~ | ~~Glean signal emission~~ — done | — | — |
| ~~I509~~ | ~~Transcript personal interpretation + sentiment~~ — done | — | — |
| ~~I507~~ | ~~Source-attributed correction feedback~~ — done | — | — |
| ~~I528~~ | ~~ADR-0098 data lifecycle infrastructure — DataSource enum, purge_source(), data_lifecycle.rs. Prerequisite for I487 + I505 purge ACs. Spec: `.docs/issues/i528.md`~~ — done | — | — |
| I435 | Token optimization — ModelTier audit; Haiku for email enrichment | P1 | Backend / Pipeline |

**Phase 2a — Dev Tools Mock Data Migration (DONE):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| ~~I536~~ | ~~Dev tools mock data migration — rewrite seed data for v1.0.0 schema (I511 tables), 6-dimension intelligence (I508), health scores (I499), signal/feedback variety (I529/I530). Consolidate scenarios 6→4. Eliminate workspace file writes. `mock-` prefix IDs. Spec: `.docs/issues/i536.md`~~ — done | — | — |

**Phase 3 — Structural Cleanup + Surfaces + GA Readiness:**

Execution model: umbrella branch `codex/v1-phase3` + short-lived issue branches, with mandatory mock+production parity gate before merge to `main`. Tracker: `.docs/plans/phase-3-execution-tracker.md`.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| ~~I514~~ | ~~Module decomposition — commands.rs → domain files, db.rs → re-export hub. Spec: `.docs/issues/i514.md`~~ — done | — | — |
| ~~I515~~ | ~~Pipeline reliability — retry with backoff, circuit breaker, partial result preservation, pipeline_failures table. Spec: `.docs/issues/i515.md`~~ — done | — | — |
| ~~I521~~ | ~~Frontend structural cleanup + production-data parity gate — ghost removal, duplicate consolidation, command/field contract registry, mock+production parity gate. Spec: `.docs/issues/i521.md`~~ — done | — | — |
| ~~I502~~ | ~~Health surfaces — render health band, dimensions, divergence across all pages~~ — done | — | — |
| ~~I493~~ | ~~Account detail enriched intelligence — Glean-sourced titles, coverage gaps, reports chapter~~ — done | — | — |
| ~~I427~~ | ~~Full-text search — Cmd+K via SQLite FTS5, < 300ms (build on clean schema post-I511)~~ — done | — | — |
| ~~I428~~ | ~~Offline/degraded mode — cached intelligence, system status indicator, no blank screens~~ — done | — | — |
| ~~I429~~ | ~~Data export — JSON ZIP of all entities, signals, intelligence; portability guarantee~~ — done | — | — |
| ~~I430~~ | ~~Privacy clarity — what's stored, how long, clear intelligence + delete all data~~ — done | — | — |
| I431 | Cost visibility — Claude call tracking, estimated weekly cost in Settings | P2 | Backend + Frontend |
| ~~I438~~ | ~~Prime DailyOS — first content ingestion step in wizard; manual or connector~~ — done | — | — |
| ~~I447~~ | ~~Design token audit — opacity tokens, rgba violations, max-width unification~~ — done | — | — |
| ~~I448~~ | ~~ActionsPage editorial rebuild — CSS module, margin grid, ChapterHeadings~~ — done | — | — |
| ~~I449~~ | ~~WeekPage + EmailsPage CSS module polish~~ — done | — | — |
| ~~I450~~ | ~~Portfolio chapter extraction — shared CSS module, conclusion-before-evidence order~~ — done | — | — |
| ~~I451~~ | ~~MeetingDetailPage polish~~ — superseded by I542 (full style migration + vocabulary) | — | — |
| ~~I452~~ | ~~Settings page editorial audit~~ — superseded by I541 (full UX rebuild: IA reorg, style migration, pagination, vocabulary) | — | — |
| ~~I453~~ | ~~Onboarding pages editorial standards — v0.16.0 pages built to editorial spec~~ — done | — | — |
| ~~I454~~ | ~~Vocabulary pass — replace all remaining user-visible system terms per ADR-0083~~ — done | — | — |
| ~~I529~~ | ~~Intelligence quality feedback UI — thumbs up/down on hover for intelligence items. Spec: `.docs/issues/i529.md`~~ — done | — | — |
| ~~I530~~ | ~~Signal taxonomy: curation vs correction — delete ≠ wrong. Spec: `.docs/issues/i530.md`~~ — done | — | — |
| ~~I537~~ | ~~Gate role presets behind feature flag — hide preset selection UI, hard-default to CS. Spec: `.docs/issues/i537.md`~~ — done | — | — |
| ~~I540~~ | ~~Actions pipeline integrity + lifecycle — 6 broken paths: Granola metadata loss, briefing blind to DB actions, archive never called, rejection source "unknown", thin free-tier summaries, deceptive tooltip. 30-day pending archive, Granola enrichment, briefing integration. Spec: `.docs/issues/i540.md`~~ — done | — | — |
| ~~I541~~ | ~~Settings page UX rebuild — IA reorg (YouCard split into Identity/Workspace/Preferences), full inline style migration to CSS modules, audit log pagination, vocabulary fixes, StatusDot consolidation. Supersedes I452. Spec: `.docs/issues/i541.md`~~ — done | — | — |
| ~~I542~~ | ~~MeetingDetailPage style migration + vocabulary — migrate 51 inline styles to CSS module, replace hardcoded colors with tokens, fix 3 ADR-0083 violations. Supersedes I451. Spec: `.docs/issues/i542.md`~~ — done | — | — |
| I543 | GA design documentation — document 11 undocumented pages, add 114 missing components to inventory, create STATE-PATTERNS.md, developer checklists. Spec: `.docs/issues/i543.md` | P1 | Documentation / Design System |
| ~~I544~~ | ~~Component DRY/SRP reconciliation — app-wide duplicate detection (StatusDot 3x, per-page empty/loading states), shared component extraction, dead code removal, SRP violations. Spec: `.docs/issues/i544.md`~~ — done | — | — |
| ~~I545~~ | ~~Entity detail pages style migration — 105 inline styles across AccountDetailEditorial (51), ProjectDetailEditorial (39), PersonDetailEditorial (15) + 7 hardcoded rgba values. Shared CSS module extraction. Spec: `.docs/issues/i545.md`~~ — done | — | — |
| I546 | Design documentation: interaction, data presentation, navigation — INTERACTION-PATTERNS.md, DATA-PRESENTATION-GUIDELINES.md, NAVIGATION-ARCHITECTURE.md. Spec: `.docs/issues/i546.md` | P2 | Documentation / Design System |
| ~~I547~~ | ~~Book of Business Review report~~ — stashed (2026-03-14) | — | — |
| ~~I549~~ | ~~Composable report slide templates + report mockups~~ — done (2026-03-14) | — | — |
| ~~I550~~ | ~~Account detail editorial redesign: margin label layout + visual storytelling~~ — pass 1 done, pass 2 suspended (2026-03-14) | — | — |
| ~~I551~~ | ~~Success Plan data model + backend~~ — done (2026-03-14) | — | — |
| ~~I552~~ | ~~Success Plan frontend~~ — done (2026-03-14) | — | — |
| ~~I553~~ | ~~Success Plan templates + starter lifecycle collection~~ — done (2026-03-14) | — | — |
| ~~I554~~ | ~~Transcript extraction signal fidelity — CS-grounded prompt definitions: 6 win sub-types, Red/Yellow/Green risk urgency, value delivered quantification, 3-level champion health (MEDDPICC), COMMITMENTS extraction, successPlanSignals schema. Absorbs I551 PTY changes. Spec: `.docs/issues/i554.md`~~ — done | — | — |
| ~~I555~~ | ~~Captures metadata + interaction dynamics persistence + architecture integration — urgency/sub_type/impact/evidence_quote columns on captures, interaction dynamics + champion health + role changes tables, captured_commitments table (dual-write to captures). Signal bus emissions (champion → person-level → propagation → callouts), reactivates `rule_meeting_frequency_drop`, upgrades 3 health scoring dimensions to behavioral, adds dynamics/commitments to intel + prep context. Absorbs I551 schema. Spec: `.docs/issues/i555.md`~~ — done | — | — |
| ~~I556~~ | ~~Report content pipeline — meeting summaries + captures for Weekly Impact/Monthly Wrapped (currently title-only), customer quote pipeline for EBR/QBR, urgency-enriched captures for Account Health + BoB. Spec: `.docs/issues/i556.md`~~ — done | — | — |
| ~~I557~~ | ~~Surface hidden intelligence on Account Detail — renders ~15 computed-but-invisible fields (valueDelivered, successMetrics, openCommitments, relationshipDepth, competitiveContext, strategicPriorities, expansionSignals, renewalOutlook, organizationalChanges, blockers). 3 new chapters. Spec: `.docs/issues/i557.md`~~ — done | — | — |
| ~~I558~~ | ~~Meeting Detail intelligence expansion — post-meeting intelligence section (engagement dynamics, champion health, categorized outcomes, role changes, sentiment), surfaces unused FullMeetingPrep fields. Spec: `.docs/issues/i558.md`~~ — done | — | — |
| ~~I559~~ | ~~Glean Agent validation spike — resolve 6 open questions (auth, rate limits, connectors, JSON output, latency, MCP tool discovery). Exploration only, no production code. GATE for I535. Spec: `.docs/issues/i559.md`~~ — done | — | — |
| I535 | Glean Agent integration — `GleanAgentClient`, agent registry, response parsing into I508 types, circuit breaker. Pulled from v1.1.0 (2026-03-14). Spec: `.docs/issues/i535.md` — Steps 1-8 implemented, blocked on testing | P0 | Backend / Connectors |
| I531 | Proactive self-healing with Glean — hygiene gap queries via agents, signal emission fixes (2 bugs + 7 missing), DB migration. Pulled from v1.1.0 (2026-03-14). Spec: `.docs/issues/i531.md` | P1 | Backend / Self-Healing / Connectors |
| I494 | Glean account discovery — "Discover from Glean" on Accounts page, CRM import, one-click add. Pulled from v1.1.0 (2026-03-14). Spec: `.docs/issues/i494.md` | P1 | Backend / Connectors + Frontend |
| I495 | Ephemeral account query — "Tell me about..." transient briefing, not persisted. Pulled from v1.1.0 (2026-03-14). Spec: `.docs/issues/i495.md` | P1 | Backend / Connectors + Frontend |
| **I563** | shellConfig render loop — `useRegisterMagazineShell` triggers max update depth warning on AccountDetailEditorial | P2 | Frontend / Architecture |
| **I564** | PTY enrichment blocks async runtime — wrap `run_enrichment()` in `spawn_blocking` in intel_queue processor | P0 | Backend / Performance |
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
| I572 | Audit log for OAuth token lifecycle | P2 | Backend / Security |
| I573 | Mutex poisoning recovery for critical state | P2 | Backend / Stability |

**v1.0.1 — Email Intelligence for CS (The Correspondent enhancements):**

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I577 | Reply debt surface — unanswered customer emails as first-class signal | P1 | Backend + Frontend / Email |
| I578 | Render `repliesNeeded` from existing `EmailBriefingData` | P1 | Frontend / Email |
| I579 | Per-email triage actions — archive, open in Gmail, pin | P1 | Frontend + Backend / Email |
| I580 | Commitment → Action promotion from email extraction | P1 | Backend + Frontend / Email |
| I581 | Email cadence awareness — silence detection for tracked accounts | P1 | Backend + Frontend / Email |
| I582 | Email-meeting linkage — surface `pre_meeting_context` on Correspondent + Meeting Detail | P1 | Frontend / Email |

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
