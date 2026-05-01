# Implementation Plan: DOS-301

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-301 makes `intelligence_claims` the source of truth for claim-shaped state while keeping legacy consumers alive through derived projections. Load-bearing ticket lines: "Per Storage Shape Review and founder direction (Option B: single-source-of-truth in spine), `intelligence_claims` is the only writer for claim-shaped state." "`DB projections sync inside the claim transaction` with shared input validation upfront so format errors are caught at claim-write time." "Immediately after DB commit, `services/derived_state.rs` writes `intelligence.json` synchronously in the same Tauri command / service call. No job queue, no eventual-consistency window." "If the file write fails: log error, mark `claim_projection_status='failed'`, return success to the caller (claim was committed)."

The 2026-04-24 projection-boundary amendment applies in full: "DB projections can be synchronous in the same SQLite transaction; `intelligence.json` cannot have rollback-equivalent transaction semantics with the DB." The chosen boundary is sync DB projection inside the claim transaction, plus sync-best-effort file projection after commit. Round 1 sync-in-transaction is rejected for file rollback fallacy; round 2 async queue is rejected for stale-window risk. Bundle 5 reads from claims, not file caches.

Storage-shape-review Knock-ons A/D/E also apply. Knock-on A requires dual projection during the spine because reader migration is v1.4.1 (`.docs/plans/storage-shape-review-2026-04-24.md:196`). Knock-on D names `success_plans` as unowned (`:199`), and Knock-on E names `accounts.company_overview` as unowned (`:200`); DOS-301 owns those as explicit projection targets for v1.4.0 compatibility.

## 2. Approach

Create `src-tauri/src/services/derived_state.rs` and register it beside existing services (`src-tauri/src/services/mod.rs:5-28`). It owns `ProjectionTarget`, `ProjectionStatus`, `ProjectionOutcome`, `DerivedStateError`, `project_claim_to_db_legacy_tx`, `project_claim_to_file_legacy_best_effort`, `repair_failed_claim_projections`, and the target-specific rules. "Entity intelligence" remains the logical target name from the ticket, but the live cache is decomposed into `entity_assessment` plus `entity_quality`; `ActionDb::upsert_entity_intelligence` writes those tables today (`src-tauri/src/intelligence/io.rs:2102-2280`) and readers reconstruct from them (`:2283-2395`).

Add `src-tauri/src/validators/json_columns.rs` plus module wiring. Move the inline JSON-column check from `db/accounts.rs:1197-1208` into shared functions: `validate_company_overview_json`, `validate_strategic_programs_json`, `validate_notes_text`, and `validate_account_ai_column(field, value)`. `db/accounts.rs` calls the shared validator before any still-allowed account-field path; W3-C `services/claims.rs::propose_claim` calls the same API for matching `(claim_type, field_path)`. `services/accounts.rs:2293-2307` currently duplicates strategic-program JSON validation and must also route through this module.

Add migration after DOS-7. Current tail is DOS-311 registered as version 125 (`src-tauri/src/migrations.rs:588-590`). DOS-7's plan reserves version 126 for claim schema; DOS-301 should tentatively add `src-tauri/src/migrations/126_dos_301_claim_projection_status.sql` registered as version 127, unless W3-C takes a different slot. Schema:

```sql
CREATE TABLE claim_projection_status (
  claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
  projection_target TEXT NOT NULL CHECK (projection_target IN (
    'entity_intelligence', 'success_plans', 'accounts_columns', 'intelligence_json'
  )),
  status TEXT NOT NULL CHECK (status IN ('committed', 'failed', 'repaired')),
  error_message TEXT,
  attempted_at TEXT NOT NULL,
  succeeded_at TEXT,
  PRIMARY KEY (claim_id, projection_target)
);
CREATE INDEX idx_claim_projection_status_failed
  ON claim_projection_status(projection_target, status)
  WHERE status = 'failed';
```

