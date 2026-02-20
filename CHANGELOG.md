# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.12.0] - 2026-02-19

The chief of staff reads your email. Signals, not summaries. Briefing, not inbox. Built on the 0.10.0 signal bus.

### Added

- **I317: Meeting-aware email intelligence** — Structured email digest organized by meeting relevance instead of raw excerpts. High-priority emails linked to upcoming meetings surface in meeting prep context.
- **I318: Thread position tracking** — "Ball in your court" detection. Tracks which email threads await the user's reply vs. waiting on others, using thread-level sender analysis.
- **I319: Entity-level email cadence monitoring** — Weekly email volume per entity with 30-day rolling average. Anomaly detection flags "gone quiet" (<50% of avg) and "activity spike" (>200% of avg) patterns.
- **I320: Hybrid email classification** — Signal-context boosting (Layer 1). Medium-priority emails from senders linked to entities with active signals get promoted to high priority automatically.
- **I321: Email commitment extraction** — Fetches full email bodies for high-priority messages, runs through Claude (Extraction tier) to identify commitments, requests, and deadlines. Creates proposed actions with `source_type=email`.
- **I322: Email briefing narrative** — Daily briefing integrates email intelligence as a synthesized narrative section covering reply urgency, entity correlations, and cadence anomalies.
- **I323: Zero-touch email disposition** — Auto-archive pipeline for low-priority emails during daily prep. Writes disposition manifest. Correction command for user feedback. Surfaced as a toggle in Settings > Intelligence > Features.
- **I324: Email signals in entity enrichment** — Enhanced signal formatting with sender name/role resolution, relative timestamps, email cadence summary with trend analysis, and AI prompt interpretation guidance. Dynamic signal limit (20 for entities with upcoming meetings).
- **I337: Calendar description steering** — Meeting calendar descriptions now steer intelligence narrative, giving the AI context about meeting purpose and agenda.
- **I338: 1:1 relationship intelligence** — Person entity resolution for 1:1 meetings. Three-file intelligence pattern (dashboard.json, intelligence.json, context.md) generates for people.
- **I353: Self-healing hygiene** — Phase 2 signal→hygiene feedback loop. Auto-merge duplicate entities, calendar name resolution, co-attendance linking.
- **I351/I339/I313: Entity architecture completion** — Person actions, week entities display, vocabulary injection for role-preset-aware AI prompts.

### Changed

- Email signal text and AI summaries render in primary text color (was secondary/light gray) for better readability of the most valuable content.
- `emailBodyAccess` feature enabled by default — commitment extraction works out of the box.
- `autoArchiveEnabled` visible in Settings UI as "Auto-Archive Email" toggle (default off, since it modifies Gmail).

### Fixed

- Audit fixes across I318, I319, I320, I322, I353 — closed gaps in signal emission, cadence computation, and narrative generation.
- DB mutex not held across async/PTY calls in email commitment extraction (two-phase pattern: async fetch → sync extraction).

## [0.11.0] - 2026-02-19

Role presets, entity architecture, and industry-aligned terminology.

### Added

- **I309–I315: Role presets** — 9 embedded presets (CS, Sales, Marketing, Partnerships, Agency, Consulting, Product, Leadership, The Desk) with role-specific vocabulary, email keywords, metadata fields, and AI prompt framing. Role selection in Settings and onboarding.
- **I143a: Lifecycle events** — Renewal metadata, lifecycle event tracking, proactive detectors, account merge support.
- **I353 Phase 1: Self-healing hygiene** — Auto-merge duplicate entities, calendar name resolution, co-attendance linking.

### Changed

- Meeting card key people sourced from calendar attendees instead of entity stakeholders.
- EntityPicker supports multiselect mode with excluded-parent child visibility.
- PersonNetwork supports optimistic multi-select entity linking without page reload.
- Back button uses browser history on all detail pages.
- StakeholderGallery searches existing people before creating new entries.

### Fixed

- Quill transcript sync hang — release DB mutex during AI pipeline to prevent deadlock.
- Internal account propagation and recursive account tree with add-child on all accounts.
- Email signal fan-out with confidence filtering, prep invalidation queue consumer.

## [0.10.1] - 2026-02-19

