# Changelog

All notable changes to DailyOS are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Changed

- Meeting Prep (`MeetingDetailPage`) redesigned from dashboard-stack to report layout with executive brief hero, agenda-first flow, right-rail navigation, and appendix-style deep context.
- Meeting metadata hierarchy tightened: lifecycle promoted to header badge; noisy snapshot fields (CSM, assessment/risk narrative) removed from primary prep surface.
- Agenda/wins content normalization: inline markdown/source artifacts are stripped for display, talking-point output is treated as Recent Wins, and sidebar wins can be filtered against agenda topics to reduce duplication.
- Prep snapshot generation in Rust (`deliver.rs`) made compact and sanitized for inline rendering (cleaned lifecycle/health/ARR/renewal values).
- Prep semantics completed end-to-end: `recentWins` + `recentWinSources` are now first-class fields in prep payloads, with legacy `talkingPoints` retained as compatibility fallback.
- Prep enrichment contract updated so Agenda and Wins are parsed separately (`AGENDA` + `WINS` blocks), with source provenance captured structurally instead of inline `source:` tails.
- Added one-time Tauri migration command `backfill_prep_semantics(dry_run)` to upgrade `_today/data/preps/*.json` and `meetings_history.prep_context_json`.

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
