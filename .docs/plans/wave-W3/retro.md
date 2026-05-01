# Wave W3 retro (substrate slice: W3-A + W3-B)

**Wave slice:** W3-A (DOS-210 ability registry + #[ability] proc macro) + W3-B (DOS-211 provenance envelope + builder)
**Date:** 2026-05-01
**Author:** orchestrator (Claude Code)

This retro covers ONLY the W3-A + W3-B substrate co-land. The other 6 W3 tickets (DOS-7, DOS-294, DOS-296, DOS-299, DOS-300, DOS-301) will produce their own retros as they ship.

## Per-layer wall-clock

| Phase | Wall-clock | Notes |
|---|---|---|
| Plan v1 drafting (DOS-210 + DOS-211) | (pre-existing) | Drafted as part of v1.4.0 wave plan work; not measured in this retro |
| L0 cycle-1 review (2 codex slots, parallel) | ~6 min | Both returned REVISE; 7 + 5 specific findings each |
| Plan v2 revisions (2 parallel codex agents) | ~5 min real-time wall-clock | Brief drafting + agent execution; minor cleanup of cross-cutting decisions (schemars non-optional, AbilityOutput ownership) |
| W3-B implementation (single codex agent) | ~25 min | 8 module files + 23 unit tests + 3 integration tests; one-shot completion |
| W3-A implementation cycle 1 (single codex agent) | **STUCK** at 1h+ | Watchdog stalled; no W3-A files written. Brief was too large for one shot |
| W3-A part 1 (codex agent, narrowed brief) | ~5 min real | Workspace conversion + proc-macro skeleton + AST visitor + 10 scoring tests |
| W3-A part 2 (codex agent) | ~12 min real | Registry + observability + 12 unit tests; agent stuck in verification loop after completing actual work |
| W3-A part 3 (codex agent) | ~20 min | Macro emission + 5 trybuild fixtures + 3 integration tests + lint script. Sandbox-blocked validation but my env passed everything cleanly |
| W3-A finding-2 manual fix (clock seam revert + test fix) | ~5 min | The agent's test had a clock-equality assertion that contradicted the agent's own macro emission; reverted to brief-spec macro + plausibility test |
| L1 validation | ~5 min | 1796 lib tests + clippy + tsc + lint scripts |
| Initial commit batch (3 commits: plan v2 + W3-B impl + W3-A impl) | ~2 min | |
| L2 cycle-1 review (1 codex slot) | ~1.5 min | BLOCK with 8 findings (3 critical + 3 high + 2 medium) |
| L2 cycle-1 fix (codex agent) | ~17 min real | 4 findings closed + lint script expanded; 4 deferred to follow-up tickets (DOS-349/350/351/352) |
| L1 validation post-L2 | ~5 min | 1796 lib tests still passing; 1 test fix (FixtureOutput Serialize derive) |
| L3 cycle-1 Wave adversarial (1 codex slot) | ~3 min | BLOCK with 5 findings (1 critical + 3 high + 1 medium) |
| L3 cycle-1 fix (codex agent) | ~27 min real | All 5 findings closed; 1797 tests passing; clippy + experimental feature build clean |
| L3 cycle-2 confirmation review (1 codex slot) | ~3 min | BLOCK with 3 findings — same root cause (module-scope porousness) |
| L3 cycle-2 finding 2 fix (manual edit) | ~2 min | Move experimental cfg over inner_fn |
| L6 escalation packet | ~5 min | Cycle-2-still-BLOCKED stop condition fired per set-and-forget protocol |
| L6 ruling (Option A) | ~30s | User: "Option A but make sure 349 is added somewhere for actual clean up later... I would prefer a future wave." |
| Linear ticket updates (DOS-349 retargeted, DOS-350/351/352 filed earlier) | ~3 min | |
| Proof bundle + retro (this) | ~15 min | |

**Total elapsed:** ~3 hours of orchestrator wall-clock for the W3-A + W3-B substrate, end-to-end (excluding the failed first W3-A attempt).

## What went wrong

1. **First W3-A codex agent stalled at 1h+ with zero files written.** The original DOS-210 implementation brief tried to land workspace conversion + proc-macro crate + AST visitor + registry + observability + 25 trybuild fixtures + 5 integration tests + lint script in one agent shot. Too much. The agent spent the entire time in analysis without writing code. Cancelled and split into 3 sequential parts; that worked.

2. **W3-A parts 1, 2, and 3 each had their codex agent stick in a verification or watchdog loop after completing actual work.** Files were written, tests passed, but the agent couldn't terminate cleanly. Multiple cancels were needed. This wasted some orchestrator time poking status. Lesson: **trust file inspection + git status over agent self-reports**; when files are present and validation passes locally, treat the work as complete and move on, even if the codex job is technically still "running".

3. **W3-A part 3 agent's macro and test contradicted each other on clock source.** The agent emitted `chrono::Utc::now()` in the macro (per the brief) but also wrote a test that asserted `record.started_at == clock.now()` (which would only work if the macro used `ctx.services().clock.now()`). Caught at L1 validation and fixed manually in 5 minutes by reverting the test to plausibility-window assertion.

4. **L2 cycle-1 had 8 findings, half of which were already real correctness bugs.** The ProvenanceBuilder::finalize() bypass via direct AbilityOutput construction was used by my OWN test (the agent's first observability test). Reviewer caught this immediately. The hollow-descriptor finding was a Rust const-evaluation limitation the original agent didn't grok; fix required type signature change (`Vec<T>` → `&'static [T]`) which propagated through the macro.

5. **L3 cycle-2 STILL BLOCK on the same root cause as L2 finding 3 (DOS-304 boundary).** Both reviews flagged the same structural truth: proc-macros and grep regexes can't enforce capability boundaries inside a single crate. Cycle-1 hardened the regex and the AST visitor, but the residual porousness (module-scope aliases, `use std::fs;` aliased imports) is not closeable without DOS-349's crate split. Stopped per set-and-forget protocol's cycle-2-still-BLOCKED rule and escalated to L6.

6. **The set-and-forget protocol's "cycle-2-still-BLOCKED stop" fired correctly here.** Without that explicit stop condition, I could have spent another cycle chasing increasingly-baroque grep regex extensions (`use crate::db as foo; use foo as bar; bar::ActionDb`), which would deliver false confidence. The protocol caught this and forced an honest escalation.

## What went right

1. **Splitting W3-A into 3 sequential codex briefs after the first agent stalled.** Each part was scoped tightly enough for a single agent shot. Total wall-clock for parts 1+2+3 was ~40 minutes, vs. >1h wasted on the original mega-brief that wrote zero code.

2. **W3-B as a single codex shot worked perfectly.** 8 module files + 26 tests in ~25 min wall-clock. The brief was precise (file-by-file enumerated, with explicit "OUT OF SCOPE" markers), the codex agent had a clean dependency graph (no parallel coordination needed), and the type design was clearly specified in the v2 plan.

3. **Brief drafting investment pays off.** Each codex brief took 10-15 min to draft (more for the implementation briefs; less for fix briefs that referenced specific findings). Briefs included exact file lists, "DO NOT touch" sections to prevent collision, validation commands, and (after the stall) explicit "stop conditions" — "if stuck looping, exit and report partial; 60 min hard ceiling". This last addition saved time in part 3 where the agent did stop cleanly when its sandbox blocked tracing-test download.

4. **Multi-cycle L2/L3 reviews caught real correctness bugs that L1 didn't.** L1 (clippy + tests) was clean throughout; both L2 and L3 found provenance/registry bugs that compiled and tested fine but violated the documented contracts (envelope bypass, hollow descriptors, async-runtime panic, LLM-trusted bug, sibling-module bypass). Skipping either review would have shipped broken substrate.

5. **L6 ruling on cycle-2-still-BLOCKED was clean.** "Option A but make sure 349 is added somewhere for actual clean up later... I would prefer a future wave." Specific, actionable, immediate. DOS-349 was already filed (during L2 fix) so retargeting cost ~3 min: priority bumped to High, title updated to call out the future-wave scheduling, scheduling section added with the hard precondition "must complete before DOS-218+ migration".

## Reviewer-independence sanity check

**L0 (plan review, 2 codex slots in parallel):**
- Both returned REVISE on plan v1 with non-overlapping critical findings
- DOS-210 review caught: missing fixture-trace test, observability under-scoped, Amendment A errors not specified, DOS-304 trybuild absent, workspace CI implications, schemars-coordination, AbilityOutput ownership ambiguous
- DOS-211 review caught: thread_ids future-framed (must be NOW), children wire format drift, finalize-time vs compile-time, FeedbackEvent over-scope, AbilityOutput ownership ambiguous
- Convergence: both flagged AbilityOutput ownership, schemars version coordination
- Cost: ~6 min wall-clock for ~25 specific findings across the two plans

**L2 (single codex slot, working tree review):**
- 8 findings (3 critical + 3 high + 2 medium); 4 closed mechanically, 4 deferred as follow-ups (filed)
- Caught the AbilityOutput public-fields bypass that my OWN observability test used — this is the kind of bug only adversarial review catches because the test PASSED with the bypass

**L3 cycle-1 (Wave-scoped review):**
- 5 findings, all closed in cycle-1 fix
- Wave-scope (`--base 17947c00`) was critical here — it scoped the review to the W3 commits specifically, not the full working tree. This is what the W2 retro mandated for substrate waves and it paid off.

**L3 cycle-2 (confirmation review):**
- 3 findings, 1 closed, 2 escalated to L6 as residual porousness
- Cycle-2 was ESSENTIAL: cycle-1 thought it had closed the alias case but only handled in-function aliases. Cycle-2 caught that module-scope aliases still bypass.

## Suite mechanics

- **Suite E:** 1797 lib tests (+38 from W2 close); stable across all W3 commits.
- **Suite P:** still NOT RUN (DOS-348 filed during W2-B closure with a v1.4.x or future plan). Now 3 waves of substrate without a perf baseline. The retro's "must escalate to merge gate by W3" recommendation has not yet been actioned.
- **Suite S:** N/A for this slice (Suite S is the bundles 1+5 substrate-discovery harness; lives in W4-B / DOS-216).

## Overhead-without-signal observations

1. **Agent watchdog stalls produce false-completion notifications.** Three different W3-A codex tasks fired "completed" notifications while their codex jobs were still running. I learned to verify with `git status` + `cargo build` before believing the wrapper agent's status. This added a poll-and-verify cost on every dispatch but prevented multiple wasted cycles.

2. **Cycle-1 fix → cycle-2 review → cycle-2 fix → cycle-2 review trail produced ~5 commits for one substrate ticket pair.** That's the pattern from W2-B (4 cycles). Each cycle delivered structural improvement; cycle-2 finding 2 (experimental cfg over inner_fn) was an incomplete cycle-1 fix, but findings 1+3 were genuinely "this needs DOS-349" rather than "your fix is wrong". That's a better signal than W2-B's cycle-3 which caught a back-door created by cycle-2's atomic bundle.

3. **Plan v2 revision agents finished cleanly in ~5 min wall-clock.** Markdown editing is the easy case for codex; the briefs were precise enough that the agents just executed.

## Recommended tuning for W4+

### Brief sizing for codex implementation agents

- **Substrate type-system work** (W3-B-shaped): one shot, ~25 min, clear file enumeration. Works.
- **Mixed proc-macro + main-crate + integration tests** (W3-A-shaped): MUST be split. Three sequential briefs of ~15-25 min each beats one mega-brief that stalls. Anything over ~10 files of distinct surface area wants splitting.
- **Mechanical mutator migration** (W2-A-shaped): single shot can handle 30-45 mutators with a tight pattern.

### Codex agent verification

- **Trust filesystem state, not agent self-reports.** Every codex agent in W3 either watchdog-stalled or got stuck in a verification loop AFTER completing the work. Validation: `git status` + `cargo build` + `cargo test --lib`. If those pass, treat the work as done even if the wrapper agent shows "running".

### Cycle-2-still-BLOCKED stop condition is load-bearing

- The set-and-forget protocol's explicit stop condition for cycle-2-still-BLOCKED prevented me from chasing increasingly-byzantine regex extensions on the DOS-304 lint. Without that stop, I'd have produced false confidence for ~30 more minutes and still left the structural gap open.

### Suite P is now structurally overdue

- Three waves with the same gap. DOS-348 is filed but unscheduled. By W4 close-out, this becomes "we're benchmarking against a 4-wave-old codebase when we finally do it." Recommend escalating DOS-348 to a hard merge gate at W4-A start.

## L6 rulings during this wave

- **2026-05-01 (L3 cycle-2):** Option A ruling — accept porous best-effort enforcement of module-scope alias detection + DOS-304 lint, file structural fix as DOS-349 retargeted to v1.5.x or future wave. W3 substrate ships with residuals documented.

## Status

- [x] Plan v1 (DOS-210 + DOS-211)
- [x] L0 cycle-1 review (2 codex slots; both returned REVISE; 12 findings closed in plan v2)
- [x] Plan v2 revision
- [x] W3-B implementation (commit `d70af6cc`)
- [x] W3-A parts 1+2+3 implementation (commit `be51a31f`)
- [x] L1 self-validation
- [x] L2 cycle-1 review + fix (commit `1432ef13`); 4 findings closed, 4 deferred to follow-up tickets (DOS-349, DOS-350, DOS-351, DOS-352)
- [x] L3 cycle-1 Wave adversarial + fix (commit `3faae549`); all 5 findings closed
- [x] L3 cycle-2 confirmation + partial fix (commit `2aa70ac5`); 1 closed, 2 escalated to L6
- [x] L6 ruling (Option A): DOS-349 retargeted to v1.5.x / future wave
- [x] Proof bundle (`.docs/plans/wave-W3/proof-bundle.md`)
- [x] Retro (this)
- [ ] Tag `v1.4.0-w3-substrate-complete` (next; immediately after this commit)

## Deferred to follow-up tickets (not lazy deferrals)

Each has a specific symbol/file/contract that doesn't exist yet:

- **DOS-349** (HIGH, v1.5.x or future wave) — Move ability runtime into separate crate. Hard precondition: must complete before first DOS-218+ migration.
- **DOS-350** (MEDIUM) — Validate composition_id at finalize/registry-time. Requires registry-declared composition metadata threading into ProvenanceBuilder + deserialize-time fabrication policy.
- **DOS-351** (LOW) — ProvenanceBuilder size-budget tombstone replacement. Requires tombstone shape design + no-progress detection algorithm.
- **DOS-352** (MEDIUM) — Fixture-trace runtime drift gate. Requires tracing instrumentation on every `services::*` mutator + observed-vs-declared comparison.

## Test count delta

W2 close: 1759 lib tests
W3-A + W3-B close: 1797 lib tests (+38)

Plus:
- 12 proc-macro scoring unit tests
- 6 trybuild fixtures (4 compile-fail + 2 compile-pass)
- 7 W3-A integration tests (1 ignored as deferred)
- 6 W3-B integration tests
