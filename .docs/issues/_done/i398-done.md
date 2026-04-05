# I398 — Risk Report Migration to ADR-0086 Architecture

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence

## Summary

The existing risk briefing (6-slide SCQA: Cover → Bottom Line → What Happened → The Stakes → The Plan → The Ask) is pre-ADR-0086: it reads entity intelligence from disk files, runs a direct PTY call outside intel_queue, and stores output in `risk-briefing.json` on disk. This issue migrates it to the report infrastructure (I396): reads from `entity_intel` DB, stores output in the `reports` table, and invalidates when entity intelligence updates.

The 6-slide output format and the `RiskBriefingPage` frontend are unchanged. This is a backend plumbing change.

## What Changes

**Before:**
- `gather_risk_input` reads from `read_intelligence_json(account_dir)` — disk file
- `run_risk_enrichment` spawns PTY directly — outside intel_queue
- Output stored in `account_dir/risk-briefing.json` — disk file
- `get_risk_briefing` reads `risk-briefing.json` — disk
- `save_risk_briefing` writes `risk-briefing.json` — disk
- No staleness detection — risk briefing silently ages

**After:**
- `generate_report(entity_id, 'risk')` reads from `entity_intel` DB table — same pattern as all other report types
- PTY call is the same prompt with the same output structure — only the input source changes
- Output stored in `reports` table under `report_type = 'risk'` — DB
- `get_report(entity_id, 'risk')` reads from `reports` table — DB
- Edits via `ReportShell` inline editing (draft state only, not persisted to reports table unless user explicitly saves)
- `is_stale = 1` when entity intel updates — user sees "Intelligence updated" banner

## What Does NOT Change

- The 6-slide structure: Cover, Bottom Line, What Happened, The Stakes, The Plan, The Ask
- The `RiskBriefingPage` component and its slide navigation
- The `RiskBriefing` Rust type and its serialization shape
- The AI prompt content and output format
- The Synthesis tier model selection

## Acceptance Criteria

1. `generate_risk_briefing` command routes through `generate_report(entity_id, 'risk')`. The old standalone `generate_risk_briefing` command still exists as a compatibility wrapper but internally calls the new path.
2. After calling `generate_report` for a known at-risk account: `SELECT content_json FROM reports WHERE entity_id = '<id>' AND report_type = 'risk'` returns a non-empty row. `risk-briefing.json` is NOT written to disk (verify: `ls <workspace>/Accounts/<account>/` shows no `risk-briefing.json`).
3. `get_risk_briefing` reads from the `reports` table, not from disk. Verify: delete the disk `risk-briefing.json` if it exists; call `get_risk_briefing`; report loads correctly.
4. Update entity intelligence for the account. Within one invalidation cycle: `SELECT is_stale FROM reports WHERE entity_id = '<id>' AND report_type = 'risk'` returns 1.
5. `RiskBriefingPage` renders correctly with the migrated data source — all 6 slides display with real content.
6. The `save_risk_briefing` command (user edits to slides) persists edits to the `reports` table `content_json`, not to disk.
7. Existing risk briefings stored on disk are readable during a transition period — if `reports` table has no row for the entity, fall back to reading `risk-briefing.json` if it exists.
8. `cargo test` passes. No behavior regressions on the risk briefing frontend.

## Dependencies

- Blocked by I396 (report infrastructure — needs the `reports` table and `generate_report` command to exist)
- ADR-0086 compliance

## Notes / Rationale

The risk briefing is the only existing "report type" in DailyOS and it predates ADR-0086. Migrating it to the new infrastructure is both a correctness fix (no more stale disk files) and a validation that the I396 infrastructure works for an existing, production-used report type before EBR/QBR ships.
