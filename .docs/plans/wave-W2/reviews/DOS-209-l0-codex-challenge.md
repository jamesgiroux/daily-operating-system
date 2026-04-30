# L0 review — DOS-209 plan — codex challenge mode

**Reviewer:** /codex challenge
**Plan revision:** v1 (2026-04-28)
**Verdict:** REVISE

## Findings

### F1 — Contract restatement is selective, not frozen (severity: Critical)
Linear DOS-209 is reachable. §1 quotes four selected fragments and paraphrases the rest, omitting load-bearing acceptance criteria: exact `ServiceContext` field types, no `Default`, all callers migrated to `new_live`, `with_transaction_async`, `Clock`/`SeededRng` signatures, the Evaluate mutation CI test, and clippy/test gate.
Location in plan: §1 'Load-bearing ticket lines read: "`ServiceContext` does not exist in the codebase today"; "`ExecutionMode` enum (`Live` | `Simulate` | `Evaluate`)"; "`check_mutation_allowed()` method exists on `ServiceContext` and returns `Err(WriteBlockedByMode)` outside `Live`"; and "Every mutation function in `services/` gains `ctx.check_mutation_allowed()?` as first line".'
What needs to change: Replace §1 with the frozen DOS-209 contract lines verbatim, at least Problem, Scope limits, Acceptance criteria, Edge cases, Build-ready checklist, and Done checklist. Do not summarize contract items.

### F2 — Mutation sweep is hand-waved and misses files (severity: Critical)
The plan does not enumerate the ~60 mutation sites. It lists some modules, then delegates discovery to future grep; the current `services/` tree also includes mutation-relevant files not named here, including `dashboard.rs`, `reports.rs`, `entities.rs`, `integrations.rs`, `signals.rs`, `commitment_bridge.rs`, and `health_debouncer.rs`.
Location in plan: §2 'Migrate mutation functions in verified service modules: `accounts.rs`, `actions.rs`, `people.rs`, `meetings.rs`, `emails.rs`, `projects.rs`, `success_plans.rs`, `mutations.rs`, `feedback.rs`, `settings.rs`, `user_entity.rs`, `entity_context.rs`, `entity_linking/*`, `intelligence.rs`, `hygiene.rs`, `linear.rs`, and related service files found by grep.'
What needs to change: Add a table of every mutation function with file, function name, mutation type, required first line, clock/RNG changes, and test coverage. Remove catch-alls like "related service files found by grep."

### F3 — The core mutation taxonomy is still an open question (severity: Critical)
The plan cannot both require every mutator to gain `ctx.check_mutation_allowed()?` and leave the definition of mutation undecided. That is exactly the implementer-invents-shapes failure mode L0 is supposed to block.
Location in plan: §10 'What exactly counts as a mutation for the ~60-site sweep: only DB write verbs, raw SQL `execute`, signal emission, filesystem writes, or all of them?'
What needs to change: Define the taxonomy now: DB writes, transaction wrappers, raw SQL, signal emission, filesystem writes, external side effects, and any explicit exclusions with rationale tied to DOS-209/ADR-0104/W1 write fence.

### F4 — W3-facing seam is not frozen (severity: Critical)
W3 depends on `ServiceContext`, but §10 leaves the external-client and error shapes open. DOS-209 already requires `external: ExternalClients`, `tx: Option<TxHandle>`, `WriteBlockedByMode`, and explicit constructors; leaving these unresolved means W3 can consume a seam that breaks.
Location in plan: §10 'Does `ExternalClients` land as a concrete struct, trait object bundle, or placeholder wrappers until ADR-0107 fixtures exist?' and 'What is the canonical `ServiceError` integration path for current services that still return `String`?'
What needs to change: Pick the concrete `ExternalClients`, `TxHandle`, constructor, and `ServiceError::WriteBlockedByMode` integration shapes in the plan, including public/private visibility and migration strategy for current `String` errors.

### F5 — Capability and raw-handle leakage is asserted, not designed (severity: High)
The security section says raw handles must not be exposed, while §2 says `db` and external carriers are included. There is no API visibility plan, accessor boundary, ability-facing contract, or test that prevents `AbilityContext` from bypassing mode-aware routing.
Location in plan: §4 'Capability-handle leakage is limited here because W2-A does not build the ability registry, but the ServiceContext API must not expose raw DB or external handles that later ability code can bypass.'
What needs to change: Specify the exact visibility of `db`, `tx`, and external wrappers; define any `ctx.services.db()` accessor; and add a test/lint target proving ability-facing code cannot receive raw `ActionDb` or live external clients.