User feedback and onboarding polish. First real user session surfaced friction — fixed fast.

### Added

- **I344: Gmail teammate suggestions** — Onboarding "About You" chapter suggests closest teammates from Gmail frequent correspondents (scans `in:sent newer_than:90d`, filters to same domain, returns top 10 by frequency). Clickable chips above manual entry field.
- **I346: Linear integration (data layer)** — Settings card with API key + test connection, background poller syncing assigned issues and team projects via GraphQL API, SQLite tables (`linear_issues`, `linear_projects`) with migration 024. Consumer side (signal bus integration, meeting prep enrichment) deferred to I306/I326/I332.

### Fixed

- **I345: Onboarding back navigation loses entered state** — AboutYou and PopulateWorkspace form data (name, company, title, domains, focus, colleagues, accounts, projects) lifted to OnboardingFlow parent component. Back navigation no longer unmounts and loses entered data.

## [0.10.0] - 2026-02-18

The intelligence release. The system that learns from you. Signals compound, corrections teach, events drive action.

### Added

- **I305: Intelligent meeting-entity resolution** — Bayesian fusion of 5 signal producers (junction table, attendee inference, group patterns, keyword match, embedding similarity) with three-tier confidence thresholds (resolved/flagged/suggestion)
- **I306: Signal bus foundation** — event log, weighted log-odds Bayesian fusion, temporal decay, email-calendar bridge. 13 modules in `signals/` with ~57 unit tests
- **I307: Correction learning** — Thompson Sampling with Beta distribution for per-source reliability weights, gated behind 5-sample minimum. User corrections update alpha/beta parameters
- **I308: Event-driven signal processing** — cross-entity propagation engine with 5 rules (job change, frequency drop, overdue actions, champion sentiment, departure+renewal risk)
- **I334: Proposed actions triage** — accept/reject flow on Actions page and Daily Briefing. Transcript-sourced actions default to "proposed" status. Auto-archive hygiene for stale proposals
- **I335: Entity-generic data model** — `meeting_entities` junction table replaces `meetings_history.account_id`. Migration 023 backfills existing data before column drop
- **I336: Entity-generic classification** — entity hints from DB, 1:1 person detection, multi-type resolution (accounts, projects, people)
- **I337: Entity-generic context building** — type-dispatched intelligence injection: accounts get dashboard/stakeholders/captures, projects get status/milestones, people get relationship signals/shared entities
- **I338: 1:1 relationship intelligence** — three-file pattern (dashboard.json, intelligence.json, context.md) for people entities, relationship-specific enrichment prompts
- **I339: Entity-generic frontend** — "person" as first-class entity type with User icon, larkspur color, `/people` routing. `formatEntityByline()` shows type-specific labels ("Acme - Customer", "Alice - 1:1")
- **I260: Proactive surfacing** — 8 pure SQL+Rust detectors (renewal gap, relationship drift, email volume spike, meeting load forecast, stale champion, action cluster, prep coverage gap, no-contact accounts) with fingerprint dedup and signal bus integration
- **I262: The Record** — content_index populated with transcripts and notes as timeline sources for entity intelligence enrichment

### Fixed

- Migration 023 blocked by foreign key constraints from captures/quill_sync_state tables — added `PRAGMA foreign_keys = OFF`
- Migration 023 dropped unique index on `calendar_event_id` during table recreation — now recreated
- Stale `account_id` column reference in `meeting_context.rs` inline SQL query after migration 023 dropped the column — updated to use `meeting_entities` junction table

## [0.9.1] - 2026-02-18

Hotfix for MCP integrations failing when app is launched from Finder/Applications.

### Fixed

- Quill, Clay, and Gravatar MCP clients fail with "connection failed" when launched from Finder — macOS GUI apps don't inherit shell PATH, so nvm-installed `node`/`npx` binaries aren't found. Added intelligent binary resolution that scans nvm versions, Homebrew, and system paths with process-lifetime caching.

## [0.9.0] - 2026-02-18

The integrations release. Four new data integrations, a plugin marketplace, and UI polish.

### Added

