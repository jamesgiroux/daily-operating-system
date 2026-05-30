# L0 review cycle 2 — DOS-209 plan v2 — codex challenge mode

**Reviewer:** /codex challenge (cycle 2)
**Plan revision under review:** v2 (2026-04-28)
**Verdict:** ESCALATE-TO-L6

## Cycle 1 finding closures verified

### F1 — closed: partial
Closure location in v2: §1 "Verbatim frozen contract from Linear DOS-209:"
Verification: v2 copies the Problem, Scope limits, Acceptance criteria, Edge cases, Build-ready checklist, and Done checklist blocks directly enough to close the narrow cycle-1 omissions. It does not quote the full frozen Linear ticket: `Why now`, `Intelligence Loop fit`, `Architectural surfaces touched`, `Dependencies`, and `Known limitations` are absent from §1, and the omitted Dependencies block matters because v2 later contradicts the ticket's landing order.

### F2 — closed: partial
Closure location in v2: §3 "Mutation catalogue from `rg` audit" and §3 "`emails.rs` | `get_emails_enriched:SIG+FS`; ... `archive_email:D+SIG+EXT`; ... `retry_failed_emails:D+BG`"
Verification: v2 now has a real table and no "..." catch-all. It is not actually exhaustive. The current code has mutation-looking functions omitted from the table, including `emails::unarchive_email` at `src-tauri/src/services/emails.rs:1124` (`db.unarchive_email` + Gmail external side effect), `emails::unsuppress_email` at line 1181 (`db.unsuppress_email`), `emails::pin_email` at line 1186 (`db.toggle_pin_email` + signal), `accounts::snooze_triage_item` at `src-tauri/src/services/accounts.rs:1941`, and `entity_linking/rules/p2_thread_inheritance.rs:19` (`db.enqueue_thread_inheritance`). Mentioned with a table is not closed if the catalogue misses live mutators.

### F3 — closed: yes
Closure location in v2: §3 "Mutation taxonomy: a mutation is any service function that performs a domain DB write through `insert/update/upsert/delete` methods, raw SQL `execute`/`execute_batch` that changes state, transaction wrapper, signal emission, filesystem write/delete/rename/permission change, background queue/in-memory scheduler side effect, or external side effect."
Verification: This directly closes the previous open taxonomy question. It also names exclusions: "Pure reads, SELECT-only raw SQL, pure value transforms, and `#[cfg(test)]` fixtures are excluded."

### F4 — closed: yes
Closure location in v2: §3 "`ServiceContext` visibility is frozen. `mode`, `clock`, `rng`, and mode-aware `external` are public read capabilities. `db`, `signals`, `intel_queue`, and `tx` are private fields."
Verification: v2 names the concrete `ExternalClients` wrapper shape, the `TxHandle`/`TxCtx` visibility boundary, and `ServiceError` variants: "`ServiceError` includes `WriteBlockedByMode(ExecutionMode)` and `NestedTransactionsForbidden`." This is enough L0 shape-freezing for implementation.

### F5 — closed: yes
Closure location in v2: §4 "Ability code must not receive raw app state, raw `ActionDb`, raw SQL, live filesystem writers, live queues, or live external clients."
Verification: v2 specifies private/raw-handle boundaries in §3 and binding proof in §4: "trybuild compile-fail test proving code under an ability-facing module cannot call `ActionDb::open`, receive `&ActionDb`, call `ctx.services.raw_db_for_service`, or construct Live external wrappers in Simulate/Evaluate." This is an actual capability-boundary design, not just an assertion.

### F6 — closed: partial
Closure location in v2: §7 "W2-B opens/lands first per coordination guidance. W2-B owns provider extraction from `services/intelligence.rs`... W2-A then rebases and touches only service mutation boundaries in `services/intelligence.rs`..."
Verification: v2 now gives an explicit edit boundary for `services/intelligence.rs`, including the exact W2-A mutation functions and the rule that moved mutation paths keep the guard before L2. The closure is only partial because the chosen sequencing contradicts the frozen DOS-209 ticket's Dependencies block, which says DOS-209 lands first and then IntelligenceProvider extraction follows.

### F7 — closed: yes
Closure location in v2: §3 "Primary transaction API: `with_transaction_async` accepts `for<'tx> FnOnce(TxCtx<'tx>) -> Pin<Box<dyn Future<Output = Result<T, ServiceError>> + 'tx>>`. Fallback if HRTB slips: a sync closure executed inside the existing SQLite writer lane from async callers, with no `.await` inside the transaction body."
Verification: v2 defines the primary API, fallback API, nested transaction behavior, rollback semantics, and the ADR-0104 external/LLM ban via `TxCtx` surface. §9 also binds tests for `nested_transaction_forbidden`, rollback on error/panic, and `txctx_has_no_external_clients`.

