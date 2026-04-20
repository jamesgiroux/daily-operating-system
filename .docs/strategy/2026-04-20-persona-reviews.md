# Persona Reviews — v1.4.0 Architecture

**Date:** 2026-04-20
**Reviewers:** Senior engineer persona, systems architect persona
**Scope:** ADRs 0100–0119, strategy doc, and the aggregate substrate design
**Status:** Draft — findings not yet addressed

Two complementary reviews. Senior engineer is grounded in day-1 shipping reality — what breaks when the code lands. Systems architect is grounded in the shape of the system — where the fault lines will open as the product grows. Both were run with explicit permission to use the brownfield-with-greenfield-license posture: backward compatibility is not a constraint if we can rip it out cleanly.

---

## Senior engineer review

Voice: I have to ship this and debug it when it breaks. What are the day-1 concerns?

### S1 — Day-1 observability plan is missing

[ADR-0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) creates `evaluation_traces` in v1.5.0. [DOS-250](https://linear.app/a8c/issue/DOS-250) builds the debug trace surface in v1.5.0+. In the meantime the runtime evaluator doubles LLM calls on Transform paths, the Trust Compiler lands in production, and the claims substrate becomes load-bearing.

When something goes wrong in the first week of v1.4.0 behind the scenes — a trust score that doesn't match intuition, a claim that got tombstoned unexpectedly, an agent whose output is getting rejected too often — what do I grep? `RUST_LOG=debug` and hope? No log schema is defined. No correlation IDs are threaded. `invocation_id` exists in provenance but nothing says log lines carry it.

**Fix:** Before v1.4.0 ships substrate, define a minimum structured-log schema that every ability invocation emits. Three fields at a bare minimum: `invocation_id`, `ability_name`, `ability_version`. Nice-to-have: `entity_id`, `actor`, `duration_ms`. Write it to stderr in JSON so `jq` works. Later upgrade to a proper log sink; not this cycle.

### S2 — Error handling contract is implicit

Abilities return `Result<AbilityOutput<T>, AbilityError>`. Happy path is obvious. What about:

- A Transform ability whose composed Read ability errored halfway through context assembly. Does it return partial output, hard error, or "output with degraded provenance"?
- A Transform ability whose LLM call timed out. Retry budget? Surface error? Silent fallback?
- Consistency-repair-retry fails a second time. Ship anomalous output? Hard error? Log and proceed?

Provenance has `warnings: Vec<ProvenanceWarning>`. The relationship between `warnings` and `AbilityError` is undefined. Recommend:

- **Hard error path:** ability fails, caller sees `AbilityError`. Surface decides user-visible message.
- **Soft degradation path:** ability returns `AbilityOutput` with `warnings` populated. Surface decides whether to show or hide.
- Contract: no ability silently logs-and-succeeds. Every failure is either a hard error or a warning-flagged soft output.

### S3 — Performance budgets named but not enforced

[ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md) §6 mentions size budgets. Nothing enforces them. A deeply-composed ability (meeting prep composing context which composes claims which compose trajectories) can build a several-hundred-KB provenance envelope. Tauri IPC has limits; we'll hit them.

**Fix:** Hard cap. `Provenance` serialized > 64 KB returns `AbilityError::ProvenanceTooLarge`. Ship with the cap; raise it only after measuring what real composition looks like. Forces either provenance summarization, shallower composition, or conscious increase.

### S4 — Testing beyond fixtures

[ADR-0110](../decisions/0110-evaluation-harness-for-abilities.md) is fixture-based per-ability scoring. Missing:

- **Property tests on factor primitives.** [ADR-0114](../decisions/0114-scoring-unification.md) has a factor library with clamping and monotonicity invariants. Property-test with 10K random tuples. Cheap at AI velocity.
- **Integration tests across abilities.** Meeting prep composes six Read abilities. Does the composition work? Test it end-to-end against fixtures.
- **Chaos tests on the signal propagator.** Inject failures (transaction rollback mid-invalidation, cycle detection triggered, dead-letter queue full). Does the system degrade gracefully?
- **Migration idempotency.** Re-run a migration on a migrated DB; verify no-op. Trivially cheap to add; prevents a whole class of ship-time panics.

### S5 — Memory lifecycle and bounds

- `agent_trust_ledger` (~200 rows expected). Acceptable to load-into-memory? Cache eviction? Reload on ledger update?
- Signal ring buffer in Evaluate mode ([ADR-0115](../decisions/0115-signal-granularity-audit.md) §8): bounded by what? Reasonable default 1024?
- Provenance composition tree depth: invalidation jobs have a cap (16); provenance composition doesn't. A deeply-composed ability can nest provenance arbitrarily. Add the same cap.
- `evaluation_traces` retention: 90 days named. Prune job? Single maintenance ability? Default + config?

### S6 — Concurrency primitives

When two abilities call `commit_claim` on the same `(entity_id, claim_type, field_path)` concurrently:

- SQLite's serializable isolation prevents corruption.
- But actor-based routing (supersede vs contradiction per [ADR-0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) R1.2) is decided at read time. Two concurrent writers could both read "no existing claim" and both insert, both succeed, both uniqueness-violate, both retry.

**Fix:** Either (a) optimistic concurrency with conflict resolution on retry, or (b) pessimistic row-lock on the target field_path during commit_claim. (b) is simpler; (a) scales better. At current volume, (b) is the right call. Document it.

### S7 — Migration practice

- ~10 new migrations expected in v1.4.0. The existing `pre-migration.bak` backup pattern is solid.
- Missing: a migration testing convention. Every migration should have a test that applies it to a fixture DB and asserts schema shape.
- Missing: migration reversal. Not every migration needs `DOWN` SQL, but state-destroying migrations should be annotated as such with explicit approval.
- Brownfield-as-greenfield unlock: we can write destructive migrations that blow away the 27 JSON columns on `entity_assessment` after backfilling to claims. R1 deferred this; no good reason to defer given the user count.

### S8 — Breaking changes to traits

[ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) amends `IntelligenceProvider`. Current implementors need updating. No versioning strategy. Brownfield-as-greenfield: delete the old trait shape, update all implementors, ship. No `DeprecatedIntelligenceProvider` compatibility shim.

Same for `ActionDb::open` compatibility wrapper in [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md) R1.3. If we have greenfield license, replace all call sites with `open_with_provider()`; drop the wrapper. Cleaner.

### S9 — Structured logging and correlation

`invocation_id` exists in provenance. Nothing threads it through log lines. An incident investigation in production requires stitching logs by timestamp and hoping.

**Fix:** `tracing` crate spans, with `invocation_id` as a span attribute. Every span inherits automatically. All log lines within the span carry it. `jq '.span.invocation_id == "..."'` filters to a single invocation's timeline. Standard Rust practice; ~1 hour of work.

### S10 — Config loading

`config/scoring.toml`, `config/trust_compiler.toml`, eval harness `quality.toml`. Where do these live at runtime? Tauri app support directory? Bundled in the binary? What's the fallback if config is missing? Hot-reload?

No ADR specifies. Needs a decision before v1.4.0:

- Bundled compiled defaults in the binary.
- User config at `$APP_SUPPORT/dailyos/config/` overrides.
- Boot-time validation fails fast on malformed config; falls back to defaults with WARN log.
- No hot-reload in v1.4.0 (restart to apply).

---

## Systems architect review

Voice: I think about the shape of the system and where it breaks at scale. What are the structural issues?

### A1 — Bounded contexts identifiable but heavy coupling

Reading the ADRs, I can name the contexts: Claims, Signals, Abilities, Surfaces, Intelligence Provider, Source Taxonomy, Evaluation, Publishing. Clean enough.

But coupling is heavy. Every context transitively depends on Claims + Provenance. The Abilities context is the aggregator-of-aggregators. Extracting a context to a separate service later — for multi-tenant, for a Glean-free distribution, for an embedded variant — would be architecturally invasive.

**Is this a problem right now?** No. Single-user native app. Coupling is acceptable.

**When does it become a problem?** Two scenarios: (a) multi-user where different users need different intelligence-provider configs per tenant, (b) an embedded "DailyOS lite" without Glean that ships with a subset. Either breaks current coupling.

**Recommendation:** Note the coupling explicitly in `ARCHITECTURE.md` (when it ships) as a known constraint with a stated "if we need to extract, here is how." Don't preemptively decouple; do acknowledge.

### A2 — Data gravity risk

Everything in a single encrypted SQLite file. Per-user today. At:

- **1 user:** fine. Current state.
- **6 users (pilot):** fine.
- **100 users multi-tenant-shared:** file becomes the bottleneck. Needs per-tenant DB or extraction to Postgres.
- **1000 users:** single-file SQLite is dead regardless.

[ADR-0116](../decisions/0116-tenant-control-plane-boundary.md) implicitly assumes per-user local DB persists into the multi-user world. [ADR-0099](../decisions/0099-remote-first-server-canonical-architecture.md) is marked Withdrawn. There is no stated multi-user data plan.

**Decision needed:** what is the data plan at 100 users? Per-user separate SQLite files synced through a control plane? Per-tenant Postgres? Shared with tenant_id filtering? Each choice has different implications for what tables we add now.

This is the most consequential architectural decision not yet made. Every new table added under v1.4.0 entrenches the default ("per-user SQLite file") by one more increment.

### A3 — Event flow is one-way, no request/response at signal layer

Signal bus is fire-and-forget. If an ability needs to know "did my signal emission cause the downstream invalidation to complete before I render," there's no way to wait. Everything async past the signal.

This makes rollback reasoning hard. If I commit a claim, emit `ClaimAsserted`, render the briefing, and *then* the invalidation job dead-letters, my briefing is stale and I don't know.

**Is this acceptable?** For most flows, yes. Eventual consistency is the right model for a synthesis app. For *some* flows — user just corrected a claim, they expect to see their correction in the next briefing immediately — it's not.

**Recommendation:** Add an explicit "synchronous propagation" policy variant. [ADR-0115](../decisions/0115-signal-granularity-audit.md) already has `PropagateSync` but the caller doesn't get to wait on the propagation completing. Extend: `PropagateSync { await_completion: bool }`.

### A4 — Consistency model is mixed and undocumented

Within a transaction: strong. Across transactions: async through invalidation jobs. Runtime evaluator retries in-transaction. Publish outbox: at-least-once with idempotency keys. Eval harness: deterministic fixture replay.

Four different consistency models in one system. No single doc names where each applies. A new contributor has to reason about each ADR independently.

**Fix:** Add a top-level CONSISTENCY.md (or equivalent section in ARCHITECTURE.md) that states per operation class:

- Claim reads: strongly consistent within transaction, eventually consistent across.
- Claim writes: strongly consistent (SQLite serializable).
- Signal propagation: eventually consistent, bounded by invalidation queue processing time.
- Trust score computation: deterministic given inputs at time T; re-computable.
- Publish: at-least-once delivery, exactly-once side-effect via idempotency keys.
- Evaluator retry: single retry in same transaction.

### A5 — Evolution pattern is ADR-heavy

Adding an ability: clear, well-documented. Adding a signal type: clear post-enum. Adding an actor class: less clear (enum extension, every scoring path needs updating). Adding a data source: [ADR-0107](../decisions/0107-source-taxonomy-alignment.md) requires ADR amendment. Adding a new claim_type: undefined.

The ADR-per-extension pattern is good for discipline. It's bad for prototyping. When we want to try something ("does a per-meeting trajectory help?"), we're writing an ADR before writing code. That's heavyweight.

**Recommendation:** Explicit "experimental" tier. An ability marked `experimental = true` in its registry entry can be added without provenance rendering, without fixtures, without category enforcement. Intended lifespan: one cycle. Promotion to non-experimental requires full ADR compliance. Gives us a velocity path for exploration without compromising substrate discipline.

### A6 — Failure domains undefined

Glean down → what still works? [ADR-0100](../decisions/0100-glean-first-intelligence-architecture.md) says PTY fallback. Does the user see "Glean unavailable, falling back"? Silent? Degraded?

Consistency-repair-retry fails → ability logs anomaly, ships output. Is this desired?

Runtime evaluator judge model unavailable → sample skipped, primary ships. Is this desired?

SQLite locked for writes → ability write fails; what does the caller render?

**Fix:** A short failure-mode matrix:

| Failure | What still works | What user sees | Recovery |
|---|---|---|---|
| Glean down | PTY fallback | Latency warning | Auto-recover on Glean health |
| PTY unavailable | Glean-only path | Error on non-Glean entities | Manual retry |
| Consistency check fails | — | Degraded output marker | Run "refresh" to retry |
| ... | ... | ... | ... |

One page. Saves hours of "what did the system do" investigation later.

### A7 — Observability is a cross-cutting concern, not a component

Some ADRs produce telemetry ([ADR-0105](../decisions/0105-provenance-as-first-class-output.md) provenance; [ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) fingerprints; [ADR-0110](../decisions/0110-evaluation-harness-for-abilities.md)/[ADR-0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) eval traces). Others are silent ([ADR-0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) claims; [ADR-0114](../decisions/0114-scoring-unification.md) scoring; [ADR-0115](../decisions/0115-signal-granularity-audit.md) signals). There is no cross-cutting observability contract.

**Fix:** ADR-0120 (or equivalent) defining what every ability and every service function emits uniformly. Minimum: structured log entry with invocation_id, timing, outcome. Consumed by debug trace surface ([DOS-250](https://linear.app/a8c/issue/DOS-250)) and future ops tooling. Don't let observability become "whatever each module does."

### A8 — Operational readiness for single-user to multi-user transition

Today: native app, single user, restart = recovery. No on-call concern.

Tomorrow (v2.x multi-user): someone is on call. What's the runbook? Where do logs go? How do you tell a user "your claim graph is corrupted" vs "your device has a clock skew issue"? The ADRs are structurally correct but operationally unfurnished.

**Recommendation:** Not urgent for v1.4.0. Flag as explicit H3 work: "ops readiness review" as a checkpoint before multi-user. Don't pretend it's done.

### A9 — Version number proliferation

Four version numbers on a single ability output:

1. `ability_version` — the ability's semver.
2. `ability_schema_version` — the ability's I/O schema version.
3. `prompt_template_version` — the prompt template's semver.
4. `provenance_schema_version` — the Provenance envelope's schema version.

Plus `trust_version` on claims. Plus migration version on DB.

Each was introduced for a defensible reason. Aggregated, evolution rules are confusing. A contributor who bumps the wrong one ships a subtle incompatibility.

**Fix:** A short "when do I bump which version" guide. Part of `ARCHITECTURE.md` or a standalone `VERSIONING.md`. 10 minutes of writing; saves cycles of "which version did I need to bump."

### A10 — Cross-cutting concerns coherence

The substrate ADRs treat privacy, security, observability, and performance unevenly:

- **Privacy:** [ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md) (rendering), [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md) (boundary). Coherent.
- **Security:** [ADR-0092](../decisions/0092-data-security-at-rest-and-operational-hardening.md) (at-rest), [ADR-0093](../decisions/0093-prompt-injection-hardening.md) (prompt injection). Coherent.
- **Observability:** scattered across many ADRs, not cross-cutting.
- **Performance:** barely named. No performance ADR.

**Recommendation:** Accept that observability and performance are currently ad-hoc. Prioritize observability (A7) before performance. Performance work is premature until we have the observability to measure it.

---

## Summary of findings — what needs to happen

| ID | Area | Urgency | Action |
|---|---|---|---|
| S1 | Day-1 observability | **High** | Define minimum log schema; thread `invocation_id` via `tracing` spans. Before v1.4.0 ships. |
| S2 | Error handling contract | High | Define warnings vs errors path. Before first ability lands. |
| S3 | Provenance size cap | High | Hard cap 64 KB. Before first composed ability. |
| S4 | Testing beyond fixtures | Medium | Property tests on factor library; chaos tests on signal propagator. H1. |
| S5 | Memory bounds | Medium | Depth cap on provenance composition; ring-buffer bounds. H1. |
| S6 | Concurrency on claim commits | **High** | Pessimistic row-lock during commit_claim. Before first concurrent use. |
| S7 | Migration testing | Medium | Idempotency test per migration. Standard for new migrations. |
| S8 | Breaking-change strategy | Low | Brownfield-as-greenfield: replace, don't shim. Already aligned. |
| S9 | Structured logging | **High** | `tracing` spans with correlation. 1 hour of work. |
| S10 | Config loading | **High** | Decide location + fallback + no-hot-reload. Before config-consuming code lands. |
| A1 | Coupling | Low | Acknowledge in ARCHITECTURE.md. |
| A2 | Data gravity | **High** | Decide the 100-user data plan. Before more substrate tables entrench the default. |
| A3 | Event flow | Medium | Add `await_completion` to PropagateSync. |
| A4 | Consistency model | Medium | Single CONSISTENCY.md doc. |
| A5 | Evolution pattern | Medium | `experimental = true` registry flag. |
| A6 | Failure domains | **High** | Failure-mode matrix doc. 1 page. |
| A7 | Observability contract | **High** | ADR for uniform observability emission. Foundational for S1 + S9. |
| A8 | Ops readiness | Low | Flag as H3. |
| A9 | Version proliferation | Low | VERSIONING.md guide. 10 minutes. |
| A10 | Cross-cutting concerns | Low | Prioritize observability over performance. Accept performance is ad-hoc. |

**High-urgency list (must happen before v1.4.0 ships substrate):** S1, S2, S3, S6, S9, S10, A2, A6, A7.

Nine items, mostly sized in hours at AI velocity. The one requiring judgment is A2 (the 100-user data plan) — that's a founder decision.

---

## What this means for the ADRs

- **New ADR required: ADR-0120 Observability Contract.** Addresses A7, S1, S9. Foundational.
- **Amendment to [ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md):** hard 64 KB cap + `AbilityError::ProvenanceTooLarge`. Addresses S3.
- **Amendment to [ADR-0102](../decisions/0102-abilities-as-runtime-contract.md):** define warning vs error vs soft-degradation contract. Addresses S2. Also add `experimental = true` registry flag (A5).
- **Amendment to [ADR-0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md):** pessimistic row-lock in commit_claim. Addresses S6.
- **Amendment to [ADR-0115](../decisions/0115-signal-granularity-audit.md):** `PropagateSync { await_completion }` variant. Addresses A3.
- **New ADR required: ADR-0121 Data Plan for Scale.** Addresses A2. Founder-approved. Could instead be a strategy-layer decision not an ADR.
- **New doc (not ADR): ARCHITECTURE.md, CONSISTENCY.md, VERSIONING.md, FAILURE-MODES.md.** Reference docs, not decisions. Addresses A1, A4, A9, A6.
