# I403 — SignalService Formalization — Formalize signals/ Module Boundary as a Service

**Status:** Open (deferred from v0.13.6)
**Priority:** P2
**Version:** 0.13.8
**Area:** Code Quality / Refactor

## Summary

SERVICE-CONTRACTS.md Phase 2 item 6. The `signals/` module is already well-modularized (bus, propagation, rules, fusion, decay, callouts, invalidation, etc.) but has no formal service boundary. Call sites reach directly into `bus::emit_signal_and_propagate()`, `propagation::PropagationEngine`, etc. A `SignalService` wrapper would provide a clean public API that:

- Encapsulates the emission + propagation pattern (callers don't need to pass `&PropagationEngine` explicitly)
- Provides a single entry point for signal operations
- Makes the signal system testable as a unit

This is lower priority than I402 since the signals/ module already has good internal structure. The formalization is about API ergonomics, not structural problems.

## Acceptance Criteria

1. `services/signals.rs` (or a `signals/service.rs` within the existing module) exists with public methods per the SignalService contract in SERVICE-CONTRACTS.md:
   - `emit(entity_id, signal_type, confidence, source, payload)` — wraps emit_signal_and_propagate with engine from AppState
   - `get_for_entity(entity_id) → Vec<SignalEvent>`
   - `get_callouts(entity_id) → Vec<Callout>`
   - `run_propagation(entity_id)` — fire cross-entity rules
   - `invalidate_preps(entity_id)` — queue affected meeting preps
2. Service methods in actions.rs, accounts.rs, people.rs, meetings.rs call the SignalService instead of `bus::emit_signal_and_propagate()` directly. This removes the need to pass `&PropagationEngine` or `&AppState` just for signal emission.
3. `cargo test` passes. `cargo clippy -- -D warnings` passes.

## Dependencies

- Should be done after I402 (IntelligenceService extraction) to avoid merge conflicts in commands.rs.
- No dependency on I401.

## Deferral History

Deferred from v0.13.6 — cross-cutting refactor of 27 call sites across 7 files with no commands.rs line reduction. Better as a standalone PR. Currently all services (accounts.rs, people.rs, meetings.rs, intelligence.rs, etc.) call `bus::emit_signal_and_propagate()` directly and pass `&state.signal_engine` manually.

v0.13.7 (self_healing/) follows the same direct-call pattern as all other services. When SignalService eventually ships, self_healing/ migrates along with everything else in a single cross-cutting refactor.

## Notes

The v0.13.2 signal chain audit (I377) confirmed the signal system is structurally sound: 30 emitters, 5 active propagation rules, 9 downstream consumers. This issue is about API formalization, not fixing broken wiring.