### F8 — closed: partial
Closure location in v2: §9 "Mandatory tests and commands:" and §9 "`services::context::tests::proptest_check_mutation_allowed_modes`: generator is `prop_oneof![Live, Simulate, Evaluate]` crossed with `MutationAttempt { kind: D|SQL|TX|SIG|FS|BG|EXT, public_catalog_index, uses_clock, uses_rng }`"
Verification: v2 makes the test names mandatory, specifies concrete generators, adds catalogue coverage, and lists exact clock/RNG lint regexes. It remains partial because the binding CI command in §9 narrows execution to `cargo test ... dos209`, while both DOS-209 acceptance and the W2 merge gate require the full `cargo test` suite to be green after this broad service migration.

### F9 — closed: yes
Closure location in v2: §5 "Measurement plan: before coding, copy W1 baseline numbers from `.docs/plans/wave-W1/proof-bundle.md`; after migration, run the same Suite P mutator microbench set plus a transaction-lock-duration probe for `with_transaction_async`."
Verification: v2 replaces the previous assumption with a concrete baseline-comparison artifact and a transaction-lock-duration probe. This closes the Medium performance finding.

## Fresh findings introduced in v2 (if any)

### NF1 — Exhaustive catalogue claim is false against current services tree (severity: Critical)
v2 over-corrects F2 by claiming a mechanical `rg` catalogue, but the catalogue misses live mutators. The omissions are not edge trivia: `unarchive_email`, `unsuppress_email`, `pin_email`, `snooze_triage_item`, and the P2 thread-inheritance enqueue path are exactly the DB, external, signal, and queue side effects DOS-209 is supposed to gate.
Location: §3 "Mutation catalogue from `rg` audit" and §3 "`emails.rs` | `get_emails_enriched:SIG+FS`; ... `archive_email:D+SIG+EXT`; ... `retry_failed_emails:D+BG`"
What needs to change: Re-run the audit from the current tree, include every omitted mutator or explicitly justify exclusions under the taxonomy, and name the audit script/source that generates `dos209_mutation_catalog.rs` so the plan table and CI catalogue cannot drift.

### NF2 — W2-B-first sequencing violates DOS-209 frozen landing order (severity: Critical)
v2 says W2-B should open and land first, but the frozen DOS-209 Dependencies block says: "Landing order: this issue first; then IntelligenceProvider trait extraction (separate issue, in parallel); then everything else." This is not a reviewer preference. It is a contract conflict introduced while fixing F6.
Location: §7 "W2-B opens/lands first per coordination guidance."
What needs to change: Either restore DOS-209-first sequencing or get L6 approval to override the Linear contract and document the amended landing order.

### NF3 — The mandatory CI command drops required full-suite evidence (severity: High)
v2's command only runs targeted DOS-209 tests before clippy. That undercuts the copied DOS-209 acceptance criterion "`cargo clippy -- -D warnings && cargo test` green after migration" and the W2 merge gate requiring `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green.
Location: §9 "CI command: `cargo test --manifest-path src-tauri/Cargo.toml dos209 && cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings`."
What needs to change: Make the binding command include full `cargo test --manifest-path src-tauri/Cargo.toml`, clippy, and `pnpm tsc --noEmit`; keep targeted `dos209` tests as additional evidence, not a replacement for the full regression suite.

## Verdict rationale

v2 closes most cycle-1 shape questions, but the remaining failures are substantive: the mutation catalogue is not exhaustive, sequencing contradicts the frozen DOS-209 contract, and the mandatory CI evidence is narrower than the wave gate. This is a cycle-2 REVISE outcome, so the correct verdict is ESCALATE-TO-L6, not a third plan revision.

- REVISE: F2, F6, and F8 failed closure verification; NF1 and NF2 are Critical blockers.
- ESCALATE-TO-L6: L6 must decide whether to override DOS-209's landing order and whether the plan author gets another catalogue repair pass despite the two-cycle cap.

## If REVISE → L6 trigger

Per the 2-revision-cycle hard cap: REVISE on cycle 2 escalates to L6, not a third revision attempt. These are not minor polish notes: the catalogue omission directly risks shipping ungated mutations, and the W2-B-first order is a frozen-contract conflict that the author cannot resolve unilaterally.
