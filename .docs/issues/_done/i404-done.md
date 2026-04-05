# I404 — AppState Decomposition Phase 1 — Core Domain Containers

**Status:** Open (0.13.8)
**Priority:** P1
**Version:** 0.13.8
**Area:** Code Quality / Refactor

## Summary

`AppState` has 28 fields in a flat struct. Every subsystem reaches into the same monolith for its dependencies — the scheduler reads `workflow_status`, the capture loop reads `capture_dismissed`, the hygiene scanner reads `hygiene_budget`, but they all go through the same `Arc<AppState>`. This makes it impossible to reason about what a subsystem actually needs.

Phase 1 groups the core domain fields into containers. AppState becomes a facade that holds domain-specific sub-structs.

## Target Structure

```rust
pub struct AppState {
    pub config: RwLock<Option<Config>>,
    pub db: Mutex<Option<ActionDb>>,
    pub workflow: WorkflowState,
    pub calendar: CalendarState,
    pub capture: CaptureState,
    pub hygiene: HygieneState,
    // Remaining fields stay flat until Phase 2
    pub intel_queue: Arc<IntelligenceQueue>,
    pub embedding_model: Arc<EmbeddingModel>,
    pub embedding_queue: Arc<EmbeddingQueue>,
    pub signal_engine: Arc<PropagationEngine>,
    pub meeting_prep_queue: Arc<MeetingPrepQueue>,
    pub prep_invalidation_queue: Arc<Mutex<Vec<String>>>,
    pub entity_resolution_wake: Arc<Notify>,
    pub active_preset: RwLock<Option<RolePreset>>,
    pub pre_dev_workspace: Mutex<Option<String>>,
    // Integration wakes (Phase 2)
    pub clay_poller_wake: Arc<Notify>,
    pub quill_poller_wake: Arc<Notify>,
    pub linear_poller_wake: Arc<Notify>,
    pub email_poller_wake: Arc<Notify>,
}

pub struct WorkflowState {
    pub status: RwLock<HashMap<WorkflowId, WorkflowStatus>>,
    pub history: Mutex<Vec<ExecutionRecord>>,
    pub last_scheduled_run: RwLock<HashMap<WorkflowId, DateTime<Utc>>>,
}

pub struct CalendarState {
    pub events: RwLock<Vec<CalendarEvent>>,
    pub week_cache: RwLock<Option<(Vec<CalendarEvent>, Instant)>>,
    pub google_auth: Mutex<GoogleAuthStatus>,
}

pub struct CaptureState {
    pub dismissed: Mutex<HashSet<String>>,
    pub captured: Mutex<HashSet<String>>,
    pub transcript_processed: Mutex<HashMap<String, TranscriptRecord>>,
}

pub struct HygieneState {
    pub report: Mutex<Option<HygieneReport>>,
    pub scan_running: AtomicBool,
    pub last_scan_at: Mutex<Option<String>>,
    pub next_scan_at: Mutex<Option<String>>,
    pub budget: HygieneBudget,
    pub full_orphan_scan_done: AtomicBool,
}
```

## Acceptance Criteria

1. `WorkflowState`, `CalendarState`, `CaptureState`, `HygieneState` sub-structs exist in `state.rs`.
2. `AppState` holds these as fields: `state.workflow`, `state.calendar`, `state.capture`, `state.hygiene`.
3. All call sites updated: `state.workflow_status` → `state.workflow.status`, `state.calendar_events` → `state.calendar.events`, etc.
4. No logic changes. No behavior changes. Purely mechanical find-and-replace.
5. `cargo test` passes. `cargo clippy -- -D warnings` passes.
6. IPC surface unchanged — no frontend changes.

## Dependencies

- Should be done after v0.13.6 (I402/I403) to avoid merge conflicts in commands.rs and services/.
- I405 builds on this.

## Notes

This is mechanical refactoring with a wide blast radius (every file that accesses these fields needs updating). The recommended approach: extract one container at a time, test after each. Start with `HygieneState` (fewest call sites), then `CaptureState`, then `CalendarState`, then `WorkflowState`.

Estimated call sites per container:
- HygieneState: ~15 call sites (hygiene.rs, commands.rs, scheduler.rs)
- CaptureState: ~20 call sites (capture.rs, commands.rs, post_meeting.rs)
- CalendarState: ~25 call sites (google.rs, commands.rs, proactive/)
- WorkflowState: ~30 call sites (executor.rs, scheduler.rs, commands.rs, workflow/)
