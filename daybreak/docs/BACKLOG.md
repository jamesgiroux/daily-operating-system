# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

---

## Issues

### Open — High Priority

**I13: No onboarding flow**
First-time user hits dead end after profile selection. If Google isn't connected, "Generate Briefing" fails. Minimal onboarding needs: profile selection (exists), Google connection (exists in Settings, not surfaced), workspace path (display exists, editing doesn't — see I7), first briefing trigger. Design constraint: Principle 4 (Opinionated Defaults). Could create `~/Documents/DailyOS/` as default workspace. Google is the only mandatory step.

**I14: Dashboard meeting cards don't link to detail page**
The most important user action — "I see my next meeting, let me review the prep" — is a dead end. ADR-0013 decided detail is a drill-down at `/meeting/$prepFile`. Cards need a `Link` wrapper + conditional logic (clickable when prep exists). Highest-impact UX fix. Depends on ADR-0033 near-term (shared ID mapping).

### Open — Medium Priority

**I7: Settings page can't change workspace path**
Displays as read-only. Needs Tauri `dialog::FileDialogBuilder` for directory picker, a `set_workspace_path(path)` command, and validation. Small scope.

**I8: No app update/distribution mechanism**
Options: Tauri's built-in updater, GitHub Releases + Sparkle, manual DMG, Mac App Store. Needs Apple Developer ID for notarization. Not blocking MVP — can ship as manual DMG.

**I9: Focus page and Week priorities are disconnected stubs**
`focus.json` returns "not yet implemented." Weekly priorities from `week-overview.json` don't flow into daily focus. `/week` should set weekly priorities; `/today` should derive daily focus from those + today's schedule.

**I15: Profile switching unavailable in Settings**
Profile selector at first launch says "You can change this later in Settings" but Settings has no switcher. Needs: dropdown/radio in Settings, writes to config.json, triggers reload.

**I16: Schedule editing requires manual config.json editing**
Settings shows raw cron expressions. Needs: time picker ("Briefing time: 6:00 AM"), writes cron to config, hides syntax. Power users can still edit JSON directly.

**I17: Post-meeting capture outcomes don't resurface in briefings**
Captured data goes to SQLite/impact log but never appears in next day's briefing. Erodes trust. Actions should appear in next day's list. Wins/risks should appear in next meeting prep for that account. Depends on ADR-0031, CS extension.

**I18: Google API calls not coordinated across callers**
`prepare_today.py`, calendar poller, and manual refresh all hit Google independently. No cache or coordination. Needs shared API cache with TTL. Not blocking MVP.

**I20: No standalone email refresh**
Emails only update with full briefing. A lightweight email-only pipeline could be valuable but raises partial-refresh semantics questions. ADR-0006 determinism boundary still applies.

**I21: FYI emails may never appear due to classification defaults**
`classify_email_priority()` defaults to "medium" (line 712). Only newsletters, automated senders, and GitHub notifications trigger "low." If a user's inbox is mostly customer + internal emails, the FYI section is permanently empty — not wrong, but means the three-tier promise (ADR-0029) is invisible. Consider: expanding low signals (marketing domains, bulk senders), or showing an explicit "0 FYI" indicator so users know the tier exists.

**I22: Action completion doesn't write back to source markdown**
ADR-0031 specifies post-enrichment hooks should write `[x]` completion markers back to source markdown files when `source_label` points to a specific file. Not implemented. Risk: if SQLite is deleted, all in-progress completion status is lost. Markdown writeback would make completion durable.

**I23: No cross-briefing action deduplication in prepare_today.py**
ADR-0031 notes that `prepare_today.py` should check SQLite before extracting actions from markdown, to avoid re-extracting already-indexed actions. Currently extracts everything and relies on `upsert_action_if_not_completed()` to not overwrite. Works but wasteful, and same action from different sources can create duplicate entries with different IDs.

**I24: schedule.json meeting IDs are local slugs, not Google Calendar event IDs**
`prepare_today.py` preserves Google Calendar event IDs, but `deliver_today.py` may generate local slugs (e.g., "0900-acme-sync") instead. ADR-0032 and ADR-0033 both depend on stable event ID matching. Verify whether IDs survive `deliver_today.py` and fix if not. Blocks hybrid calendar overlay and meeting entity unification.

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
