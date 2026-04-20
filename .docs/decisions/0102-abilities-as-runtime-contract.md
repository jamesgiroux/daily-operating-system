# ADR-0102: Abilities as the Runtime Contract

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0101](0101-service-boundary-enforcement.md), [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0082](0082-entity-generic-prep-pipeline.md)  
**Related:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0097](0097-account-health-scoring-architecture.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md), [ADR-0100](0100-glean-first-intelligence-architecture.md)

## Context

DailyOS delivers depth-of-context before every interaction. A user walking into a meeting receives not a summary of their calendar but a briefing shaped by the full state of the relationships, accounts, commitments, and signals relevant to that moment. This is the product's differentiation and its moat: depth compounds over time as more signals flow in, more context accumulates, and more relationships are understood.

Depth of this kind cannot be assembled on demand, feature by feature. It has to be maintained continuously and composed consistently. Today, every feature in the product assembles context in its own command handler: meeting prep queries one set of tables, daily readiness queries another, the MCP sidecar queries a third. Context builders (`build_intelligence_context`, `gather_account_context`) are shared helpers but not the single invocation path. The result:

- New signal sources require touching every feature that should benefit from them.
- Consumers receive subtly different context depending on how they entered the system (app vs. MCP vs. background worker).
- Capability quality cannot be measured at the capability level, because capabilities are not first-class units.
- Prompts, synthesis logic, and provenance are distributed across the codebase rather than owned by a capability.

The substrate ADRs — services ([ADR-0101](0101-service-boundary-enforcement.md)), signals ([ADR-0080](0080-signal-intelligence-architecture.md)), entities ([ADR-0082](0082-entity-generic-prep-pipeline.md)), providers ([ADR-0091](0091-intelligence-provider-abstraction.md)), data governance ([ADR-0098](0098-data-governance-source-aware-lifecycle.md)), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)) — establish how state is mutated, how signals flow, how entities are modeled, and how intelligence is retrieved. None of them establishes how a **product capability** is shaped, named, versioned, or exposed. That layer is missing.

This ADR introduces it.

## Decision

Every product capability in DailyOS is implemented as an **ability**: a named, typed, versioned function that synthesizes state and signals into a structured output. All consumers — the desktop app, the MCP server, background workers, and any future surface — invoke capabilities exclusively through the abilities layer. The abilities layer is the runtime contract of the product.

### 1. What an Ability Is

An ability is a single function that:

- Has a stable, public name in `verb_subject` snake_case form (`prepare_meeting`, `get_entity_context`, `detect_risk_shift`).
- Takes a typed input struct and an `AbilityContext`.
- Returns a typed output wrapped in `AbilityOutput<T>`, which carries the domain output alongside a mandatory provenance envelope. Provenance shape is specified in [ADR-0105](0105-provenance-as-first-class-output.md). The domain output itself does not re-embed provenance.
- Corresponds to a concrete product capability visible to a user or an agent. Abilities are not computational primitives; they are the things a chief of staff does.

**Boundary scope.** The abilities layer governs synthesized user-facing and agent-facing outputs — briefings, narratives, entity context, risk detection, curated publications. Purely mechanical operations (list entity names for a picker, format a timestamp, read a single field for a toolbar badge) remain in their owning service or command and are not abilities. A good heuristic: if an output is structured, composed from multiple sources, or involves synthesis, it is an ability. If it is a single read or a trivial projection, it is not.

An ability is not:

- A bare CRUD function (those belong in `services/`, per [ADR-0101](0101-service-boundary-enforcement.md)).
- A prompt template (those are internal to the ability that owns them).
- A context builder (those are composed by abilities but are not themselves abilities).

### 2. Ability Location and Module Structure

Abilities live in `src-tauri/src/abilities/`:

```
src-tauri/src/abilities/
├── mod.rs                      # Registry + AbilityContext
├── context.rs                  # AbilityContext definition
├── provenance.rs               # Provenance types (detail in ADR-0105)
├── result.rs                   # AbilityResult, AbilityError
├── read/
│   ├── get_entity_context.rs
│   ├── get_daily_readiness.rs
│   ├── list_open_loops.rs
│   └── explain_salience.rs
├── transform/
│   ├── prepare_meeting.rs
│   ├── detect_risk_shift.rs
│   ├── generate_weekly_narrative.rs
│   └── summarize_recent_changes.rs
├── publish/
│   ├── publish_to_p2.rs
│   └── export_report.rs
└── maintenance/
    ├── refresh_entity_state.rs
    ├── reconcile_signals.rs
    └── repair_graph_links.rs
```

Each ability is a single module — typically one file, but a directory is permitted for abilities large enough to warrant submodules (e.g., `prepare_meeting/` containing `mod.rs`, `prompts.rs`, `synthesis.rs`, `tests.rs`). The module has one public entry point (the ability function) and opaque internals. Eval fixtures live under `src-tauri/tests/abilities/{ability_name}/` (detail in [ADR-0107](0107-evaluation-harness-for-abilities.md)).

