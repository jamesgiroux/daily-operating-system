# I405 — AppState Decomposition Phase 2 — Integration and Signal Containers

**Status:** Open (0.13.8)
**Priority:** P2
**Version:** 0.13.8
**Area:** Code Quality / Refactor

## Summary

Continues AppState decomposition from I404. Groups the remaining flat fields into `IntegrationState` (poller wake signals) and `SignalState` (signal engine, entity resolution, prep invalidation). After both phases, AppState is a clean facade with domain containers — no flat fields except `config`, `db`, and core queues.

## Target Structure

After I404 + I405, the final AppState:

```rust
pub struct AppState {
    pub config: RwLock<Option<Config>>,
    pub db: Mutex<Option<ActionDb>>,
    pub workflow: WorkflowState,
    pub calendar: CalendarState,
    pub capture: CaptureState,
    pub hygiene: HygieneState,
    pub signals: SignalState,
    pub integrations: IntegrationState,
    pub intel_queue: Arc<IntelligenceQueue>,
    pub embedding_model: Arc<EmbeddingModel>,
    pub embedding_queue: Arc<EmbeddingQueue>,
    pub meeting_prep_queue: Arc<MeetingPrepQueue>,
    pub active_preset: RwLock<Option<RolePreset>>,
    pub pre_dev_workspace: Mutex<Option<String>>,
}

pub struct SignalState {
    pub engine: Arc<PropagationEngine>,
    pub entity_resolution_wake: Arc<Notify>,
    pub prep_invalidation_queue: Arc<Mutex<Vec<String>>>,
}

pub struct IntegrationState {
    pub clay_poller_wake: Arc<Notify>,
    pub quill_poller_wake: Arc<Notify>,
    pub linear_poller_wake: Arc<Notify>,
    pub email_poller_wake: Arc<Notify>,
}
```

## Acceptance Criteria

1. `SignalState` and `IntegrationState` sub-structs exist in `state.rs`.
2. `AppState` holds these as `state.signals` and `state.integrations`.
3. All call sites updated: `state.signal_engine` → `state.signals.engine`, `state.clay_poller_wake` → `state.integrations.clay_poller_wake`, etc.
4. If I403 (SignalService) was completed in v0.13.6, the SignalService wraps `state.signals` internally — callers don't need to know the container path.
5. No logic changes. No behavior changes.
6. `cargo test` passes. `cargo clippy -- -D warnings` passes.
7. IPC surface unchanged.
8. After I404 + I405: AppState has ≤15 top-level fields (down from 28).

## Dependencies

- Blocked by I404 (Phase 1 must complete first).
- Benefits from I403 (SignalService) — if SignalService exists, `state.signals.engine` is only accessed internally by the service, reducing the call site blast radius.

## Notes

Smaller blast radius than I404 since integration wake signals are only accessed in their respective pollers (clay/, quill/, linear/, google.rs) and signal fields are accessed in signals/ and services/. Estimated ~20 total call sites.
