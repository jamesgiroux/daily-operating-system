# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.7.1] - 2026-02-12

Six sprints of work across meeting intelligence, entity relationships, security hardening, and app responsiveness. 574 Rust tests. 69 Architecture Decision Records.

### Meeting Intelligence

- Meeting prep redesigned as a report: executive brief hero, agenda-first flow, right-rail navigation, appendix-style deep context
- Agenda and Wins are now semantically separate enrichment blocks with structured source provenance (replaces flat talking points)
- User-authored prep fields (`userAgenda`, `userNotes`) are DB-authoritative with freeze/editability rules (ADR-0065)
- Meeting identity hardened: calendar event ID is canonical primary key across poller, reconcile, and DB (ADR-0061)
- Unified meeting intelligence contract (`get_meeting_intelligence`) combines prep, outcomes, and transcript metadata in a single backend call (ADR-0066)
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
- Focus capacity computes from live calendar, schedule artifact retained for briefing narrative only (ADR-0062)

### Security & Auth

- OAuth hardened with PKCE (`S256`) challenge + state parameter validation (ADR-0068)
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
- Resume latency instrumentation with p50/p95/max rollups and budget violation tracking (ADR-0067)
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
- Radix UI components migrated to explicit standalone packages, resolving dual-install portal bug (ADR-0069)
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