### F6 — W2-B coordination ignores `services/intelligence.rs` (severity: High)
The plan includes `services/intelligence.rs` in W2-A's sweep, but §7 coordinates only provider files. DOS-259 extracts PTY/Glean paths from the existing intelligence service, so ownership/order for `services/intelligence.rs` must be explicit.
Location in plan: §2 '`intelligence.rs`' and §7 'W2-B/DOS-259 owns `src-tauri/src/intelligence/provider.rs`, `glean_provider.rs`, and `pty_provider.rs`; W2-A does not touch those files.'
What needs to change: Add a W2-A/W2-B edit boundary for `src-tauri/src/services/intelligence.rs`: exact functions W2-A may change, functions W2-B may extract, and merge ordering or rebase protocol.

### F7 — Transaction fallback is not implementable safely (severity: High)
The plan names the HRTB fallback but does not specify the fallback signature, `TxCtx` surface, nested-transaction behavior, rollback semantics, or the ADR-0104 ban on external/LLM calls inside transactions.
Location in plan: §8 'HRTB slippage rolls to the ticket-approved sync-within-async fallback.'
What needs to change: Define both primary and fallback transaction APIs, including whether the fallback closure is sync-only, how it avoids holding SQLite writer locks across `.await`, what `TxCtx` exposes, how nesting returns `NestedTransactionsForbidden`, and how rollback on error/panic is tested.

### F8 — Test evidence is named but not binding or generative (severity: High)
§9 uses "such as" and names a property test without generators. One integration test against one mutation cannot prove the ~60-site sweep, and the lint test does not specify CI wiring or patterns covering both `Utc::now()` and `chrono::Utc::now()` forms.
Location in plan: §9 'Add focused tests such as `services::context::tests::proptest_check_mutation_allowed_modes`, `services::context::tests::constructors_set_expected_modes`, `src-tauri/tests/dos209_mode_boundary.rs::evaluate_mutation_returns_write_blocked_by_mode`, and `src-tauri/tests/dos209_lint_regex_test.rs::lint_blocks_direct_utc_now_and_thread_rng`.'
What needs to change: Make test names mandatory, define concrete proptest generators for `ExecutionMode` and mutation attempts, add per-category real-mutator coverage or generated guard coverage, and name the CI lint file/command plus exact regexes for `Utc::now`, `chrono::Utc::now`, and `thread_rng`.

### F9 — Performance relies on assumptions instead of baseline comparison (severity: Medium)
The plan touches every mutating service path and transaction behavior but provides no measurement command or comparison against the W1 Suite P baseline.
Location in plan: §5 'without benchmark data, assume sub-microsecond local overhead and no meaningful p99 shift unless `with_transaction_async` accidentally lengthens SQLite writer locks.'
What needs to change: Add a minimal measurement plan or explicit W1 baseline comparison artifact for transaction lock duration and representative mutator overhead, even if W2 has no new Suite P gate.

## Summary
REVISE: the plan is directionally aligned with DOS-209, but it is not implementable at L0 because the frozen contract, mutation sweep, ServiceContext seam, and transaction fallback still require implementer judgment. The main risk is not bad code; it is that three downstream waves consume an underspecified runtime boundary and then force L2/L3 rework.

## If APPROVE
n/a

## If REVISE
1. Replace §1 with verbatim DOS-209 frozen contract lines, not selected paraphrase.
2. Enumerate every mutation site and define the mutation taxonomy before coding.
3. Freeze `ServiceContext`, `ExternalClients`, `TxHandle`, `WriteBlockedByMode`, and raw-handle visibility shapes.
4. Specify the primary and fallback transaction APIs, including safety constraints and tests.
5. Add explicit W2-B ownership/order for `src-tauri/src/services/intelligence.rs` and exact W1 migration/write-fence coordination if any migration appears.
6. Make test and CI-lint evidence binding, generative, and broad enough to prove the sweep and direct clock/RNG ban.
