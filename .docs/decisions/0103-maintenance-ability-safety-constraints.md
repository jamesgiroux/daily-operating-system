# ADR-0103: Maintenance Ability Safety Constraints

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0101](0101-service-boundary-enforcement.md)  
**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md) (prerequisite for mode enforcement in §3, audit in §8, transactions in §2)  
**Related:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of the four have naturally bounded risk surfaces:

- Read abilities do not mutate.
- Transform abilities synthesize but do not mutate.
- Publish abilities write externally under explicit user or policy confirmation.

Maintenance abilities are qualitatively different. They:

- Mutate internal state — the entity graph, signal weights, computed trajectories, derived indices.
- Run on a schedule or in response to system events, not user actions.
- Often span multiple services in a single invocation (`reconcile_signals` reads signals, adjusts weights, refreshes entity state, updates indices).
- Execute without a user waiting on the result — their latency budget is hours, not milliseconds, which makes silent failures easy to miss.

Service-layer mutations ([ADR-0101](0101-service-boundary-enforcement.md)) are bounded by user action: each mutation corresponds to something a user or agent explicitly did, the blast radius is contained to that action, and errors surface immediately to the caller. Maintenance mutations have none of those natural constraints. A bug in a maintenance ability is the single largest new risk surface introduced by [ADR-0102](0102-abilities-as-runtime-contract.md).

Examples of the problems this can produce:

- A bug in `reconcile_signals` re-weights every signal source on every entity, silently corrupting health scoring across the corpus.
- A bug in `refresh_entity_state` emits a signal that triggers another refresh, causing a signal-propagation storm that saturates the queue.
- A race between a user mutation and a concurrent `repair_graph_links` invocation leaves the entity in an inconsistent state.
- A maintenance ability invoked in `ExecutionMode::Evaluate` writes to real tables because the mode is advisory rather than enforced.
- An MCP agent discovers `repair_graph_links` in the registry and invokes it with bad inputs, corrupting the graph.

The ability pattern is the right host for maintenance operations — they need the registry, provenance, observability, and composability that abilities provide. But they require structural safety constraints that Read and Transform abilities do not.

This ADR specifies those constraints.

## Decision

Every ability in the Maintenance category operates under all rules of [ADR-0102](0102-abilities-as-runtime-contract.md) plus the eleven constraints below, covering idempotency, transactional boundaries, execution-mode and dry-run semantics, multi-dimensional change budgets, signal containment, actor policy, blast radius with user-confirmed global, operational audit records, snapshot-diff evaluation, scoped observability, and compile-time separation. These constraints are enforced by type system, registry metadata, runtime checks, and review discipline — not by convention.

### 1. Idempotency Contract

A maintenance ability must produce the same terminal system state when invoked twice with the same input against the same starting state. Running `refresh_entity_state(id)` a second time does not compound changes; it converges.

**Enforcement:**
- Every maintenance ability has a required `test_idempotent` fixture in `src-tauri/tests/abilities/{name}/idempotency.rs`.
- Test structure: snapshot state → run ability → snapshot → run ability again → assert snapshots equal.
- CI gate: missing idempotency test fails the ability's test suite, which blocks the build.

**Exception:** An ability that explicitly accumulates (e.g., `archive_old_signals`) declares `idempotent = false` in its registry entry and must document what it accumulates and why. These are audited manually.

### 2. Transactional Boundaries

A maintenance ability that mutates across two or more services must wrap those mutations in a transaction. Partial failure must not leave visible inconsistent state.

ADR-0101's `with_transaction` is synchronous; maintenance abilities need an async variant because they invoke async service functions (network I/O, intelligence provider calls inside the transaction are forbidden, but async DB functions are common). [ADR-0104](0104-execution-mode-and-mode-aware-services.md) introduces `with_transaction_async` built on SQLite's write-serialization. Maintenance uses it:

