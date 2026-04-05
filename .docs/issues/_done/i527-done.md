# I527 — Intelligence Consistency Guardrails — Deterministic Contradiction Checks + Repair

**Priority:** P0  
**Area:** Backend / Intelligence + Frontend / UX + Docs  
**Version:** 0.16.1  
**Depends on:** I407, I470, I477

## Summary

Prevent contradictory or cross-entity-bleed statements from persisting in `intelligence.json` by adding a balanced guardrail pipeline:

1. deterministic contradiction detection against local meeting/signal evidence,  
2. deterministic repairs,  
3. one repair retry for unresolved high-severity findings,  
4. visible corrected/flagged transparency in meeting briefing UI.

Also publish an operational scoring + consistency troubleshooting reference.

## Problem

Generated intelligence can include logical contradictions that reduce trust, including:

- claiming a stakeholder "never appeared in a recorded meeting" despite attendance history,
- claiming "no new progress signals" when recent signal activity exists,
- introducing stakeholder names not linked to the entity context.

These contradictions are currently not blocked, corrected, or explicitly surfaced.

## Scope

### 1) Deterministic consistency checker

- Add `src-tauri/src/intelligence/consistency.rs`.
- Add APIs:
  - `build_fact_context(db, entity_id, entity_type) -> FactContext`
  - `check_consistency(intel, fact_context) -> ConsistencyReport`
  - `apply_deterministic_repairs(intel, report, fact_context) -> IntelligenceJson`

`FactContext` includes:
- linked stakeholders (`entity_people` + `people`),
- stakeholder attendance counts and last-seen timestamps (`meeting_attendees` + `meeting_entities` + `meetings_history`),
- recent signal count window for the entity (14 days).

Rules:
- `ABSENCE_CONTRADICTION` (high)
- `NO_PROGRESS_CONTRADICTION` (high)
- `AUTHORITY_UNKNOWN_CONTRADICTION` (medium)
- `CROSS_ENTITY_BLEED_SUSPECT` (medium)

### 2) Intelligence schema metadata (backward-compatible)

- Update `src-tauri/src/intelligence/io.rs`.
- Add optional fields:
  - `consistencyStatus?: "ok" | "corrected" | "flagged"`
  - `consistencyFindings?: [{ code, severity, fieldPath, claimText, evidenceText, autoFixed }]`
  - `consistencyCheckedAt?: string`
- Update frontend mirror types in `src/types/index.ts`.

### 3) Guardrails in enrichment write path (balanced policy)

- Update `src-tauri/src/intel_queue.rs` in `write_enrichment_results(...)`.
- Flow:
  - run deterministic check after user-edit preservation and stakeholder reconciliation,
  - apply deterministic repairs,
  - if unresolved high findings remain, run exactly one repair retry,
  - if still unresolved, persist with `consistencyStatus = "flagged"` and findings.
- Writes continue (no hard block), refresh pipeline remains overwriteable on future runs.

### 4) Prompt grounding improvements

- Update `src-tauri/src/intelligence/prompts.rs`.
- Add compact "Verified Stakeholder Meeting Presence" block.
- Add explicit rule prohibiting "never attended/appeared" when verified attendance exists.
- Keep prompt budget bounded (top stakeholders by recent presence).

### 5) UI trust transparency

- Update `src/pages/MeetingDetailPage.tsx`.
- Show consistency banner when status is not `ok`:
  - `corrected`: "Context auto-corrected against meeting records."
  - `flagged`: "Some context could not be fully verified from meeting records."
- Copy must remain ADR-0083 compliant.

### 6) Scoring and consistency troubleshooting reference

- Add `.docs/design/INTELLIGENCE-CONSISTENCY-REFERENCE.md` with:
  - scoring/context-selection flow,
  - current thresholds used by context retrieval and linking,
  - consistency rule catalog + severities,
  - diagnostic SQL for bleed/contradiction triage,
  - Janus/Matt Wickham worked example.

## Acceptance Criteria

1. A false "never attended/appeared" claim cannot persist when deterministic attendance evidence exists.
2. Contradictions are either auto-corrected or explicitly flagged, never silently accepted.
3. At most one repair retry is attempted for unresolved high-severity contradictions.
4. Corrected/flagged output never blocks future refreshes from overwriting with better output.
5. Meeting briefing UI surfaces corrected/flagged status with ADR-0083-compliant language.
6. Scoring/consistency reference doc is published and linked from architecture docs.
7. Real Janus scenario: if Matt Wickham has attendance evidence, refreshed intelligence must not claim he has never appeared.

## Test Plan

1. Unit tests:
- contradiction detection for absence/no-progress/bleed,
- deterministic repairs rewrite contradictory claims,
- serialization/deserialization of new optional fields remains backward-compatible.

2. Integration tests:
- seed attendance + signal facts for an entity,
- inject contradictory model output,
- verify persisted intelligence is corrected or flagged per balanced policy,
- verify only one retry attempt on unresolved high findings.

3. UI checks:
- corrected banner renders for `consistencyStatus="corrected"`,
- flagged banner renders for `consistencyStatus="flagged"`,
- no banner for `ok`.

4. Performance checks:
- negligible overhead in no-contradiction path,
- retry path only on unresolved high findings,
- no added synchronous blocking on UI navigation.

## Notes

- Policy is explicitly **Balanced**: auto-correct + one retry + flagged fallback.
- All schema changes are optional fields and backward-safe.
