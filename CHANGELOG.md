# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.12.1] - 2026-02-19

The first release that subtracts. Every surface asked "does this earn its keep?" — what failed got cut, system jargon got replaced with product language, and 0.12.0 email intelligence got an editorial UI.

### The Correspondent — Email Intelligence Page

- Email page redesigned as "The Correspondent" — an editorial dispatch, not an email client
- 76px narrative headline synthesized from inbox signals (replies waiting, meeting-linked threads, cadence anomalies)
- Four margin-grid sections: Your Move (replies needed), Commitments (extracted promises), Open Questions (with account/sender context), Signals (per-entity prose assessments)
- Entity-scoped relevance filtering — only emails linked to tracked accounts/projects surface intelligence
- Noise filtering excludes support tickets, notifications, marketing, and billing emails automatically
- Inline dismiss on every item with SQLite persistence for future relevance learning
- Enrichment prompts now request contextual prose ("Sarah Chen committed to delivering the revised SOW by Friday") instead of terse fragments

### Surface Cuts

- Week page: removed Meetings, Open Time, and Commitments chapters — keeps The Three and The Shape only
- Meeting detail: removed Deep Dive zone and Appendix (2931 → 2061 lines) — keeps Brief, Risks, Room, Plan, Finis
- Daily briefing: merged Hero and Focus into single Day Frame section, cut Later This Week action group
- Actions page: three tabs only (proposed, pending, completed) with smart default
- Entity pages: removed Value Delivered, Portfolio Summary, Resolution Keywords, meeting readiness callouts
- Deleted 5 unused components (ActionItem, ActionList, EmailList, WatchItem, AppSidebar)

### Product Vocabulary

- "Build Intelligence" → "Refresh" across all entity heroes
- "Account Intelligence" → "Last updated" with timestamp
- "Entity mode" → "Work mode" in settings
- "AI enrichment" → "AI analysis" in status messages and onboarding
- "intelligence layer" → "daily briefings" in settings

### Intelligence Quality Indicators

- New IntelligenceQualityBadge component with freshness dots (green < 24h, amber < 48h, saffron > 48h, gray = none)
- Integrated into all entity heroes (accounts, people, projects)

### Inline Editing

- EditableText rewritten: textarea-first default, Tauri event emission on commit, Tab/Shift+Tab keyboard navigation, Escape cancels
- New EditableList component with HTML5 drag-to-reorder and grip handles

### Email Intelligence Backend

- Email enrichment groups by thread_id for thread-level context before AI analysis
- Commitments, questions, and sentiment extracted per email and persisted to emails.json
- Semantic email reclassification: opt-in AI re-scoring of medium-priority emails (behind semanticEmailReclass feature flag)
- Entity thread signal summaries upgraded from mechanical counts to editorial prose

### Navigation

- Dropbox added to nav island (above Actions, after separator) for document/file inbox
- Mail nav item for email intelligence page
- InboxPage folio label updated to "Dropbox"

### Settings

- Settings page refactored into component modules (YouCard, ConnectionsGrid, SystemStatus, DiagnosticsSection)
- Day start time picker for morning briefing schedule

### Changed

- Email narrative headline capped at 12 words for 76px readability
- Extracted commitments and questions render in primary text color with per-item source context (entity, sender, subject)
- Entity signal summaries are editorial prose instead of "2 risks, 1 expansion" counts

### Stats

- 915 Rust tests passing, 0 clippy warnings, 29 frontend tests passing
- Net -1,333 lines across 71 files

## [0.12.0] - 2026-02-19

The chief of staff reads your email. Signals, not summaries. Briefing, not inbox. Built on the 0.10.0 signal bus.

### Email Intelligence

- Meeting-aware email digest: high-priority emails organized by meeting relevance instead of raw excerpts, surfaced in meeting prep context
- Thread position tracking: "ball in your court" detection identifies which threads await your reply vs. waiting on others
- Entity-level email cadence monitoring: weekly volume per entity with 30-day rolling average, anomaly detection flags "gone quiet" and "activity spike" patterns
- Hybrid email classification: medium-priority emails from senders linked to entities with active signals get promoted to high priority automatically
- Email commitment extraction: fetches full email bodies for high-priority messages, runs through Claude to identify commitments, requests, and deadlines — creates proposed actions automatically
- Email briefing narrative: daily briefing integrates a synthesized narrative covering reply urgency, entity correlations, and cadence anomalies
- Zero-touch email disposition: auto-archive pipeline for low-priority emails during daily prep, with disposition manifest and correction feedback. Surfaced as "Auto-Archive Email" toggle in Settings (off by default, since it modifies Gmail)
- Enhanced email signals in entity enrichment: sender name/role resolution, relative timestamps, cadence summary with trend analysis, AI prompt interpretation guidance, dynamic signal limit (20 for entities with upcoming meetings)

### Intelligence