```rust
pub async fn reconcile_signals(
    ctx: &AbilityContext,
    input: ReconcileSignalsInput,
) -> AbilityResult<ReconcileReport> {
    ctx.services.with_transaction_async(|tx| async move {
        tx.signals.rebalance_weights(...).await?;
        tx.entities.refresh_state(...).await?;
        tx.entities.recompute_health(...).await?;
        Ok(report)
    }).await
}
```

**Rules:**
- No LLM provider calls inside a transaction. Inference is slow and may hold the SQLite write lock for seconds. Synthesize first, then open the transaction to write.
- No external API calls inside a transaction. See the exception pattern below.
- Transactions commit or abort atomically. If a composed ability inside the transaction fails, the whole transaction aborts.

**Exception — outbox pattern for external mutations.** Abilities that must span a local DB write and an external API call (e.g., `sync_to_salesforce`, `publish_to_p2` when invoked by system policy) cannot be transactional across both. Instead, they use an outbox pattern:

1. Inside a local transaction, write the intended external mutation to an `outbox` table with an idempotency key and a pending status.
2. Commit the transaction.
3. Outside the transaction, perform the external call using the idempotency key. On success, mark the outbox entry as completed. On failure, a background retry (bounded retries, exponential backoff) re-attempts. On permanent failure, mark failed and raise a `MaintenanceOutcome::Failed`.
4. The external side deduplicates by idempotency key — a retry does not produce a double-mutation.

An explicit trait captures this:

```rust
#[async_trait]
pub trait HasOutboxEffect {
    fn idempotency_key(&self, input: &Self::Input) -> IdempotencyKey;
    async fn apply_external(&self, ctx: &AbilityContext, entry: OutboxEntry) -> Result<(), ExternalError>;
    async fn compensate(&self, ctx: &AbilityContext, entry: OutboxEntry, failure: ExternalError) -> Result<CompensationReport, Error>;
}
```

The registry records the pattern a maintenance ability uses (`transactional`, `outbox`, or `pure`) so reviewers and operators know its failure envelope.

### 3. Execution Mode and Dry-Run Semantics

This ADR relies on [ADR-0104](0104-execution-mode-and-mode-aware-services.md) which makes `ExecutionMode` load-bearing in the service layer. The three modes (`Live`, `Simulate`, `Evaluate`) have precise and non-overlapping semantics, summarized below. Maintenance abilities additionally honor a `dry_run` input flag that is orthogonal to mode. All four operational paths rely on a common **plan/apply split**: abilities compute a planned mutation set via non-mutating planning calls, then either apply the plan (Live, live-apply phase) or short-circuit before application (Simulate, Evaluate, or Live+dry_run).

| Concern | Live | Simulate | Evaluate | Live + dry_run=true |
|---------|------|----------|----------|---------------------|
| Mutations to domain tables | Applied in apply phase | Rejected by service layer if reached | Rejected by service layer if reached | Skipped by ability before apply phase |
| Signals emitted to signal bus | Yes (apply phase) | No (captured in tracer only) | No (captured in tracer only) | No (captured in tracer only) |
| External API calls | Yes (apply phase) | No (stubbed or replay fixture) | No (stubbed or replay fixture) | No |
| Intelligence provider calls | Yes (real, planning phase) | Stubbed or replay | Stubbed or replay (deterministic) | Yes (real, planning phase) |
| Audit records | Written to `maintenance_audit` | Captured to tracer only | Captured to tracer only (normalized) | Written to `maintenance_audit` |
| `MaintenanceReport` produced | Yes | Yes | Yes | Yes |
| `planned_mutations` populated | Yes (what was done) | Yes (what would be done) | Yes (what would be done) | Yes (what would be done) |

**Key distinctions:**

