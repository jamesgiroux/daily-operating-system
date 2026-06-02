# v1.5.0 W1-B Channel Inventory

**Wave:** W1 (Account composition reference)
**Scope:** W1-B sectioned projection + provenance-resolution contract
**Status:** L1 artifact for signal policy registry lint

This inventory names every signal-facing channel that W1 relies on while adding
sectioned composition projection for the Account surface. W1 does not add a new
signal table writer. Signal persistence stays behind `signals::bus` and
`services::signals`; render refresh remains driven by existing composition
version/cache state and the producer-owned `AbilityOutputChanged` policy for
`dailyos/account-overview`.

## Policy Boundary

- `src-tauri/src/signals/bus.rs` remains the only production writer to
  `signal_events`.
- `src-tauri/src/signals/policy_registry.rs` remains the policy registry for
  typed signal routing. The registry exposes `policy_for(signal: &SignalType)`.
- W1 Account projection reads from ability output, sourced account fields, and
  provenance metadata. It does not emit a new W1-specific signal type.
- The new Tauri projection command is a render/read bridge. It must not write
  `signal_events` directly.

## Required Channels

| Channel | W1-B role | Signal policy stance |
| --- | --- | --- |
| `src-tauri/src/services` | Owns command/shared-service orchestration, account snapshot reads, cache lookup/store, and existing signal helpers. | Services call `signals::bus`/`services::signals`; no direct `signal_events` writes. |
| `src-tauri/src/abilities` | Tauri command module exposes `get_projected_composition` for the first-party app. | Read/render bridge only; producer output changes stay ability-routed. |
| `src-tauri/src/bridges/tauri.rs` | First-party app bridge identity for render policy and correction surface selection. | Bridge passes surface identity; no signal writes. |
| `src-tauri/src/bridges/mcp.rs` | MCP bridge remains a separate surface/client identity for existing composition routes. | Existing MCP routing remains policy-gated; W1 does not broaden it. |
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

## W1-B Checks

- Sectioned projection adds safe section metadata to `ProjectedComposition`.
- Unknown/custom block payloads still pass through fallback projection before
  React can render them.
- Fallback projection carries visible trust/fallback metadata without exposing
  private diagnostics.
- Account composition rendering consumes the projected sections and blocks; it
  does not create a parallel frontend dossier model.
- Any future W1 signal-channel change must update this inventory and the signal
  policy registry lint in the same PR.