- Calendar description steering: meeting calendar descriptions now steer intelligence narrative, giving the AI context about meeting purpose and agenda
- 1:1 relationship intelligence: person entity resolution for 1:1 meetings with three-file intelligence pattern (dashboard.json, intelligence.json, context.md)
- Self-healing hygiene: signal→hygiene feedback loop with auto-merge duplicates, calendar name resolution, co-attendance linking
- Person actions, week entities display, and vocabulary injection for role-preset-aware AI prompts

### Changed

- Email signal text and AI summaries render in primary text color for better readability of the most valuable content
- Email commitment extraction enabled by default — no feature flag needed

### Fixed

- Closed gaps in signal emission, cadence computation, and narrative generation across the email intelligence pipeline
- DB mutex not held across async/PTY calls in email commitment extraction (two-phase pattern: async fetch, sync extraction)

## [0.11.0] - 2026-02-19

Role presets, entity architecture, and industry-aligned terminology. The system speaks your language now.

### Role Presets

- 9 embedded presets (CS, Sales, Marketing, Partnerships, Agency, Consulting, Product, Leadership, The Desk) with role-specific vocabulary, email keywords, metadata fields, and AI prompt framing
- Role selection in Settings and onboarding — the system adapts its entire vocabulary to your function
- Role-aware email classification keywords boost domain-specific signals

### Entity Architecture

- Lifecycle events: renewal metadata, lifecycle event tracking, proactive detectors, account merge support
- EntityPicker supports multiselect mode with excluded-parent child visibility
- PersonNetwork supports optimistic multi-select entity linking without page reload
- StakeholderGallery searches existing people before creating new entries

### Changed

- Meeting card key people sourced from calendar attendees instead of entity stakeholders
- Back button uses browser history on all detail pages

### Fixed

- Quill transcript sync hang — release DB mutex during AI pipeline to prevent deadlock
- Internal account propagation and recursive account tree with add-child on all accounts
- Email signal fan-out with confidence filtering, prep invalidation queue consumer

## [0.10.1] - 2026-02-19

User feedback and onboarding polish. First real user session surfaced friction — fixed fast.

### Added

- Gmail teammate suggestions: onboarding "About You" chapter suggests closest teammates from Gmail frequent correspondents (scans sent mail, filters to same domain, returns top 10 by frequency). Clickable chips above manual entry field.
- Linear integration (data layer): Settings card with API key + test connection, background poller syncing assigned issues and team projects via GraphQL API

### Fixed

- Onboarding back navigation no longer loses entered state — form data lifted to parent component so back navigation preserves everything you've typed

## [0.10.0] - 2026-02-18

The intelligence release. The system that learns from you. Signals compound, corrections teach, events drive action.

### Signal Intelligence

- Intelligent meeting-entity resolution: Bayesian fusion of 5 signal producers (junction table, attendee inference, group patterns, keyword match, embedding similarity) with three-tier confidence thresholds
- Signal bus foundation: event log, weighted log-odds Bayesian fusion, temporal decay, email-calendar bridge
- Correction learning: Thompson Sampling with Beta distribution for per-source reliability weights, gated behind 5-sample minimum — your corrections make the system smarter
- Event-driven signal processing: cross-entity propagation engine with 5 rules (job change, frequency drop, overdue actions, champion sentiment, departure+renewal risk)
- Proactive surfacing: 8 pure SQL+Rust detectors (renewal gap, relationship drift, email volume spike, meeting load forecast, stale champion, action cluster, prep coverage gap, no-contact accounts) with fingerprint dedup

### Entity Architecture

- Entity-generic data model: `meeting_entities` junction table replaces account-only meeting linking — meetings can now relate to accounts, projects, and people
- Entity-generic classification: entity hints from DB, 1:1 person detection, multi-type resolution
- Entity-generic context building: type-dispatched intelligence injection — accounts get dashboard/stakeholders/captures, projects get status/milestones, people get relationship signals
- 1:1 relationship intelligence: three-file pattern for people entities with relationship-specific enrichment prompts
- Person as first-class entity type with dedicated icon, color, and `/people` routing
- Content index populated with transcripts and notes as timeline sources for entity intelligence enrichment

### Actions

- Proposed actions triage: accept/reject flow on Actions page and Daily Briefing — transcript-sourced actions default to "proposed" status with auto-archive hygiene for stale proposals

### Fixed

- Migration blocked by foreign key constraints — resolved with `PRAGMA foreign_keys = OFF`
- Stale column reference in meeting context SQL after schema migration

## [0.9.1] - 2026-02-18

Hotfix for MCP integrations failing when app is launched from Finder/Applications.

### Fixed

- Quill, Clay, and Gravatar MCP clients fail with "connection failed" when launched from Finder — macOS GUI apps don't inherit shell PATH. Added intelligent binary resolution that scans nvm versions, Homebrew, and system paths with process-lifetime caching.

## [0.9.0] - 2026-02-18

The integrations release. Four new data integrations, a plugin marketplace, and UI polish.

### Integrations

