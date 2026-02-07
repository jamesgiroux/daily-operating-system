# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

---

## Issues

<!-- Thematic grouping for orientation:
  Data Trust:          I23, I33        — Actions dedup, outcomes resurface in preps
  Inbox Pipeline:      I30, I31, I32   — Rich enrichment matching CLI capabilities
  Settings Self-Serve: I7, I15, I16    — User can configure without editing JSON
  Email Pipeline:      I20, I21        — Three-tier email promise (ADR-0029)
  UI Consistency:      I25, I9         — Badge unification, stub pages
  First-Run & Ship:    I13, I8         — Onboarding, distribution
  Infrastructure:      I26, I27, I28, I29 — Extension/MCP/schema systems
-->

### Open — High Priority

**I23: No cross-briefing action deduplication in prepare_today.py**
ADR-0031 notes that `prepare_today.py` should check SQLite before extracting actions from markdown, to avoid re-extracting already-indexed actions. Currently extracts everything and relies on `upsert_action_if_not_completed()` to not overwrite. Works but wasteful, and same action from different sources can create duplicate entries with different IDs. Stable content-hash IDs + title-based dedup guard address the ID instability side; the SQLite pre-check is separate future work.

**I33: Captured wins/risks don't resurface in meeting preps**
Post-meeting capture stores wins, risks, and context in SQLite, but none of it appears in the next prep for that account. When you meet with Acme on Monday and capture a risk, Tuesday's Acme prep should reference it. Depends on account linking (captured outcome → account_id → prep lookup). Blocked by I30 (rich metadata extraction) and account registry (I27 extension system).

### Open — Medium Priority

**I7: Settings page can't change workspace path**
Displays as read-only. Needs Tauri `dialog::FileDialogBuilder` for directory picker, a `set_workspace_path(path)` command, and validation. Small scope.

**I8: No app update/distribution mechanism**
Options: Tauri's built-in updater, GitHub Releases + Sparkle, manual DMG, Mac App Store. Needs Apple Developer ID for notarization. Not blocking MVP — can ship as manual DMG.

**I9: Focus page and Week priorities are disconnected stubs**
`focus.json` returns "not yet implemented." Weekly priorities from `week-overview.json` don't flow into daily focus. `/week` should set weekly priorities; `/today` should derive daily focus from those + today's schedule.

**I13: No onboarding flow**
First-time user hits dead end after profile selection. If Google isn't connected, "Generate Briefing" fails. Minimal onboarding needs: profile selection (exists, defaults to CS per ADR-0038), Google connection (exists in Settings, not surfaced), workspace path (display exists, editing doesn't — see I7), first briefing trigger. Design constraint: Principle 4 (Opinionated Defaults). Could create `~/Documents/DailyOS/` as default workspace. Google is the only mandatory step.

**I15: Profile switching unavailable in Settings**
Profile selector at first launch says "You can change this later in Settings" but Settings has no switcher. Needs: dropdown/radio in Settings, writes to config.json, triggers reload.

**I16: Schedule editing requires manual config.json editing**
Settings shows raw cron expressions. Needs: time picker ("Briefing time: 6:00 AM"), writes cron to config, hides syntax. Power users can still edit JSON directly.

**I18: Google API calls not coordinated across callers**
`prepare_today.py`, calendar poller, and manual refresh all hit Google independently. No cache or coordination. Needs shared API cache with TTL. Not blocking MVP.

**I20: No standalone email refresh**
Emails only update with full briefing. A lightweight email-only pipeline could be valuable but raises partial-refresh semantics questions. ADR-0006 determinism boundary still applies.

**I21: FYI emails may never appear due to classification defaults**
`classify_email_priority()` defaults to "medium" (line 712). Only newsletters, automated senders, and GitHub notifications trigger "low." If a user's inbox is mostly customer + internal emails, the FYI section is permanently empty — not wrong, but means the three-tier promise (ADR-0029) is invisible. Consider: expanding low signals (marketing domains, bulk senders), or showing an explicit "0 FYI" indicator so users know the tier exists.

**I25: Unify meeting badge/status rendering**
MeetingCard has 5 independent status signals (isCurrent, hasPrep, isPast, overlayStatus, type) each with their own conditional. Consolidate into a computed MeetingDisplayState. Relates to ADR-0033.

**I26: Web search for unknown external meetings not implemented**
ADR-0022 specifies proactive research via local archive + web for unknown meetings. Local archive search works in `prepare_today.py`. Web search does not exist. Likely a Phase 2 task — Claude can invoke web search during enrichment (Phase 2). Low urgency since archive search provides some coverage.

**I27: Extension registry and schema system not implemented**
ADR-0026 accepts extension architecture (profile-activated modules with post-enrichment hooks, data schemas, UI contributions). Current state: profile field exists, hook execution checks profile, UI route stubs exist. Missing: formal extension registration mechanism, extension schemas, template system. Phase 4 per ADR. Profile-specific classification depends on this. ADR-0039 adds feature toggle granularity within extensions.