- **I226: Granola integration** — background poller syncs meeting transcripts from Granola's local cache, matches to calendar events by time window and attendee overlap, writes to entity Meeting-Notes directories
- **I229: Gravatar integration** — MCP-based avatar and profile enrichment with local image caching, background poller for stale email refresh, Avatar component with `convertFileSrc` asset loading
- **I228: Clay integration** — MCP client (SSE primary / stdio fallback) for contact and company enrichment: title, company, LinkedIn, Twitter, phone, bio, industry, HQ, company size. Signal detection for job changes, funding rounds, and leadership transitions. Background poller with bulk enrich wake signal via `tokio::sync::Notify`
- **I276: Plugin Marketplace** — two Claude Code plugins (`dailyos` with 9 commands + 9 skills, `dailyos-writer` with 4 commands + 11 skills) bundled as installable zips with Settings UI for export
- Enrichment log table for tracking all enrichment events across sources with signal classification
- Clay Settings UI section: enable/disable, API key, connection test, bulk enrich, enrichment status
- Gravatar Settings UI section: enable/disable, API key, cache stats
- Granola Settings UI section: enable/disable, sync status
- Person schema extended with enrichment fields: LinkedIn URL, Twitter handle, phone, photo URL, bio, title history, company industry/size/HQ, last enrichment timestamp, enrichment sources
- Avatar component for person images with Gravatar cache lookup and initials fallback

### Fixed

- Literal `\u2026`, `\u2318`, `\u2713`, `\u2197` escape sequences rendering as text in JSX attribute strings and text content — replaced with actual Unicode characters across 16 frontend files
- Gravatar images showing as broken blue boxes — CSP `img-src` now includes `https://asset.localhost` for Tauri's asset protocol
- Avatar component falls back to initials on image load error instead of showing broken image icon
- Clay "Enrich All" button queued work but poller didn't process until next 24-hour cycle — poller now wakes immediately via `tokio::select!` against `Notify` signal

### Changed

- Person detail pages show LinkedIn and Twitter external links with arrow indicators
- Onboarding copy uses actual Unicode characters (em dash, en dash, curly quotes) instead of escape sequences

## [0.8.4] - 2026-02-17

Hotfix for Claude Desktop MCP integration.

### Fixed
- MCP server stdout pollution: native library output (ONNX Runtime, fastembed) during embedding model initialisation was leaking onto stdout before rmcp took ownership of the stdio transport, corrupting the JSON-RPC stream and causing Claude Desktop to lose terminal styling. Fixed by redirecting stdout → stderr (via `dup`/`dup2`) for the duration of embedding init.

## [0.8.3] - 2026-02-17

Cleanup and hardening. Type safety, migration resilience, input validation, and AI prompt robustness.

### Fixed
- `LinkedEntity.entityType` narrowed at source — removes band-aid cast, fixes entity picker for projects (I303)
- Transcript action extraction resolves `@Tag` to real account ID via case-insensitive lookup — fixes silent FK violations that dropped actions
- Path traversal guard added to `resolve_prep_path`
- Stale agenda overwrite when hiding attendees — `agenda` parameter now optional in `update_meeting_user_agenda`

### Changed
- Migrations 006, 007, 011 hardened with `IF NOT EXISTS` for crash-recovery safety
- Orphan migration file `007_chat_sessions.sql` removed
- Input bounds on user agenda layer: max 50 items per list, 500 chars per string, UTF-8-safe truncation
- Transcript prompt handles null title/account gracefully instead of producing malformed prompts (I304)
- Folio bar transcript button shows spinner and `not-allowed` cursor when processing

## [0.8.2] - 2026-02-17

Polish sprint. Meeting intelligence redesigned as editorial briefing, audit trail for AI-generated data, person deduplication, and print-ready PDF export.

### Added

- **I297**: Audit trail module for AI-generated data — tracks provenance through the enrichment pipeline
- **I302**: Person email aliasing and cross-domain deduplication — merges duplicate contacts across domains
- Meeting Intelligence Report redesigned as a full editorial briefing with outcomes pinned to top
- Transcript attach button added to folio bar on all meetings (not just those with existing transcripts)
- Print styles for clean briefing PDF output — `Cmd+P` produces a readable document
- `export_briefing_html` command for browser-based PDF export
- Claude Code skill templates distributed to user workspaces for slash-command workflows
- "+ Business Unit" button on account detail folio bar
- Attendee RSVP status carried through the full calendar pipeline

