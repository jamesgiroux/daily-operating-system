# I400 — EBR/QBR Report Type (CS-First)

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence + Frontend / Reports

## Summary

The Executive Business Review (EBR) or Quarterly Business Review (QBR) is the highest-stakes CS deliverable — a customer-facing document prepared 2–3 weeks before a quarterly meeting with the customer's executive team. Its purpose: demonstrate value delivered, align on the relationship's strategic direction, and secure the customer's commitment to the next period (renewal, expansion, or mutual program).

A pre-populated EBR draft from DailyOS intelligence is the flagship v0.14.0 use case. A CSM can open the EBR report for an account, see a pre-populated draft from the account's signal history and meeting intelligence, edit it to add nuance or correct anything, and export a PDF ready to send to the customer.

## Report Sections

1. **Partnership Overview** — tenure, ARR (if available), core team, account type. Factual framing. Source: account metadata.
2. **Goals Recap** — commitments made at last review or onboarding. Source: `entity_intel.open_commitments` (resolved subset), meeting history.
3. **Value Delivered** — measurable outcomes, adoption wins, ROI evidence, case study moments. Source: `entity_intel.value_delivered`. The most differentiating section — what DailyOS extracts from meeting transcripts and signals that a CSM would otherwise have to manually compile.
4. **Success Metrics** — agreed KPI targets vs. actual current state. Source: `entity_intel.success_metrics`. Table format: metric name / target / current / status.
5. **Challenges & Resolutions** — what went wrong and how it was addressed. Honesty here builds credibility. Source: risk signals with temporal resolution inference (resolved by absence after 30+ days, or resolved by subsequent positive signal), meeting history.
6. **Strategic Roadmap** — what's coming from the vendor's side; how it serves the customer's goals. Source: AI synthesis from account quarterly priorities (linked to user priorities via I414), open commitments, and value_delivered trajectory. User-editable.
7. **Customer Asks** — open feature requests, support escalations, resource requests. Source: `entity_intel.open_commitments` (customer-owned, open).
8. **Next Period Priorities** — 3–5 agreed action items with owners and proposed dates. AI-suggested from open items; user-edited to confirm.

## Acceptance Criteria

1. `generate_report(entity_id, 'ebr_qbr')` produces a report with all 8 sections. For an account with 3+ meetings and 5+ signals, every section has non-trivial content.
2. The **Value Delivered** section is the quality gate: it must reference at least one specific, real event from the account's meeting or signal history — a completed commitment, a positive signal, a meeting outcome. Generic filler ("the team has been working closely together") is a failure. Verify by reading `content_json.value_delivered` and checking that at least one item cites a real source (meeting ID or signal type).
3. The **Success Metrics** section renders as a table. If `entity_intel.success_metrics` is empty (CSM hasn't defined KPIs yet), the section shows a placeholder row "Add your success metrics" — not hidden. The EBR should prompt the CSM to fill this in.
4. The report is flagged as **customer-facing** in the `ReportShell` header — this affects the PDF export (adds a DailyOS-branded but subdued cover treatment, removes internal notes sections). Verify: exported PDF does not include internal signal debug data or confidence scores.
5. All 8 sections are individually editable inline. The CSM workflow is: review → edit → export. Changes are local draft state only.
6. Export to PDF: clean editorial layout, customer-presentable quality. Section headings are clear, no internal DailyOS terminology visible (no "signal", "entity_intel", "confidence score"). File under 2MB.
7. When opened for an account that has no prior EBR generated, the UI offers "Generate EBR/QBR" with an estimated generation time (PTY call, ~60–120s). A progress indicator shows during generation.
8. When `entity_intel` updates, `is_stale = 1` is set. The "Intelligence updated — regenerate to include latest signals" banner appears.
9. The report type is accessible from the account detail page under a "Reports" action — not buried in a folio bar dropdown.

## Design Decisions

### Strategic Roadmap section (Section 6) — AI synthesis, not a new field

Does NOT require a new `strategic_programs` field on `entity_intel`. Instead, the prompt synthesizes from existing data: the account's quarterly priorities (linked to user priorities via I414), open commitments, and recent wins that suggest trajectory. The prompt frames it as "where is this relationship heading based on what's been delivered and what's outstanding." This is an AI synthesis section, not a data lookup — the intelligence is in the juxtaposition of what's been accomplished against what's still open.

Update to section 6 description: source is account quarterly priorities, open commitments, and value_delivered history — not a dedicated `strategic_programs` field.

### Challenges & Resolutions (Section 5) — temporal inference for "resolved" risks

Uses temporal inference rather than a schema field to determine resolution. Two patterns:

1. **Resolved by absence** — A risk signal emitted 30+ days ago with no follow-up signal of the same type is inferred as resolved. The absence of recurrence is the evidence.
2. **Resolved by outcome** — A risk signal followed by a positive signal on the same entity (health improvement, commitment completed) is inferred as resolved by that outcome. The positive signal is the evidence.

The prompt instructs the AI to pair old risk signals with subsequent positive events and frame them as challenge-resolution narratives. This is a prompt engineering problem, not a schema problem — no `resolved` field needed on risk signals.

## Dependencies

- Blocked by I396 (report infrastructure)
- Blocked by I395 (intelligence.json fields — `value_delivered`, `success_metrics`, `open_commitments`, `health_trend` all needed)
- Complements I398 (Account Health Review) — Health Review is internal prep; EBR/QBR is the customer-facing output
- See `.docs/research/cs-report-types.md`

## Notes / Rationale

The EBR/QBR is what justifies building the entire report infrastructure. CSMs spend 2–8 hours manually assembling EBRs from notes, meeting recordings, email threads, and spreadsheets. DailyOS has all of this data. A pre-populated EBR draft that the CSM reviews and edits — instead of writing from scratch — is the clearest demonstration of "intelligence leaving the app as shareable, useful artifacts."

The quality gate on Value Delivered (criterion 2) is non-negotiable. An EBR with generic content is worse than no EBR — it creates more work to fix than to write from scratch. The AI prompt for this section must be designed to extract specific, citable events from the meeting and signal history, not synthesize generic relationship language.