### 3. Ability Categories

Abilities belong to one of four categories. **Category is determined by the ability's call-graph effect on services, not by author preference.** A static check (runtime during Phase 3, compile-time during Phase 4) enforces the classification: an ability whose transitive service calls include mutations is a Publish or Maintenance ability; one whose calls are read-only is a Read or Transform ability.

| Category | Call-graph effect | Consumes LLM? | Examples |
|----------|------------------|---------------|----------|
| **Read** | No service mutation anywhere in the call graph. May emit ephemeral logs and telemetry but never writes domain state or emits propagating signals. | Usually no | `get_entity_context`, `list_open_loops`, `get_daily_readiness` |
| **Transform** | No service mutation anywhere in the call graph. May invoke the intelligence provider. Results must not be used to authorize mutation without an additional trust signal (see §12). | Yes | `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative` |
| **Publish** | Writes externally (third-party APIs, file exports). Requires explicit user or policy confirmation. | Sometimes | `publish_to_p2`, `export_report` |
| **Maintenance** | Mutates internal state through services. Subject to [ADR-0103](0103-maintenance-ability-safety-constraints.md). | Sometimes | `refresh_entity_state`, `reconcile_signals` |

Category consequences:

- **Read** abilities are idempotent and safe to cache by a composite key that includes `ctx.user`, `ctx.services.clock.now()` (bucketed), provider settings, Glean connection state, and DB watermarks. Caching keyed solely on input is forbidden — outputs depend on ambient context.
- **Transform** abilities are eval-gated on output quality (see [ADR-0107](0107-evaluation-harness-for-abilities.md)). Their outputs are marked untrusted by default (§10); they cannot authorize mutation on their own, but they can participate in mutation-authorizing composition when paired with a trust signal (user confirmation, policy pre-authorization, or schema-constrained value).
- **Publish** abilities require a `ConfirmationToken` from the invoking surface. A maintenance ability cannot call a publish ability unless its registry policy explicitly grants `may_publish = true`, which requires an ADR amendment.
- **Maintenance** abilities are bound by [ADR-0103](0103-maintenance-ability-safety-constraints.md). Contributors writing a maintenance ability MUST read ADR-0103; the constraints are non-optional.

**Leakage is enforced, not trusted.** If a Read or Transform ability accidentally introduces a mutation (through an imported service function or a composed ability), the call-graph check flags it at registration time and the registry refuses to bind the ability.

### 4. Ability Signature

Every ability has the same shape:

```rust
pub async fn prepare_meeting(
    ctx: &AbilityContext,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    // 1. Gather: compose read abilities + signal queries
    // 2. Synthesize: invoke IntelligenceProvider where needed
    // 3. Attribute: attach provenance to each output field
    // 4. Return: typed output + provenance
}
```

`AbilityResult<T>` is `Result<AbilityOutput<T>, AbilityError>`, where `AbilityOutput<T>` wraps the domain output with required provenance, version metadata, and diagnostics.

### 5. AbilityContext

`AbilityContext` is the single dependency entry point for abilities, analogous to `ServiceContext` ([ADR-0101](0101-service-boundary-enforcement.md)) but operating one layer higher. `ServiceContext` itself is amended to carry `ExecutionMode` in [ADR-0104](0104-execution-mode-and-mode-aware-services.md), which is a prerequisite to this ADR.

```rust
pub struct AbilityContext<'a> {
    pub services: &'a ServiceContext<'a>,      // Carries DB, signals, queue, mode, clock, RNG — see ADR-0104
    pub intelligence: &'a dyn IntelligenceProvider,  // ADR-0091; scope expansion noted in §12
    pub user: &'a UserEntity,                   // ADR-0089
    pub tracer: &'a AbilityTracer,              // Observability and audit capture
    pub actor: Actor,                           // Who invoked this ability (User | Agent | System)
    pub confirmation: Option<ConfirmationToken>, // Present when user/policy confirmed; required for Publish
}
```

Consequences of this shape:

- Abilities never open DB connections. They go through services.
- Abilities never instantiate an LLM provider. They receive one via `ctx.intelligence`, which is replayable in `Simulate`/`Evaluate` modes per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6.
- Abilities never read the system clock directly. They use `ctx.services.clock.now()`. There is exactly one clock contract — on `ServiceContext` — and abilities inherit it via composition.
- Abilities never access Glean directly. Glean access is provided through mode-aware services at `ctx.services.external.glean.*` per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §2, which route to the live Glean client in `Live` mode and to replay fixtures in `Simulate`/`Evaluate`. A direct `GleanClient` reference on `AbilityContext` would bypass mode-aware external-call gating and is forbidden.
- Execution mode lives on `ServiceContext`, not `AbilityContext`. Services enforce mode at the mutation boundary; abilities do not need to remember it. See [ADR-0104](0104-execution-mode-and-mode-aware-services.md).
- Abilities receive a declared `actor` so they can enforce policy. The registry pre-filters by actor policy; the `actor` field is available for ability-internal logic (e.g., a Read ability that returns richer detail to the owning user than to an agent).

