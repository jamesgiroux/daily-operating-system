# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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
