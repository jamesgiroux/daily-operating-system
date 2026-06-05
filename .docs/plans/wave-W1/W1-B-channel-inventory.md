# W1-B Signal Policy Channel Inventory

**Current consumers:** v1.4.9 W1 / DOS-758 request-scoped MCP sidecar; v1.5.0 W1 account composition reference
**Status:** L1 artifact for signal policy registry lint

This inventory names every signal-facing channel that W1 relies on while keeping the core invariant explicit: production signal rows must route through `src-tauri/src/signals/policy_registry.rs` via the consolidated `emit_signal` family in `src-tauri/src/signals/bus.rs`. W1 Account projection and W1 MCP request-context work do not add a new signal table writer.

## Policy Boundary

- `src-tauri/src/signals/bus.rs` remains the only production writer to `signal_events`.
- `src-tauri/src/signals/policy_registry.rs` remains the policy registry for typed signal routing. The registry exposes `policy_for(signal: &SignalType)`.
- W1 Account projection reads from ability output, sourced account fields, and provenance metadata. It does not emit a new W1-specific signal type.
- The Tauri projection command is a render/read bridge. It must not write `signal_events` directly.
- MCP sidecar tools invoke abilities/services through request-scoped context. Any correctness signal must still route through the service facade or the registry-backed bus.

## Required Channels

| Channel | W1-B role | Signal policy stance |
| --- | --- | --- |
| `src-tauri/src/services` | Owns command/shared-service orchestration, account snapshot reads, cache lookup/store, MCP handler helpers, and existing signal helpers. | Services call `signals::bus`/`services::signals`; no direct `signal_events` writes. |
| `src-tauri/src/abilities` | Ability descriptors and Tauri ability command surfaces. | Read/render bridge only; producer output changes stay ability-routed. |
| `src-tauri/src/bridges/tauri.rs` | First-party app bridge identity for render policy and correction surface selection. | Bridge passes surface identity; no signal writes. |
| `src-tauri/src/bridges/mcp.rs` | MCP bridge remains a separate surface/client identity for existing composition and tool routes. | Existing MCP routing remains policy-gated; W1 does not broaden it. |
| `src-tauri/src/bridges/worker.rs` | Worker bridge remains available for background ability execution. | Worker-triggered signals must route through service/bus APIs. |
| `src-tauri/src/bridges/eval.rs` | Eval bridge remains a distinct actor/surface for evaluation paths. | Eval does not bypass signal policy. |
| `src-tauri/src/signals/derived_state_subscribers.rs` | Registry of derived-state rebuild subscribers after signals are emitted. | Subscriber-only; no direct `signal_events` writes. |
| `src-tauri/src/signals/event_trigger.rs` | Background trigger entrypoint for existing signal-driven work. | Trigger-originated events must use service/bus APIs. |
| `src-tauri/src/devtools/mod.rs` | Dev/test fixture surfaces can seed or inspect signal rows through fixture helpers. | Fixture-only helpers remain isolated from production direct writes. |
| `src-tauri/src/migrations.rs` | Owns schema and migration-time data repair for `signal_events`. | Lifecycle override only; production writes stay behind bus. |

## Binary Entrypoints

| Binary | W1-B signal stance |
| --- | --- |
| `src-tauri/src/bin/generate_canonicalization_corpus.rs` | Offline corpus generation; must not introduce production signal writes. |
| `src-tauri/src/bin/reconcile_post_migration.rs` | Migration follow-up utility; signal lifecycle changes must stay explicit and reviewed. |
| `src-tauri/src/bin/release_gate.rs` | Release verification only; no signal writes. |
| `src-tauri/src/bin/repair_entity_linking.rs` | Repair utility; any signal effects must route through service/bus APIs. |
| `src-tauri/src/bin/workspace_backfill.rs` | Backfill utility; emits or records effects only through approved lifecycle/service paths. |
| `src-tauri/src/bin/workspace_graph_audit.rs` | Audit-only utility; no signal writes. |

## Extended Emission Channel Map