### 6. Input and Output Contracts

Every ability declares typed input and output structs in its own module. The output is always wrapped in `AbilityOutput<T>`, which carries provenance exactly once — the domain output does not re-embed it.

```rust
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct PrepareMeetingInput {
    pub meeting_id: MeetingId,
    pub depth: ContextDepth,              // Shallow | Standard | Deep
    pub include_open_loops: bool,
    pub schema_version: SchemaVersion,    // Required; no implicit LATEST for external callers
}

#[derive(Serialize, JsonSchema)]
pub struct MeetingBrief {
    pub meeting: MeetingSummary,
    pub topics: Vec<Topic>,
    pub attendee_context: Vec<AttendeeContext>,
    pub open_loops: Vec<OpenLoop>,
    pub what_changed_since_last: Vec<ChangeMarker>,
    pub suggested_outcomes: Vec<SuggestedOutcome>,
    pub schema_version: SchemaVersion,
    // Note: no `provenance` field here. It lives in the AbilityOutput<T> wrapper below.
}

// The actual return type from prepare_meeting:
pub type Output = AbilityOutput<MeetingBrief>;

pub struct AbilityOutput<T> {
    pub data: T,
    pub provenance: Provenance,           // Canonical location — see ADR-0105
    pub ability_version: AbilityVersion,  // Which version of the ability produced this
    pub diagnostics: Diagnostics,         // Non-fatal warnings, missing sources, etc.
}
```

Rules:

- Provenance lives on `AbilityOutput<T>`, exactly one place. Domain output types never re-declare a `provenance` field. Reviewers reject such declarations at registration.
- Field-level provenance attribution (how `suggested_outcomes[0]` ties back to specific signals) is specified in [ADR-0105](0105-provenance-as-first-class-output.md). This ADR requires that the mechanism exist; 0105 defines the shape.
- Inputs are versioned via `SchemaVersion` (newtype wrapping `u32`). External callers (MCP, future REST) MUST declare `schema_version`; no implicit latest. Internal callers (Tauri commands) may default to current since they rebuild with the ability. Versioning rules in §8.
- Outputs are JSON-serializable via `serde` and expose a JSON Schema via `schemars` for external introspection. No trait objects, no opaque handles. This makes Tauri bridge, MCP tool registration, and docs generation all derive from the same schema source.

### 7. Registry and Invocation

The registry is the authoritative enumeration of abilities in the product. Because abilities have heterogeneous typed inputs and outputs, the registry cannot simply store function pointers in a `HashMap`. Instead, abilities are registered via a derive macro that generates a type-erased wrapper operating on `serde_json::Value`. The registry stores the wrappers and the schema metadata; consumers that want typed invocation go through a separate typed path.

#### 7.1 Registration

Each ability declares itself with an attribute macro that generates its registration, schema, and type-erased wrapper. The macro cannot infer transitive service calls or ability composition from the function body alone; it relies on **explicit declarative metadata** that the author provides and the registry validates at registration time:

```rust
#[ability(
    name = "prepare_meeting",
    category = Transform,
    // AbilityPolicy fields — canonical schema used across all ADRs:
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    idempotent = true,
    // Explicit dependency metadata — not inferred:
    mutates = [],                                    // Which mutating service functions this calls (none for Transform)
    composes = [get_entity_context, list_open_loops], // Which other abilities this invokes
)]
pub async fn prepare_meeting(
    ctx: &AbilityContext,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> { ... }
```

The `AbilityPolicy` schema is canonical across all ADRs:

```rust
pub struct AbilityPolicy {
    pub allowed_actors: Vec<Actor>,       // Who can invoke: User | Agent | System
    pub allowed_modes: Vec<ExecutionMode>, // Which modes permit invocation (default: all three)
    pub requires_confirmation: bool,       // Whether invocation requires a ConfirmationToken
    pub may_publish: bool,                 // Whether Maintenance may invoke Publish (default false)
}
```

ADR-0103 §6 uses these exact field names. There is no `actor_policy` shorthand — the macro attribute names match the struct fields one-to-one.

**Why declarative metadata and not inference.** A proc-macro operates on the function's AST at parse time. It cannot see through arbitrary typed imports, trait-object calls, or conditional branches to compute a transitive call graph. Requiring authors to declare `mutates` and `composes` explicitly makes the enforcement checkable and keeps the contract honest.

**Metadata validation.** At test time, a registry self-check verifies the declared metadata against runtime tracing of each ability's execution under fixture inputs. If the declared `mutates` list omits a service call the ability actually makes, the test fails with a specific error (`MetadataDrift { ability, declared, observed }`). This catches drift between declaration and reality without requiring compile-time inference.

The macro emits:

