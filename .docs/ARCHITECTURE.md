# DailyOS Architecture

**Audience:** Any contributor (human or AI session) picking up work. One-click reach from the repo root.
**Date:** 2026-04-20 | **Reflects:** ADRs 0100–0120, v1.4.0 substrate in progress
**Purpose:** Read this first. It will orient you in 10 minutes. Every detail is linked to its source of truth.

## What DailyOS is

DailyOS is a native macOS app (Tauri + React) acting as a personal chief of staff for Customer Success. The product promise: open the app, your day is already assembled with more context than any competitor. Depth-before-interaction is the moat.

Single user per install. Local-first. Content encrypted at rest ([ADR-0092](decisions/0092-data-security-at-rest-and-operational-hardening.md)). Data plan at scale: **per-user SQLite forever** (founder decision 2026-04-20, [D1](strategy/2026-04-20-v1.4.0-architectural-strategy.md#signature-block)). Team features, if ever needed, accept migration cost.

The intelligence layer is built as an **AI harness** — infrastructure around an LLM that assembles context, verifies output, stores trusted claims, and learns from feedback. The harness thesis ([ADR-0118](decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)) is that harness quality dominates model capability for long-horizon tasks.

## The shape at a glance

```
┌────────────────────────────────────────────────────────────┐
│  Surfaces                                                  │
│  Tauri UI · MCP server · Background workers · Eval fixtures│
├────────────────────────────────────────────────────────────┤
│  Abilities runtime (ADR-0102)                              │
│  Read · Transform · Publish · Maintenance                  │
│  All invoked via registry. Mandatory provenance.           │
├────────────────────────────────────────────────────────────┤
│  Services layer (ADR-0101, ADR-0104)                       │
│  All mutations here. Mode-aware ServiceContext.            │
│  check_mutation_allowed() guards every write.              │
├────────────────────────────────────────────────────────────┤
│  Substrate                                                 │
│  Claims (0113) · Trust Compiler (0114) · Signals (0115)    │
│  Temporal primitives (0109) · Provenance (0105)            │
│  Source taxonomy (0107) · Prompt fingerprint (0106)        │
├────────────────────────────────────────────────────────────┤
│  Storage                                                   │
│  Encrypted SQLite (ADR-0092) · LocalKeychain (ADR-0116)    │
└────────────────────────────────────────────────────────────┘
```

Dependency order (ship first to last): [ADR-0101](decisions/0101-service-boundary-enforcement.md) Services + [ADR-0104](decisions/0104-execution-mode-and-mode-aware-services.md) ExecutionMode → [ADR-0102](decisions/0102-abilities-as-runtime-contract.md) Abilities + [ADR-0105](decisions/0105-provenance-as-first-class-output.md) Provenance → everything else in dependency order per strategy doc.

## The substrate, in plain English

### Claims ([ADR-0113](decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md))

Facts about entities — "Alice is the champion at Acme," "renewal is 45 days out," "ARR is 320K." Every claim has:

- An **actor** (`user`, `agent:name:version`, `human:role:id`, `system:component`, `external:salesforce`).
- A **state** (`proposed`, `committed`, `tombstoned`, `superseded`, `withdrawn`).
- A **trust score** (computed by the Trust Compiler).
- **Append-only history** — when a claim changes, a new row supersedes the old; nothing overwritten.

Stored in `intelligence_claims` table. Agents propose; users commit. Or agents commit when enough sources corroborate. When a user removes a value, a **tombstone** claim records the intent — no agent can silently repopulate.

### Trust ([ADR-0114](decisions/0114-scoring-unification.md) + DOS-5 Trust Compiler)

Every claim gets a computed `trust_score` from six factors: source reliability, freshness, corroboration, contradiction, user feedback, meeting relevance. Score → band (`likely_current` / `use_with_caution` / `needs_verification`). Pure functions; deterministic under fixed inputs.

### Signals ([ADR-0080](decisions/0080-signal-intelligence-architecture.md), [ADR-0115](decisions/0115-signal-granularity-audit.md))

Events — "EntityUpdated," "ClaimAsserted," "ClaimTrustChanged," "AbilityOutputChanged." Every signal has a typed propagation policy from a compile-time-checked registry. Durable invalidation jobs replace depth-limited-drop. Coalescing and rate limits keep the bus safe under load.

### Abilities ([ADR-0102](decisions/0102-abilities-as-runtime-contract.md))

Every product capability (`prepare_meeting`, `get_entity_context`, `detect_risk_shift`, `publish_to_p2`) is a named, typed, versioned function in one of four categories. All surfaces invoke through one registry; no bespoke command handlers.

- **Read** — no mutation anywhere in call graph.
- **Transform** — composes Reads + invokes LLM; no mutation; outputs are untrusted for auto-commit.
- **Publish** — writes externally (P2, S3, webhook); Pencil/Pen two-phase with `ConfirmationToken`.
- **Maintenance** — mutates internal state; subject to safety constraints in [ADR-0103](decisions/0103-maintenance-ability-safety-constraints.md).

The `experimental = true` flag ([ADR-0102 Amendment](decisions/0102-abilities-as-runtime-contract.md#amendment--2026-04-20--error-handling-contract--experimental-ability-flag)) lets you prototype new abilities without full provenance + fixture + category compliance. One cycle to graduate or remove.

### Provenance ([ADR-0105](decisions/0105-provenance-as-first-class-output.md))

Every ability output carries a `Provenance` envelope — identity, temporal context, source attribution, composition tree, field-level attribution, prompt fingerprint ([ADR-0106](decisions/0106-prompt-fingerprinting-and-provider-interface.md)), trust assessment. Surfaces render per [ADR-0108](decisions/0108-provenance-rendering-and-privacy.md). Hard cap: 64 KB serialized ([ADR-0108 Amendment](decisions/0108-provenance-rendering-and-privacy.md#amendment--2026-04-20--enforce-64-kb-provenance-size-cap)).

### Evaluation ([ADR-0110](decisions/0110-evaluation-harness-for-abilities.md), [ADR-0119](decisions/0119-runtime-evaluator-pass-for-transform-abilities.md))

Two layers:

- **Test-time** ([ADR-0110](decisions/0110-evaluation-harness-for-abilities.md)): hermetic fixtures score abilities with exact-match (Read), LLM-as-judge (Transform), or snapshot-diff (Maintenance). CI gate.
- **Runtime** ([ADR-0119](decisions/0119-runtime-evaluator-pass-for-transform-abilities.md)): optional lightweight evaluator pass after opt-in Transform abilities. Same rubric as test-time. Low scores trigger critique-based retry.

Plus: harness-stripping fixtures ([ADR-0110 §9](decisions/0110-evaluation-harness-for-abilities.md#amendment--2026-04-19--9-harness-stripping-fixtures-closes-adr-0118-gap-b)) test whether each scaffold still earns its keep as models improve.

### Observability ([ADR-0120](decisions/0120-observability-contract.md))

Every invocation emits an `InvocationRecord` with `invocation_id`, timing, outcome, and correlation. NDJSON to stderr. `caused_by_invocation_id` threaded through `signal_events`, `intelligence_claims`, `evaluation_traces`. Debug trace surface ([DOS-250](https://linear.app/a8c/issue/DOS-250)) consumes them.

### Publish ([ADR-0117](decisions/0117-publish-boundary-pencil-and-pen.md))

Two-phase protocol: Pencil (reviewable draft) → Pen (irreversible commit via outbox with idempotency keys). User-initiated push to destinations the user configures. **Strategically load-bearing for enterprise:** it's how "leadership wants team visibility" gets answered without softening [ADR-0116](decisions/0116-tenant-control-plane-boundary.md)'s boundary.

### Control plane boundary ([ADR-0116](decisions/0116-tenant-control-plane-boundary.md))

Firm founder commitment (D2, 2026-04-20): "metadata only, content never." Any future server component sees user identity + capability grants + aggregate telemetry. It never sees user content, claims, prompts, or responses. Softening requires founder approval + named compensating control.

## Where to look

| If you want to know... | Look at |
|---|---|
| The decision behind X | `.docs/decisions/NNNN-*.md` + `.docs/decisions/README.md` index |
| The strategic shape | `.docs/strategy/2026-04-20-v1.4.0-architectural-strategy.md` |
| Senior-engineer and systems-architect findings | `.docs/strategy/2026-04-20-persona-reviews.md` |
| What it takes to go from decision to execution | `.docs/strategy/2026-04-20-execution-readiness.md` |
| Data model — what tables exist, what columns, relationships | `.docs/architecture/DATA-MODEL.md` |
| Module map — where code lives | `.docs/architecture/MODULE-MAP.md` |
| Consistency model per operation class | `.docs/architecture/CONSISTENCY.md` |
| Failure modes and degradation behavior | `.docs/architecture/FAILURE-MODES.md` |
| Version numbers — when to bump which | `.docs/architecture/VERSIONING.md` |
| Data flows through the system | `.docs/architecture/DATA-FLOWS.md` |
| Lifecycle state machines | `.docs/architecture/LIFECYCLES.md` |
| How to write a good issue or ADR | `.docs/SPEC-TEMPLATE.md` |
| CLI commands, development workflow | `CLAUDE.md` |

## How to contribute

### Starting a new issue

Every v1.4.0+ issue uses [SPEC-TEMPLATE.md](SPEC-TEMPLATE.md). Core block + one of seven shape-specific blocks (new capability, schema change, migration, bug fix, refactor, research spike, prompt edit). Mark `spec:needs-review` when drafted; another engineer walks the 12-surface architectural vet list before `spec:ready`.

### Starting a new capability (ability)

1. Determine category: Read / Transform / Publish / Maintenance ([ADR-0102 §3](decisions/0102-abilities-as-runtime-contract.md)).
2. Implement in `src-tauri/src/abilities/<category>/<name>.rs`.
3. Output wraps in `AbilityOutput<T>` with populated `Provenance` ([ADR-0105](decisions/0105-provenance-as-first-class-output.md)).
4. Mutations go through services only ([ADR-0101](decisions/0101-service-boundary-enforcement.md)).
5. At least one eval fixture ([ADR-0110](decisions/0110-evaluation-harness-for-abilities.md)).
6. Register with `#[ability]` macro; observability spans open automatically ([ADR-0120](decisions/0120-observability-contract.md)).
7. Surface-agnostic — invokable through Tauri + MCP via [ADR-0111](decisions/0111-surface-independent-ability-invocation.md).

Prototyping something new? Mark `experimental = true` in the registry ([ADR-0102 Amendment B](decisions/0102-abilities-as-runtime-contract.md#b-experimental--true-registry-flag-addresses-a5)). Less ceremony; one cycle to graduate or remove.

### Starting a new mutation

Services layer only. Function takes `&ServiceContext`. First line: `ctx.check_mutation_allowed()?`. Emit signal inside the service call (signal emission is service responsibility, not caller's). No `Utc::now()` or `rand::thread_rng()` — clock and RNG come from `ctx`. Respects [ADR-0101](decisions/0101-service-boundary-enforcement.md), [ADR-0104](decisions/0104-execution-mode-and-mode-aware-services.md).

### Starting a new signal type

1. Add variant to `SignalType` enum in `src-tauri/src/signals/types.rs`.
2. Register policy in `signals/policy_registry.rs` — the exhaustiveness check forces you to.
3. Declare coalescing key per [ADR-0115 R1.5](decisions/0115-signal-granularity-audit.md#r15-coalescing-key--add-granularity).
4. If `PropagateSync`, decide `await_completion` per [ADR-0115 R2](decisions/0115-signal-granularity-audit.md#revision-r2--2026-04-20--propagatesync--await_completion--variant).

### Adding a new claim source

[ADR-0107](decisions/0107-source-taxonomy-alignment.md) `DataSource` enum is the source of truth. Adding a new source requires ADR amendment. Scoring class + lifecycle behavior declared at the enum variant level.

### Adding a new actor class

Extends `ClaimActor` per [ADR-0113 R1.5](decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r15-actor-representation--align-with-adr-0102-actor-enum). Requires ADR-0102 amendment and every scoring path to handle the new variant.

### Adding a new publish destination

New `DestinationClient` implementation in `src-tauri/src/publish/clients/`. Pencil/Pen protocol unchanged. No [ADR-0117](decisions/0117-publish-boundary-pencil-and-pen.md) amendment required — destinations are extensions.

### Adding observability

Usually nothing extra. `#[ability]` macro and `#[instrument]` on service functions do it automatically. If adding a new storage table that could correlate to an invocation, include `caused_by_invocation_id` per [ADR-0120](decisions/0120-observability-contract.md). Don't log prompt or response content — logs carry shape, not content.

## Current state of the rebuild

v1.4.0 substrate is **landing in progress** as of 2026-04-20. ADRs are in writing; most amendments are applied. Code implementation begins after two hard blockers land:

- [DOS-209](https://linear.app/a8c/issue/DOS-209) ServiceContext (Phase 0 of [ADR-0104](decisions/0104-execution-mode-and-mode-aware-services.md)).
- [DOS-259](https://linear.app/a8c/issue/DOS-259) IntelligenceProvider trait extraction ([ADR-0091](decisions/0091-intelligence-provider-abstraction.md) + [ADR-0106](decisions/0106-prompt-fingerprinting-and-provider-interface.md)).

After those, the end-to-end slice on `get_entity_context` lands first ([strategy § Path forward](strategy/2026-04-20-v1.4.0-architectural-strategy.md#path-forward) action 1). That's the proof the substrate works end-to-end.

Brownfield-as-greenfield posture: users are minimal, so backward-compat shims are not required. Aggressive replacement is preferred over incremental migration where it simplifies the final shape.

## Ten principles worth remembering

From [ADR-0118](decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md):

1. Context engineering over context stuffing (dimension splits, scoped prompts).
2. Durable external state as memory (memory lives in SQLite, not context window).
3. Verification before trust (validation, consistency checks, TrustAssessment).
4. Generator/evaluator separation (ADR-0110 test-time + ADR-0119 runtime).
5. Subjective quality is gradable (`quality.toml` rubrics).
6. Typed capability surface (ability categories, services boundary).
7. Observability per invocation (ADR-0120).
8. Model-provider abstraction (IntelligenceProvider trait).
9. Source attribution as output (Provenance `field_attributions`).
10. Feedback-learning loop (Thompson sampling on source weights, Trust Compiler factors).

If a design you're considering violates one of these, the burden is on the design to justify why. Usually it's a smell.

## Answers you'll want

**"Where does X happen?"** Start with DATA-FLOWS.md for the bigger flows (enrichment, meeting prep, signal propagation, publish). For specific functions, MODULE-MAP.md.

**"Why was X decided?"** The ADR for X. Use `.docs/decisions/README.md` as the index. Every decision has a context section naming the forces in play.

**"What state is X in right now?"** In code: the module. In decisions: the ADR's `Status:` line (Proposed / Accepted / Superseded). In rollout: the Linear project + issue status.

**"Is this safe to change?"** Check SPEC-TEMPLATE.md's 12-surface architectural vet list. If the change touches claims or provenance or services, yes it's substrate; go carefully. If it touches rendering or a UI surface only, it's mostly contained.

**"How do I run tests?"** `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` — every CLAUDE.md says this and every PR enforces it.

## The north star

Users never open a DailyOS briefing and find a fact that's silently wrong, silently stale, or silently fabricated. Every field knows where it came from (provenance). Every field knows how much to trust itself (TrustAssessment). Every user correction is honored (tombstones, reversibility). Every AI claim is measurable (rubric + evaluator + harness-stripping).

The substrate exists to make that experience structural, not aspirational.
