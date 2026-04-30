# L0 review cycle 2 — DOS-209 plan v2 — codex consult mode

**Reviewer:** /codex consult (cycle 2)
**Plan revision under review:** v2 (2026-04-28)
**Verdict:** APPROVE

## Cycle 1 finding closures verified

### F1 (DOS-304 / 2026-04-24 amendment) — closed: yes
Closure location: `DOS-209-plan.md` §1 lines 59; §3 lines 79, 87; §4 lines 126-128.
Verification: Linear DOS-304 says `ServiceContext` capability handles are the enforcement boundary, raw `ActionDb`/direct SQL/file-write/live external access must be unavailable to abilities, proc macros are not proof, one registry decision gates DOS-210/DOS-217, and startup/background worker behavior must be explicit. v2 reads that correctly: "capability handles, not proc-macro inspection and not `check_mutation_allowed()` convention alone" are the boundary, raw handles are private or service-only, ability-facing DB is scoped read-only, DOS-210/DOS-217 registry choice remains gated, and Evaluate constructors do not spawn live workers.

### F2 (caller construction path) — closed: yes
Closure location: `DOS-209-plan.md` §2 lines 65-73; §9 line 168.
Verification: v2 names each construction surface: Tauri through `ServiceLayer::new_live`, MCP sidecar live/read-only unless mutating, background workers at dequeue, simulation via `new_simulate`, evaluation via fixture-only `new_evaluate`, and tests via `#[cfg(test)] test_live()`. It also binds this to `dos209_surface_constructors.rs::all_live_surfaces_construct_new_live`, so caller migration is testable rather than implicit.

### F3 (Intelligence Loop 5-question check) — closed: yes
Closure location: `DOS-209-plan.md` §6 lines 140-145.
Verification: v2 replaces the prior n/a with all five answers: mode-aware signal routing to the ADR-0115 in-memory ring buffer, deterministic health scoring/debouncer via injected clock/RNG, future context-builder propagation, no briefing callouts, and feedback mutations included in the guard catalogue.

### F4 (W1 hard pre-code gate) — closed: yes
Closure location: `DOS-209-plan.md` §7 lines 151-155; §5 lines 134-135.
Verification: v2 states the hard pre-code gate directly: "DOS-310 and DOS-311 must be merged, W1 L3 cleared, and the W1 proof bundle published before DOS-209 implementation starts." It also says DOS-209 consumes W1's migration fence, schema epoch, write fence, and Suite P baseline, with no planned W2-A SQL migration unless gated through the W1 proof bundle.

## Fresh findings (if any)
None.

## Verdict rationale
The revised plan closes all four cycle-1 findings with concrete contract text, construction surfaces, capability boundaries, wave gates, and named tests. Fresh review found enough implementation shape for coding: context carriers and visibility, transaction API/fallback, mutator catalogue, error variants, caller migration, capability compile-fail tests, runtime boundary tests, and CI lint seams are all specified.

## If APPROVE
Confidence comes from the concrete W2-A implementation spine in `DOS-209-plan.md` §2-§4 and §9: `services/context.rs` ownership, exact carrier/visibility rules, `ServiceError` variants, `with_transaction_async` signature plus fallback, exhaustive mutator catalogue, surface-constructor test, mutation-catalog structural lint, mode-boundary runtime test, clock/RNG lint, and capability trybuild fixtures.
