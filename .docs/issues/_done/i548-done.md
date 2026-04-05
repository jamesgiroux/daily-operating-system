# I548 — Inline Editing Audit & Consistency Fix

**Priority:** P2
**Area:** Frontend / Interaction Patterns
**Version:** v1.0.0 Phase 4
**Depends on:** None (all editing components exist)

## Problem

INTERACTION-PATTERNS.md defines a clear inline editing model: `EditableText` for prose, `EditableInline` for single-line labels, `EditableTextarea` for multiline blocks, `EditableDate` for dates, `EditableVitalsStrip` for structured vitals, `EditableList` for ordered lists. The spec mandates: edit in place, Escape to cancel, Tab navigation between fields, debounced save for reports, toast or folio bar save status.

In practice, implementation is inconsistent across pages:

1. **ActionDetailPage defines its own local `EditableText`** (line 396) instead of using the shared component. The local version lacks Tab navigation (`data-editable-text`), Tauri event emission (`editable-text:commit`), hover hints, and the terracotta bottom-border edit indicator. It's a bare click-to-input with none of the design system's visual treatment.

2. **RiskBriefingPage uses `save_risk_briefing` command** while every other report page uses the unified `save_report` command. This is a persistence inconsistency — same user-facing pattern, different backend path.

3. **MeetingDetailPage has no inline editing on AI-generated content.** The briefing sections (context, risks, plan, attendee assessments) are read-only. Meeting prep is the flagship surface — users should be able to correct/refine AI-generated briefing text the same way they edit report slides. The `isEditable` flag and `saveStatus` infrastructure already exist on the page.

4. **EditableText Tauri event `editable-text:commit` has no backend listener.** The event is emitted on every save but nothing receives it. Either wire it to the audit log / signal bus, or remove the dead emission.