- **`Simulate` vs `Evaluate`.** Both reject mutations and signals. `Evaluate` additionally supplies a deterministic clock, seeded RNG, and replay providers (per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6) so snapshot-diff scoring is stable across runs. `Simulate` is a developer-facing "let me try this without side effects" mode where determinism is not guaranteed and tracer output is verbose but not normalized.
- **`dry_run` vs `Simulate`.** `dry_run` runs in `Live` mode with real providers and real reads; the ability short-circuits before the apply phase. Use it for user-initiated "show me what you would do" flows where synthesis quality must match production. `Simulate` runs with stubbed providers and replay fixtures — use it for offline developer work where real API costs or latency are unwanted.
- **Audit contract resolution.** `MaintenanceIntent` and `MaintenanceOutcome` (§8) are operational audit records, not signals. In `Live` mode, they are persisted to a dedicated `maintenance_audit` table, not the signal bus. In `Simulate` and `Evaluate` modes, they are captured to the tracer. `Live + dry_run=true` writes them to the audit table — the invocation happened; the mutations were chosen to be skipped.

**Enforcement:**
- Service-layer mode enforcement is specified in [ADR-0104](0104-execution-mode-and-mode-aware-services.md).
- Every maintenance ability supports `dry_run: bool` in its input. CI-generated conformance tests verify both `Live + dry_run=true` and a full `Evaluate` run produce identical `planned_mutations`.
- Dry-run input example:
```rust
pub struct RepairGraphLinksInput {
    pub scope: RepairScope,
    pub dry_run: bool,
    pub schema_version: SchemaVersion,
}
```

### 4. Multi-Dimensional Change Budget