W3-C integration: `commit_claim` validates the proposal, inserts the claim, then calls `derived_state::project_claim_to_db_legacy_tx(ctx, tx, &claim)` inside the same `BEGIN IMMEDIATE` transaction (`src-tauri/src/db/core.rs:60-87`). After commit, `commit_claim` calls `project_claim_to_file_legacy_best_effort(ctx, &claim)`. Non-Live modes do not project: ADR-0104 says Simulate/Evaluate block mutations and side effects (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:25-42`, `:154-177`).

Refactor legacy writers through claims, not readers. Leave legacy reader code alone per wave instruction. The write inventory starts with `services/intelligence.rs` DB/file writes (`:523-590`, `:912-948`, `:1255-1328`, `:1694-1728`, `:1836-1867`), intel queue final enrichment writes (`src-tauri/src/intel_queue.rs:2436-2595`), progressive dimension writes (`:1988-2020`), account AI columns (`src-tauri/src/db/accounts.rs:1277-1284`), and I644 backfill writes (`src-tauri/src/db/core.rs:762-870`). `scripts/check_write_fence_usage.sh:21-22` already says the transitional `services/intelligence.rs` and `intel_queue.rs` allowlist is removed after W3; DOS-301 is that cleanup.

Add `src-tauri/src/bin/repair_claim_projections.rs` and a `[[bin]]` entry beside existing repair binaries (`src-tauri/Cargo.toml:101-113`). CLI: `cargo run --bin repair_claim_projections -- [--target=...] [--entity=...]`. It scans failed status rows, reloads the authoritative claims, reruns idempotent projections, and prints per-target before/after counts without logging claim text.

## 3. Key decisions

DB projection rule failures do not roll back the authoritative claim. Shared validators reject malformed projection payloads before insert; after that, per-target projection errors become `claim_projection_status.status='failed'` rows. This is the only way to satisfy the acceptance criterion "one rule fails -> others succeed -> status row records failure" while keeping `intelligence_claims` authoritative.

SAVEPOINT API is explicit. `services::claims::TxCtx` must expose `tx.savepoint("rule_name")?`; each rule uses:

```rust
let sp = tx.savepoint("entity_intelligence")?;
match project_entity_intelligence(&sp, claim) {
    Ok(()) => { mark_committed(tx, claim.id, ProjectionTarget::EntityIntelligence)?; sp.commit()?; }
    Err(e) => { sp.rollback()?; mark_failed(tx, claim.id, ProjectionTarget::EntityIntelligence, &e)?; }
}
```

The status write happens outside the failed rule's savepoint but inside the claim transaction. If status recording itself fails, abort the transaction, because repair would have no durable worklist.

File projection uses the DOS-311 fence directly, not `post_commit_fenced_write`, because DOS-301 must return an outcome and mark status. Use `FenceCycle::capture` plus `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:67-109`, `:223-238`) after DB commit. On `FenceError::EpochAdvanced` or `WriteFailed`, write `claim_projection_status(status='failed', projection_target='intelligence_json')` in a new writer-lane call and still return success to the original caller.

ProjectionTarget labels are stable public values: `entity_intelligence`, `success_plans`, `accounts_columns`, `intelligence_json`. The first maps to current `entity_assessment`/`entity_quality`; the label stays ticket-compatible so repair tooling and DOS-302 manifest terms do not churn.

Projection is idempotent. Rules calculate the full legacy target state from current active claims for the subject and replace/upsert deterministically. The repair command uses the same functions as commit-time projection; no separate repair logic is allowed.

DOS-302 manifest is consumed, not reinvented. The Linear amendment requires a mapping from `(claim_type, field_path)` to target, merge behavior, tombstone/delete behavior, ordering, parity assertion, and transactional/cache classification. If DOS-302 stays contract-only, DOS-301 must still implement against its frozen manifest.

## 4. Security

Primary risk is cross-account projection bleed: a claim attached to the wrong subject or field path could update another account's legacy cache. Every rule validates `subject_ref`, entity type, target entity id, and field-path compatibility before writing. The DOS-287 guard in `intel_queue.rs:2460-2554` remains pre-persistence defense for enrichment; DOS-301 adds a regression proving projections cannot copy another account's claim into the target account.

Input validation centralizes in `validators/json_columns.rs`. No projection rule parses ad hoc JSON; `company_overview` and `strategic_programs` use typed `serde_json` validation before `commit_claim`. `notes` stays text but passes length/control-character validation so repair cannot write malformed data into UI surfaces.

The repair binary is a privileged local operator tool. It opens the local encrypted DB like existing binaries (`src-tauri/src/bin/reconcile_post_migration.rs:48-54`), accepts optional `--target` and `--entity`, and never logs claim text, account names, source excerpts, or PII. Logs use claim id, target, status, error class, and counts.

Mode-aware behavior is a security boundary. `project_claim_to_db_legacy_tx` starts with `ctx.check_mutation_allowed()` and also no-ops for `Simulate`/`Evaluate` as defense in depth. ADR-0104's non-Live modes must not mutate DB, files, queues, signals, or external systems (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:225-231`, `:256-265`).