5. **Error handling is inconsistent.** Report pages catch save failures with `console.error` only — no user-visible feedback. ActionDetailPage shows folio bar "Saving…/Saved" status but report pages vary (some show it, some don't for error cases). `EditableText.tsx` line 91 catches Tauri emit errors silently.

6. **Save status indicator is missing from entity detail pages.** AccountDetailPage, PersonDetailPage, and ProjectDetailPage save via `update_entity_field` / `update_intelligence_field` but show no "Saving…/Saved" feedback in the folio bar. The user has no confirmation their edit persisted.

## Scope

### A. ActionDetailPage: Replace local EditableText with shared component

Remove the local `EditableText` function (lines 392–458) and replace usage at line 191 with the shared `EditableText` from `@/components/ui/EditableText`. Adapt props: shared component uses `onChange` (not `onSave`), `as="h1"` for display, `multiline={false}` for single-line. Verify: hover hint, terracotta border, Tab navigation, Escape cancel all work.

### B. RiskBriefingPage: Migrate to unified `save_report` command

Replace `invoke("save_risk_briefing", ...)` with `invoke("save_report", { entityId, entityType: "account", reportType: "risk_briefing", contentJson })`. Verify backend `save_report` handler supports the `risk_briefing` report type. If `save_risk_briefing` has special logic beyond `save_report`, consolidate it. Remove dead command if fully replaced.

### C. MeetingDetailPage: Add inline editing to briefing sections + signal bus integration

The briefing content sections (meeting context/summary, attendee assessments, risks, your plan) should use `EditableText` with `multiline={true}`. Edits save to the `prep_frozen_json` field via a new or existing Tauri command (e.g., `update_meeting_prep_field`). Scope:

- Meeting context / executive summary text
- Attendee assessment text (per attendee)
- Risk items text
- "Your plan" / preparation items text

Guard behind `isEditable` flag (already exists). Wire to existing `saveStatus` folio bar indicator.

**Signal bus integration:** Every briefing edit is a user correction to AI-generated intelligence. The existing pattern in `services/intelligence.rs:304-316` (update_intelligence_field) already handles this — `user_correction` signal type with `user_edit` source at confidence 1.0 for text edits, `intelligence_curated` with `user_curation` at 0.5 for removals/dismissals. The same pattern applies here:

| User action | Signal type | Source | Confidence | Entity target |
|---|---|---|---|---|
| Edit briefing text (context, assessment, risk, plan) | `user_correction` | `user_edit` | 1.0 | Meeting's linked entity (account/project) |
| Remove/dismiss a risk or prep item | `intelligence_curated` | `user_curation` | 0.5 | Meeting's linked entity |
| Edit attendee assessment | `user_correction` | `user_edit` | 1.0 | Person entity for that attendee |

Why this matters: `user_correction` signals have 365-day half-life and Tier 1 weight (1.0). Bayesian feedback in `signals/feedback.rs` penalizes the AI source that generated the wrong content (`beta += 1`). When this person or account appears in future meetings, the correction loop improves output. Without signals, edits are cosmetic — they fix the current briefing but teach the system nothing.

The backend command (`update_meeting_prep_field` or equivalent) should:
1. Update `prep_frozen_json` in the `meeting_prep` table
2. Emit `user_correction` or `intelligence_curated` signal via `emit_signal_and_propagate()` targeting the meeting's linked entity
3. Include field path in signal value JSON (e.g., `{"field": "attendee_assessment", "person_id": "..."}`)

**Out of scope:** Editing computed fields (attendee signals, account snapshots, calendar metadata). Those are source data, not curated content.

### D. Save status feedback on entity detail pages

AccountDetailPage, PersonDetailPage, and ProjectDetailPage should show "Saving…" / "Saved" in the folio bar when inline edits (name, vitals, stakeholder fields) persist. Pattern: same `saveStatus` state + `folioStatusText` as report pages and ActionDetailPage.

### E. Error handling standardization

All inline edit save paths should surface failures via toast (`toast.error("Failed to save …")`), not `console.error`. Standardize across:

- Report pages (all 6: AccountHealth, RiskBriefing, EbrQbr, Swot, WeeklyImpact, MonthlyWrapped)
- Entity detail pages (Account, Person, Project)
- ActionDetailPage
- StakeholderGallery
- MeetingDetailPage sections

### F. EditableText Tauri event cleanup

Either:
- Wire `editable-text:commit` to the audit log as an observability event, OR
- Remove the `emit()` call from EditableText.tsx (line ~91) if the event serves no purpose

Decide based on whether audit log tracking of field-level edits is desired for v1.0.0.

## Out of scope

- New editable fields on pages that are intentionally read-only (Dashboard, Inbox, Calendar views)
- Drag-and-drop reordering (already works in EditableList on MePage)
- EditableVitalsStrip changes (already well-implemented on entity detail pages)
- New backend commands beyond what's needed for meeting prep field updates

## Acceptance Criteria

1. ActionDetailPage title uses shared `EditableText` component. Hover hint, terracotta border, Tab navigation, Escape cancel all work. Local `EditableText` function deleted.
2. RiskBriefingPage saves via `save_report` command. `save_risk_briefing` command removed or consolidated. Edits persist and reload correctly.
3. MeetingDetailPage briefing text (context, attendee assessments, risks, plan) is inline-editable when `isEditable` is true. Edits persist to DB and survive page reload.
4. Meeting briefing edits emit `user_correction` signals (text edits) or `intelligence_curated` signals (removals) targeting the meeting's linked entity. Attendee assessment edits target the person entity. Signals visible in signal log with correct source/type/confidence.
5. AccountDetailPage, PersonDetailPage, ProjectDetailPage show "Saving…/Saved" in folio bar on inline edits.
6. All save error paths show `toast.error()` — no silent `console.error` failures.
7. `editable-text:commit` event is either wired to audit log or removed. No dead code.
8. All editable fields follow INTERACTION-PATTERNS.md spec: click to edit, Escape to cancel, blur/Enter to commit, visual edit indicator.