### Changed

- Schedule cards show QuickContext instead of PrepGrid, with internal stakeholders filtered out
- Risk briefing Regenerate button moved to folio bar; byline is now click-to-edit
- Featured meeting remains visible in the schedule list (no longer removed when featured)
- Prep summaries hydrated from entity intelligence fields for richer meeting context
- Meeting entity chips use optimistic local state for instant feedback
- `create_action` IPC switched to request object parameter (breaking IPC change)

### Fixed

- MCP sidecar binary missing executable permission after build
- Meeting card padding and prep summary hydration from prep files
- Clippy warnings resolved for 0.8.2 release
- TypeScript error in meeting entity chips rollback path

## [0.8.1] - 2026-02-16

Hardening release. Security, database integrity, token optimization, and proposed actions workflow.

### Security
- **I295**: Prompt injection hardening — all 7 PTY enrichment sites now wrap untrusted data (calendar titles, email subjects, file contents, entity names) in `<user_data>` XML blocks
- **I296**: Output size limits — capped all parsed AI arrays (20 risks, 50 actions, 10 wins, 20 stakeholders, 10 value items) to prevent unbounded growth

### Database
- **I285**: Foreign key constraints added to `actions` (3 FKs), `account_team`, and `account_domains` via table recreation migration. FK enforcement enabled at connection level (`PRAGMA foreign_keys = ON`)
- **I231**: Fixed `unwrap()` panic in `focus_capacity.rs` during DST spring-forward gaps — new `resolve_local_datetime` helper handles all chrono timezone edge cases

### Token Optimization
- **I286**: Entity intelligence prompts filtered by vector search relevance — context budget capped at 10KB (down from ~25KB), mandatory files always included
- **I288**: Entity intelligence output switched from pipe-delimited to JSON format with backwards-compatible fallback parser

### Actions
- **I256**: Proposed actions workflow — AI-extracted actions now insert as `proposed` status with accept/reject UX, "AI Suggested" badge, and 7-day auto-archive via scheduler

### Performance
- **I234**: IntelligenceQueue `last_enqueued` HashMap now pruned every 60s to prevent unbounded memory growth
- **I235**: Dashboard DB reads consolidated into single lock acquisition (`DashboardDbSnapshot`) reducing lock contention

### Stats
- 688 tests passing (up from 684), 0 clippy warnings

## [0.8.0] - 2026-02-16

The editorial release. Every page redesigned as a magazine-style document you read top to bottom. New typography, new color system, new layout engine. Plus semantic search, MCP integration, and security hardening.

### Editorial Design

- Complete visual overhaul: every page now renders as a magazine-style editorial document with chapter-based navigation
- New typography: Newsreader (serif body) and Montserrat (sans headings) replace the previous system fonts
- New color palette: 14 material-named colors across four families (Paper, Desk, Spice, Garden) replace generic tokens
- Magazine shell layout with atmosphere layer, floating navigation island, and folio bar replaces the sidebar
- Daily briefing reimagined: hero headline, focus block, featured meeting with full prep, schedule rows, tapering priorities — read top to bottom, then you're briefed
- Briefing refresh button shows live workflow progress (Preparing / AI Processing / Delivering) instead of silent wait
- Email visibility: briefing falls through to medium-priority emails when no high-priority exist, with contextual section labels
- Account, project, and person detail pages rebuilt as 7-chapter editorial narratives with shared layout template
- Meeting detail page redesigned with editorial treatment
- Action detail page redesigned with editorial treatment
- Emails, Inbox, History, and Settings pages moved into magazine shell
- Focus capacity and action prioritization folded directly into the daily briefing
- Week page editorial polish with folio bar integration
- Shared editorial components: ChapterHeading, FinisMarker, PullQuote, StateBlock, TimelineEntry, WatchItem, EditableText
- Asterisk brand mark integrated into navigation

### Risk Briefing

- Executive risk briefing redesigned as a 6-slide presentation (Cover, Bottom Line, What Happened, The Stakes, The Plan, The Ask) — each slide fills the viewport with scroll-snap navigation
- Keyboard shortcuts: keys 1-6 jump to slides, arrow keys navigate
- All text fields are click-to-edit — fix names, titles, or facts before presenting, changes auto-save silently to disk
- Tighter AI output: hard word limits prevent verbose slides, health arc rendered as color-coded timeline bars