**I28: MCP server and client not implemented**
ADR-0027 accepts dual-mode MCP (server exposes workspace tools to Claude Desktop, client consumes Clay/Slack/Linear). IPC commands are designed to be MCP-exposable (good foundation from ADR-0025). No MCP protocol code exists. Phase 4 per ADR.

**I29: Structured document schemas not implemented**
ADR-0028 accepts JSON-first schemas for account dashboards, success plans, and structured documents (`dashboard.json` + `dashboard.md` pattern). Briefing JSON pattern exists as a template. Account dashboard UI is a stub. No schema validation system. Blocked by extension architecture (I27) for CS-specific schemas.

**I30: Inbox action extraction lacks rich metadata**
Owner, due date, priority, context fields exist in SQLite schema but inbox processor never populates them. AI enrichment prompt (`enrich.rs`) should extract these when processing transcripts and meeting notes. High impact for closing I17's feedback loop.

**I31: No transcript summarization in inbox processor**
CLI generates customer/internal meeting summaries from transcripts. App's AI enrichment gives a one-line summary only. Needs customer/internal summary templates in the enrichment prompt.

**I32: Inbox processor doesn't update account intelligence**
Post-enrichment hooks framework exists (ADR-0026) but no CS hook writes wins/risks/last-contact to account profiles. Depends on I30 (need rich extraction first).

### Open — Low Priority

**I2: Compact meetings.md format for dashboard dropdowns**
Archive contains a compact format with structured prep summaries. Could be useful for quick-glance meeting cards. Post-MVP.

**I3: Browser extension for web page capture to _inbox/**
Chromium extension for page capture to markdown in `_inbox/`. Aligns with "system does the work." Post-Phase 2 when inbox processing is stable.

**I4: Motivational quotes as personality layer**
Viable placements: overview greeting (daily rotating), empty states ("you crushed it"). Rejected approach: welcome interstitial (adds required click, violates Principle 2).

**I6: Processing history page**
`processing_log` table exists in SQLite with `get_processing_log()`. Missing: Tauri command + UI to render history. Supports Principle 9 (Show the Work).

**I10: No shared glossary of app terms**
Overlapping terms (briefing, workflow, capture, focus, etc.) used inconsistently. Needs shared definitions in DEVELOPMENT.md or a GLOSSARY.md.

**I19: AI enrichment failure not communicated to user**
When Phase 2 fails, briefing renders thin with no indication. Recommended: quiet "AI-enriched" badge (absence = not enriched). Fits Principle 9.

### Closed

**I1: Config directory naming** — Resolved. Renamed `.daybreak` → `.dailyos`.

**I5: Orphaned pages (Focus, Week, Emails)** — Resolved. All three now have defined roles: Focus = drill-down from dashboard, Week = sidebar item (Phase 2+), Emails = drill-down from dashboard. See ADR-0010.

**I11: Phase 2 email enrichment not fed to JSON** — Resolved. `deliver_today.py` gained `parse_email_enrichment()` which reads `83-email-summary.md` and merges into `emails.json`.

**I12: Email page missing AI context** — Resolved. Email page shows summary, recommended action, conversation arc per priority tier. Removed fake "Scan emails" button.

**I14: Dashboard meeting cards don't link to detail page** — Resolved. MeetingCard renders "View Prep" button linking to `/meeting/$prepFile` when prep exists. Added in Phase 1.5.

**I17: Post-meeting capture outcomes don't resurface in briefings** — Resolved (actions side). Non-briefing actions (post-meeting, inbox) now merge into dashboard via `get_non_briefing_pending_actions()` with title-based dedup. Wins/risks resurfacing split to I33.

**I22: Action completion doesn't write back to source markdown** — Resolved. `sync_completion_to_markdown()` in `hooks.rs` runs during post-enrichment hooks. Queries recently completed actions with `source_label`, writes `[x]` markers back to source files. Lazy writeback is acceptable — SQLite is working store, markdown is archive.

**I24: schedule.json meeting IDs are local slugs, not Google Calendar event IDs** — Resolved. Added `calendarEventId` field alongside the local slug `id` in both `schedule.json` and `preps/*.json`. Local slug preserved for routing/filenames; calendar event ID available for cross-source matching (ADR-0032, ADR-0033).

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | No | Need onboarding check (I13) |
| A2 | Workspace follows PARA structure | No | Should handle variations gracefully |
| A3 | `_today/` files use expected markdown format | Partial | Parser handles basic cases |
| A4 | Users have Google Workspace (Calendar + Gmail) | No | Personal Gmail, Outlook, iCloud not supported in MVP |

---

## Dependencies

| ID | Dependency | Type | Status | Notes |
|----|------------|------|--------|-------|
| D1 | Claude Code CLI | Runtime | Available | Requires user subscription |
| D2 | Tauri 2.x | Build | Stable | Using latest stable |
| D3 | Google Calendar API | Runtime | Optional | For calendar features |

---

*Migrated from RAIDD.md on 2026-02-06. Decisions are now tracked in [docs/decisions/](decisions/README.md).*