## 5. Performance

Hot path is `commit_claim` under SQLite's single writer. The live DB service has one writer connection and N reader connections (`src-tauri/src/db_service.rs:11-18`, `:233-237`); `AppState::db_write` serializes mutations through that writer (`src-tauri/src/state.rs:1214-1241`). The prompt's `state.rs:983-1009` reference is stale in the current tree; those lines now cover context-provider atomic swap, not writer constraints.

Budget: projection p99 below 50ms at W3 gate, with separate numbers for DB projection and post-commit file write. DB target rules must avoid LLM/provider calls and external I/O inside the transaction, matching ADR-0104's ban on provider/external calls in transactions (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:227-231`).

SAVEPOINT overhead is accepted but measured. Each committed claim pays up to three DB projection savepoints plus status upserts; WAL, `busy_timeout=5000`, and `synchronous=NORMAL` are already configured on connections (`src-tauri/src/db/core.rs:134-144`, `:210-217`). Suite P measures savepoint cost and writer-lane contention under concurrent claim commits.

File projection happens after commit, so it does not extend the claim transaction. It still runs synchronously in the service call to avoid the round-2 stale-window. Large `intelligence.json` regeneration should read active claims with indexed subject/field filters and serialize once per entity touch.

## 6. Coding standards

Services-only mutations: ADR-0101 says services own all domain mutations (`.docs/decisions/0101-service-boundary-enforcement.md:21-36`). DOS-301 creates the one service exception for legacy projection writes: `services/derived_state.rs`. Commands, queue workers, db helpers, and dev tools call claims/derived-state services instead of updating legacy AI surfaces directly.

CLAUDE.md 5-question check (`CLAUDE.md:7-14`): `claim_projection_status` emits no product signal because it is repair metadata; claim commits already drive invalidation. Projected `success_plans` and account columns can feed health/context through existing readers. Briefing callouts continue reading legacy projections during the spine. Feedback into Bayesian source weights is through DOS-294 `claim_feedback`, not projection status.

No `Utc::now()` or `thread_rng()` in new services. Use `ctx.clock.now()` for `attempted_at` and `succeeded_at`. Existing direct clock calls in `intel_queue.rs:1971`, `:2369`, and account/services paths are not copied into DOS-301.

CI lints: add `scripts/check_dos301_legacy_projection_writers.sh` and `src-tauri/tests/dos301_projection_lint_test.rs` to reject direct writes to `entity_assessment`, `entity_quality`, `account_objectives`, `account_milestones`, `accounts.company_overview`, `accounts.strategic_programs`, `accounts.notes`, and `write_intelligence_json` outside allowed projection/fence implementation files. Add `scripts/check_dos301_json_validators.sh` and `src-tauri/tests/dos301_json_validator_lint_test.rs` to reject inline `serde_json::from_str` validation for those account columns outside `validators/json_columns.rs`.

## 7. Integration with parallel wave-mates

W3-C / DOS-7 is the closest collaborator. DOS-7 lands `intelligence_claims`, `claim_feedback` skeleton, and `services/claims.rs::commit_claim` first; DOS-301 layers `claim_projection_status` and projection calls onto that API in the same PR train. Migration numbering: if DOS-7 takes version 126, DOS-301 takes version 127; do not merge both with the same migration slot.

W3-C also needs `validators/json_columns.rs` early. `propose_claim` calls the shared validators from day one, while `db/accounts.rs:1197-1208` stops owning duplicate JSON parsing. If W3-D lands after W3-C, W3-C temporarily blocks on a small validator stub or rebases to consume this module.

W3-E / DOS-294 feedback writes must be observed by projection. Minimum DOS-301 test stub: insert or update a claim via W3-E's `claim_feedback` path, then assert projection sees the resulting claim state and either updates or tombstones the legacy target. ADR-0123 makes `claim_feedback` append-only and idempotent (`.docs/decisions/0123-typed-claim-feedback-semantics.md:91-106`); DOS-301 must not mutate feedback rows.

Hard dependencies remain DOS-308 (`is_suppressed` correctness), DOS-310 (per-entity invalidation), and DOS-311 (write fence). Storage-shape Finding 4 says `is_suppressed()` already carries tombstone resurrection semantics (`.docs/plans/storage-shape-review-2026-04-24.md:64-72`); DOS-301's Bundle 5 parity reads claims and verifies projections do not resurrect suppressed content.