### Semantic Search

- Local embedding model (nomic-embed-text-v1.5) for semantic vector search over entity content — downloads automatically on first launch, works offline after that
- Background embedding processor: entity files are chunked and embedded automatically as they change
- Hybrid search combining vector similarity (70%) and keyword matching (30%) for best-of-both retrieval
- Semantic search integrated into entity intelligence enrichment — AI now finds relevant historical content instead of relying on recency alone

### MCP Server

- Chat tools for querying entities, searching content, and retrieving briefings via external AI assistants (Claude Desktop via MCP)
- Semantic search tool (`search_content`) exposes hybrid vector+keyword search to Claude Desktop — ask about specific details in workspace files
- Chat session persistence — conversations are remembered across sessions
- Managed CLAUDE.md and settings written to workspace for Claude Desktop discovery

### Security

- Content Security Policy (CSP) enforced on the webview — restricts script, style, image, and connection sources to the app itself
- `reveal_in_finder` command validates paths against workspace and config directories before opening Finder — prevents arbitrary filesystem traversal
- `copy_to_inbox` command restricts source paths to Documents, Desktop, and Downloads — prevents copying from arbitrary filesystem locations

### Reliability

- Database renamed from `actions.db` to `dailyos.db` with automatic migration and WAL checkpoint
- Embedding model initializes asynchronously in the background — app window appears immediately instead of blocking during the 137MB model download
- Database migration framework tolerates duplicate-column errors for safe re-application
- Database indexes added for meeting-entity lookups, calendar event deduplication, and action filtering — faster page loads as data grows
- Removed unused frontend dependencies (lighter install, smaller attack surface)
- Dev database isolation: pattern-based purge, config backup, no Keychain writes in dev mode
- Apple notarization re-enabled in CI release pipeline

## [0.7.5] - 2026-02-14

### Fixed

- All AI enrichment calls (email, briefing, prep, week, entity intelligence, transcript, inbox) hardened against PTY output corruption: TERM=dumb suppresses escape codes, 4096-column width prevents hard line wrapping, ANSI stripping as safety net
- Debug logging of raw Claude output for all enrichment calls — parse failures now include the first 500 bytes for diagnosis
- Email enrichment "No enrichments parsed" caused by ANSI escape codes corrupting structured markers

## [0.7.4] - 2026-02-14

### Fixed

- Claude Code CLI not found when app is launched from Finder — the app now resolves the binary from common install locations (`~/.local/bin`, `/usr/local/bin`, `/opt/homebrew/bin`) instead of relying on shell PATH
- Email retry clearing the error banner without verifying enrichment succeeded — the banner now stays visible if enrichment fails during a retry, instead of falsely reporting success

## [0.7.3] - 2026-02-13

647 Rust tests. 71 Architecture Decision Records. First release with auto-updater.

### Proactive Intelligence

- Weekly briefing with AI narrative, priority synthesis, and readiness assessment
- Live proactive suggestions during workflow execution with progress stepper
- Email signal extraction: timeline events, risks, expansion signals, escalations linked to entities
- Email signals displayed on entity detail pages with signal-type badges and relative dates
- Agenda draft dialog for pre-meeting preparation with AI-generated starter content

### Entity Management

- Internal team setup: create your organization with root account, team, colleagues, and domain auto-linking
- Parent-child account hierarchy with directory scaffolding and domain inheritance
- Account team management: link people to accounts with roles (CSM, TAM, executive sponsor, etc.)
- Bulk person creation form for onboarding flows
- Entity picker filters archived entities from queries
- Account domains tracked in dedicated junction table with N+1 query elimination (single JOIN)

### Onboarding

- Internal Team Setup chapter: configure company, team, colleagues, and domains during onboarding
- Prime Briefing chapter: trigger first briefing from onboarding wizard
- Onboarding flow enhanced with demo data and educational content

### Personality System

- Configurable personality for AI copy across empty states and notifications
- PersonalityProvider context with 5 personality options (Professional, Friendly, Playful, Zen, Direct)
- SectionEmpty and InlineEmpty shared components replace ad-hoc empty states across all pages
- PersonalityCard and UserProfileCard in Settings

