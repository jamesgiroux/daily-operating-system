# I396 — intelligence.json Report Fields — health_trend, value_delivered, success_metrics, open_commitments, health_score, relationship_depth

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence

## Summary

Six new fields added to entity intelligence enrichment to support the CS report types shipping in v0.14.0. These fields are entity-level enrichments (populated by the intel_queue cycle, not per-report AI calls), and are consumed mechanically by the report renderer once stored. They follow ADR-0086: one AI enrichment per entity update, all reports for that entity read from DB without additional AI calls.

All six fields apply to account entities in v0.14.0. Some apply to projects and people and will be wired in subsequent versions.

## New Fields

### `health_score` (number, 0–100)
A normalized health score derived from signal patterns, meeting cadence, stakeholder engagement, and open risk signals. Enables portfolio-level sorting and filtering in the Book of Business report. More machine-comparable than the qualitative `executive_assessment`.

### `health_trend` (object)
Direction of health over the last 60–90 days: `improving`, `stable`, `declining`, or `volatile`. Includes a `rationale` string (1–2 sentences). Derivable from signal confidence trends over time — no new data source required. Essential for EBR, Account Health Review, and Renewal Readiness sections that ask "is this getting better or worse?"

### `value_delivered` (array of outcome objects)
Completed commitments, confirmed wins, and measurable outcomes extracted from meeting transcripts and email signals. Each item: `description` (string), `date` (ISO), `source` (meeting ID or signal ID), `category` (adoption / ROI / milestone / relationship). The "what we've done for you" field that populates the EBR's Value Delivered section.

### `success_metrics` (array of KPI objects)
Tracked metrics and their current status. Each item: `name` (string), `target` (string), `current` (string), `status` (on_track / at_risk / achieved / not_started), `owner` (string, optional). Populated by user input or AI-extracted from meeting discussions about goals. Populates the EBR's KPI dashboard section.

### `open_commitments` (array of commitment objects)
Outstanding commitments made by either party, extracted from transcripts and email signals. Each item: `description`, `owner` (vendor / customer), `due_date` (optional), `source`, `status` (open / overdue / resolved). Already partially tracked via `commitment_received` signals — this surfaces them as a first-class entity field.

### `relationship_depth` (object)
Assessment of relationship coverage and strength:
- `champion_strength`: strong / moderate / weak / unknown
- `executive_access`: active / limited / none / unknown
- `stakeholder_coverage`: percentage (0–100) of known buying committee with meaningful recent engagement
- `coverage_gaps`: array of role strings where no contact exists (e.g., ["IT Lead", "Finance Approver"])

## Acceptance Criteria

1. All six fields are defined in the `entity_intel` DB schema and `intelligence.json` TypeScript types. `SELECT * FROM entity_intel WHERE entity_id = '<account>'` shows the new columns (may be NULL for accounts not yet enriched with the new prompt).
2. The AI enrichment prompt for accounts includes instructions to populate all six fields. Verify by reading the assembled prompt for one account enrichment — all six field names appear in the output schema definition.
3. After enrichment runs for an account with 3+ signals and 2+ meetings: `health_score` is a number 0–100; `health_trend.direction` is one of the four valid values; `value_delivered` has at least one entry if any completed commitment or win signal exists; `open_commitments` reflects unresolved commitment signals.
4. For accounts with sparse intelligence (1 signal, 1 meeting): fields that cannot be determined are `null` or empty arrays — not hallucinated. `health_score` may be null if insufficient data.
5. Existing intelligence fields are unaffected. No regressions on `executive_assessment`, `risks`, `stakeholder_insights`, or any existing field. `cargo test` passes.
6. The TypeScript types for `EntityIntelligence` on the frontend include all six fields as optional. `pnpm tsc` passes clean.

## Dependencies

- Foundation for I397 (report infrastructure) — report types read from these fields
- Foundation for I398 (Account Health Review) and I399 (EBR/QBR)
- Complements existing `risks` and `stakeholder_insights` fields
- See ADR-0086 (entity intelligence as shared service)

## Notes / Rationale

The six fields were selected based on what CS report types need most frequently (see `.docs/research/cs-report-types.md`). They are ordered by v0.14.0 necessity: `health_score` and `health_trend` appear in every CS report; `value_delivered` and `open_commitments` are the EBR's distinguishing sections; `success_metrics` completes the EBR KPI view; `relationship_depth` enables the health review's stakeholder coverage section.

`renewal_context` and `competitive_context` are intentionally deferred to v0.14.1 — they require additional data sources (contract metadata, competitive signal types) beyond what current signal enrichment produces.
