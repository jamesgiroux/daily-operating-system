# ADR-0111: Surface-Independent Ability Invocation

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0027](0027-mcp-dual-mode.md)  
**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.

This ADR specifies the exact binding pattern for Tauri commands and MCP tools, how abilities are generated into both surfaces from the registry, and how surfaces construct `AbilityContext` with mode, actor, and provenance renderer wiring.

## Decision

### 1. Surface Construction Rule

Each surface holds exclusive responsibility for constructing `AbilityContext` for its invocations. Abilities never construct their own context; composed abilities inherit their caller's context. The surfaces are:

| Surface | Actor | Default Mode | Context constructed by |
|---------|-------|--------------|------------------------|
| Tauri app | `User` | `Live` (or `Live + dry_run` for maintenance) | `TauriAbilityBridge` |
| MCP server | `Agent` | `Live` (never escalated) | `McpAbilityBridge` |
| Background worker | `System` | `Live` | `WorkerAbilityBridge` |
| Evaluation harness | `System` | `Evaluate` | `EvalAbilityBridge` per [ADR-0110](0110-evaluation-harness-for-abilities.md) |
| Integration tests | declared | `Simulate` or `Live` (test DB) | `TestAbilityBridge` per test |

Each bridge owns construction logic but invokes abilities through the shared registry. There is one invocation entry point per bridge; no ability-specific code in the bridge.

### 2. Tauri Command Binding

v1.4.0 ships a single generic Tauri command, `invoke_ability`, that dispatches by name:

```rust
#[tauri::command]
pub async fn invoke_ability(
    state: tauri::State<'_, AppState>,
    ability_name: String,
    input_json: serde_json::Value,
    dry_run: Option<bool>,
    confirmation: Option<ConfirmationToken>,
) -> Result<AbilityResponseJson, String> {
    let bridge = state.tauri_bridge();
    bridge.invoke(
        &ability_name,
        input_json,
        InvocationContext {
            actor: Actor::User,
            mode: ExecutionMode::Live,
            dry_run: dry_run.unwrap_or(false),
            confirmation,
        },
    ).await
}
```

The bridge:

1. Looks up the ability descriptor by name in the registry.
2. Validates `input_json` against the ability's input JSON schema.
3. Checks actor policy (`User` is allowed).
4. Constructs `AbilityContext` with Live mode, User actor, and the app's production services.
5. Invokes the ability through the registry's erased wrapper.
6. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, User, TauriApp)`.
7. Returns `AbilityResponseJson` containing domain output plus rendered provenance.

**Per-ability typed Tauri commands** are not generated in v1.4.0. The single `invoke_ability` command is ergonomic enough for the app's internal call sites (which can define thin TypeScript wrappers) and avoids generating ~20 commands per ability migration. Per-ability commands may be added later if type ergonomics become a pain point.

### 3. MCP Tool Registration

The MCP server iterates the registry at startup and registers one MCP tool per ability whose `AbilityPolicy.allowed_actors` includes `Agent`:

```rust
for descriptor in AbilityRegistry::iter().filter(|d| d.policy.allowed_actors.contains(&Actor::Agent)) {
    mcp_server.register_tool(McpTool {
        name: descriptor.name,
        description: descriptor.description,
        input_schema: descriptor.input_schema.clone(),
        handler: Box::new(McpAbilityHandler::new(descriptor.clone())),
    });
}
```

Each `McpAbilityHandler`:

1. Receives the agent's invocation with JSON inputs.
2. Validates inputs against the ability's schema.
3. Checks actor policy (`Agent` is allowed).
4. Refuses invocation if `allowed_modes` does not include `Live` with no additional surface (agents do not escalate to `Evaluate` or `Simulate`).
5. Constructs `AbilityContext` with Live mode, Agent actor, production services.
6. Invokes the ability through the registry.
7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, Agent, McpTool)`.
8. Returns the MCP response with domain output and filtered provenance.

**No hand-written MCP tool handlers.** Every MCP tool is an ability. Pre-existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) that do not correspond to an ability are either migrated to an ability or removed during Phase 3 cutover.

### 4. Actor-Filtered Discovery

MCP tool discovery (the `list_tools` endpoint) is filtered by actor. An agent enumerating the server's tools sees only abilities whose policy permits `Agent`. Maintenance abilities, admin-only abilities, and abilities that specifically exclude agents never appear in the list.

The Tauri bridge has a complementary `list_abilities(actor: Actor)` endpoint for the app's UI when it needs to enumerate capabilities (e.g., the Maintenance settings panel lists maintenance abilities; a developer panel lists all).

### 5. Background Worker Binding

The scheduled maintenance worker uses its own bridge:

```rust
impl WorkerAbilityBridge {
    pub async fn run_scheduled(&self, schedule: &MaintenanceSchedule) {
        for entry in schedule.due_now() {
            let descriptor = self.registry.get(entry.ability_name).expect("scheduled ability must exist");
            let ctx = AbilityContext::new(
                &self.services,
                &self.intelligence,
                &self.user,
                &self.tracer,
                Actor::System,
                None,  // No confirmation needed for scheduled System invocations
            );
            let _ = self.registry.invoke_maintenance(descriptor, entry.input_json, &ctx).await;
        }
    }
}
```

Scheduled maintenance runs in `Live` mode with `Actor::System`. The worker is the only code path that invokes `global` blast radius maintenance without user confirmation (per [ADR-0103](0103-maintenance-ability-safety-constraints.md) §7).

### 6. Evaluation Harness Binding

[ADR-0110](0110-evaluation-harness-for-abilities.md) defines the harness; this ADR specifies how it binds into the same registry:

```rust
impl EvalAbilityBridge {
    pub async fn run_fixture(&self, ability_name: &str, fixture: &EvalFixture) -> EvalRunResult {
        let services = ServiceContext::new_evaluate(
            &fixture.db, &fixture.signals, &fixture.intel_queue, fixture,
        );
        let intelligence = fixture.replay_provider();
        let ctx = AbilityContext::new(
            &services, &intelligence, &fixture.user_entity, &fixture.tracer,
            Actor::System,
            None,
        );
        self.registry.invoke_by_name(ability_name, fixture.inputs.clone(), &ctx).await
    }
}
```

### 7. Phased Cutover

**Phase 2 (v1.4.0 migration):**

- `invoke_ability` Tauri command added alongside existing hand-written commands.
- MCP server registers ability-derived tools alongside existing hand-written tools (both paths active).
- App and MCP progressively switch from hand-written to registry-backed.

**Phase 3 (v1.4.0 cutover):**

- Hand-written Tauri commands for capability-level operations removed; only mechanical-read commands remain.
- Hand-written MCP tools removed; all MCP tools come from the registry.
- Compile-time enforcement: the registry is the only pub-visible invocation path from surfaces.

**Phase 4 (post-v1.4.0):**

- Per-ability typed Tauri commands may be generated if ergonomics warrant.
- Module-visibility constraints prevent direct ability function imports from surface code.

## Consequences

### Positive

1. **Surface parity is structural.** App, MCP, worker, eval harness all invoke abilities through the same registry path; output quality is identical across surfaces.
2. **MCP tool registration is automatic.** Adding a new Agent-accessible ability adds a new MCP tool with no hand-written code.
3. **Actor-filtered discovery.** Agents only see what they are permitted to invoke.
4. **Generic `invoke_ability` Tauri command keeps surface thin.** No per-ability Rust code generation required in v1.4.0.
5. **Worker and harness use the same pattern.** One invocation contract; different surface bridges.
6. **Provenance rendering integrated into bridges.** Every response carries appropriately-rendered provenance per [ADR-0108](0108-provenance-rendering-and-privacy.md).

### Negative

1. **Generic Tauri command loses TypeScript ergonomics.** Frontend callers work with untyped JSON plus a name string; a TS wrapper layer around `invoke_ability` is needed for type safety.
2. **MCP migration is non-trivial.** Existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) must be refactored into abilities or retired.
3. **Bridge code is mostly boilerplate.** Four bridges (Tauri, MCP, worker, eval) with similar shape. Potential to factor but not in v1.4.0.

### Risks

1. **Hand-written MCP tool drift.** A contributor adds a hand-written MCP tool in Phase 2 that never gets migrated. Mitigation: Phase 3 gate requires zero hand-written MCP tools; CI enforces.
2. **Tauri command name collisions.** Two abilities with the same name would collide. Mitigation: registry enforces unique names at registration time.
3. **Confirmation token misuse in MCP.** Agents attempting to supply confirmation tokens via MCP. Mitigation: MCP bridge refuses `confirmation` field; tokens are user-initiated via Tauri only.
4. **Action latency growth.** Generic dispatch + schema validation + provenance rendering adds ~1–5ms per invocation. Mitigation: negligible for synthesis-bound abilities; measurable only for trivial reads.
5. **List-tools API bloat.** If many abilities are Agent-accessible, the MCP tool list becomes large. Mitigation: actor filtering already narrows scope; categorization lets agents discover by category.

## References

- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Registry and §7 binding rules.
- [ADR-0027: MCP Dual-Mode](0027-mcp-dual-mode.md) — Existing MCP sidecar; Phase 3 retires its hand-written tools in favor of registry-derived ones.
- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — Mode is set by surfaces here.
- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Bridges call the renderer on return.
- [ADR-0110: Evaluation Harness for Abilities](0110-evaluation-harness-for-abilities.md) — Eval bridge is a surface using the same pattern.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — Worker is the only path for scheduled `global` maintenance.