### Settings & Security

- Settings tabs with deep-link support (`/settings?tab=...`) for Profile, Integrations, Workflows, Intelligence, Hygiene, and Diagnostics
- Intelligence Hygiene status API + manual scan with gap-specific actions
- OAuth failure event (`google-auth-failed`) surfaces real auth errors without hanging
- OAuth hardened with PKCE S256 challenge + state parameter validation
- Removed hardcoded Google OAuth `client_secret` from source; loaded via `option_env!`
- CI guard to fail builds when committed OAuth secret patterns are detected

### Reliability

- Schema migration framework: numbered SQL migrations, pre-migration backup, forward-compat guard, bootstrap for existing databases
- Transaction wrapper on `create_internal_organization` — atomic multi-record creation with rollback on failure
- Race guard on WeekPage polling — prevents overlapping IPC calls during workflow execution
- Email validation on person creation
- WebKit date compatibility: `parseDate` utility handles Safari's strict timestamp parsing
- PTY subprocess strips Claude Code env vars to prevent nested session detection
- Stale capacity warning suppressed when briefing schedule is from today
- Transcript attachment error visibility improved
- Workflow delivery history tracks explicit failure phase and retry metadata

### Auto-Updater

- Tauri updater plugin with Minisign signature verification
- "Check for Updates" UI in Settings with download + relaunch flow
- Release pipeline generates signed `latest.json` for update endpoint
- Code signing and notarization in CI release workflow

### Infrastructure

- Shared `helpers.rs` module: `normalize_key`, `normalize_domains`, `build_external_account_hints` (DRY consolidation)
- Centralized date formatters in `utils.ts`: `formatRelativeDateLong`, `formatBidirectionalDate`, `formatDayTime`, `formatShortDate`
- Meeting context preparation extracted into dedicated module (`prepare/meeting_context.rs`)
- 5 SQL migrations: baseline, internal_teams, account_team, account_team_role_index, email_signals
- Focus page: isolated refresh command, P1 action cap, agenda anchored to calendar notes
- Proactive intelligence query layer (`queries/proactive.rs`)

### Fixed

- "View All Actions" count now reflects P1 actions only
- Hygiene system: NaN bug, manual scan trigger, detail breakdown
- `latest.json` generation handles multiline Minisign signatures correctly
- Clippy warnings resolved for CI (`-D warnings` enforcement)

## [0.7.2] - 2026-02-12

### Fixed

- OAuth token exchange restored: client_secret was stripped during PKCE migration but Google Desktop App clients still require it — every auth attempt was returning 400 Bad Request
- OAuth callback no longer shows "Authorization successful" before the token exchange completes — browser now waits for the full exchange + Keychain save, and shows the actual error on failure
- Token refresh no longer strips client_secret from saved tokens, preventing refresh failures after the initial hour
- Added diagnostic logging at every step of the OAuth flow for troubleshooting

## [0.7.1] - 2026-02-12

Six sprints of work across meeting intelligence, entity relationships, security hardening, and app responsiveness. 574 Rust tests. 69 Architecture Decision Records.

### Meeting Intelligence

- Meeting prep redesigned as a report: executive brief hero, agenda-first flow, right-rail navigation, appendix-style deep context
- Agenda and Wins are now semantically separate enrichment blocks with structured source provenance (replaces flat talking points)
- User-authored prep fields (`userAgenda`, `userNotes`) are DB-authoritative with freeze/editability rules
- Meeting identity hardened: calendar event ID is canonical primary key across poller, reconcile, and DB
- Unified meeting intelligence contract (`get_meeting_intelligence`) combines prep, outcomes, and transcript metadata in a single backend call
- Enriched prep context persisted to `meetings_history` for durable post-meeting records
- Meeting search across entities via Cmd+K command menu with debounced cross-entity lookup
- Calendar description pipeline extracted and exposed in prep as `calendarNotes`
- Account snapshot enrichment with compact, sanitized prep rendering
- People-aware prep support for internal meeting types
- Immutable prep snapshots written to entity `Meeting-Notes/` during archive

### Entity Relationships

