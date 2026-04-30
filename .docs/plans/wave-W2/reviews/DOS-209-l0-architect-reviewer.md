# L0 review — DOS-209 plan — architect-reviewer

**Reviewer:** architect-reviewer (substrate / schema profile)
**Plan revision:** v1 (2026-04-28)
**Verdict:** REVISE

## Findings

### F1 — `IntelligenceProvider` placement vs. `intel_queue` in ServiceContext is ambiguous (severity: High)
ADR-0104 §"Known Limitations" #5 is explicit: `IntelligenceProvider` lives on `AbilityContext`, **not** `ServiceContext`, while `ExternalClients` (Glean, Slack, Gmail, Salesforce) lives on `ServiceContext`. Mode-aware routing of LLM calls is therefore an `AbilityContext` concern; the service layer only carries the queue handle.

The plan's §2 mixes these layers without naming the seam: it lists `intel_queue` as a carrier on `ServiceContext` (correct, inherited from ADR-0101) but does not state that the `IntelligenceProvider` will *not* land on `ServiceContext` here — even though §7 acknowledges this for W2-B coordination. Downstream, W3-A's ability registry and W4-A's Trust Compiler both consume `AbilityContext`, and they will fail-closed if the provider seam migrates.

What needs to change: §2 should name explicitly which carriers land on `ServiceContext` in this PR (db, signals, intel_queue, mode, clock, rng, external, tx) and which deliberately do **not** (IntelligenceProvider — owned by `AbilityContext`, wired by surface, per ADR-0104 §"Known Limitations" #5). Without this, a reviewer cannot tell whether W2-A is freezing the right seam shape for W3.

Location in plan: §2 "Approach" — "Define `ExecutionMode`, `ServiceContext`, `Clock`, `SeededRng`, `ExternalClients`, `TxHandle`/transaction context shape" — silent on what's excluded.

### F2 — Plan/Apply split is omitted from the substrate scope (severity: High)
ADR-0104 §3.1 introduces `PlannedMutationSet` / `PlannedMutation` / `ProvenanceRef` and the `plan_*` planner-function naming convention as the substrate that makes mode-aware semantics actually usable for maintenance abilities (ADR-0103). The plan does not mention these types at all. DOS-209 lands `check_mutation_allowed()` (defense-in-depth) but does not name whether the plan/apply substrate types are in scope here, deferred, or already covered by another wave.

Either is defensible — but leaving it implicit creates a downstream drift risk: W4-C (`invoke_ability` bridge) and W3-A (registry maintenance-ability wrapper) both depend on `PlannedMutationSet` being a known type. If those types arrive later in W3 they may end up being defined twice, or W4-C may discover the type doesn't exist when it tries to consume it.

What needs to change: §2 or §3 should explicitly name `PlannedMutationSet` / `ProvenanceRef` / planner-function naming as either (a) in scope for this PR, (b) deferred to a named later wave with a contract, or (c) intentionally absent because no Phase-0 consumer needs them. ADR-0104 §8 marks these as Phase 1 / Phase 2; the plan should pick a wave.

Location in plan: §2, §3 — silent on plan/apply split.

### F3 — Migration strategy is not concrete enough on staging vs. one-shot (severity: High)
The ticket and ADR-0104 §8 sequence the work as Phase 0 (struct + constructors) → Phase 1 (~60 mutator migration). The plan in §2 lists a long roster of files to migrate "in one PR" but §8 says "the existing roughly 60 mutators must be swept in one PR." This is a single-blast-radius operation against ~24 service modules with the highest risk in the project. Failure modes include: a missed grep on raw SQL, a missed signal emitter, a partially-migrated service that compiles but bypasses the guard, and the cycle-2 lesson from DOS-309 that "compiles clean" is not "works."

The plan does not name a concrete enumeration strategy — just "grep audit over DB write verbs plus manual review of raw SQL and signal emitters" (§3). It does not name:
- An audit script that lists candidate sites and checks them off (cf. DOS-309 cycle-2 ruling that an audit script was a precondition).
- Whether the migration goes through a temporary `ServiceContext::test_live()` (mentioned as `#[cfg(test)]`-only in §3) and whether that helper is also used as a transitional shim for production callers.
- A staged-vs-atomic decision: is there a working-tree state mid-PR where some mutators take `&ServiceContext` and some still take raw `&ActionDb`? If yes, how does CI stay green during that window? If no, the PR is huge and hard to review.

What needs to change: §3 should name (a) the enumeration mechanism (audit script committed alongside the PR vs. one-time grep), (b) whether mid-PR working-tree state has mixed signatures, and (c) whether `test_live()` is purely a test helper or also a transitional production shim. Pick one and justify; the ticket permits "all in one PR" but the plan must show how it will be made bisectable.

Location in plan: §3 "Enumerate mutation sites with a grep audit over DB write verbs plus manual review of raw SQL and signal emitters" — too thin.

### F4 — W2-B rebase coordination is named but not concrete (severity: Medium)
W2-A and W2-B both modify `services/intelligence.rs`: W2-A adds `ctx.check_mutation_allowed()?` to mutation functions; W2-B extracts PTY orchestration into `intelligence/pty_provider.rs` (which lifts code *out of* the same file). Both PRs land in the same wave with no merge-order rule. The plan's §7 says "W2-A does not touch those [provider] files. Coordinate only on whether future provider implementations receive full `&ServiceContext`..." which addresses signature coordination but not the file-level rebase.

If W2-B lands first, every PTY-extracted function W2-A planned to add a guard to has moved to a new file under W2-B's deny list. If W2-A lands first, W2-B's extraction loses the new mutation guards on the moved functions and must re-add them.

What needs to change: §7 should name an explicit merge order (likely W2-A first, since W2-B is the smaller surface and rebases more cheaply onto a `ServiceContext`-using `services/intelligence.rs`) OR a coordination protocol (e.g. "W2-B reviewers verify any functions extracted from `services/intelligence.rs` retain their `check_mutation_allowed` guard if they are mutation paths, and add new guards if missed").

Location in plan: §7 "W2-B/DOS-259 owns ... W2-A does not touch those files. Coordinate only on whether future provider implementations..." — coordination on signature only, not file ordering.

### F5 — `ExternalClients` shape is a deferred Open Question, but it is in scope per the ticket (severity: Medium)
§10 lists "Does `ExternalClients` land as a concrete struct, trait object bundle, or placeholder wrappers until ADR-0107 fixtures exist?" as Open. ADR-0104 §2 specifies it as a struct with named fields (`glean`, `slack`, `gmail`, `salesforce`) holding mode-aware wrappers; ADR-0104 §"Known Limitations" #4 explicitly says the *replay* interface is contract-only until ADR-0107 lands.

This is a real question, but the wave-2 contract is that the seam freezes here. Leaving the shape unresolved means W3-A and W4-A (which consume external calls via `ctx.services.external.glean.*`) will be coding against an unknown surface, which is exactly what the wave protocol was designed to prevent.

What needs to change: §10 should be promoted to §3 as a key decision. Pick: (a) concrete struct with placeholder wrappers (matches ADR-0104 §2 + §"Known Limitations" #4 — Live wrappers real, Simulate/Evaluate wrappers panic with `unimplemented!()` until ADR-0107), or (b) trait-object bundle. The wave seam must be a real type, not a TBD.

Location in plan: §10 "Does `ExternalClients` land as a concrete struct, trait object bundle, or placeholder wrappers..."

### F6 — Performance section is hand-waved when this PR is the most contended hot path the project will land (severity: Medium)
§5 says "without benchmark data, assume sub-microsecond local overhead and no meaningful p99 shift." This is not a Suite-P gate (correct — Suite P baseline is W1 end), but every mutation in `services/` will go through one extra branch + at least one `dyn Clock` indirection + transaction wrapper changes, and the ticket explicitly calls out that the Suite-P W1 baseline is the comparand for the W2-end re-measurement.

The plan should at minimum identify which mutators are p99-relevant (the Daily Loop hot path: `services/accounts.rs::update_account_field_inner`, `services/intelligence.rs` mutation paths, `services/meetings.rs` outcome writes) and name a budget so re-measurement at W2 end has a pass/fail criterion. "Sub-microsecond local overhead" is the intuitive answer, but the wave protocol's whole point is to commit to a number.

What needs to change: §5 should name the Suite-P-relevant mutator set and a regression budget (e.g. "p99 deviation < 5% on the W1 baseline mutator microbenches"). Without it, "no regression" is unmeasurable when re-measured.

Location in plan: §5 "Suite P baseline is established at W1; W2-A contributes no new Suite P gate but must not regress when the wave is re-measured after W2."

### F7 — DB-side `CURRENT_TIMESTAMP` determinism gap is acknowledged but not contained (severity: Medium)
§3 says "DB `CURRENT_TIMESTAMP` defaults do not satisfy the direct Rust clock ban but are a determinism gap called out by DOS-209/ADR-0104; document offenders and file or attach the follow-on rather than silently treating them as fixed." ADR-0104 §"Known Limitations" #3 says the same.

This is correct — but it leaves Evaluate-mode determinism partial. Suite E's property test on `check_mutation_allowed` (named in §9) will be green even though Evaluate runs against schemas with `DEFAULT CURRENT_TIMESTAMP` will produce non-reproducible timestamps. W3-G's `source_asof` work and W4-A's Trust Compiler freshness math both consume timestamps; if they read DB-defaulted values during an Evaluate run, snapshot-diff scoring will be flaky.

What needs to change: §3 should commit to (a) producing the audit list of `DEFAULT CURRENT_TIMESTAMP` / trigger-set timestamp columns in this PR (cheap), and (b) naming the follow-on ticket (Linear) so W3-G and W4-A's plans can reference it as a known-deferred risk. Otherwise the gap silently becomes load-bearing for downstream evaluation.

Location in plan: §3 "DB `CURRENT_TIMESTAMP` defaults ... document offenders and file or attach the follow-on rather than silently treating them as fixed."

### F8 — Property test characterization is too thin to credibly cover the invariant (severity: Medium)
§9 names "`services::context::tests::proptest_check_mutation_allowed_modes`" — a property test on `check_mutation_allowed` (random ExecutionMode × random mutation → correct accept/reject). The function under test is a 4-line match on an enum. A property test there is fine but exercises near-zero of the actual surface area the invariant cares about.

The architecturally interesting failure mode is *not* "did `check_mutation_allowed` return the right thing for the right mode" — it's "did every mutation function actually call `check_mutation_allowed` first, before any DB write or signal emission, on every code path including raw-SQL paths." That property requires either a CI grep test (named in the wave protocol §"CI-enforced architecture invariants") or a runtime test where every public service mutation function is invoked under `Simulate`/`Evaluate` and asserts `WriteBlockedByMode`.

What needs to change: §9 should add a contract test that enumerates every public mutation function in `services/` (via the audit script from F3) and invokes each under Evaluate mode against a test DB, asserting that none of them succeed. That test catches the "missed migration" failure mode named in §8 — the property test as currently named does not.

Location in plan: §9 "Add focused tests such as `services::context::tests::proptest_check_mutation_allowed_modes` ... `dos209_mode_boundary.rs::evaluate_mutation_returns_write_blocked_by_mode`" — the second test name is closer but the plan does not say it iterates over the mutator catalogue.

### F9 — Intelligence Loop 5-question check ack is dismissed too quickly (severity: Low)
§6 reads "Intelligence Loop 5-question check from `CLAUDE.md`: n/a -- runtime infrastructure, no new table, column, or data surface." This is mechanically correct — DOS-209 adds no schema. But the wave protocol's CI invariants table says the *enforcement* of the 5-question check on schema PRs *first activates* at W2. The plan should name that DOS-209 is responsible for landing the CI mechanism (PR template checklist + CI bot comment), not just opting out of answering the 5 questions for itself.

What needs to change: §6 should split into (a) does this PR need to answer the 5 questions for itself (no — runtime infra) and (b) does this PR land the CI enforcement that activates the invariant for the rest of W2+ (yes, per the wave protocol invariants table, row "Intelligence Loop 5-question check on schema PRs"). Today the plan only addresses (a).

Location in plan: §6 "Intelligence Loop 5-question check from `CLAUDE.md`: n/a"

## End-state alignment assessment

The plan moves the substrate in the right direction. ADR-0104's Phase 0 deliverable is `ServiceContext` with the seven named carriers, explicit per-mode constructors, no `Default`, `check_mutation_allowed`, and `with_transaction_async`. The plan covers all of these and correctly identifies that the brownfield migration is "all callers go through `new_live`" with no compatibility shim. It correctly takes the sync-within-async fallback as the cut-line for HRTB slippage (per DOS-209 explicit permission), it correctly excludes provider files from W2-A's deny list, and §3's `#[cfg(test)]`-only `test_live()` is the right shape — guarded so production references compile-error.

The risk is not direction but *seam clarity* for downstream consumers. W3-A (ability registry) consumes `AbilityContext` which composes over `ServiceContext`; W3-C (claims layer) consumes `&ServiceContext` directly through the new `services/claims.rs::commit_claim`; W4-A (Trust Compiler) reads `ctx.clock` and `ctx.services.external.*` indirectly. If `ExternalClients`'s shape is left as an Open Question (F5), if `IntelligenceProvider`'s placement on `AbilityContext` (not `ServiceContext`) is not named explicitly (F1), and if the plan/apply substrate types are silently deferred without a wave assignment (F2), three downstream consumers in W3 and W4 will discover ambiguities at code time — exactly what L0 is designed to prevent. The findings above are revisions for clarity, not a re-architecting.

## Verdict rationale

REVISE because the plan covers the Phase-0 mechanics correctly but leaves three load-bearing seams under-specified for downstream consumers (F1, F2, F5) and under-specifies the migration strategy for the highest-blast-radius operation in the project (F3). None of these are architectural disagreements; they are precision gaps that L0 should close before code, not after.

## If REVISE

1. **F1 — Name `IntelligenceProvider` exclusion.** §2 enumerates ServiceContext's eight carriers explicitly and names that `IntelligenceProvider` is deliberately on `AbilityContext` (per ADR-0104 §"Known Limitations" #5), not migrating in this PR. Required for W3-A registry consumer clarity.
2. **F2 — Decide plan/apply substrate scope.** §2 or §3 names whether `PlannedMutationSet` / `ProvenanceRef` / `plan_*` planner naming convention land here, in W3, or in W4. Pick a wave; do not leave implicit. Required for W4-C bridge consumer clarity.
3. **F3 — Concrete migration strategy.** §3 names (a) audit script committed alongside PR, (b) mid-PR working-tree state plan (mixed signatures or atomic), (c) `test_live()` scope (test-only vs. transitional shim). Pick and justify. Required because this is the ~60-site sweep.
4. **F4 — W2-A/W2-B rebase order.** §7 names W2-A-first or W2-B-first explicitly, with reviewer protocol for the cross-file mutation-guard preservation. Required to prevent guard-loss on PTY extraction.
5. **F5 — `ExternalClients` shape resolved.** Promote §10's ExternalClients open question to §3 as a key decision. Pick concrete struct with placeholder wrappers (Live real, Simulate/Evaluate `unimplemented!()` until ADR-0107) — or trait-object bundle — and ship the type in this PR. Required for W3-A and W4-A consumer surface freeze.
6. **F6 — Suite P-relevant budget.** §5 names the Daily-Loop p99-relevant mutator set and a regression budget (e.g. "<5% deviation against W1 baseline microbenches"). Required so W2-end re-measurement has pass/fail criterion.
7. **F7 — DB-clock audit list shipped, follow-on filed.** §3 commits to producing the audit list in this PR and naming the deferred Linear ticket so W3-G and W4-A can reference it.
8. **F8 — Mutation-coverage contract test.** §9 adds a test that iterates the mutator catalogue (from F3's audit script) and asserts `WriteBlockedByMode` under Evaluate mode for every entry. The pure proptest on the 4-line match function is insufficient by itself.
9. **F9 — Activate the 5-question CI enforcement.** §6 splits into "does this PR answer the 5 questions" (n/a) and "does this PR land the enforcement mechanism" (yes — per wave invariants table). Wire the PR template checklist or CI bot comment in this PR.