| Channel family | Current entrypoints / file glob | Expected policy path | Allowed exclusions | Drift guard |
|---|---|---|---|---|
| Services facade | `src-tauri/src/services/signals.rs`; service callers under `src-tauri/src/services/**/*.rs` | `services::signals::*` gates mode/capability and delegates to `signals::bus::emit_signal*`, which resolves `policy_registry::policy_for` before any row write. | Read-only `SELECT ... FROM signal_events`; test modules seeding fixtures. | Existing `src-tauri/tests/dos209_regression.rs` blocks raw bus emission in `services/`; W1-B single-writer lint blocks direct production `INSERT` / `UPDATE` to `signal_events`. |
| Infrastructure services and processors | `src-tauri/src/intel_queue.rs`, `src-tauri/src/processor/**/*.rs`, `src-tauri/src/prepare/**/*.rs`, `src-tauri/src/context_provider/**/*.rs`, `src-tauri/src/intelligence/**/*.rs`, `src-tauri/src/proactive/**/*.rs`, `src-tauri/src/workflow/**/*.rs` | Direct infrastructure callers may call the bus, but every bus entrypoint routes through `policy_registry.rs`. Signal-derived background invalidation remains behind bus propagation hooks. | Telemetry `app.emit(...)`; test modules; read-only queries. | W1-B single-writer lint rejects direct production event-log writes outside the bus and policy registry. |
| Connector/background processors | `src-tauri/src/google.rs`, `src-tauri/src/executor.rs`, `src-tauri/src/clay/**/*.rs`, `src-tauri/src/gravatar/**/*.rs`, `src-tauri/src/linear/**/*.rs`, transcript/file processors under `src-tauri/src/granola`, `src-tauri/src/quill`, `src-tauri/src/processor` | Connector-originated correctness signals use `services::signals::*` or the bus; both traverse the registry. | UI refresh events such as `emails-updated`, auth status events, and operation progress are Tauri telemetry, not bus signals. | W1-B lint covers direct `signal_events` writes; inventory drift test requires these globs to stay listed. |
| Abilities runtime | `src-tauri/src/abilities/**/*.rs`; ability descriptors in `src-tauri/src/abilities/registry.rs` | Abilities currently declare `signal_policy` metadata only. Future output-change emission must go through `AbilityContext` / `ServiceContext` to `services::signals::*` and then `policy_registry.rs`. | Read abilities with `emits_on_output_change = []`; evaluation fixtures with no DB writer. | Ability surface tests continue to check descriptor metadata; W1-B inventory drift test pins the `abilities/` channel. |
| Tauri bridge | `src-tauri/src/bridges/tauri.rs`; command surfaces under `src-tauri/src/commands/**/*.rs` | Commands call services; services emit through the facade and registry. Bridge descriptors may declare ability signal policy but do not write `signal_events` directly. | `app.emit(...)` UI events are telemetry/progress and cannot enqueue invalidation jobs. | W1-B inventory drift test pins `bridges/tauri.rs` and `commands/`; single-writer lint catches event-log SQL. |
| MCP bridge | `src-tauri/src/bridges/mcp.rs`; `src-tauri/src/mcp/**/*.rs` | MCP invokes abilities/services. Signal declarations remain descriptor metadata until a service emits through the registry-backed bus. | Static MCP schema fixtures and descriptor tests; no raw event-log writer. | W1-B inventory drift test pins bridge and MCP globs; single-writer lint covers SQL writes. |
| Worker bridge | `src-tauri/src/bridges/worker.rs`; worker loops in `src-tauri/src/intel_queue.rs`, `src-tauri/src/meeting_prep_queue.rs`, schedulers under `src-tauri/src/scheduler.rs` and `src-tauri/src/hygiene/**/*.rs` | Worker-originated correctness signals use service facade or bus, then registry. Queue progress events are not bus signals. | Queue enqueue/dequeue state, leases, and UI progress events; read-only signal queries. | W1-B single-writer lint plus inventory drift test. |
| Eval bridge and replay harness | `src-tauri/src/bridges/eval.rs`, `src-tauri/src/harness/**/*.rs`, `src-tauri/src/services/external_replay/**/*.rs` | Evaluate/simulate paths may declare intended signals, but side-effecting DB emission must be blocked by `ServiceContext` or captured by explicit fixture helpers before any Live flush. | Hermetic replay data, fixture reads, and no-op descriptor defaults. | Existing external replay lint plus W1-B inventory drift test. |
| Trigger-derived state and subscribers | `src-tauri/src/signals/derived_state_subscribers.rs`, `src-tauri/src/signals/event_trigger.rs`, `src-tauri/src/services/derived_state.rs`, `src-tauri/src/services/entity_linking/**/*.rs` | Subscribers consume registry-backed emissions and write owned derived state through services. They may not insert `signal_events` directly. | Subscriber registry declarations and read-side queries. | W1-B single-writer lint catches direct event-log writes from subscribers or derived-state services. |
| Signal propagation rules | `src-tauri/src/signals/propagation.rs`, `src-tauri/src/signals/rules.rs`, `src-tauri/src/signals/invalidation.rs` | Source and derived signals are appended by the `emit_signal` family only; propagation rules return `DerivedSignal` data and the bus handles registry policy. | Unit-test fixture inserts in `#[cfg(test)]` modules. | W1-B single-writer lint allows production writes only in the bus emit family. |
| Replay / operational binaries | `src-tauri/src/bin/generate_canonicalization_corpus.rs`, `src-tauri/src/bin/reconcile_post_migration.rs`, `src-tauri/src/bin/release_gate.rs`, `src-tauri/src/bin/repair_entity_linking.rs`, `src-tauri/src/bin/workspace_backfill.rs`, `src-tauri/src/bin/workspace_graph_audit.rs` | Current binaries do not emit bus signals. Any future production-like binary must call the registry-backed bus; one-shot migration overrides must be explicit and inventoried here. | `release_gate` writes only gate artifacts; reconcile/repair/workspace audit binaries read or repair through services. | W1-B inventory drift test enumerates every file under `src-tauri/src/bin`. |
| Dev mock fixture seeding | `src-tauri/src/devtools/mod.rs` | Mock `signal_events` rows use a registry-backed bus fixture helper so seeded rows still traverse policy lookup. | Debug/dev guard must remain in place; this is not a production mutation path. | W1-B single-writer lint permits only the bus helper to write event rows. |
| Migrations and schema checks | `src-tauri/src/migrations.rs`, `src-tauri/src/migrations/**/*.sql` | Schema creation and migration verification may seed tables directly as part of migration tests, not runtime emission. | Migration/test-only direct `INSERT` accepted. | W1-B single-writer lint allowlists migrations. |
| Telemetry/progress sinks | `app.emit(...)` calls across commands, queues, reports, risk briefing, Google sync, watcher, executor, and intelligence services | Non-bus, lossy UI/process telemetry. These events must never be consumed for claim correctness or invalidation jobs. | All Tauri event names such as `emails-updated`, `workflow-status`, `transcript-progress`, `operation-delivered`. | Inventory drift test pins telemetry as excluded; single-writer lint focuses on SQL event-log writes. |
| Privacy/purge/lifecycle cleanup | `src-tauri/src/privacy.rs`, `src-tauri/src/db/data_lifecycle.rs`, `src-tauri/src/db/accounts.rs` | Cleanup and merge paths may delete or reassign historical rows; they do not emit new signals. New correctness notifications caused by these paths must use services/bus. | Direct `DELETE FROM signal_events` and merge reassignment are data lifecycle operations, not emissions. | W1-B lint rejects direct `INSERT` / `UPDATE signal_events SET ...` except documented lifecycle reassignment and bus-owned supersede. |

## W1-B Checks

- Sectioned projection adds safe section metadata to `ProjectedComposition`.
- Unknown/custom block payloads still pass through fallback projection before React can render them.
- Fallback projection carries visible trust/fallback metadata without exposing private diagnostics.
- Account composition rendering consumes the projected sections and blocks; it does not create a parallel frontend dossier model.
- Request-scoped MCP handler work does not add handler-local DB/keychain opens and does not create a parallel sidecar signal path.
- Any future W1 signal-channel change must update this inventory and the signal policy registry lint in the same PR.
