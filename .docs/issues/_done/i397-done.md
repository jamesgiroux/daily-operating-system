# I397 — Report Infrastructure — reports Table, intel_hash Invalidation, Report Shell, PDF Export

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence + Frontend / Reports

## Summary

The shared infrastructure that all DailyOS report types build on. Establishes a `reports` DB table, intel_hash-based cache invalidation, a report shell renderer (shared layout with editable inline fields), and PDF export. Absorbs I258 (Report Mode) and I302 (PDF export), which are superseded by this more complete design.

The report architecture follows ADR-0086: `entity_intel` is the input, reports are on-demand AI enrichment stored in DB, subsequent views are mechanical reads with no AI call. A report is stale when the entity's intelligence has been updated since the report was generated — surfaced as "intelligence updated, regenerate?" not silent staleness.

## Architecture

```
entity_intel (DB, always current — updated by intel_queue)
    │
    ▼  on-demand, user-initiated or auto-trigger
generate_report(entity_id, report_type) → PTY call with entity_intel as context
    │
    ▼  stored
reports table (entity_id, report_type, content_json, generated_at, intel_hash)
    │
    ▼  mechanical reads, no AI
ReportPage → ReportShell → sections → editable inline fields → PDF export
```

## DB Schema

```sql
CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    report_type TEXT NOT NULL,          -- 'account_health', 'ebr_qbr', 'risk', 'swot', etc.
    content_json TEXT NOT NULL,
    generated_at DATETIME NOT NULL,
    intel_hash TEXT NOT NULL,           -- hash of entity_intel.content_json at generation time
    is_stale INTEGER DEFAULT 0,         -- 1 when entity_intel has updated since generation
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (entity_id) REFERENCES accounts(id)
);
```

When `entity_intel` is updated for an entity, all rows in `reports` for that entity are marked `is_stale = 1`. This is the invalidation mechanism — no polling, no TTL.

## Report Shell (shared renderer)

All report types render inside `ReportShell`, which provides:
- Editorial document layout (A4-proportioned, print-ready margins)
- Report header (account name, report type, generated date, staleness indicator)
- Section renderer — each section is a named block with title and content
- Inline editing — all text fields editable; edits are local state only (not persisted to DB unless explicitly saved as a "draft")
- Export button — triggers PDF export
- Regenerate button — clears stale flag, triggers new `generate_report` call

## PDF Export

Uses the existing `export_briefing_html` mechanism (added in v0.8.2). The report view renders as HTML with print CSS. Tauri's print-to-PDF produces the file. Key requirements:
- Newsreader serif headers, DM Sans body (already in print CSS)
- A4 page breaks at section boundaries
- No HTML artifacts in output
- Offline (no cloud service)

## Report Types Shipping in v0.14.0

Three report types ship with the infrastructure:
1. **SWOT** — synthesized from existing entity_intel signals, simple format, validates the infrastructure
2. **Account Health Review** (I398) — internal CS report, reads from entity_intel fields
3. **EBR/QBR** (I399) — flagship CS report, requires I396 fields

## Acceptance Criteria

1. `reports` table exists in the DB with all specified columns. A migration creates it. `cargo test` passes.
2. `generate_report(entity_id, report_type)` command exists. Calling it for a known account with `report_type = 'swot'` produces a non-empty `content_json` in the `reports` table within 2 minutes (PTY call completes).
3. `get_report(entity_id, report_type)` returns the cached report without a PTY call. Verify: call `generate_report`, then call `get_report` twice — the second call is instant (< 100ms), no PTY process spawned.
4. Intel hash invalidation: update an account's `entity_intel`. Verify: `SELECT is_stale FROM reports WHERE entity_id = '<account>'` returns 1 for that account's reports within one invalidation cycle.
5. The report UI shows a "Intelligence updated since this report was generated — regenerate?" banner when `is_stale = 1`. The banner disappears after regenerating.
6. `ReportShell` renders correctly for all three report types (SWOT, Account Health Review, EBR/QBR). All text fields are editable inline. Edits are not persisted to the database — verify by editing a field, closing the report, reopening it: the original AI-generated content is shown.
7. "Export PDF" from any report produces a file: serif headers, sans-serif body, design token colors, under 2MB, readable in Preview. No HTML artifacts.
8. The existing risk briefing (RiskBriefingPage) is unaffected — it reads from its own path. Risk report migration (I397) is a separate issue.
9. SWOT format: four quadrants (Strengths, Weaknesses, Opportunities, Threats), each pre-populated from entity_intel signals. At least 2 items per quadrant for an account with 3+ signals. Items reference real signal types or meeting titles — not generic placeholder text.

## Dependencies

- Requires I396 (intelligence.json report fields) for Account Health Review and EBR/QBR sections
- I397 (risk report migration) is a follow-on that migrates the existing risk briefing to this infrastructure
- I398 (Account Health Review) and I399 (EBR/QBR) are the report types built on this foundation
- See ADR-0086

## Notes / Rationale

Supersedes I258 (Report Mode) and I302 (Shareable PDF Export) — those issues described the problem correctly but underspecified the architecture. The `reports` table + intel_hash invalidation pattern is the key insight: reports are cached entity intelligence enrichments, not per-view AI calls. The report shell being shared across all report types means adding a new report type (community templates eventually) is purely a prompt + content_json schema addition.
