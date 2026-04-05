# I399 — Account Health Review Report Type (CS-First)

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence + Frontend / Reports

## Summary

The Account Health Review is an internal CS report — for the CSM and their manager, not customer-facing. It synthesizes an account's current health state into a one-document snapshot: health score and trend, key risk signals, stakeholder coverage, engagement cadence, and open commitments. CSMs use it for monthly account reviews, manager 1:1s, and renewal preparation.

This is the simpler of the two new CS report types in v0.14.0 (EBR/QBR is I399). It builds on the I396 report infrastructure and requires the I395 intelligence fields. It validates the report pipeline end-to-end before tackling the more complex EBR/QBR.

## Report Sections

1. **Health Summary** — One paragraph executive assessment. Source: `entity_intel.executive_assessment` + `health_trend.rationale`.
2. **Health Score & Trend** — Normalized score (0–100) and direction (improving / stable / declining / volatile) with 1-sentence rationale. Source: `entity_intel.health_score`, `entity_intel.health_trend`.
3. **Key Risks** — Top 3–5 active risk signals with urgency rating and brief description. Source: `entity_intel.risks`.
4. **Stakeholder Coverage** — Champion strength, executive access status, contacts with no recent engagement, coverage gaps. Source: `entity_intel.relationship_depth`, `entity_intel.stakeholder_insights`.
5. **Engagement Cadence** — Meeting frequency over last 60 days, last meaningful interaction date, email response patterns. Source: meeting history from `entity_intel` context.
6. **Open Commitments** — Unresolved commitments from both sides. Source: `entity_intel.open_commitments`.
7. **Renewal Outlook** (if applicable) — Renewal date and risk assessment if renewal context exists. Source: account metadata, `entity_intel.health_trend`.

## Acceptance Criteria

1. `generate_report(entity_id, 'account_health')` produces a report with all 7 sections populated (or gracefully absent if data is insufficient — e.g., no "Renewal Outlook" section if no renewal date is set).
2. The report renders in `ReportShell` on the account detail page. Access path: account detail page → "Reports" button or tab → "Account Health Review".
3. All section content is sourced from `entity_intel` fields — verify by checking `content_json` in the `reports` table: the AI-generated text references the account's real signal data (account name, actual risk descriptions, real stakeholder names from `stakeholder_insights`).
4. For an account with sparse intelligence (1 signal, 1 meeting): the report renders with reduced content and includes a note like "Limited intelligence — some sections may be incomplete." No hallucinated content.
5. All text fields in the rendered report are editable inline. Edits are local draft state only.
6. Export to PDF produces a clean document: all 7 sections visible and readable, serif headers, under 2MB.
7. When `entity_intel` is updated for the account, the report's `is_stale` flag is set. The "Intelligence updated — regenerate?" banner appears on next view.
8. The report type label in the UI is "Account Health Review" — not "Report" or "Intelligence Summary".

## Design Decisions

1. **Engagement Cadence** — Uses meeting frequency as the primary signal. `meetings_history` provides meeting count per entity over any time window. Engagement cadence = meetings per month, trend direction (increasing/declining/stable), and days since last meeting. Email signal count from the signal bus adds depth. Do NOT pull Gmail response times — new API surface for marginal value. Meetings + email signals = enough.

2. **Renewal Outlook condition** — Section appears when the account has a `renewal_date` (or `contract_end`) field set in account metadata/vitals. If the field is null/empty, the section is omitted.

## Dependencies

- Blocked by I396 (report infrastructure)
- Blocked by I395 (intelligence.json fields — `health_score`, `health_trend`, `relationship_depth`, `open_commitments` all needed)
- See `.docs/research/cs-report-types.md` for full report type rationale

## Notes / Rationale

The Account Health Review is the most frequently used CS report and the simplest to generate — all content comes from fields already in entity intelligence. Building it first validates the I396 infrastructure end-to-end with a straightforward prompt before tackling EBR/QBR's more complex structure. Every section maps directly to an existing or I395-added entity_intel field; no additional AI synthesis is needed beyond the report generation step.
