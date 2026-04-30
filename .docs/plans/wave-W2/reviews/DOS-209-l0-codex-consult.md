# L0 review — DOS-209 plan — codex consult mode

**Reviewer:** /codex consult
**Plan revision:** v1 (2026-04-28)
**Verdict:** REVISE

## Findings

### F1 — 2026-04-24 capability-boundary amendment is treated as non-applicable (severity: Critical)
The Linear contract has a 2026-04-24 plan-hardening amendment and DOS-304 blocker saying `ServiceContext` capability handles are the enforcement boundary, not `check_mutation_allowed()` convention alone. The plan instead says no 2026-04-24 amendment applies, then leaves capability leakage as a later-registry concern.
Location in plan: §1 "No 2026-04-24 ADR amendment appears to apply"
What needs to change: Acknowledge DOS-304 / the 2026-04-24 amendment in §1, then state the W2-A API decision that prevents raw `ActionDb`, direct SQL, file-write, queue, and external-client handles from reaching ability code, or explicitly gate W2-A coding on DOS-304 resolution.

### F2 — Caller construction path for `ServiceContext::new_live` is missing (severity: High)
The ticket requires all existing callers to migrate to explicit `ServiceContext::new_live`; the plan inventories service mutators but not the surfaces that construct and pass the context. That leaves Tauri commands, MCP/server paths, background workers, and tests without an implementation path.
Location in plan: §2 "Migrate mutation functions in verified service modules: `accounts.rs`, `actions.rs`, `people.rs`, ... and related service files found by grep."
What needs to change: Add a caller/surface migration subsection naming where `new_live`, `new_simulate`, `new_evaluate`, and `#[cfg(test)] test_live()` are constructed and how raw `ActionDb` call sites are retired.

### F3 — Intelligence Loop check is boilerplate against this ticket (severity: Medium)
DOS-209's ticket has specific Intelligence Loop implications: mode-aware signal emission, health-scoring determinism via injected clock, and eventual context builders taking `&ServiceContext`. The plan collapses the check to n/a, so implementation could miss signal-mode behavior while still satisfying the written plan.
Location in plan: §6 "Intelligence Loop 5-question check from `CLAUDE.md`: n/a -- runtime infrastructure, no new table, column, or data surface."
What needs to change: Replace the n/a with the five explicit answers for this ticket, especially the signal-routing expectation in non-Live modes and the health-scoring clock/RNG implication.

### F4 — W1 dependency is only captured as migration numbering, not as a hard pre-code gate (severity: Medium)
The wave protocol says no W2 coding starts until W1 clears the required gate; DOS-209 also relates to DOS-310/DOS-311. The plan only mentions W1 if an unexpected metadata-row migration is needed, not the required DOS-310/DOS-311 merge/L3 dependency before coding.
Location in plan: §7 "If a metadata-row helper is unexpectedly required, pick up after W1-A migration N and W1-B migration N+1, so W2 starts at N+2."
What needs to change: Add the hard pre-code dependency: DOS-310 and DOS-311 must be merged and W1 gate/proof-bundle cleared before DOS-209 implementation starts, with any schema/write-fence assumptions consumed from that bundle.

## Summary
The core shape is implementable, but the plan does not yet clear L0 because it misses a live Linear amendment/blocker and leaves important caller and wave-gate coordination implicit. These are plan-level fixes, not evidence that the DOS-209 approach itself is wrong.

## Strengths
The plan correctly identifies `ServiceContext` as greenfield, names the mode enum and constructors, includes `check_mutation_allowed()`, covers the HRTB fallback for `with_transaction_async`, and calls out DB `CURRENT_TIMESTAMP` as a determinism gap. §9 also gives concrete, liftable test names, including a random ExecutionMode mutation property test and direct clock/RNG lint test.

## If REVISE
1. Add DOS-304 / 2026-04-24 capability-boundary handling to §1/§3/§4.
2. Add explicit caller/surface construction and raw-DB retirement steps to §2.
3. Replace the §6 Intelligence Loop n/a with the ticket-specific five-question answers.
4. Add the DOS-310/DOS-311 W1 gate dependency to §7.