A single "number of mutations" is insufficient — one SQL update can touch 10,000 rows, one service call can emit 100 signals, one maintenance run can invalidate an entire corpus (Codex finding #22). Change budget is declared and enforced across four dimensions:

```rust
#[ability(
    name = "reconcile_signals",
    category = Maintenance,
    // AbilityPolicy fields (canonical schema per ADR-0102 §7.1):
    allowed_actors = [System],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    // Maintenance-specific metadata (this ADR):
    blast_radius = entity_class,
    budget = Budget {
        service_calls: 50,
        affected_rows: 500,
        emitted_signals: 100,
        external_calls: 0,
    },
    idempotent = true,
    transactionality = Transactional, // Transactional | Outbox | Pure
    // Composition metadata (per ADR-0102 §7.1):
    mutates = [signals::rebalance_weights, entities::refresh_state, entities::recompute_health],
    composes = [get_entity_context],
)]
pub async fn reconcile_signals(...) -> AbilityResult<ReconcileReport> { ... }
```

At runtime, the maintenance wrapper tracks each dimension. Exceeding any dimension aborts the ability with `Error::BudgetExceeded(dimension, limit, actual)`. Services return affected-row counts via their result types; signal emission is tracked by `AbilityContext.tracer`; external calls are tracked by the outbox.

Budget scales with blast radius (defaults; can be overridden per ability with documented rationale):

| Blast radius | service_calls | affected_rows | emitted_signals | external_calls | Override path |
|--------------|---------------|---------------|-----------------|----------------|---------------|
| `single_entity` | 10 | 50 | 20 | 0 | Ability-level with rationale |
| `entity_class` | 50 | 500 | 100 | 0 | Ability-level with rationale |
| `account_scope` | 200 | 2,000 | 500 | 10 | ADR amendment |
| `global` | — | — | — | — | ADR amendment; no defaults |

A runaway bug cannot corrupt beyond the declared envelope.

### 5. Signal Containment

Signals emitted during maintenance operations preserve their original `data_source` per [ADR-0098](0098-data-governance-source-aware-lifecycle.md) (e.g., `glean`, `salesforce`, `user`, `ai`) — **never** overwritten with a synthetic "Maintenance" source, which would violate the data-origin model and break purge-on-revocation (Codex finding #15). Instead, signals emitted from inside a maintenance invocation carry two additional fields that describe the operational context without conflating it with data provenance:

```rust
pub struct SignalEvent {
    // ... existing ADR-0080/0098 fields unchanged ...
    pub data_source: DataSource,                       // original source, per ADR-0098
    
    // New fields added in this ADR:
    pub maintenance_invocation_id: Option<InvocationId>,  // Set when emitted inside a maintenance ability
    pub propagation_depth_limit: Option<u8>,               // Set when maintenance containment applies
}
```

**Containment rules:**
- Signals emitted inside a maintenance invocation are automatically tagged with `maintenance_invocation_id` by the service layer.
- `PropagationEngine` ([ADR-0080](0080-signal-intelligence-architecture.md)) treats signals with `maintenance_invocation_id.is_some()` as containment-limited by default: their `propagation_depth_limit` is set to 1 unless the ability's registry metadata declares otherwise.
- At each propagation hop, the engine decrements the remaining depth. When it hits zero, propagation stops and a tracer event logs "maintenance propagation contained at depth N."
- Registry flag `propagation_override = Depth::Unlimited | Depth::N(k)` lifts the default but requires a documented rationale and a passing cascade test in the eval harness.

**Why this structure.** `data_source` remains the truthful origin of the data (for reliability weighting, purge cascades, trust). Maintenance context is orthogonal metadata, not a conflicting `source`. A signal emitted by a maintenance job that updates health scoring still has `data_source = ai` (or whatever computed it); its maintenance-invocation-id tells the audit story without rewriting its data lineage.

### 6. Actor Policy in Registry

Every ability declares which actors may invoke it:

```rust
pub enum Actor {
    User,       // Human invocation via Tauri command
    Agent,      // MCP tool invocation
    System,     // Scheduled worker or internal trigger
}

// Canonical AbilityPolicy schema per ADR-0102 §7.1
pub struct AbilityPolicy {
    pub allowed_actors: Vec<Actor>,
    pub allowed_modes: Vec<ExecutionMode>,
    pub requires_confirmation: bool,
    pub may_publish: bool,              // Whether a Maintenance ability may invoke Publish
}
```

Maintenance abilities default to `allowed_actors: [System]`. Agents and users invoking a maintenance ability through the registry receive `Error::ActorNotAllowed` at the boundary, before the ability executes. Opt-in to `Agent` or `User` invocation requires explicit ADR amendment and a documented justification.

**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.

Actor policy alone is insufficient in-process: a direct Rust import of a maintenance ability function would still execute regardless of the registry policy. Compile-time separation (§11) closes that gap post-v1.4.0. Until then, review discipline and the typed registry invocation paths from [ADR-0102](0102-abilities-as-runtime-contract.md) §7.2 prevent accidental bypass.

### 7. Blast Radius Declaration and User-Confirmed Global

Every maintenance ability declares its blast radius. Invocation pathways check blast radius against invoker privilege:

| Blast radius | Who can invoke | Confirmation UX |
|--------------|----------------|-----------------|
| `single_entity` | System, User (with entity ownership) | None |
| `entity_class` | System | None |
| `account_scope` | System | None |
| `global` | System (scheduled worker only) OR User with explicit UI confirmation | Typed confirmation dialog in-app |

DailyOS is a single-user local desktop app; there is no separate "admin" identity (Codex finding #32). Global maintenance is either:

1. **Scheduled-worker invocation** — the built-in background worker runs `global` maintenance on a declared schedule (weekly, monthly). The schedule is static code; it is not user-triggered. The worker is the only `System` path that may invoke `global`.
2. **User-confirmed invocation** — the app surfaces a `global` maintenance operation through a dedicated Settings panel ("Maintenance > Run Reconciliation"). Clicking the action opens a typed confirmation dialog that:
   - Displays the ability name, planned blast radius, expected duration, and budget.
   - Requires the user to type the ability name to confirm.
   - Issues a single-use `ConfirmationToken` scoped to the ability, valid for 60 seconds.
   - Is the only in-app path to invoke `global` maintenance from a user action.

Agents (MCP) cannot invoke `global` maintenance under any circumstance, regardless of actor policy overrides. This is a hard floor enforced at the registry.

**Blast radius validation.** Self-declaration is not trustworthy by itself (Codex finding #23). Services that accept mutation calls declare their mutation scope in their function signature (e.g., `entities.refresh_state(entity_id)` is `single_entity`; `entities.recompute_all()` is `global`). The maintenance ability wrapper computes the maximum declared scope of services called and compares it to the ability's declared `blast_radius`. If the ability declares `single_entity` but calls a `global`-scoped service, registration fails with `Error::BlastRadiusUnderdeclared`.

### 8. Intent-and-Outcome Audit (not signals)

Every maintenance ability writes two audit records — NOT signals (Codex finding #16). Signals feed intelligence fusion, health scoring, and propagation; they affect the product's understanding of the world. Maintenance audit records are operational metadata describing what the system did to itself. Mixing them would contaminate scoring and propagation.

Audit records are written to a dedicated `maintenance_audit` table:

```rust
pub struct MaintenanceAuditRecord {
    pub invocation_id: InvocationId,
    pub ability_name: &'static str,
    pub ability_version: AbilityVersion,
    pub actor: Actor,
    pub mode: ExecutionMode,
    pub input_hash: Hash,
    pub phase: AuditPhase,                       // Intent | Outcome
    pub timestamp: DateTime<Utc>,
    pub affected_entity_ids: Vec<EntityId>,      // Indexed; supports "what ran on entity X" lookups
    
    // Provenance envelope — shape defined by ADR-0105
    pub provenance: ProvenanceOrMasked,          // Full provenance or masked marker if sources revoked
    
    // Intent-phase fields:
    pub planned_mutations: Option<PlannedMutationSummary>,
    pub budget: Option<Budget>,
    pub blast_radius: Option<BlastRadius>,
    
    // Outcome-phase fields:
    pub mutations_applied: Option<AppliedMutationSummary>,
    pub signals_emitted: Option<u32>,
    pub external_calls: Option<u32>,
    pub duration_ms: Option<u64>,
    pub outcome: Option<OutcomeStatus>,          // Success | Aborted | Failed
    pub error: Option<ErrorDetail>,
}

// Either full provenance, or a mask record replacing it after source revocation.
// Masking rules and shape defined by ADR-0108.
pub enum ProvenanceOrMasked {
    Full(Provenance),
    Masked(ProvenanceMasked),
}
```

`affected_entity_ids` stores the list of entity IDs the invocation planned to touch (Intent) or did touch (Outcome). It is stored as a JSON array column and indexed via a supporting `maintenance_audit_entity_xref` table that maps `(invocation_id, entity_id)` — SQLite does not support GIN-style array indexes, so cross-reference via a join table is the idiomatic pattern.

**Two records per invocation, always.** Both records are produced by the registry's maintenance-ability wrapper (not by the ability body) after the plan and apply phases specified in [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §3.

1. **Intent** — written after the plan phase completes and before any mutation. Contains invocation ID, ability name/version, input hash, actor, mode, planned mutation summary, budget, blast radius, timestamp.
2. **Outcome** — written after completion, whether apply ran or was short-circuited. Contains the same invocation ID, actual mutation count (0 in dry-run / non-Live), signal emission count, external call count, duration, outcome status, error details if any, and a `skipped_reason: Option<SkippedReason>` field:
   - `None` — apply phase ran (Live, dry_run=false)
   - `Some(DryRun)` — Live mode with `dry_run=true`; apply was skipped deliberately
   - `Some(ModeNonLive)` — Simulate or Evaluate; apply was skipped by plan/apply discipline
   - `Some(BudgetExceeded { dimension, limit, planned })` — plan exceeded budget, apply was not attempted
   - `Some(ValidationFailed { reason })` — plan validation failed before apply

**Mode-specific behavior:**
- **Live and Live+dry_run:** records persist to `maintenance_audit`.
- **Simulate and Evaluate:** records captured to `AbilityTracer` only, never persisted. This honors the mode contract (no side effects in Simulate/Evaluate) while still producing a full trace for debugging or evaluation scoring.

**Implementation discipline:**
- The registry's maintenance-ability wrapper writes audit records automatically. Individual ability implementations cannot skip them.
- Audit writes go through a dedicated `audit` service that is not subject to the maintenance ability's change budget (audit must not be rationed).
- Audit records do not propagate, do not affect health scoring, do not feed the intelligence queue. They are pure operational metadata.

**Why this matters.** When a user reports "my data is wrong," the first diagnostic query is:
```sql
SELECT ma.*
FROM maintenance_audit ma
JOIN maintenance_audit_entity_xref x ON x.invocation_id = ma.invocation_id
WHERE x.entity_id = ?
ORDER BY ma.timestamp DESC
LIMIT 50;
```
— what maintenance ran on this entity, when, with what planned vs. actual effect. SQLite-native, indexed, fast. This query works without corrupting the signal bus or the intelligence model.

### 9. Snapshot-Diff Evaluation

Maintenance abilities are evaluated differently from Read and Transform abilities:

- **Fixture:** an input state snapshot (entity states, signals, weights) captured from anonymized local data, with fixture governance per [ADR-0107](0107-evaluation-harness-for-abilities.md) (source tagging, retention, purge-on-revocation).
- **Run:** invoke the ability in `ExecutionMode::Evaluate` with `dry_run = true`.
- **Score:** compare the ability's `planned_mutations` against an expected mutation set curated by reviewer or captured from a known-good prior run.
- **Regression:** a PR that changes an ability's planned output set without updating the expected set fails the eval gate.

This is more expensive to set up than transform-ability eval but becomes tractable once §3 (ExecutionMode enforcement and dry-run contract) is in place. [ADR-0107](0107-evaluation-harness-for-abilities.md) specifies the harness mechanics and fixture governance.

### 10. Observability Scoped to Single-User Reality

DailyOS is a single-user local desktop app. A full ops-center dashboard with anomaly alerting, rolling-median thresholds, and budget-near-miss reports would be theater (Codex finding #31). Observability is scoped to what the actual user and the developer need.

Every maintenance invocation produces structured telemetry captured by the `AbilityTracer` from [ADR-0102](0102-abilities-as-runtime-contract.md) §5, with maintenance-specific fields persisted to the `maintenance_audit` table (see §8):

| Field | Purpose |
|-------|---------|
| `invocation_id` | Correlates across audit records, traces, logs |
| `ability_name`, `ability_version` | What ran |
| `actor`, `mode` | Who invoked, in which mode |
| `started_at`, `ended_at` | Duration |
| `mutations_planned`, `mutations_applied` | Detect anomalies |
| `budget_used`, `budget_limit` | Watch for budget near-misses |
| `external_calls` | Outbox activity indicator |
| `outcome`, `error` | Debugging |

**Surfaces this drives (single-user appropriate):**

1. **Structured logs** — every invocation logs start/end at INFO, errors at ERROR. Standard log hygiene, no dashboards.
2. **Settings > Maintenance view** — a simple list of recent maintenance runs with status, duration, and mutations applied. "Did something run overnight? What happened?" User-visible troubleshooting, not ops monitoring.
3. **Automatic error notification** — if a maintenance invocation fails or aborts (budget exceeded, compensation failed), the app shows a non-blocking notification on next open. The user sees "Reconciliation aborted on 2026-04-18: budget exceeded" with a link to details.
4. **Eval harness regression gate** — CI catches quality regressions (via [ADR-0107](0107-evaluation-harness-for-abilities.md)). This is the only "alerting" — and it fires pre-merge, not in production.

No anomaly-detection dashboards, no rolling-median thresholds, no pager alerts. Those belong in products with ops teams. The user-visible maintenance view plus CI eval gates are sufficient for a six-user local app, and the architecture leaves room to add more telemetry if DailyOS ever becomes a multi-tenant product.

### 11. Compile-Time Separation

Maintenance abilities are typed distinctly from Read and Transform:

```rust
pub type ReadAbilityFn = fn(&AbilityContext, ReadInput) -> AbilityResult<ReadOutput>;
pub type TransformAbilityFn = fn(&AbilityContext, TransformInput) -> AbilityResult<TransformOutput>;
pub type MaintenanceAbilityFn = fn(&AbilityContext, MaintenanceInput) -> AbilityResult<MaintenanceReport>;
```

The registry exposes three typed invocation paths:

```rust
pub struct AbilityRegistry {
    pub fn invoke_read(&self, name: &str, input: ReadInput, ctx: &AbilityContext) -> ...;
    pub fn invoke_transform(&self, name: &str, input: TransformInput, ctx: &AbilityContext) -> ...;
    pub fn invoke_maintenance(&self, name: &str, input: MaintenanceInput, ctx: &AbilityContext) -> ...;
}
```

A surface that has access only to `invoke_read` and `invoke_transform` cannot reach maintenance abilities even by name. Tauri commands for read/transform invocation do not expose a maintenance path. MCP tools for non-maintenance abilities route through the non-maintenance invokers. This is phased:

- **v1.4.0 Phase 2–3:** maintenance typed separately in registry; runtime check for actor policy at invocation.
- **v1.4.1+:** surfaces get typed-narrow references to the registry that omit maintenance entry points unless the surface is the scheduled worker or the user-initiated maintenance flow from Settings (per §7).

## Consequences

### Positive

1. **Blast radius of maintenance bugs is structurally bounded.** Multi-dimensional change budget, idempotency, and containment rules prevent a bug from corrupting arbitrary state. Bugs fail loud or fail small.
2. **Maintenance operations are auditable by default.** Intent/outcome records in `maintenance_audit` form a durable audit trail separate from the signal bus. Every state change by the system on itself is retrievable without contaminating intelligence fusion.
3. **Eval of maintenance operations is tractable.** Snapshot-diff evaluation gives maintenance the same quality discipline that transforms get from output scoring. Enabled by the dry-run contract and mode-aware services.
4. **Actor and blast-radius policy prevents registry surface attacks.** Agents cannot invoke maintenance by default. Global operations require explicit scheduled-worker path or typed user confirmation. Registry introspection is actor-filtered so agents cannot even see maintenance abilities.
5. **Signal bus integrity preserved.** Maintenance-emitted signals keep their true `data_source` ([ADR-0098](0098-data-governance-source-aware-lifecycle.md)), with containment tracked via orthogonal metadata. Purge cascades and source reliability weighting continue to work.
6. **Dry-run as a contract enables development without risk.** Developers can run maintenance abilities against real fixtures with zero-risk and full output visibility.
7. **Outbox pattern handles external mutations cleanly.** Abilities that span local state and external APIs use idempotency keys, durable pending state, and explicit compensation — not implicit two-phase guarantees that do not exist.
8. **Observability is scoped to the product's actual scale.** Single-user local app gets a troubleshooting view and CI eval gates, not a full ops dashboard. Architecture leaves room to grow if deployment model changes.

### Negative

1. **Maintenance abilities are expensive to write.** Idempotency test, dry-run conformance test, snapshot-diff fixture, multi-dimensional budget declaration, outbox scaffolding where needed. Intentional cost — these operations warrant it — but it is real cost.
2. **Service layer takes on mode-checking responsibility.** Every mutation function honors `ExecutionMode` (per [ADR-0104](0104-execution-mode-and-mode-aware-services.md)). Touches a lot of code but is mechanical.
3. **Signal schema gains two fields.** `maintenance_invocation_id` and `propagation_depth_limit` added to `SignalEvent`. Requires a migration and indexing on the invocation id.
4. **The registry grows more complex.** Ability metadata carries category, actor policy, blast radius, multi-dimensional budget, idempotency flag, transactionality mode, outbox flag. More surface to get right.
5. **New audit table.** `maintenance_audit` is a new schema and a new write path. Modest migration work.
6. **Eval-harness complexity increases.** Snapshot-diff evaluation is mechanically more involved than output scoring. [ADR-0107](0107-evaluation-harness-for-abilities.md) must account for this.

### Risks

1. **Constraint erosion over time.** Contributors under schedule pressure file abilities without dry-run tests, claim idempotency without verifying, or under-declare a blast radius. Mitigation: CI gates on conformance tests, review checklist, blast-radius validation from service call graph, regular audit of `idempotent = false` exceptions.
2. **Mis-categorized abilities.** A transform-shaped operation gets filed as maintenance (or vice versa) to bypass or escape category-specific discipline. Mitigation: category determined by call-graph effect per [ADR-0102](0102-abilities-as-runtime-contract.md) §3 — static check at registration.
3. **Outbox retry failure.** External mutations via the outbox can retry forever on a transient failure or get permanently stuck. Mitigation: bounded retries with exponential backoff, permanent-failure status after N attempts with operator notification via the Maintenance Settings view.
4. **Actor policy drift.** Over time, pressure to let agents invoke maintenance ("but I promise it's safe") erodes the default-deny posture. Mitigation: actor policy changes require ADR amendment, not a one-line registry edit.
5. **Blast radius under-declaration.** An ability that should be `global` gets filed as `account_scope` to avoid confirmation requirements. Mitigation: §7 — registry computes maximum declared scope from service call graph and rejects under-declared bindings.
6. **Budget tuned too tight.** Legitimate maintenance operations hit their budget mid-run and abort, leaving partial state. Mitigation: observability surfaces budget-near-miss in the Settings > Maintenance view (§10). Budget review follows. Compensation/outbox handles the abort cleanly.
7. **Idempotency theater.** A test passes because the snapshot comparison is too shallow (e.g., ignores updated_at timestamps that the ability modifies). Mitigation: idempotency scope is documented per ability — business-state hash separately from operational telemetry. Reviewers check that business-state scope is sufficient.
8. **Loophole for "simple" maintenance.** The ADR allows trivial cross-service cleanup operations to live in a scheduled job rather than as an ability. This could be abused to file anything as "simple" and skip the constraints (Codex finding #38). Mitigation: "simple" is defined narrowly — single-service mutation, no synthesis, no external calls. Anything touching multiple services or emitting signals is an ability. Review enforces.
9. **Mode contract violations.** A maintenance ability or a composed sub-ability accidentally writes in Simulate/Evaluate mode. Mitigation: enforcement lives in the service layer ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)) — abilities cannot bypass it.
10. **Outbox staleness.** An outbox entry sits as "pending" indefinitely because a retry is stuck. Mitigation: a dedicated `outbox_reaper` maintenance ability runs hourly; entries pending longer than their declared SLA are marked permanent-failure and surfaced.

## References

- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Establishes the ability pattern this ADR constrains. Maintenance is one of four categories; this ADR defines its safety envelope.
- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — Services remain the mutation mechanism. Maintenance abilities orchestrate services; they do not bypass them. Rule 3 (multi-write transactionality) extends to the cross-service case here via `with_transaction_async`.
- [ADR-0080: Signal Intelligence Architecture](0080-signal-intelligence-architecture.md) — Signal schema and propagation engine. This ADR adds `maintenance_invocation_id` and `propagation_depth_limit` fields for containment.
- [ADR-0098: Data Governance — Source-Aware Lifecycle](0098-data-governance-source-aware-lifecycle.md) — `data_source` remains the canonical origin marker. Maintenance context does not overwrite it; orthogonal fields preserve lifecycle semantics.
- **[ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md)** — Prerequisite. Specifies how `ExecutionMode` flows through `ServiceContext`, how mutation functions honor it, and `with_transaction_async` primitive.
- **ADR-0105 (forthcoming): Provenance as First-Class Output** — Defines the provenance shape that maintenance abilities populate (separately from the operational audit records in §8 of this ADR).
- **ADR-0107 (forthcoming): Evaluation Harness for Abilities** — Specifies the snapshot-diff evaluation mechanism referenced in §9, including fixture governance and anonymization rules.