- Granola integration: background poller syncs meeting transcripts from Granola's local cache, matches to calendar events by time window and attendee overlap, writes to entity Meeting-Notes directories
- Gravatar integration: MCP-based avatar and profile enrichment with local image caching, background poller for stale email refresh
- Clay integration: MCP client for contact and company enrichment — title, company, LinkedIn, Twitter, phone, bio, industry, HQ, company size. Signal detection for job changes, funding rounds, and leadership transitions. Background poller with bulk enrich wake signal
- Plugin Marketplace: two Claude Code plugins (`dailyos` with 9 commands + 9 skills, `dailyos-writer` with 4 commands + 11 skills) bundled as installable zips with Settings UI for export
- Person schema extended with enrichment fields: LinkedIn URL, Twitter handle, phone, photo URL, bio, title history, company industry/size/HQ
- Avatar component for person images with Gravatar cache lookup and initials fallback
- Settings UI sections for Clay, Gravatar, and Granola configuration

### Fixed

- Unicode escape sequences rendering as literal text in JSX — replaced with actual Unicode characters across 16 frontend files
- Gravatar images showing as broken blue boxes — CSP updated for Tauri's asset protocol
- Avatar component falls back to initials on image load error
- Clay "Enrich All" button now wakes poller immediately instead of waiting for next 24-hour cycle

### Changed

- Person detail pages show LinkedIn and Twitter external links with arrow indicators

## [0.8.4] - 2026-02-17

Hotfix for Claude Desktop MCP integration.

### Fixed

- MCP server stdout pollution: native library output during embedding model init was corrupting the JSON-RPC stream. Fixed by redirecting stdout to stderr during init.

## [0.8.3] - 2026-02-17

Cleanup and hardening. Type safety, migration resilience, input validation, and AI prompt robustness.

### Fixed

- Entity type narrowed at source — removes band-aid cast, fixes entity picker for projects
- Transcript action extraction resolves `@Tag` to real account ID via case-insensitive lookup — fixes silent FK violations that dropped actions
- Path traversal guard added to prep path resolution
- Stale agenda overwrite when hiding attendees — agenda parameter now optional

### Changed

- Migrations hardened with `IF NOT EXISTS` for crash-recovery safety
- Input bounds on user agenda layer: max 50 items per list, 500 chars per string, UTF-8-safe truncation
- Transcript prompt handles null title/account gracefully instead of producing malformed prompts
- Folio bar transcript button shows spinner and `not-allowed` cursor when processing

## [0.8.2] - 2026-02-17

Polish sprint. Meeting intelligence redesigned as editorial briefing, audit trail for AI-generated data, person deduplication, and print-ready PDF export.

### Added

- Audit trail module for AI-generated data — tracks provenance through the enrichment pipeline
- Person email aliasing and cross-domain deduplication — merges duplicate contacts across domains
- Meeting Intelligence Report redesigned as a full editorial briefing with outcomes pinned to top
- Transcript attach button added to folio bar on all meetings
- Print styles for clean briefing PDF output — `Cmd+P` produces a readable document
- Claude Code skill templates distributed to user workspaces for slash-command workflows
- "+ Business Unit" button on account detail folio bar
- Attendee RSVP status carried through the full calendar pipeline

### Changed

- Schedule cards show QuickContext instead of PrepGrid, with internal stakeholders filtered out
- Risk briefing Regenerate button moved to folio bar; byline is now click-to-edit
- Featured meeting remains visible in the schedule list
- Prep summaries hydrated from entity intelligence fields for richer meeting context
- Meeting entity chips use optimistic local state for instant feedback

### Fixed

- MCP sidecar binary missing executable permission after build
- Meeting card padding and prep summary hydration from prep files

## [0.8.1] - 2026-02-16

Hardening release. Security, database integrity, token optimization, and proposed actions workflow.

### Security

- Prompt injection hardening: all 7 PTY enrichment sites now wrap untrusted data in `<user_data>` XML blocks
- Output size limits: capped all parsed AI arrays (20 risks, 50 actions, 10 wins, 20 stakeholders, 10 value items) to prevent unbounded growth

### Database

- Foreign key constraints added to actions, account_team, and account_domains via table recreation migration with FK enforcement at connection level
- Fixed panic in focus capacity during DST spring-forward gaps — new timezone-aware datetime resolver handles all chrono edge cases

### Token Optimization

- Entity intelligence prompts filtered by vector search relevance — context budget capped at 10KB (down from ~25KB), mandatory files always included
- Entity intelligence output switched from pipe-delimited to JSON format with backwards-compatible fallback parser

### Actions

- Proposed actions workflow: AI-extracted actions now insert as "proposed" status with accept/reject UX, "AI Suggested" badge, and 7-day auto-archive via scheduler

### Performance

- Intelligence queue memory pruned every 60s to prevent unbounded growth
- Dashboard DB reads consolidated into single lock acquisition, reducing lock contention

### Stats

- 688 tests passing, 0 clippy warnings

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
