# I425 — Linear signal wiring

**Status:** Open
**Priority:** P1
**Version:** 0.13.9
**Area:** Backend / Linear + Intelligence

## Summary

Linear syncs to `linear_issues` and `linear_projects` tables (both currently empty — not yet configured in production) but emits zero signals and has zero consumers in the briefing pipeline. Data is stored but never surfaced. This issue: emit signals on issue state changes, build entity linking, surface blocked/overdue issues in meeting prep, and show issue counts in the briefing attention section for project-linked entities.

## Acceptance Criteria

1. **Signal emission on sync:** When Linear issues are synced via `upsert_issues`, signals are emitted for state changes. Signal types:
   - `linear_issue_blocked` — when an issue's `state_type` changes to `started` or `inProgress` with `state_name` containing "blocked"
   - `linear_issue_completed` — when `state_type` changes to `completed`
   - `linear_issue_overdue` — when `due_date` is in the past and `state_type` is not `completed`
   Verify: `SELECT signal_type, source FROM signal_events WHERE source = 'linear' ORDER BY created_at DESC LIMIT 10` — returns rows after a Linear sync with active issues.

2. **Entity linking:** When Linear issues or projects are linked to DailyOS entities (accounts or projects), the signal carries the entity context. A new `linear_entity_links` table (or a `entity_id`/`entity_type` column on `linear_issues`) maps Linear issues to DailyOS entities by project name matching and user-confirmed links.

3. **Meeting prep enrichment:** When assembling meeting prep for a meeting linked to a project entity that has associated Linear issues, the prep includes a "Related work" section showing: blocked issues, overdue issues, issues completed since the last meeting. Verify by opening a meeting detail page for a meeting linked to a project that has Linear issues — "Related work" appears with real issue data.

4. **Briefing surface:** In the daily briefing attention section, a meeting with project-linked Linear issues that are blocked or overdue shows a signal callout. Verify: `SELECT count(*) FROM signal_events WHERE source = 'linear'` is non-zero after configuring Linear with a real API key.

5. Linear Settings card (`LinearConnection.tsx`) shows: connected status, issue count, project count, "last synced" timestamp, and a list of the most recently synced issues. Currently it shows counts but no list — add the issue list (last 5 issues, truncated with a "View all" action if applicable).

## Dependencies

Requires a valid Linear API key to verify signal emission, but the code changes are self-contained.

## Notes / Rationale

**Key files:**
- Linear syncer (location TBD in code review, likely `src-tauri/src/linear/` or similar)
- Meeting prep assembly — likely `src-tauri/src/intelligence/compute.rs` or `src-tauri/src/workflow/`
- Briefing attention section — frontend component in `src/pages/` or `src/components/`

**Entity linking strategy:**
Linear projects map to DailyOS project entities by name. The mapping can be automatic (exact match) or user-confirmed via Settings. Once linked, any issue in that project inherits the entity context, so `linear_issue_blocked` signals automatically know which entity they apply to.

**Meeting prep "Related work" section:**
When opening a meeting detail page for a meeting linked to a Salesforce Agentforce project (or any project with Linear issues), the prep should include:
- Issues blocked or waiting (might be blocking the meeting itself)
- Issues overdue (background context on what's behind schedule)
- Issues closed since the last meeting (context on progress)

This gives the meeting attendees immediate visibility into the project's work status without leaving DailyOS.

**Rationale:**
Linear is the system of record for project work. By integrating it as a signal source, meetings become contextual — they know what work is blocked, what's overdue, and what's progressing. The briefing attention section can then prioritize meetings or entities where Linear work is stalled or at risk.