- A static `AbilityDescriptor` in the registry inventory, carrying the ability's name, category, actor policy, input JSON schema, output JSON schema, version, and metadata.
- A type-erased wrapper: `fn(ctx, serde_json::Value) -> BoxFuture<AbilityResult<serde_json::Value>>`, which deserializes the input, invokes the typed ability, and re-serializes the output.
- An inventory registration call executed at program startup via `inventory::submit!` so the registry assembles itself without hand-registration.

#### 7.2 Typed vs. Erased Invocation

There are two invocation paths:

- **Typed invocation** (used by internal Rust callers including other abilities and hand-written Tauri commands): direct import of the ability function. No serialization round-trip.
- **Erased invocation** (used by external consumers — Tauri bridge, MCP, future REST): goes through the registry wrapper with JSON boundaries. Validated against schema on entry and exit.

Both paths share the same function body. Erased invocation adds schema validation and actor-policy enforcement at the registry boundary.

#### 7.3 What the Registry Drives

The registry is a single source of truth for:

- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
- Tauri command generation. A build-time helper iterates the inventory and emits Tauri command wrappers. Alternatively, a single generic Tauri command `invoke_ability(name, input_json)` handles all abilities with runtime dispatch. v1.4.0 ships the generic dispatch; per-ability wrappers can be added later if needed for type ergonomics.
- Eval harness discovery ([ADR-0107](0107-evaluation-harness-for-abilities.md)).
- Documentation generation — every ability's input/output schema, category, actor policy, and prose description render to `.docs/abilities/{name}.md` via a build script.

#### 7.4 Actor-Filtered Introspection

Registry enumeration is actor-scoped. An MCP client enumerating available tools sees only abilities whose `AbilityPolicy.allowed_actors` includes `Agent`. It does not see maintenance abilities, admin-only abilities, or internal composition helpers. Introspection does not leak ability names, input schemas, or blast radius to callers that are not authorized to invoke.

The Tauri bridge, invoked by the first-party app, sees the full registry subject to `User` and `System` filtering. Background workers see the `System` filter.

#### 7.5 Surfaces Do Not Bypass the Registry

Surfaces (Tauri commands, MCP handlers, background workers) invoke abilities through the registry or through typed imports as described in §7.2. They do not construct `AbilityContext` directly and do not call ability functions outside these two paths. Phase 4 enforces this by module visibility.

### 8. Versioning

Ability versioning uses two coordinates: an **ability version** (major.minor of the ability itself) and a **schema version** (monotonic integer per ability that the input and output share).

```rust
pub struct AbilityVersion {
    pub major: u16,
    pub minor: u16,
}

pub struct SchemaVersion(pub u32);  // Monotonic per ability name
```

Rules:

1. **Additive output changes** (new optional field) bump `SchemaVersion` and `AbilityVersion.minor`. External callers declaring an older schema version receive the older shape (fields they don't know about are stripped by the registry before return). Internal callers always get the latest.
2. **Additive input changes** (new optional field with a default) bump `SchemaVersion` and `AbilityVersion.minor`. Required new inputs are breaking.
3. **Breaking changes** (renames, removals, semantic changes) require a new ability: `prepare_meeting_v2`. Both versions coexist in the registry until the deprecation window closes. The registry does not name a "default" version for external callers — they must declare which one they want. Internal callers compile against a specific typed function.
4. **Prompt changes are not ability version changes.** Prompt iteration is owned by the ability and evaluated per [ADR-0107](0107-evaluation-harness-for-abilities.md). A prompt regression is caught by eval, not by version.

External callers (MCP, REST) MUST include `schema_version` in every invocation. The registry rejects calls without it. This eliminates the "implicit latest" footgun where a breaking change silently returns a different shape to agents.

### 9. Invocation Rules

The following rules are enforced by review, by registry structure, and (in v1.4.x) by compile-time visibility constraints:

**Rule 1 — Synthesized user-facing and agent-facing outputs go through abilities.**  
If a consumer needs a structured brief, a narrative, composed entity context, a risk assessment, or any LLM-shaped output intended for a user or agent, it invokes an ability. Mechanical reads (a single field for a badge, a list of entity names for a picker, a timestamp render) remain in their owning service or command. The boundary test: if the output is synthesized or composed across multiple sources, it is an ability. If it is a single projection or trivial lookup, it is not.

**Rule 2 — Surfaces do not bypass the registry for erased invocation.**  
Tauri commands, MCP handlers, and future surfaces either invoke abilities through the registry (erased, validated, schema-checked) or through typed imports as described in §7.2. They do not construct `AbilityContext` themselves and do not invoke ability internals.

**Rule 3 — Abilities compose services, not the other way around.**  
Services ([ADR-0101](0101-service-boundary-enforcement.md)) do not call abilities. If a service needs synthesized context, the correct pattern is for the caller to invoke an ability and pass its output to the service.

**Rule 4 — Abilities compose abilities within category constraints.**  
A Transform may invoke Read. A Publish may invoke Transform and Read. Maintenance may invoke Read and Transform. **Maintenance may NOT invoke Publish unless `may_publish = true` is explicitly declared in the registry and justified by ADR amendment** — otherwise scheduled jobs could publish externally without user confirmation. Composition cycles are rejected at registration time (see §11).

**Rule 5 — Every ability output carries provenance via `AbilityOutput<T>`.**  
Provenance lives exactly once, on the wrapper. Domain output types never re-declare it. The mechanism is what makes depth explainable and quality measurable. Shape specified in [ADR-0105](0105-provenance-as-first-class-output.md).

**Rule 6 — Maintenance abilities operate under additional safety constraints.**  
Any ability in the Maintenance category is further bound by [ADR-0103](0103-maintenance-ability-safety-constraints.md). A maintenance ability that does not honor those constraints is non-conforming regardless of its compliance with Rules 1–5.

**Rule 7 — Transform outputs are untrusted for mutation authorization (prompt injection boundary).**  
Details in §10. The short version: output from a Transform ability (which may be shaped by attacker-influenced input via emails, transcripts, Glean-retrieved documents) cannot by itself authorize a mutation. A Maintenance or Publish ability that consumes a Transform output must require an additional trust signal (typed user confirmation, policy pre-authorization, or value falling within a declared safe range).

### 10. Prompt Injection Boundary

Transform abilities consume text from sources that are not under the user's control: email bodies, meeting transcripts, documents retrieved via Glean, CRM notes authored by third parties. Any of those sources can contain adversarial instructions aimed at the model. Without a clear trust boundary, an attacker could cause a Transform output to carry instructions that a downstream Maintenance or Publish ability naively obeys — exfiltrating data, mutating state, or triggering external writes.

#### 10.1 The Rule

Transform outputs are **untrusted data** for the purposes of mutation or external-write authorization. A Maintenance or Publish ability cannot accept a Transform output as the sole justification for a mutation or external call. One of the following must also hold:

1. **Typed user confirmation.** The Publish ability's caller presents a `ConfirmationToken` issued by a user-initiated UI flow. The token is scoped, single-use, and expires in 60 seconds.
2. **Policy pre-authorization.** The action was pre-authorized by a declared policy that the Transform output cannot modify (e.g., "auto-publish weekly narrative to P2 every Monday at 9am" is policy; "publish this specific draft now" is not).
3. **Schema-validated safe range.** The Transform output field is a typed value constrained to a safe enumeration or numeric range (e.g., `priority: Priority::Low | Normal | High`). The enum values must be defined such that *any* value in the range is acceptable without further authorization — i.e., the attack surface is already bounded by the enum's design. An attacker who compromises the Transform output can choose the worst option in the enum, so the enum itself must not include a value whose effects require additional trust. Free text is never sufficient. Numeric ranges must have explicit bounds that bound the worst-case outcome, not just the valid-call contract.

#### 10.2 Enforcement

- Transform outputs carry a `trust: Trust::Untrusted` marker in their provenance ([ADR-0105](0105-provenance-as-first-class-output.md)).
- Maintenance and Publish ability signatures that accept a Transform output type must also accept the confirmation or policy justification as a typed parameter; the registry rejects bindings that take a Transform output without the paired justification.
- Services that mutate in response to ability output check the `trust` marker. Attempting to mutate based on untrusted data without a confirmation returns `Error::UntrustedInput`.

#### 10.3 Separation of Concerns

- **Transform abilities** suggest, summarize, analyze. They do not decide.
- **Maintenance and Publish abilities** decide and act, but only on trusted inputs (user-confirmed, policy-authorized, or schema-constrained).
- This separation prevents a prompt injection from converting "summarize this email" into "publish this data to an attacker-controlled destination."

### 11. Composition Rules

Abilities compose other abilities to build richer capabilities. Composition introduces risks — cycles, transaction propagation, provenance merging, trace nesting — that must be specified.

#### 11.1 Cycle Detection

The registry validates the ability dependency graph at registration time. If ability A's call graph transitively reaches ability A, the registry refuses to bind and the build fails. Cycles are not permitted under any circumstance.

#### 11.2 Transaction Propagation

When a Maintenance ability invokes another ability while holding a `with_transaction_async` scope, the invoked ability **inherits the transactional service context** — its service calls participate in the same transaction automatically.

The type of `AbilityContext.services` does not change during a transaction; it remains `&ServiceContext<'a>`. What changes is the internal state: `ServiceContext` holds an `Option<TxHandle>` that is `Some` during an active transaction and `None` otherwise. Service mutation functions consult this internal handle: if present, they route the write through it; if absent, they write directly to the DB. This keeps the public type stable for composition (children just receive the same `&ServiceContext` reference) while letting the substrate route correctly. `with_transaction_async` sets the internal handle on entry and clears it on commit/rollback. This is propagation, not reentrancy: the child does not open a new transaction; it uses the parent's.

**Nested transactions are forbidden.** A child ability that calls `with_transaction_async` while one is already active returns `Error::NestedTransactionsForbidden` per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §4. Child abilities that need transactional writes rely on the parent's transaction or are refactored to be invoked outside a transactional scope.

**Publish inside a transaction is forbidden.** A Publish ability called inside a `with_transaction_async` scope fails at registration time — the registry flags the composition attempt. DailyOS does not cross transactional and external-mutation boundaries within a single call stack. If a maintenance ability needs to publish after its writes, it uses the outbox pattern specified in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §2: local writes inside the transaction, external call outside it with idempotency key.

ADR-0101's `with_transaction` is synchronous. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) introduces `with_transaction_async` for async composition. Maintenance abilities that require transactional composition use the async variant. Transform and Read abilities do not open transactions.

#### 11.3 Provenance Merging

When ability A invokes ability B, A's provenance envelope includes B's provenance as a nested child. The full provenance tree shows the composition. Deduplication rules and merge semantics specified in [ADR-0105](0105-provenance-as-first-class-output.md).

#### 11.4 Tracer Nesting

`AbilityTracer` records each ability invocation as a span. Composed invocations nest inside the caller's span. Traces form a tree rooted at the top-level invocation (the one initiated by a surface). This gives a full view of "the app asked for `get_daily_readiness`; that invoked `prepare_meeting` three times, `detect_risk_shift` twice, and `list_open_loops` once."

### 12. Intelligence Provider and Glean-First Orchestration

Two substrate ADRs constrain how abilities consume intelligence: [ADR-0091](0091-intelligence-provider-abstraction.md) (pluggable LLM providers) and [ADR-0100](0100-glean-first-intelligence-architecture.md) (Glean is the primary intelligence engine when connected).

#### 12.1 IntelligenceProvider Scope

ADR-0091 introduced `IntelligenceProvider` for the intel queue's LLM calls. This ADR expands its use to be the canonical LLM dependency for all Transform abilities. The scope expansion is intentional and supersedes ADR-0091's implicit limit to the intel queue. `AbilityContext.intelligence` is the only LLM entry point available to abilities; direct PTY construction or provider instantiation within an ability is forbidden. Pre-existing PTY call sites that ADR-0091 deliberately did not migrate remain permitted for non-ability code paths (ingestion, background processors) but are not accessible to abilities.

#### 12.2 Glean-First Orchestration

When Glean is connected, it is the primary engine for entity intelligence computation (health scoring, entity_assessment, relationship state). Transform abilities do NOT re-implement what Glean computes. Instead, they:

1. Call Read abilities (e.g., `get_entity_context`) that honor Glean-first retrieval — these return Glean-computed state when available, local fallback otherwise.
2. Compose that state into a user-facing or agent-facing synthesis (briefing, narrative, prep document) that is ability-specific, not engine-specific.

The division is: **Glean computes entity intelligence; abilities synthesize it into product-shaped outputs.** A Transform ability that replicates Glean's computation is non-conforming.

When Glean is not connected, Read abilities fall back to local PTY-based computation per [ADR-0100](0100-glean-first-intelligence-architecture.md). The Transform ability is unchanged — it composes whatever the Read returned. This keeps abilities engine-agnostic while preserving the Glean-first posture.

### 13. Enforcement Phases

**Phase 0 (prerequisite): Mode-aware services land.**  
[ADR-0104](0104-execution-mode-and-mode-aware-services.md) is accepted and implemented. `ServiceContext` carries `ExecutionMode`. Mutation functions honor the mode. Without this, §5 and §11.2 of this ADR are not enforceable.

**Phase 1 (v1.4.0 planning window): Design and catalog.**  
[ADR-0103](0103-maintenance-ability-safety-constraints.md) through [ADR-0109](0109-migration-strategy.md) land as Proposed. Initial ability catalog drafted. Eval harness scaffolding in place per [ADR-0107](0107-evaluation-harness-for-abilities.md). No implementation churn yet.

**Phase 2 (v1.4.0 refactor): Migrate highest-value capabilities first.**  
`prepare_meeting`, `get_daily_readiness`, `get_entity_context`, `list_open_loops`, `detect_risk_shift` migrate behind abilities. App Tauri commands become thin wrappers. MCP sidecar query handlers become thin wrappers over the same abilities. Parallel-run validation per [ADR-0109](0109-migration-strategy.md).

**Phase 3 (v1.4.0 cutover): Registry becomes authoritative.**  
Remaining capabilities migrated. Direct Tauri commands for capability-level operations removed. MCP query handlers removed in favor of ability-wrapped tools. Pre-commit hook blocks new capability code outside `src-tauri/src/abilities/`.

**Phase 4 (post-v1.4.0): Compile-time enforcement.**  
Ability invocation restricted by module visibility. Direct imports of ability modules from surface code become compile errors. Registry becomes the only entry point.

### 14. Migration from Current Architecture

The current architecture — Tauri commands that query services and build context inline, plus an MCP sidecar with parallel query paths — remains operational throughout the migration. Each capability is migrated individually:

1. A new ability is written in `src-tauri/src/abilities/`.
2. The Tauri command is replaced with a thin wrapper that invokes the ability.
3. The corresponding MCP tool is replaced with a thin wrapper over the same ability.
4. Parallel-run validation compares old and new outputs on live traffic until confidence is sufficient (detail in [ADR-0109](0109-migration-strategy.md)).
5. Old command handlers are removed.

Services per [ADR-0101](0101-service-boundary-enforcement.md) are untouched except for the mode-aware amendment in [ADR-0104](0104-execution-mode-and-mode-aware-services.md). The abilities layer sits above services, not beside them.

## Consequences

### Positive

1. **Depth compounds automatically.** A new signal source integrated into entity enrichment deepens every ability that composes entity context. No per-feature integration work.
2. **Capability quality becomes measurable.** Abilities are the unit of eval. Regressions are caught at ability granularity. Quality claims become numbers backed by eval scores.
3. **Surface parity is structural.** App, MCP, and any future surface receive identical outputs because they invoke the same abilities. No "MCP version is thinner."
4. **Explainability is a product feature, not an afterthought.** Every output carries provenance. Users and agents see why a thing surfaced. Trust is legible.
5. **Prompt iteration becomes disciplined.** Prompts are owned by abilities and evaluated per [ADR-0107](0107-evaluation-harness-for-abilities.md). Prompt changes ship with eval deltas.
6. **Agents receive full product quality.** MCP tools are thin wrappers over abilities, not reduced subsets. When an agent invokes DailyOS, it receives the same briefing the app renders.
7. **Contributor pattern is unambiguous.** The answer to "how do I add a capability?" becomes "write an ability." Category and location follow.
8. **Simulate and Evaluate modes enable offline work.** Abilities can be run against fixtures without touching live state, which makes development, debugging, and evaluation tractable.
9. **Versioning discipline enables safe evolution.** Breaking changes ship alongside the existing ability. A/B comparison is structural.

### Negative

1. **Short-term feature velocity cost.** During the v1.4.0 migration, new capability work must target the new pattern. Contributors will take longer to land the first few abilities.
2. **Contract surface area increases.** Every capability now has a named input struct, output struct, provenance, and eval fixtures. This is more code to maintain.
3. **Two implementations during migration.** Commands call both old paths and new abilities until cutover. Managed per [ADR-0109](0109-migration-strategy.md) but genuinely more work in the intermediate state.
4. **Discipline is required.** A command handler that bypasses an ability defeats the point. Enforcement must be real (hooks, review, eventual compile-time constraints).
5. **Provenance is not free.** Every ability must attach attribution to every output field. This is a design discipline that takes effort to apply consistently.

### Risks

1. **Contract scope creep.** A contributor makes `prepare_meeting` return "everything one might want" to avoid writing a second ability. Mitigation: abilities correspond to concrete product capabilities, not convenience bundles. Output shape is justified against user-visible or agent-visible use.
2. **Over-abstraction.** Generic abilities that try to be everything to everyone. Mitigation: if an ability has no user-visible output, it is almost certainly wrong. Read abilities that serve only other abilities are permitted but should be named specifically (`get_entity_recent_signals`, not `get_entity_data`).
3. **Premature versioning.** Teams create `v2` when minor iteration would suffice. Mitigation: versioning only on breaking output shape changes, documented in the ability file.
4. **Leaky abstractions.** A surface needs something the ability doesn't expose, and reaches past the ability to get it. Mitigation: if a surface needs something, the correct move is to expand the ability's output contract, not bypass the layer.
5. **Eval harness dependency.** Abilities only deliver their promised benefit if evaluated. Without [ADR-0107](0107-evaluation-harness-for-abilities.md) landing alongside this one, the discipline decays and quality becomes unmeasurable again. The eval harness is non-negotiable co-dependency.
6. **Migration stall.** Phase 2 requires holding other capability-level feature work steady for the duration. If product pressure breaks this, the refactor stalls half-complete. Mitigation: v1.4.0 is a dedicated version with no competing feature scope, per the roadmap decision.
7. **Hidden state in abilities.** An ability caches or memoizes in a way that makes it non-deterministic across runs. Mitigation: all caching lives in `AbilityContext` (tracer, mode-aware), not in ability locals. Eval mode is fully deterministic.

## References

- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — Services remain the mutation boundary. Abilities compose services; they do not replace them. Amended by [ADR-0104](0104-execution-mode-and-mode-aware-services.md) for mode awareness.
- [ADR-0091: IntelligenceProvider Abstraction](0091-intelligence-provider-abstraction.md) — Abilities consume `IntelligenceProvider` via `AbilityContext`. Scope is expanded by this ADR from the intel queue to all Transform abilities; see §12.1.
- [ADR-0082: Entity-Generic Prep Pipeline](0082-entity-generic-prep-pipeline.md) — The entity model is the substrate on which abilities operate. `get_entity_context` generalizes the prep pipeline.
- [ADR-0080: Signal Intelligence Architecture](0080-signal-intelligence-architecture.md) — Signals are the primary input to transform abilities. Provenance references signal IDs.
- [ADR-0098: Data Governance — Source-Aware Lifecycle](0098-data-governance-source-aware-lifecycle.md) — Data source tagging flows through into ability provenance.
- [ADR-0100: Glean-First Intelligence Architecture](0100-glean-first-intelligence-architecture.md) — Abilities orchestrate around Glean; they do not replace its computation. See §12.2.
- [ADR-0089: User Entity and Professional Context](0089-user-entity-and-professional-context.md) — `AbilityContext.user` carries professional context that shapes synthesis.
- **[ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md)** — Specifies the eleven structural guards that bind maintenance abilities. Co-dependency with this ADR; maintenance abilities without these constraints are non-conforming.
- **[ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md)** — Prerequisite. Amends ADR-0101 to make `ServiceContext` mode-aware. Required for §5, §11.2, and async transaction support.
- **ADR-0105 (forthcoming): Provenance as First-Class Output** — Specifies the provenance shape, field-level attribution, trust markers, merge semantics, and rendering contract referenced throughout this ADR.
- **ADR-0106 (forthcoming): Temporal Primitives in the Entity Graph** — Defines the trajectory types (engagement curves, role progressions, health trajectory) that abilities consume.
- **ADR-0107 (forthcoming): Evaluation Harness for Abilities** — Defines how abilities are scored, gated, and regressed. Co-dependency per Risk #5.
- **ADR-0108 (forthcoming): Surface-Independent Ability Invocation** — Specifies MCP and Tauri binding rules concretely (which follows from §7.3).
- **ADR-0109 (forthcoming): Migration Strategy — Parallel Run and Cutover** — Specifies the per-capability migration process, parallel-run validation, and cutover criteria.

---

## Amendment — 2026-04-20 — Error-handling contract + experimental ability flag

Two amendments addressing persona-review findings S2 (error handling contract) and A5 (evolution pattern friction).

### A. Error, warning, and soft-degradation contract (addresses S2)

The original ADR defines `AbilityResult<T> = Result<AbilityOutput<T>, AbilityError>` but does not specify how abilities handle partial failure, composition failure, or soft degradation. The distinction matters because surfaces render different things based on outcome class.

**Three outcome paths. Every ability uses exactly one per invocation:**

1. **Hard error** — ability returns `Err(AbilityError::...)`. Caller sees no output. Surface decides user-visible message per [ADR-0108](0108-provenance-rendering-and-privacy.md). No partial data leaks.

2. **Soft degradation** — ability returns `Ok(AbilityOutput<T>)` with `Provenance::warnings` populated. Output is usable; warnings tell the surface that parts were degraded. Examples: child Read timed out so context is thin; a stale source was included flagged as stale; LLM output anomaly was detected but the output itself still validated.

3. **Hard success** — ability returns `Ok(AbilityOutput<T>)` with `warnings` empty (or only informational markers). Full output, no caveats.

**Forbidden:** an ability silently logs an anomaly and returns "normal" output. Every anomaly is either hard-error-surfaced, warning-surfaced, or not worth recording. The log-and-proceed path that [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)'s critique flagged is explicitly prohibited.

**Composition failure semantics:**

When a composed ability (Transform composing a Read composing another Read) has a failure at any depth:

- **Default:** propagate as hard error upward. The composing Transform sees `Err` from the Read and returns `Err` itself.
- **Opt-in soft degradation:** a Transform can declare a composed Read as `optional` in its composition tree; a failure there is captured as a warning, not an error. Composition is declared explicitly; this is not the default for any composition.

The `warnings[]` field on Provenance is the canonical record of "what went soft." Surfaces render it per [ADR-0108](0108-provenance-rendering-and-privacy.md).

### B. `experimental = true` registry flag (addresses A5)

The ADR contract is designed for shipping abilities. It is heavy for prototyping. At AI-native velocity, trying a new ability shape should be hours, not days-of-ADR.

**Add:** a boolean `experimental` flag in the ability registry entry. When `experimental = true`:

- Provenance envelope requirement: minimal. A bare `Provenance::experimental()` constructor is permitted with only `invocation_id` + `produced_at` populated.
- Fixture requirement: waived. No eval harness fixture required to register.
- Category enforcement: waived. Call-graph-based category classification is still logged for observability but does not block registration.
- Surface exposure: experimental abilities are registered and invokable only when a feature flag is active. Not exposed through MCP. Not exposed through Tauri commands unless the dev-mode flag is set.
- Lifespan: one cycle maximum. An `experimental` ability either graduates to non-experimental (full contract compliance) within one v1.x.y release or is removed from the registry.
- Trust score: forced to zero. Claims produced by an experimental ability have `TrustAssessment::experimental = true`; downstream consumers know not to treat them as authoritative.

Rationale: exploration needs a fast path. Discipline comes from graduation (promotion requires full ADR compliance), not from blocking experimentation entirely. The one-cycle lifespan prevents "experimental" from becoming a permanent escape hatch.

**Tracking:** a registry query `experimental_abilities()` returns currently experimental abilities and their registration date. Anything older than one cycle is flagged for graduation-or-removal review.