- Person-entity auto-linking via meeting attendance with full cascade
- Multi-entity MeetingCard: add/remove entity associations with people + intelligence queue cascade
- Multi-domain user configuration with tag/chip input UX, auto-reclassification of people and meeting types on domain change
- Entity archive/unarchive with parent cascade (DB flag only, filesystem untouched)
- Strategic programs inline editing on AccountDetailPage with debounced auto-save
- People merge and delete with full cascade across attendees, entities, actions, intelligence, and filesystem

### Focus & Capacity

- Focus page redesigned with live capacity engine computing from calendar events
- Deterministic action prioritization with urgency/impact scoring, top-3 recommendations, risk radar
- Focus capacity computes from live calendar, schedule artifact retained for briefing narrative only

### Security & Auth

- OAuth hardened with PKCE (`S256`) challenge + state parameter validation
- macOS Keychain token storage with one-time legacy file migration and removal
- Secretless token exchange and refresh with compatibility fallback for legacy clients
- IPC input validation DTOs for action create/update with centralized validators
- CI gates: `cargo clippy -D warnings`, `cargo audit`, `pnpm audit` enforced on every build

### Email

- Email sync status tracking with structured health metadata on `emails.json`
- Sticky sync banner with retry affordance when fetch or delivery fails
- Model fallback: email enrichment retries with synthesis model when extraction model unavailable
- Last-known-good email preservation on delivery failures

### Reliability & Performance

- App responsiveness: `check_claude_status` moved to async with `spawn_blocking`, background tasks open own SQLite connections instead of competing for shared Mutex
- Google API retry policy with exponential backoff wired into auth, calendar, and Gmail
- Resume latency instrumentation with p50/p95/max rollups and budget violation tracking
- Split-lock enrichment pattern with `nice -n 10` PTY execution for background AI operations
- Archive lifecycle reordered: reconciliation and prep freezing happen before `_today/data` cleanup
- Claude auth check timeout reduced from 8s to 3s, focus debounce intervals increased

### AI Operations

- Model tiering for AI operations: Synthesis/Extraction/Mechanical tiers with configurable model names per tier
- Prep enrichment contract splits Agenda and Wins parsing with separate blocks and source governance
- One-time migration command `backfill_prep_semantics` for upgrading existing prep files

### UX & Polish

- Frontend meeting routes consolidated to canonical `/meeting/$meetingId` with history route as redirect
- Theme toggle fixed: replaced broken dropdown (Radix dual-install issue) with segmented button group
- Radix UI components migrated to explicit standalone packages, resolving dual-install portal bug
- Calendar poller polls immediately on startup (5s auth delay) instead of sleeping first
- Empty prep page shows "generating" message instead of blank
- Binary size and bundle measurement scripts for repeatable performance tracking

## [0.7.0] - 2026-02-09

### Added

- Native desktop app (Tauri v2) -- complete rewrite from CLI
- Daily briefing with AI-enriched meeting prep
- Account intelligence -- executive assessments, risks, wins, stakeholder insights
- Project intelligence -- status tracking, content indexing
- People tracking -- relationship history, meeting patterns, auto-created from calendar
- Meeting-entity relationship graph with manual reassignment
- Email triage with three-tier AI priority classification
- Action tracking from briefings, transcripts, inbox, and manual creation
- Transcript processing with outcome extraction (actions, captures, decisions)
- Entity directory template (Call-Transcripts, Meeting-Notes, Documents)
- Proactive intelligence maintenance (hygiene scanner, pre-meeting refresh)
- Week page with AI narrative and priority synthesis
- Focus page with gap analysis
- Inbox processing with file classification and routing
- Onboarding wizard with Google OAuth integration
- Production Google OAuth credentials (no user-supplied credentials.json needed)
- Background scheduling (daily briefing, archive reconciliation, intelligence refresh)
- 500 Rust backend tests
- 59 Architecture Decision Records

### Changed

- CLI archived to `_archive/dailyos/`
- Python runtime eliminated -- all operations now in Rust
- Config directory: `~/.dailyos/` (was `~/.daybreak/`)

### Removed

- Python Phase 1/Phase 3 scripts (replaced by Rust-native Google API client)
- CLI commands (/today, /week, /wrap) -- replaced by app UI
