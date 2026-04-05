# I402 — IntelligenceService Extraction — Move Intelligence/Enrichment Business Logic to services/intelligence.rs

**Status:** Done (v0.13.6, partial contract)
**Priority:** P1
**Version:** 0.13.6
**Area:** Code Quality / Refactor

## Summary

SERVICE-CONTRACTS.md Phase 2 item 5. `commands.rs` still contains ~15 intelligence/enrichment command handlers with thick business logic: entity enrichment orchestration (enrich_account, enrich_person, enrich_project), intelligence field updates, stakeholder edits, executive intelligence assembly, risk briefing generation, hygiene status. These should move to `services/intelligence.rs` following the same pattern established in v0.13.2 Phase 1 (actions, accounts, people, meetings).

This extraction is prep work for v0.14.0 (CS Reports), which will add report generation that consumes entity intelligence — a clean IntelligenceService boundary makes that work straightforward.

## Acceptance Criteria

1. `services/intelligence.rs` exists with public methods per the IntelligenceService contract in SERVICE-CONTRACTS.md:
   - `get_executive() → ExecutiveIntelligence`
   - `enrich_account(id)`, `enrich_person(id)`, `enrich_project(id)` — queue AI enrichment via intel_queue
   - `update_field(entity_id, field, value)` — user edits with signal emission
   - `update_stakeholders(entity_id, stakeholders)` — stakeholder edits with signal emission
   - `create_person_from_stakeholder(stakeholder) → Person`
   - `get_hygiene_status() → IntelligenceHygieneStatus`
   - `generate_risk_briefing()`, `get_risk_briefing()`, `save_risk_briefing()`
2. All intelligence command handlers in `commands.rs` are thin wrappers (parse args → call service → return).
3. Signal emissions in extracted methods use `emit_signal_and_propagate()` (not bare `emit_signal()`).
4. `commands.rs` line count further reduced. Target: measure before and after.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes. IPC surface unchanged.

## Dependencies

- Builds on v0.13.2 Phase 1 extraction (I380).
- Informed by SERVICE-CONTRACTS.md IntelligenceService contract.
- No dependency on I401 or I403.

## Resolution

Shipped in v0.13.6 with 6 of 9 contract methods:
- `enrich_entity(entity_id, entity_type, state)` — unified enrichment (replaces separate enrich_account/person/project)
- `update_intelligence_field(entity_id, entity_type, field_path, value, state)`
- `update_stakeholders(entity_id, entity_type, stakeholders, state)`
- `generate_risk_briefing(state, account_id)`
- `get_risk_briefing(db, state, account_id)`
- `save_risk_briefing(db, state, account_id, briefing)`

**3 methods deferred** (not blocking any downstream work):
- `get_executive()` — executive intelligence assembly (still in commands.rs)
- `create_person_from_stakeholder()` — person creation from stakeholder card (still in commands.rs)
- `get_hygiene_status()` — hygiene scan results (still in commands.rs)

These can be extracted incrementally; none are on the critical path for v0.13.7 self-healing.

## Notes

The `update_stakeholders` handler already had its signal emission changed to `emit_signal_and_propagate` in v0.13.2, but the business logic still lived in `commands.rs`. This extraction moved it to the service layer.