## 8. Failure modes + rollback

DB projection failure for one target: rollback that target's savepoint, mark `failed`, continue other targets, commit claim. A synthetic trigger failure on `entity_assessment` must still leave `accounts_columns` and `success_plans` committed with their own status rows.

File projection failure: DB is already committed. Mark `intelligence_json` failed and return success. If the status update fails too, log a high-severity repair message; the caller still succeeds because the claim is committed, but the wave gate test must cover the normal path where status is durable.

Migration failure: no schema version should be recorded unless SQL succeeds. The existing migration runner records after successful execution (`src-tauri/src/migrations.rs:1078-1085`); DOS-301 should stay pure SQL for `claim_projection_status` so rollback is the normal SQLite migration rollback/restore path. No destructive data migration is needed for this table.

Repair rollback is idempotency. Rerunning repair after partial success recomputes projections from active claims and overwrites the same deterministic target state. DB targets run in their own transaction; file target uses the DOS-311 fence and records failed again if the epoch advances or the filesystem write fails.

W1-B universal write fence is honored: all `intelligence.json` projection writes go through `fenced_write_intelligence_json`. Direct `write_intelligence_json` remains implementation-only inside `intelligence/io.rs` and `intelligence/write_fence.rs` (`scripts/check_write_fence_usage.sh:12-22`).

## 9. Test evidence to be produced

Unit/integration tests: `dos301_commit_claim_projects_db_targets_in_transaction`, `dos301_savepoint_failure_isolates_entity_intelligence_target`, `dos301_file_projection_failure_marks_failed_and_returns_success`, `dos301_malformed_account_json_rejected_before_claim_insert`, `dos301_simulate_evaluate_skip_legacy_projection`, `dos301_repair_command_reprojects_failed_targets`, `dos301_claim_feedback_state_change_reprojects_legacy_stub`.

Property and parity tests: `proptest_dos301_projection_idempotent_by_claim_target`, `dos301_shared_validator_parity_accounts_and_propose_claim`, `dos301_reader_smoke_dev_db_twenty_entities_claims_match_legacy`, `dos301_bundle5_correction_resurrection_reads_claims_not_files`, and `dos301_cross_account_bleed_regression_dos287_green`.

Failure simulation details: DB I/O failure test uses a temporary SQLite trigger such as `CREATE TEMP TRIGGER dos301_fail_entity_assessment BEFORE INSERT ON entity_assessment BEGIN SELECT RAISE(FAIL, 'dos301 synthetic'); END;`. File failure test points the entity projection at a read-only or missing directory and asserts DB status plus caller success.

Lint evidence: `dos301_projection_lint_blocks_direct_legacy_writes`, `dos301_projection_lint_allows_derived_state_only`, `dos301_json_validator_lint_blocks_inline_from_str`, and current-workspace clean runs for both lint scripts.

Wave merge-gate artifact: attach reviewer-ready evidence for architect-reviewer, security-auditor, and performance-engineer: migration number, projection manifest coverage, status-table counts, repair dry-run output, DOS-287 regression, p99 projection latency/SAVEPOINT cost, and `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`. Suite S gets SQL injection/log/tenant evidence; Suite P gets projection latency and writer-lane contention; Suite E gets validator fuzz, idempotency, Bundle 5, and reader parity.

## 10. Open questions

1. DOS-302 blocker status: Linear says DOS-301 is blocked by DOS-302, and the amendment requires a projection manifest before implementation. Is DOS-302 a hard PR-open dependency, or is its manifest already absorbed into the W3 plan?
2. `db/accounts.rs` conflict: the ticket requires `db/accounts.rs:1197-1208` to call shared validators, but also says no direct legacy AI writes outside `services/derived_state.rs`. Should user-authored `services/accounts.rs` notes/program edits become claim writes now, or is the lint scoped to AI/projection writes only?
3. `success_plans` projection scope: should DOS-301 project only AI-suggested `success_plan_signals_json`, or also create/update `account_objectives` and `account_milestones` rows? DOS-302 manifest should close this.
4. Repair command shape: prompt calls it a "Tauri command" but specifies `cargo run --bin repair_claim_projections`. Plan assumes a binary plus shared service function; confirm no frontend-invoked Tauri command is required in W3.
5. File-status update failure policy: if `intelligence.json` write fails and the follow-up status write also fails, should the operator-facing result remain success with log-only repair, or should `commit_claim` surface a warning payload?
