# I57 — First-Run Wizard — Connect, Configure, and Launch

**Status:** Open
**Priority:** P0
**Version:** 0.16.0
**Area:** Frontend / Onboarding

## Summary

The six decisions that determine whether DailyOS works for someone (Claude Code, Google connection, role preset, user domains, first account, user context basics) currently happen across different Settings sections with no guidance on order or urgency. Claude Code is step 1 — without it, DailyOS cannot build context for anything and is effectively a calendar viewer. The wizard surfaces all six in sequence, with context for why each matters, and gets the user to a live briefing in under 5 minutes.

## Acceptance Criteria

1. On first launch with no Google auth and no workspace data, a full-screen first-run wizard appears rather than the empty dashboard. It does not appear on subsequent launches.

2. The wizard has 6 steps, each completable in under 60 seconds:
   - **Step 1 — Claude Code** (required): Verify Claude Code is installed and authenticated. Calls `PtyManager::is_claude_available()` and `PtyManager::is_claude_authenticated()`. If not installed: shows instructions to install Claude Code (claude.ai/download) with a "Check again" button. If not authenticated: shows instructions to run `claude` in terminal and sign in. This step cannot be skipped — DailyOS without AI is a calendar viewer. The step makes this explicit: "DailyOS uses Claude Code to build briefings and insights. Without it, the app cannot find new information about your meetings, accounts, or emails."
   - **Step 2 — Connect Google**: Connect Calendar + Gmail (existing OAuth flow). Skip option available — the app works with manual data if Google isn't connected.
   - **Step 3 — Your role**: Role preset selection (existing presets). Default: Customer Success.
   - **Step 4 — Your domain**: Enter your work email domain(s) so DailyOS knows which contacts are internal vs. external.
   - **Step 5 — Your first account**: Add one customer account by name. Optional but encouraged.
   - **Step 6 — About you** (if v0.14.0 is shipped): Prefills the user entity with value proposition and one priority. If v0.14.0 hasn't shipped, this step is skipped.

3. Completing all steps lands the user on the daily briefing with a "Your first briefing is generating" state — not an empty state. If Google is connected, the poller runs immediately. If not, the demo briefing template is shown.

4. Skipping the entire wizard (clicking "Skip setup") lands on the empty dashboard with a persistent "Complete setup" prompt in the Settings area.

5. Each step's data persists immediately on completion — if the user closes the app mid-wizard, progress is retained on next launch.

## Dependencies

Blocked by I56 — demo mode establishes the empty state that the wizard replaces on first run. Benefits from v0.14.0 being shipped (Step 5 — About you). If v0.14.0 hasn't shipped, Step 5 is skipped gracefully.

## Notes

The wizard is a full-screen surface, not a modal on top of the app. It should feel like a distinct "setup" experience before entering the main app shell. Each step is independently skippable. The wizard never blocks access to the app permanently — always a "Skip setup" escape hatch. Data from each completed step persists immediately; closing mid-wizard does not lose progress.
