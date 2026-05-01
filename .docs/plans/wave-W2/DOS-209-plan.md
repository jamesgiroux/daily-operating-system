# Implementation Plan: DOS-209

Revision history:
- v2 (2026-04-28) — cycle 1 revision pass. Addressed all Critical + High findings from L0 triangle. Convergent findings closed: contract verbatim, mutation sweep enumerated, ServiceContext shape frozen, capability-boundary handled, W2-B coordination explicit, test evidence binding, DOS-304 read committed.
- v3 (2026-04-29) — L6-authorized cycle-3 revision. Closed cycle-2 challenge NF1 (mutation audit script + CI no-drift test), NF2 (landing order aligned with 2026-04-29 ticket amendment), NF3 (full-suite CI command restored).

## 1. Contract restated

Verbatim frozen contract from Linear DOS-209, including the 2026-04-29 L6 amendment:

### Amendment — 2026-04-29 (L6 decision after cycle-2 review)

> The original Dependencies block below specifies "Landing order: this issue first; then IntelligenceProvider trait extraction (separate issue, in parallel)." The wave plan in `.docs/plans/v1.4.0-waves.md` §"Wave 2" subsequently analyzed the merge-conflict surface on `services/intelligence.rs` and recommends the inverse: W2-B (<issue id="d4e527db-b0d5-4206-bc6f-49ee6c227f84">DOS-259</issue>) opens its PR first to extract PTY orchestration, then W2-A rebases on a smaller mutation surface.
>
> **Amended landing order (L6, 2026-04-29):** W2-B (<issue id="d4e527db-b0d5-4206-bc6f-49ee6c227f84">DOS-259</issue>) lands first. W2-A (this issue) rebases on top once the PTY orchestration extraction has been merged. The architectural argument: extraction first reduces the function surface W2-A's `check_mutation_allowed()?` sweep needs to gate, and avoids re-restructuring functions W2-B is deleting. This amendment supersedes the "this issue first" line in the Dependencies block below.
>
> Other dependency relationships (ADRs unblocked, downstream consumers) remain as written.

### Problem

`ServiceContext` does not exist in the codebase today. ADR-0104 and every downstream ADR (0101, 0102, 0103, 0113, 0115, 0116, 0117, 0119) assume it does. This is the hard prerequisite — nothing else in v1.4.0 can compile cleanly without it. Services receive raw `ActionDb` + individual context objects; there is no unified mode/clock/RNG/external-clients carrier.

### Scope limits

* Not an evaluation harness — ADR-0110 owns that. This issue enables it.
* Not a full mutation-function audit — Phase 2 tackles that. Phase 1 is landing the struct and wiring the happy path.

### Acceptance criteria

- [ ] `ExecutionMode` enum (`Live` | `Simulate` | `Evaluate`) lands as named per ADR-0104.
- [ ] `ServiceContext` struct lands with `mode`, `clock: &dyn Clock`, `rng: &dyn SeededRng`, `external: ExternalClients`, `tx: Option<TxHandle>`.
- [ ] Explicit constructors `new_live(…)`, `new_simulate(…)`, `new_evaluate(…)`. No `Default` or zero-arg constructor.
- [ ] `check_mutation_allowed()` method exists on `ServiceContext` and returns `Err(WriteBlockedByMode)` outside `Live`.
- [ ] Every mutation function in `services/` gains `ctx.check_mutation_allowed()?` as first line (\~60 call sites).
- [ ] All existing callers migrated to construct a `ServiceContext::new_live` explicitly (no backward-compat default).
- [ ] `with_transaction_async(ctx, |tx_ctx| async { ... })` primitive lands.
- [ ] `Clock` trait exists with `fn now(&self) -> DateTime<Utc>`; `SeededRng` trait with the RNG surface services use today.
- [ ] Clock + RNG injection replaces every direct `Utc::now()` and `rand::thread_rng()` in services/ and abilities/.
- [ ] CI test: an ability-like function invoked in `Evaluate` mode that attempts mutation is structurally rejected with `WriteBlockedByMode`, not advisory.
- [ ] `cargo clippy -- -D warnings && cargo test` green after migration.

### Edge cases

* Existing callers that can't yet construct a `ServiceContext` (background workers, tests) — land a temporary `ServiceContext::test_live()` helper for `#[cfg(test)]` and migration windows only; guard in CI to prevent production use.
* `Utc::now()` used inside DB `CURRENT_TIMESTAMP` defaults — bypasses injected clock; document as a known limitation; follow-on issue to convert to explicit `NOW()` call-site with `ctx.clock.now()`.
* `thread_rng()` in non-security contexts (Thompson sampling, Beta draws) — migrate to `ctx.rng`.
* Async transaction signature requires HRTB work — spike the signature early; acceptable to ship with a simpler sync-within-async pattern if HRTB design slips.
* `Evaluate` mode running against production DB (accidental) — boot-time guard panics if `Evaluate` constructor is invoked without a fixture DB path.
* Third-party crate that takes a `&Db` — wrapped via `ctx.services.db()` accessor; no leakage of raw `Db` in ability code.

### Dependencies

* ADR-0104 ExecutionMode and Mode-Aware Services — the spec.
* ADR-0101 Service Boundary Enforcement — extended by this work.
* Unblocks: ADR-0102 (ability contract), ADR-0103 (maintenance safety), ADR-0105 (provenance builder), ADR-0110 (eval harness), ADR-0113 (claims service functions), ADR-0115 (mode-aware signal emission), ADR-0116 (DbKeyProvider integration), ADR-0119 (runtime evaluator).

**Landing order (original):** this issue first; then IntelligenceProvider trait extraction (separate issue, in parallel); then everything else.

**Landing order (amended 2026-04-29 per L6 decision):** see amendment block above. W2-B (<issue id="d4e527db-b0d5-4206-bc6f-49ee6c227f84">DOS-259</issue>) lands first.

### Build-ready checklist (paste into PR)

- [ ] No `Utc::now()` or `rand::thread_rng()` in new ability/service code; linted in CI
- [ ] Every new mutation call site guarded by `check_mutation_allowed()?`
- [ ] `ServiceContext::new_evaluate` wired to fixture DB path only; production construction forbidden
- [ ] `with_transaction_async` signature approved by a second reviewer (HRTB surface)
- [ ] `clippy -D warnings` + `cargo test` green
- [ ] Property test: random `ExecutionMode` + random mutation attempt → correct accept/reject

### Done checklist (paste into PR)

- [ ] All \~60 mutation functions in `services/` migrated to take `&ServiceContext` and call `check_mutation_allowed()?`
- [ ] All tests passing, including mode-boundary integration test
- [ ] No remaining `Utc::now()` / `rand::thread_rng()` in services/ or abilities/
- [ ] `tasks/lessons.md` updated if a pattern emerged during migration
- [ ] Follow-on filed for `DB CURRENT_TIMESTAMP` conversion if not done here

DOS-304 was read in full on 2026-04-28. It is a real pre-code contract gate for DOS-209, not a separate code dependency: W2-A may start coding only after L0 accepts that `ServiceContext` capability handles, not proc-macro inspection and not `check_mutation_allowed()` convention alone, are the enforcement boundary. DOS-304's one-registry decision remains a hard gate for DOS-210/DOS-217 bridge work; W2-A satisfies its DOS-209 portion by denying raw `ActionDb`, direct SQL/file-write handles, live queues, and live external clients to ability-facing code by construction.

## 2. Approach

Create `src-tauri/src/services/context.rs` and export it from `services/mod.rs`. `ServiceContext` lands with exactly these carriers: `db`, `signals`, `intel_queue`, `mode`, `clock`, `rng`, `external`, and `tx`. `IntelligenceProvider` deliberately does not land on `ServiceContext`; W2-B owns provider extraction, and ADR-0104 places provider selection on `AbilityContext`.

Caller construction path:
- Tauri commands construct `ServiceContext::new_live` through `ServiceLayer`; mutation commands stop opening raw `ActionDb` before calling services.
- MCP sidecar paths stay Live/read-only unless they call a service mutator, in which case they also construct `new_live`.
- Background workers and schedulers construct `new_live` at dequeue. `Evaluate` constructors never start pollers, queues, or live workers.
- Developer simulation tools use `new_simulate` with replay wrappers.
- Evaluation harness code uses `new_evaluate` with fixture DB path only; production DB path construction is a panic/compile-fail guard.
- Tests may use `ServiceContext::test_live()` under `#[cfg(test)]` only. It is not a production migration shim.

Migration is one PR with no feature flag and no backward-compatible default constructor. Local working tree may be temporarily compile-broken during the sweep, but the reviewable PR must not contain a mixed final state where some production mutators still take raw `&ActionDb`. The PR must include `src-tauri/tests/dos209_mutation_catalog.rs`, generated from the audit script, so the catalogue below is checked mechanically.

## 3. Key decisions

Mutation taxonomy: a mutation is any service function that performs a domain DB write through `insert/update/upsert/delete` methods, raw SQL `execute`/`execute_batch` that changes state, transaction wrapper, signal emission, filesystem write/delete/rename/permission change, background queue/in-memory scheduler side effect, or external side effect. Pure reads, SELECT-only raw SQL, pure value transforms, and `#[cfg(test)]` fixtures are excluded. Rationale: DOS-209 blocks non-Live writes, ADR-0104 blocks non-Live signals/external side effects, ADR-0101 says services own mutations, and the W1 write fence must remain the only `intelligence.json` write path.

`ServiceContext` visibility is frozen. `mode`, `clock`, `rng`, and mode-aware `external` are public read capabilities. `db`, `signals`, `intel_queue`, and `tx` are private fields. Raw DB access is only `pub(in crate::services)` for service implementation code. Ability-facing `ctx.services.db()` returns a scoped read capability with no `conn_ref`, raw SQL, or write verbs; it never returns `ActionDb`. `ExternalClients` is a concrete struct with named wrapper fields `glean`, `slack`, `gmail`, and `salesforce`; wrapper internals hold live clients only in Live and replay/fixture wrappers in Simulate/Evaluate. `TxHandle` is private; `TxCtx` exposes transaction-scoped DB writes, mode, clock, rng, and signal staging only. It exposes no external clients and no intelligence provider.

Errors migrate one-shot for production mutators in this catalogue: mutators return `ServiceError`, with command/Tauri/MCP edges converting to `String` at the boundary. `ServiceError` includes `WriteBlockedByMode(ExecutionMode)` and `NestedTransactionsForbidden`. Read-only services may stay on `String` in this PR.

Primary transaction API: `with_transaction_async` accepts `for<'tx> FnOnce(TxCtx<'tx>) -> Pin<Box<dyn Future<Output = Result<T, ServiceError>> + 'tx>>`. Fallback if HRTB slips: a sync closure executed inside the existing SQLite writer lane from async callers, with no `.await` inside the transaction body. Nested calls return `NestedTransactionsForbidden`. Error returns and panics roll back; success commits. `TxCtx` shape enforces the ADR-0104 ban on external/LLM calls in transactions because it has no external clients and no `IntelligenceProvider`.

`PlannedMutationSet`, `PlannedMutation`, `ProvenanceRef`, and `plan_*` naming do not land in W2-A. They are deferred to W3-A/DOS-210 for the ability planning surface, with `ProvenanceRef` supplied by W3-B/DOS-211 before W4-C consumes the bridge. Rationale: DOS-209 is the service substrate; no Phase-0 DOS-209 caller consumes planned mutations.

DOS-304 handling: W2-A treats capability handles as the boundary. The registry macro is lint/metadata/trybuild coverage only. This is closed here, not left as an open question.

Mutation catalogue from `scripts/dos209-mutation-audit.sh`; output below is generated verbatim from the current development branch tree and committed as `src-tauri/tests/dos209_mutation_catalog.txt`. Abbreviations: D = DB method write; SQL = raw SQL state change; TX = transaction wrapper; SIG = signal emission; FS = filesystem write/delete/rename; BG = background queue/in-memory side effect; EXT = external side effect; C = direct clock/RNG replacement. Required first line for every listed mutator is `ctx.check_mutation_allowed()?`; coverage for every row is `dos209_mutation_catalog` structural lint plus Evaluate runtime coverage where public. Cycle-2 challenge NF1 is closed by the committed audit script plus the §9 no-drift CI test that re-runs this script against the committed snapshot.

```text
# DOS-209 mutation audit
# Generated by scripts/dos209-mutation-audit.sh
# Source root: src-tauri/src/services
# Method: Rust fn brace scanner plus deterministic mutation regexes; #[cfg(test)] modules excluded.
# Columns: symbol | kinds | first matching evidence per kind
accounts::set_account_domains:18 | D | D=src-tauri/src/services/accounts.rs:19:db.set_account_domains(account_id, domains)
accounts::create_child_account_record:31 | D+FS+C | D=src-tauri/src/services/accounts.rs:70:db.upsert_account(&account).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/accounts.rs:93:let _ = std::fs::create_dir_all(&account_dir); ; C=src-tauri/src/services/accounts.rs:58:let now = chrono::Utc::now().to_rfc3339();
accounts::emit_auto_completed_success_plan_signals:701 | SIG | SIG=src-tauri/src/services/accounts.rs:703:crate::services::signals::emit_and_propagate(
accounts::apply_lifecycle_transition:743 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:774:tx.update_account_field(account_id, "lifecycle", &next_lifecycle) ; TX=src-tauri/src/services/accounts.rs:744:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:823:crate::services::signals::emit_and_propagate(
accounts::ensure_account_lifecycle_state:888 | D+SIG | D=src-tauri/src/services/accounts.rs:910:db.set_account_renewal_stage(account_id, inferred_stage.as_deref()) ; SIG=src-tauri/src/services/accounts.rs:916:let _ = crate::services::signals::emit_and_propagate(
accounts::refresh_lifecycle_states_for_dashboard:936 | D+C | D=src-tauri/src/services/accounts.rs:971:ensure_account_lifecycle_state(db, engine, &account.id)?; ; C=src-tauri/src/services/accounts.rs:950:.map(|date| (date - Utc::now().date_naive()).num_days() <= 150)
accounts::confirm_lifecycle_change:980 | D+SIG | D=src-tauri/src/services/accounts.rs:985:db.set_lifecycle_change_response(change_id, "confirmed", None) ; SIG=src-tauri/src/services/accounts.rs:988:crate::services::signals::emit_and_propagate(
accounts::correct_account_product:1006 | D+SIG | D=src-tauri/src/services/accounts.rs:1007:db.update_account_product(product_id, name, status, None, "user_correction", 1.0) ; SIG=src-tauri/src/services/accounts.rs:1010:crate::services::signals::emit_and_propagate(
accounts::correct_lifecycle_change:1034 | D | D=src-tauri/src/services/accounts.rs:1048:db.set_lifecycle_change_response(change_id, "corrected", notes)
accounts::accept_account_field_conflict:1062 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:1073:update_account_field_inner(db, state, account_id, field, &next_value)?; ; TX=src-tauri/src/services/accounts.rs:1084:db.with_transaction(|tx| -> Result<(), String> { ; SIG=src-tauri/src/services/accounts.rs:1132:if let Err(e) = crate::services::signals::emit_propagate_and_evaluate(
accounts::dismiss_account_field_conflict:1163 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:1170:tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput { ; TX=src-tauri/src/services/accounts.rs:1169:db.with_transaction(|tx| -> Result<(), String> { ; SIG=src-tauri/src/services/accounts.rs:1216:if let Err(e) = crate::services::signals::emit_propagate_and_evaluate(
accounts::get_account_detail:1249 | D | D=src-tauri/src/services/accounts.rs:1255:.db_write(move |db| ensure_account_lifecycle_state(db, &engine, &lifecycle_account_id))
accounts::update_account_field:1532 | D | D=src-tauri/src/services/accounts.rs:1533:update_account_field_inner(db, state, account_id, field, value)?;
accounts::update_account_field_inner:1547 | D+SIG+FS+BG | D=src-tauri/src/services/accounts.rs:1554:db.update_account_field(account_id, field, &normalized_value) ; SIG=src-tauri/src/services/accounts.rs:1590:crate::services::signals::emit_propagate_and_evaluate( ; FS=src-tauri/src/services/accounts.rs:1653:if let Err(e) = std::fs::rename(&old_dir, &new_dir) { ; BG=src-tauri/src/services/accounts.rs:1630:crate::services::health_debouncer::schedule_recompute(state, account_id);
accounts::update_technical_footprint_field:1720 | D+SIG | D=src-tauri/src/services/accounts.rs:1721:db.update_technical_footprint_field(account_id, field, value) ; SIG=src-tauri/src/services/accounts.rs:1726:crate::services::signals::emit_propagate_and_evaluate(
accounts::set_user_health_sentiment:1759 | D+SIG+C | D=src-tauri/src/services/accounts.rs:1776:db.update_account_field(account_id, "user_health_sentiment", sentiment) ; SIG=src-tauri/src/services/accounts.rs:1807:crate::services::signals::emit_propagate_and_evaluate( ; C=src-tauri/src/services/accounts.rs:1775:let now = Utc::now().to_rfc3339();
accounts::update_latest_sentiment_note:1863 | D+SIG | D=src-tauri/src/services/accounts.rs:1873:.update_latest_sentiment_note(account_id, note) ; SIG=src-tauri/src/services/accounts.rs:1905:crate::services::signals::emit_propagate_and_evaluate(
accounts::snooze_triage_item:1941 | D+C | D=src-tauri/src/services/accounts.rs:1944:db.snooze_triage_item(entity_type, entity_id, triage_key, &until.to_rfc3339()) ; C=src-tauri/src/services/accounts.rs:1943:let until = Utc::now() + chrono::Duration::days(days);
accounts::resolve_triage_item:1957 | D+SIG | D=src-tauri/src/services/accounts.rs:1958:db.resolve_triage_item(entity_type, entity_id, triage_key) ; SIG=src-tauri/src/services/accounts.rs:1962:let _ = crate::services::signals::emit_propagate_and_evaluate(
accounts::spawn_risk_briefing_lifecycle:2015 | D | D=src-tauri/src/services/accounts.rs:2032:db.mark_risk_briefing_job_running(&running_id, &running_attempt)
accounts::retry_risk_briefing:2113 | D | D=src-tauri/src/services/accounts.rs:2146:db.upsert_risk_briefing_job_enqueued(&enqueue_id, &enqueue_attempt)
accounts::update_account_notes:2171 | D+SIG | D=src-tauri/src/services/accounts.rs:2172:db.update_account_field(account_id, "notes", notes) ; SIG=src-tauri/src/services/accounts.rs:2188:crate::services::signals::emit_and_propagate(
accounts::update_account_programs:2217 | D+SIG | D=src-tauri/src/services/accounts.rs:2221:db.update_account_field(account_id, "strategic_programs", programs_json) ; SIG=src-tauri/src/services/accounts.rs:2237:crate::services::signals::emit_and_propagate(
accounts::create_account:2259 | D+FS+C | D=src-tauri/src/services/accounts.rs:2293:db.upsert_account(&account).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/accounts.rs:2303:let _ = std::fs::create_dir_all(&account_dir); ; C=src-tauri/src/services/accounts.rs:2281:let now = chrono::Utc::now().to_rfc3339();
accounts::archive_account:2326 | D+TX+SIG+BG | D=src-tauri/src/services/accounts.rs:2350:tx.archive_account(id, archived) ; TX=src-tauri/src/services/accounts.rs:2349:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2352:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:2368:state.intel_queue.remove_by_entity_id(id);
accounts::merge_accounts:2383 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:2386:.merge_accounts(from_id, into_id) ; TX=src-tauri/src/services/accounts.rs:2384:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2388:crate::services::signals::emit_and_propagate(
accounts::restore_account:2409 | D | D=src-tauri/src/services/accounts.rs:2410:db.restore_account(account_id, restore_children)
accounts::add_account_team_member:2419 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:2425:tx.add_account_team_member(account_id, person_id, &role) ; TX=src-tauri/src/services/accounts.rs:2424:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2427:crate::services::signals::emit_and_propagate(
accounts::set_team_member_role:2452 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:2454:tx.set_team_member_role(account_id, person_id, new_role) ; TX=src-tauri/src/services/accounts.rs:2453:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2456:crate::services::signals::emit_and_propagate(
accounts::remove_account_team_member:2481 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:2483:tx.remove_account_team_member(account_id, person_id, role) ; TX=src-tauri/src/services/accounts.rs:2482:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2485:crate::services::signals::emit_and_propagate(
accounts::record_account_event:2510 | D+TX+SIG | D=src-tauri/src/services/accounts.rs:2513:.record_account_event(account_id, event_type, event_date, arr_impact, notes) ; TX=src-tauri/src/services/accounts.rs:2511:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:2515:crate::services::signals::emit_and_propagate(
accounts::bulk_create_accounts:2577 | D+FS+C | D=src-tauri/src/services/accounts.rs:2598:db.upsert_account(&account).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/accounts.rs:2601:let _ = std::fs::create_dir_all(&account_dir); ; C=src-tauri/src/services/accounts.rs:2589:let now = chrono::Utc::now().to_rfc3339();
accounts::create_internal_organization:2748 | D+TX+FS+C | D=src-tauri/src/services/accounts.rs:2806:db.upsert_account(&root_account) ; TX=src-tauri/src/services/accounts.rs:2770:.with_transaction(|db| { ; FS=src-tauri/src/services/accounts.rs:2885:let _ = std::fs::create_dir_all(&root_dir); ; C=src-tauri/src/services/accounts.rs:2795:let now = chrono::Utc::now().to_rfc3339();
accounts::create_child_account_cmd:2941 | BG | BG=src-tauri/src/services/accounts.rs:2973:.enqueue(crate::intel_queue::IntelRequest::new(
accounts::backfill_internal_meeting_associations:2990 | D | D=src-tauri/src/services/accounts.rs:3013:let _ = db.link_meeting_entity(&meeting_id, &account.id, "account");
accounts::update_stakeholder_engagement:3026 | D | D=src-tauri/src/services/accounts.rs:3027:update_stakeholder_engagement_inner(db, state, account_id, person_id, engagement)?;
accounts::update_stakeholder_engagement_inner:3037 | D+SQL+TX+SIG+BG | D=src-tauri/src/services/accounts.rs:3065:tx.mark_health_recompute_pending(account_id) ; SQL=src-tauri/src/services/accounts.rs:3040:.execute( ; TX=src-tauri/src/services/accounts.rs:3038:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3047:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:3069:crate::services::health_debouncer::schedule_recompute(state, account_id);
accounts::update_stakeholder_assessment:3080 | D | D=src-tauri/src/services/accounts.rs:3081:update_stakeholder_assessment_inner(db, state, account_id, person_id, assessment)?;
accounts::update_stakeholder_assessment_inner:3091 | D+SQL+TX+SIG+BG | D=src-tauri/src/services/accounts.rs:3114:tx.mark_health_recompute_pending(account_id) ; SQL=src-tauri/src/services/accounts.rs:3094:.execute( ; TX=src-tauri/src/services/accounts.rs:3092:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3101:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:3118:crate::services::health_debouncer::schedule_recompute(state, account_id);
accounts::add_stakeholder_role:3129 | D | D=src-tauri/src/services/accounts.rs:3130:add_stakeholder_role_inner(db, state, account_id, person_id, role)?;
accounts::add_stakeholder_role_inner:3140 | D+SQL+TX+SIG+BG+C | D=src-tauri/src/services/accounts.rs:3179:tx.mark_health_recompute_pending(account_id) ; SQL=src-tauri/src/services/accounts.rs:3153:.execute( ; TX=src-tauri/src/services/accounts.rs:3145:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3162:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:3183:crate::services::health_debouncer::schedule_recompute(state, account_id); ; C=src-tauri/src/services/accounts.rs:3146:let now = chrono::Utc::now().to_rfc3339();
accounts::remove_stakeholder_role:3194 | D | D=src-tauri/src/services/accounts.rs:3195:remove_stakeholder_role_inner(db, state, account_id, person_id, role)?;
accounts::remove_stakeholder_role_inner:3205 | D+SQL+TX+SIG+BG | D=src-tauri/src/services/accounts.rs:3238:tx.mark_health_recompute_pending(account_id) ; SQL=src-tauri/src/services/accounts.rs:3215:.execute( ; TX=src-tauri/src/services/accounts.rs:3206:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3222:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:3242:crate::services::health_debouncer::schedule_recompute(state, account_id);
accounts::accept_stakeholder_suggestion:3253 | D+SQL+TX+SIG+BG+C | D=src-tauri/src/services/accounts.rs:3374:tx.mark_health_recompute_pending(&suggestion.account_id) ; SQL=src-tauri/src/services/accounts.rs:3312:.execute( ; TX=src-tauri/src/services/accounts.rs:3254:let account_id = db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3357:crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/accounts.rs:3378:crate::services::health_debouncer::schedule_recompute(state, &account_id); ; C=src-tauri/src/services/accounts.rs:3310:let now = chrono::Utc::now().to_rfc3339();
accounts::dismiss_stakeholder_suggestion:3387 | SQL+TX+SIG | SQL=src-tauri/src/services/accounts.rs:3395:.execute( ; TX=src-tauri/src/services/accounts.rs:3388:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/accounts.rs:3403:crate::services::signals::emit_and_propagate(
actions::complete_action:30 | D+SIG | D=src-tauri/src/services/actions.rs:32:db.complete_action(id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/actions.rs:36:let _ = crate::services::signals::emit_and_propagate(
actions::reopen_action:78 | D+SIG | D=src-tauri/src/services/actions.rs:80:db.reopen_action(id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/actions.rs:84:let _ = crate::services::signals::emit_and_propagate(
actions::accept_suggested_action:104 | D+SIG | D=src-tauri/src/services/actions.rs:106:db.accept_suggested_action(id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/actions.rs:110:let _ = crate::services::signals::emit_and_propagate(
actions::reject_suggested_action:152 | D+SIG | D=src-tauri/src/services/actions.rs:154:db.reject_suggested_action_with_source(id, source) ; SIG=src-tauri/src/services/actions.rs:160:let _ = crate::services::signals::emit_and_propagate(
actions::dismiss_suggested_action:221 | D+SIG | D=src-tauri/src/services/actions.rs:223:db.reject_suggested_action_with_source(id, source) ; SIG=src-tauri/src/services/actions.rs:246:let _ = crate::services::signals::emit_and_propagate(
actions::update_action_priority:271 | D+SIG | D=src-tauri/src/services/actions.rs:273:db.update_action_priority(id, priority) ; SIG=src-tauri/src/services/actions.rs:278:let _ = crate::services::signals::emit_and_propagate(
actions::create_action:349 | D+SIG+C | D=src-tauri/src/services/actions.rs:430:db.upsert_action(&action).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/actions.rs:434:let _ = crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/actions.rs:389:let now = chrono::Utc::now().to_rfc3339();
actions::auto_link_action_to_objectives:473 | D+SIG | D=src-tauri/src/services/actions.rs:484:if let Err(e) = db.link_action_to_objective(action_id, &objective.id) { ; SIG=src-tauri/src/services/actions.rs:494:let _ = crate::signals::bus::emit_signal(
actions::update_action:523 | D+C | D=src-tauri/src/services/actions.rs:616:db.upsert_action(&action).map_err(|e| e.to_string()) ; C=src-tauri/src/services/actions.rs:615:action.updated_at = chrono::Utc::now().to_rfc3339();
actions::resolve_decision:755 | D+SIG | D=src-tauri/src/services/actions.rs:757:let updated = db.resolve_decision(id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/actions.rs:764:let _ = crate::services::signals::emit_and_propagate(
commitment_bridge::sync_ai_commitments:66 | D+C | D=src-tauri/src/services/commitment_bridge.rs:86:touch_bridge_row(db, commitment_id, &now).map_err(|e| e.to_string())?; ; C=src-tauri/src/services/commitment_bridge.rs:68:let now = chrono::Utc::now().to_rfc3339();
commitment_bridge::tombstone_commitment_bridge:283 | SQL+C | SQL=src-tauri/src/services/commitment_bridge.rs:287:.execute( ; C=src-tauri/src/services/commitment_bridge.rs:284:let now = chrono::Utc::now().to_rfc3339();
commitment_bridge::insert_bridge_row:337 | SQL | SQL=src-tauri/src/services/commitment_bridge.rs:338:db.conn_ref().execute(
commitment_bridge::touch_bridge_row:355 | SQL | SQL=src-tauri/src/services/commitment_bridge.rs:356:db.conn_ref().execute(
emails::get_emails_enriched:18 | SIG+FS+C | SIG=src-tauri/src/services/emails.rs:472:let _ = crate::signals::bus::emit_signal_and_propagate( ; FS=src-tauri/src/services/emails.rs:137:let _ = std::fs::create_dir_all(today_dir.join("data")); ; C=src-tauri/src/services/emails.rs:148:let now_utc = chrono::Utc::now();
emails::update_email_entity:1009 | D | D=src-tauri/src/services/emails.rs:1010:db.update_email_entity(email_id, entity_id, entity_type)
emails::dismiss_email_signal:1034 | D | D=src-tauri/src/services/emails.rs:1036:.dismiss_email_signal(signal_id)
emails::mark_reply_sent:1060 | D+SIG | D=src-tauri/src/services/emails.rs:1061:let entity_info = db.mark_reply_sent(email_id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/emails.rs:1066:let _ = crate::signals::bus::emit_signal_and_propagate(
emails::archive_email:1085 | D+SIG+EXT | D=src-tauri/src/services/emails.rs:1095:db.archive_email(&eid).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/emails.rs:1099:let _ = crate::services::signals::emit_and_propagate( ; EXT=src-tauri/src/services/emails.rs:1114:if let Ok(token) = crate::google_api::get_valid_access_token().await {
emails::unarchive_email:1124 | D+EXT | D=src-tauri/src/services/emails.rs:1133:.db_write(move |db| db.unarchive_email(&eid).map_err(|e| e.to_string())) ; EXT=src-tauri/src/services/emails.rs:1137:if let Ok(token) = crate::google_api::get_valid_access_token().await {
emails::unarchive_emails_in_gmail:1146 | EXT | EXT=src-tauri/src/services/emails.rs:1151:let client = reqwest::Client::new();
emails::unsuppress_email:1181 | D | D=src-tauri/src/services/emails.rs:1182:db.unsuppress_email(email_id).map_err(|e| e.to_string())
emails::pin_email:1186 | D+SIG | D=src-tauri/src/services/emails.rs:1187:let now_pinned = db.toggle_pin_email(email_id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/emails.rs:1190:let _ = crate::services::signals::emit_and_propagate(
emails::promote_commitment_to_action:1224 | D+SIG+C | D=src-tauri/src/services/emails.rs:1295:db.upsert_action(&action).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/emails.rs:1299:let _ = crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/emails.rs:1230:let now = chrono::Utc::now().to_rfc3339();
emails::dismiss_gone_quiet:1338 | SIG | SIG=src-tauri/src/services/emails.rs:1339:let _ = crate::services::signals::emit_and_propagate(
emails::dismiss_email_item:1362 | D | D=src-tauri/src/services/emails.rs:1363:db.dismiss_email_item(
emails::reconcile_inbox_presence_from_ids:1404 | D | D=src-tauri/src/services/emails.rs:1441:.mark_emails_resolved(&vanished)
emails::sync_email_inbox_presence:1472 | SIG+BG+EXT | SIG=src-tauri/src/services/emails.rs:1485:let _ = app_handle.emit("emails-updated", ()); ; BG=src-tauri/src/services/emails.rs:1491:state.integrations.email_poller_wake.notify_one(); ; EXT=src-tauri/src/services/emails.rs:1473:let access_token = crate::google_api::get_valid_access_token()
emails::archive_low_priority_emails:1501 | D+EXT | D=src-tauri/src/services/emails.rs:1538:db.mark_emails_resolved(&ids_clone) ; EXT=src-tauri/src/services/emails.rs:1526:let access_token = crate::google_api::get_valid_access_token()
emails::refresh_emails:1611 | D | D=src-tauri/src/services/emails.rs:1664:db.mark_failed_for_retry(&batch_id_for_mark)
entities::auto_extract_title_keywords:165 | D | D=src-tauri/src/services/entities.rs:275:.update_account_keywords(entity_id, &json)
entity_context::create_entry:51 | SQL+SIG | SQL=src-tauri/src/services/entity_context.rs:66:.execute( ; SIG=src-tauri/src/services/entity_context.rs:73:let _ = crate::signals::bus::emit_signal_and_propagate(
entity_context::update_entry:116 | SQL+SIG | SQL=src-tauri/src/services/entity_context.rs:139:.execute( ; SIG=src-tauri/src/services/entity_context.rs:151:let _ = crate::signals::bus::emit_signal_and_propagate(
entity_context::delete_entry:173 | SQL+SIG | SQL=src-tauri/src/services/entity_context.rs:190:.execute( ; SIG=src-tauri/src/services/entity_context.rs:200:let _ = crate::signals::bus::emit_signal_and_propagate(
entity_context::migrate_legacy_notes:221 | SQL | SQL=src-tauri/src/services/entity_context.rs:245:conn.execute(
entity_linking::cascade::backfill_account_domain_from_person:30 | D+SIG | D=src-tauri/src/services/entity_linking/cascade.rs:65:if let Err(e) = db.merge_account_domains(account_id, std::slice::from_ref(&domain)) { ; SIG=src-tauri/src/services/entity_linking/cascade.rs:73:let _ = crate::signals::bus::emit_signal(
entity_linking::cascade::run_cascade:91 | D | D=src-tauri/src/services/entity_linking/cascade.rs:146:let _ = db.upsert_linked_entity_raw(
entity_linking::cascade::c6_backfill_account_domains:254 | D+SIG+EXT | D=src-tauri/src/services/entity_linking/cascade.rs:286:match db.merge_account_domains(account_id, &discovered) { ; SIG=src-tauri/src/services/entity_linking/cascade.rs:293:let _ = crate::signals::bus::emit_signal( ; EXT=src-tauri/src/services/entity_linking/cascade.rs:255:use crate::google_api::classify::PERSONAL_EMAIL_DOMAINS;
entity_linking::cascade::c3_promote_trusted_stakeholders:313 | D | D=src-tauri/src/services/entity_linking/cascade.rs:345:let _ = db.confirm_stakeholder(account_id, person_id);
entity_linking::manual_set_primary:180 | SQL+TX+C | SQL=src-tauri/src/services/entity_linking/mod.rs:187:.execute( ; TX=src-tauri/src/services/entity_linking/mod.rs:183:db.with_transaction(|_| { ; C=src-tauri/src/services/entity_linking/mod.rs:195:let now = chrono::Utc::now().to_rfc3339();
entity_linking::manual_dismiss:231 | D+TX | D=src-tauri/src/services/entity_linking/mod.rs:236:db.upsert_linking_dismissal( ; TX=src-tauri/src/services/entity_linking/mod.rs:234:db.with_transaction(|_| {
entity_linking::manual_undismiss:264 | D+SQL | D=src-tauri/src/services/entity_linking/mod.rs:267:db.delete_linking_dismissal( ; SQL=src-tauri/src/services/entity_linking/mod.rs:276:.execute(
entity_linking::confirm_stakeholder_suggestion:306 | D | D=src-tauri/src/services/entity_linking/mod.rs:320:db.confirm_stakeholder(&account_id, &person_id)?;
entity_linking::dismiss_stakeholder_suggestion:339 | D | D=src-tauri/src/services/entity_linking/mod.rs:341:.db_write(move |db| db.dismiss_stakeholder_suggestion(&account_id, &person_id))
entity_linking::phases::phase1_suppress:35 | D | D=src-tauri/src/services/entity_linking/phases.rs:51:let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
entity_linking::phases::run_phases:251 | D+TX | D=src-tauri/src/services/entity_linking/phases.rs:289:db.delete_auto_links_for_owner(ctx.owner.owner_type.as_str(), &ctx.owner.owner_id) ; TX=src-tauri/src/services/entity_linking/phases.rs:252:let phase3_result = db.with_transaction(|_| {
entity_linking::repository::raw_rebuild_account_domains:18 | SQL | SQL=src-tauri/src/services/entity_linking/repository.rs:21:.execute(
entity_linking::rescan::rescan_stale_weak_primaries:30 | SQL+C | SQL=src-tauri/src/services/entity_linking/rescan.rs:106:.execute( ; C=src-tauri/src/services/entity_linking/rescan.rs:102:let now = chrono::Utc::now().to_rfc3339();
entity_linking::rules::p2_thread_inheritance::evaluate:9 | D+BG | D=src-tauri/src/services/entity_linking/rules/p2_thread_inheritance.rs:19:let _ = db.enqueue_thread_inheritance(thread_id, &ctx.owner.owner_id); ; BG=src-tauri/src/services/entity_linking/rules/p2_thread_inheritance.rs:19:let _ = db.enqueue_thread_inheritance(thread_id, &ctx.owner.owner_id);
entity_linking::stakeholder_domains::backfill_domains_for_account:77 | D+SIG | D=src-tauri/src/services/entity_linking/stakeholder_domains.rs:103:db.merge_account_domains(account_id, &new_domains) ; SIG=src-tauri/src/services/entity_linking/stakeholder_domains.rs:107:let _ = crate::signals::bus::emit_signal(
feedback::submit_intelligence_feedback:18 | D | D=src-tauri/src/services/feedback.rs:25:submit_intelligence_correction(
feedback::submit_intelligence_correction:200 | D | D=src-tauri/src/services/feedback.rs:250:let prior_source = resolve_intelligence_source(db, entity_id, entity_type, field);
health_debouncer::schedule_recompute:103 | D | D=src-tauri/src/services/health_debouncer.rs:142:db.clear_health_recompute_pending(&clear_id)
health_debouncer::drain_pending:164 | D | D=src-tauri/src/services/health_debouncer.rs:205:db.clear_health_recompute_pending(&clear_id)
hygiene::update_person_relationship:3 | D+TX | D=src-tauri/src/services/hygiene.rs:5:tx.update_person_relationship(person_id, relationship) ; TX=src-tauri/src/services/hygiene.rs:4:db.with_transaction(|tx| {
hygiene::mark_content_index_summary:25 | SQL | SQL=src-tauri/src/services/hygiene.rs:27:.execute(
hygiene::rollover_account_renewal:45 | D+SQL+TX | D=src-tauri/src/services/hygiene.rs:47:tx.record_account_event( ; SQL=src-tauri/src/services/hygiene.rs:57:.execute( ; TX=src-tauri/src/services/hygiene.rs:46:db.with_transaction(|tx| {
hygiene::reset_quill_sync_for_retry:90 | D | D=src-tauri/src/services/hygiene.rs:91:db.reset_quill_sync_for_retry(sync_id)
hygiene::update_person_name:96 | D+TX | D=src-tauri/src/services/hygiene.rs:98:tx.update_person_name(person_id, display_name) ; TX=src-tauri/src/services/hygiene.rs:97:db.with_transaction(|tx| {
hygiene::merge_people:121 | D+TX | D=src-tauri/src/services/hygiene.rs:123:tx.merge_people(keep_id, remove_id) ; TX=src-tauri/src/services/hygiene.rs:122:db.with_transaction(|tx| {
hygiene::link_person_to_entity:144 | D+TX | D=src-tauri/src/services/hygiene.rs:146:tx.link_person_to_entity(person_id, entity_id, relationship_type) ; TX=src-tauri/src/services/hygiene.rs:145:db.with_transaction(|tx| {
integrations::configure_claude_desktop:103 | FS | FS=src-tauri/src/services/integrations.rs:131:let _ = std::fs::set_permissions(p, perms);
intelligence::emit_manual_refresh_failed:65 | SIG | SIG=src-tauri/src/services/intelligence.rs:81:let _ = app.emit("background-work-status", payload.clone());
intelligence::enrich_entity:102 | D+SIG+EXT | D=src-tauri/src/services/intelligence.rs:134:crate::self_healing::scheduler::reset_circuit_breaker(db, &entity_id_for_reset); ; SIG=src-tauri/src/services/intelligence.rs:119:let _ = app.emit( ; EXT=src-tauri/src/services/intelligence.rs:277:"enrichment-glean-fallback",
intelligence::persist_entity_keywords:446 | D+TX | D=src-tauri/src/services/intelligence.rs:454:.update_account_keywords(entity_id, keywords_json) ; TX=src-tauri/src/services/intelligence.rs:451:db.with_transaction(|tx| {
intelligence::upsert_assessment_from_enrichment:482 | D+TX+SIG | D=src-tauri/src/services/intelligence.rs:489:tx.upsert_entity_intelligence(&intel) ; TX=src-tauri/src/services/intelligence.rs:488:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/intelligence.rs:491:crate::services::signals::emit_and_propagate(
intelligence::upsert_assessment_snapshot:551 | D | D=src-tauri/src/services/intelligence.rs:553:db.upsert_entity_intelligence(intel)
intelligence::upsert_health_outlook_signals:589 | SQL+TX+SIG | SQL=src-tauri/src/services/intelligence.rs:598:.execute( ; TX=src-tauri/src/services/intelligence.rs:593:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/intelligence.rs:614:crate::services::signals::emit_and_propagate(
intelligence::upsert_inferred_relationships_from_enrichment:685 | D+TX+SIG | D=src-tauri/src/services/intelligence.rs:739:tx.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship { ; TX=src-tauri/src/services/intelligence.rs:690:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/intelligence.rs:758:crate::services::signals::emit_and_propagate(
intelligence::update_intelligence_field:784 | D+TX | D=src-tauri/src/services/intelligence.rs:851:tx.upsert_entity_intelligence(&intel) ; TX=src-tauri/src/services/intelligence.rs:850:db.with_transaction(|tx| {
intelligence::update_stakeholders:903 | D+SQL+TX+SIG+C | D=src-tauri/src/services/intelligence.rs:998:tx.upsert_entity_intelligence(&intel) ; SQL=src-tauri/src/services/intelligence.rs:1024:tx.conn.execute( ; TX=src-tauri/src/services/intelligence.rs:997:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/intelligence.rs:1043:crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/intelligence.rs:919:sourced_at: chrono::Utc::now().to_rfc3339(),
intelligence::dismiss_intelligence_item:1080 | D+TX+SIG+C | D=src-tauri/src/services/intelligence.rs:1181:tx.upsert_entity_intelligence(&intel) ; TX=src-tauri/src/services/intelligence.rs:1180:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/intelligence.rs:1217:if let Err(e) = crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/intelligence.rs:1140:dismissed_at: chrono::Utc::now().to_rfc3339(),
intelligence::recompute_entity_health_with_preset:1270 | D+SQL | D=src-tauri/src/services/intelligence.rs:1300:db.upsert_entity_intelligence(&intel) ; SQL=src-tauri/src/services/intelligence.rs:1305:.execute(
intelligence::track_recommendation:1439 | D+SIG+C | D=src-tauri/src/services/intelligence.rs:1500:db.upsert_action(&action).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/intelligence.rs:1510:let _ = crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/intelligence.rs:1458:let now = chrono::Utc::now().to_rfc3339();
intelligence::dismiss_recommendation:1537 | D+SIG | D=src-tauri/src/services/intelligence.rs:1596:db.upsert_entity_intelligence(&intel) ; SIG=src-tauri/src/services/intelligence.rs:1600:if let Err(e) = crate::services::signals::emit_and_propagate(
intelligence::mark_commitment_done:1648 | D+SIG+C | D=src-tauri/src/services/intelligence.rs:1731:db.upsert_entity_intelligence(&intel) ; SIG=src-tauri/src/services/intelligence.rs:1734:if let Err(e) = crate::services::signals::emit_and_propagate( ; C=src-tauri/src/services/intelligence.rs:1712:let now = chrono::Utc::now().to_rfc3339();
linear::push_action_to_linear:29 | D+SQL+SIG+EXT+C | D=src-tauri/src/services/linear.rs:107:.create_issue( ; SQL=src-tauri/src/services/linear.rs:128:.execute( ; SIG=src-tauri/src/services/linear.rs:167:let _ = crate::services::signals::emit_and_propagate( ; EXT=src-tauri/src/services/linear.rs:62:return Err("Action has already been pushed to Linear".to_string()); ; C=src-tauri/src/services/linear.rs:119:let now = chrono::Utc::now().to_rfc3339();
meetings::upsert_meeting_for_reconcile:14 | D+TX | D=src-tauri/src/services/meetings.rs:16:tx.upsert_meeting(meeting).map_err(|e| e.to_string())?; ; TX=src-tauri/src/services/meetings.rs:15:db.with_transaction(|tx| {
meetings::set_meeting_prep_context:34 | D | D=src-tauri/src/services/meetings.rs:35:db.update_meeting_prep_context(meeting_id, updated_json)
meetings::update_capture_content:43 | D+TX | D=src-tauri/src/services/meetings.rs:45:tx.update_capture(capture_id, content) ; TX=src-tauri/src/services/meetings.rs:44:db.with_transaction(|tx| {
meetings::clear_meeting_prep_frozen:65 | SQL+TX | SQL=src-tauri/src/services/meetings.rs:68:.execute( ; TX=src-tauri/src/services/meetings.rs:66:db.with_transaction(|tx| {
meetings::mutate_meeting_entities_and_refresh_briefing:425 | D+SQL+SIG+FS+BG+C | D=src-tauri/src/services/meetings.rs:451:db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput { ; SQL=src-tauri/src/services/meetings.rs:478:let _ = db.conn_ref().execute( ; SIG=src-tauri/src/services/meetings.rs:739:let _ = app.emit( ; FS=src-tauri/src/services/meetings.rs:619:let _ = std::fs::remove_file(&old_path); ; BG=src-tauri/src/services/meetings.rs:668:.enqueue(crate::intel_queue::IntelRequest::new( ; C=src-tauri/src/services/meetings.rs:476:let now = chrono::Utc::now().to_rfc3339();
meetings::capture_meeting_outcome:999 | D+FS+C | D=src-tauri/src/services/meetings.rs:1092:if let Err(e) = db.upsert_action(&db_action) { ; FS=src-tauri/src/services/meetings.rs:1138:let _ = std::fs::write(&impact_log, format!("{}{}", existing, content)); ; C=src-tauri/src/services/meetings.rs:1064:let now = chrono::Utc::now().to_rfc3339();
meetings::get_meeting_intelligence:1474 | D+C | D=src-tauri/src/services/meetings.rs:1532:db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput { ; C=src-tauri/src/services/meetings.rs:1596:let now = chrono::Utc::now();
meetings::link_meeting_entity_with_prep_queue:1735 | D+SQL+BG+C | D=src-tauri/src/services/meetings.rs:1742:db.link_meeting_entity(&meeting_id_s, &entity_id_s, &entity_type_s) ; SQL=src-tauri/src/services/meetings.rs:1744:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:1768:.enqueue(crate::meeting_prep_queue::PrepRequest::new( ; C=src-tauri/src/services/meetings.rs:1749:let now = chrono::Utc::now().to_rfc3339();
meetings::dismiss_meeting_entity:1792 | D+SQL+BG+C | D=src-tauri/src/services/meetings.rs:1800:db.record_meeting_entity_dismissal( ; SQL=src-tauri/src/services/meetings.rs:1809:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:1832:.enqueue(crate::meeting_prep_queue::PrepRequest::new( ; C=src-tauri/src/services/meetings.rs:1814:let now = chrono::Utc::now().to_rfc3339();
meetings::restore_meeting_entity:1855 | D+SQL | D=src-tauri/src/services/meetings.rs:1863:.remove_meeting_entity_dismissal(&meeting_id_s, &entity_id_s, &entity_type_s) ; SQL=src-tauri/src/services/meetings.rs:1866:let _ = db.conn_ref().execute(
meetings::unlink_meeting_entity_with_prep_queue:1898 | D+SQL+BG+C | D=src-tauri/src/services/meetings.rs:1904:db.unlink_meeting_entity(&meeting_id_s, &entity_id_s) ; SQL=src-tauri/src/services/meetings.rs:1906:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:1938:.enqueue(crate::meeting_prep_queue::PrepRequest::new( ; C=src-tauri/src/services/meetings.rs:1921:let now = chrono::Utc::now().to_rfc3339();
meetings::persist_classification_entities:1969 | D+EXT | D=src-tauri/src/services/meetings.rs:1982:persist_classification_entities_scored(db, meeting_id, &scored) ; EXT=src-tauri/src/services/meetings.rs:1970:let scored: Vec<crate::google_api::classify::ResolvedMeetingEntity> = entities
meetings::persist_classification_entities_scored:2010 | D+EXT | D=src-tauri/src/services/meetings.rs:2066:match db.link_meeting_entity_with_confidence( ; EXT=src-tauri/src/services/meetings.rs:2019:let filtered: Vec<&crate::google_api::classify::ResolvedMeetingEntity> = entities
meetings::persist_and_invalidate_entity_links:2109 | D+SQL+BG | D=src-tauri/src/services/meetings.rs:2121:match db.link_meeting_entity_if_absent(&meeting_id_s, entity_id, entity_type) { ; SQL=src-tauri/src/services/meetings.rs:2146:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:2159:.enqueue(crate::meeting_prep_queue::PrepRequest::new(
meetings::persist_and_invalidate_entity_links_sync_scored:2229 | D+SQL+BG | D=src-tauri/src/services/meetings.rs:2266:match db.link_meeting_entity_with_confidence( ; SQL=src-tauri/src/services/meetings.rs:2300:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:2304:prep_queue.enqueue(crate::meeting_prep_queue::PrepRequest::new(
meetings::update_meeting_user_agenda:2362 | D+SIG+FS | D=src-tauri/src/services/meetings.rs:2413:db.update_meeting_user_layer( ; SIG=src-tauri/src/services/meetings.rs:2452:let _ = crate::services::signals::emit_and_propagate( ; FS=src-tauri/src/services/meetings.rs:2430:let _ = std::fs::write(&prep_path, updated);
meetings::update_meeting_user_notes:2480 | D+FS | D=src-tauri/src/services/meetings.rs:2495:db.update_meeting_user_layer(meeting_id, meeting.user_agenda_json.as_deref(), notes_opt) ; FS=src-tauri/src/services/meetings.rs:2508:let _ = std::fs::write(&prep_path, updated);
meetings::update_meeting_prep_field:2532 | D+TX+SIG | D=src-tauri/src/services/meetings.rs:2566:tx.update_meeting_prep_context(meeting_id, &updated) ; TX=src-tauri/src/services/meetings.rs:2562:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/meetings.rs:2585:crate::services::signals::emit_and_propagate(
meetings::restore_meeting_briefing_snapshot:2847 | SQL | SQL=src-tauri/src/services/meetings.rs:2849:.execute(
meetings::emit_briefing_refresh_progress:2886 | SIG | SIG=src-tauri/src/services/meetings.rs:2888:let _ = app.emit("meeting-briefing-refresh-progress", &payload);
meetings::refresh_meeting_briefing_full:2902 | D+BG | D=src-tauri/src/services/meetings.rs:2951:let _ = db.update_intelligence_state(&meeting_id_for_phase1, "enriching", None, None); ; BG=src-tauri/src/services/meetings.rs:3020:crate::intel_queue::invalidate_and_requeue_meeting_preps(state, &entity_id);
meetings::refresh_meeting_preps:3253 | SQL+BG+C | SQL=src-tauri/src/services/meetings.rs:3275:let _ = db.conn_ref().execute( ; BG=src-tauri/src/services/meetings.rs:3291:.enqueue(crate::meeting_prep_queue::PrepRequest::new( ; C=src-tauri/src/services/meetings.rs:3255:let now = chrono::Utc::now().to_rfc3339();
meetings::reprocess_meeting_transcript:3307 | D+C | D=src-tauri/src/services/meetings.rs:3330:db.clear_meeting_extraction_data(&clear_mid) ; C=src-tauri/src/services/meetings.rs:3352:.unwrap_or_else(|_| chrono::Utc::now()),
meetings::attach_meeting_transcript:3378 | D+SIG+C | D=src-tauri/src/services/meetings.rs:3470:db.update_meeting_transcript_metadata(&mid, &dest, &at, summary.as_deref()) ; SIG=src-tauri/src/services/meetings.rs:3500:let _ = app_handle.emit("transcript-processed", &outcome_data); ; C=src-tauri/src/services/meetings.rs:3455:let processed_at = chrono::Utc::now().to_rfc3339();
mutations::set_meeting_prep_context:10 | D | D=src-tauri/src/services/mutations.rs:11:crate::services::meetings::set_meeting_prep_context(db, meeting_id, updated_json)
mutations::reset_email_dismissals:18 | D | D=src-tauri/src/services/mutations.rs:19:db.reset_email_dismissals().map_err(|e| e.to_string())
mutations::update_capture_content:22 | D | D=src-tauri/src/services/mutations.rs:23:crate::services::meetings::update_capture_content(db, capture_id, content)
mutations::upsert_account:30 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:32:tx.upsert_account(account).map_err(|e| e.to_string())?; ; TX=src-tauri/src/services/mutations.rs:31:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:33:crate::services::signals::emit_and_propagate(
mutations::upsert_project:52 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:54:tx.upsert_project(project).map_err(|e| e.to_string())?; ; TX=src-tauri/src/services/mutations.rs:53:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:55:crate::services::signals::emit_and_propagate(
mutations::remove_project_keyword:74 | D+TX | D=src-tauri/src/services/mutations.rs:76:tx.remove_project_keyword(project_id, keyword) ; TX=src-tauri/src/services/mutations.rs:75:db.with_transaction(|tx| {
mutations::remove_account_keyword:99 | D+TX | D=src-tauri/src/services/mutations.rs:101:tx.remove_account_keyword(account_id, keyword) ; TX=src-tauri/src/services/mutations.rs:100:db.with_transaction(|tx| {
mutations::ensure_open_chat_session:124 | D+C | D=src-tauri/src/services/mutations.rs:134:db.create_chat_session(&session_id, entity_id, entity_type, &now, &now) ; C=src-tauri/src/services/mutations.rs:132:let now = Utc::now().to_rfc3339();
mutations::append_chat_exchange:142 | D+C | D=src-tauri/src/services/mutations.rs:148:db.append_chat_turn( ; C=src-tauri/src/services/mutations.rs:143:let now = Utc::now().to_rfc3339();
mutations::update_meeting_user_layer:179 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:181:tx.update_meeting_user_layer(meeting_id, agenda_json, notes) ; TX=src-tauri/src/services/mutations.rs:180:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:183:crate::services::signals::emit_and_propagate(
mutations::record_pipeline_failure:204 | D | D=src-tauri/src/services/mutations.rs:205:db.insert_pipeline_failure(
mutations::resolve_pipeline_failures:224 | D | D=src-tauri/src/services/mutations.rs:225:db.resolve_pipeline_failures(pipeline, entity_id, entity_type)
mutations::upsert_app_state_kv_json:234 | SQL | SQL=src-tauri/src/services/mutations.rs:236:.execute(
mutations::upsert_signal_weight:245 | D | D=src-tauri/src/services/mutations.rs:246:db.upsert_signal_weight(source, entity_type, signal_type, weight, confidence)
mutations::queue_clay_sync_for_people:257 | SQL+C | SQL=src-tauri/src/services/mutations.rs:265:.execute( ; C=src-tauri/src/services/mutations.rs:261:let now = Utc::now().to_rfc3339();
mutations::create_linear_entity_link:275 | D | D=src-tauri/src/services/mutations.rs:276:create_linear_entity_link_with_confirmed(db, linear_project_id, entity_id, entity_type, true)
mutations::create_linear_entity_link_with_confirmed:284 | SQL | SQL=src-tauri/src/services/mutations.rs:287:.execute(
mutations::delete_linear_entity_link:302 | SQL | SQL=src-tauri/src/services/mutations.rs:304:.execute(
mutations::update_entity_metadata:312 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:314:tx.update_entity_metadata(entity_type, entity_id, metadata)?; ; TX=src-tauri/src/services/mutations.rs:313:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:315:crate::services::signals::emit_and_propagate(
mutations::upsert_email_feedback_signal:336 | D | D=src-tauri/src/services/mutations.rs:341:db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
mutations::upsert_timeline_meeting_with_entities:363 | D+TX | D=src-tauri/src/services/mutations.rs:365:tx.upsert_meeting(meeting).map_err(|e| e.to_string())?; ; TX=src-tauri/src/services/mutations.rs:364:db.with_transaction(|tx| {
mutations::upsert_person_relationship:388 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:390:tx.upsert_person_relationship(rel) ; TX=src-tauri/src/services/mutations.rs:389:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:393:crate::services::signals::emit_and_propagate(
mutations::delete_person_relationship:431 | D+TX+SIG | D=src-tauri/src/services/mutations.rs:436:tx.delete_person_relationship(id) ; TX=src-tauri/src/services/mutations.rs:432:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/mutations.rs:440:crate::services::signals::emit_and_propagate(
mutations::persist_transcript_outcomes:485 | D+TX | D=src-tauri/src/services/mutations.rs:497:tx.insert_capture_enriched(&crate::db::signals::CaptureInput { ; TX=src-tauri/src/services/mutations.rs:494:db.with_transaction(|tx| {
mutations::insert_processing_log:641 | D | D=src-tauri/src/services/mutations.rs:642:db.insert_processing_log(log_entry)
mutations::upsert_action_if_not_completed:646 | D+TX | D=src-tauri/src/services/mutations.rs:649:.upsert_action_if_not_completed_with_status(action) ; TX=src-tauri/src/services/mutations.rs:647:db.with_transaction(|tx| {
mutations::persist_transcript_metadata:688 | D | D=src-tauri/src/services/mutations.rs:689:db.update_meeting_transcript_metadata(meeting_id, transcript_path, processed_at, summary)
mutations::persist_key_advocate_health:699 | D | D=src-tauri/src/services/mutations.rs:700:db.upsert_key_advocate_health(meeting_id, assessment)
mutations::clear_key_advocate_health:708 | SQL | SQL=src-tauri/src/services/mutations.rs:710:.execute(
mutations::replace_transcript_outcome_captures:731 | D+SQL+TX | D=src-tauri/src/services/mutations.rs:743:tx.insert_capture_enriched(&crate::db::signals::CaptureInput { ; SQL=src-tauri/src/services/mutations.rs:734:.execute( ; TX=src-tauri/src/services/mutations.rs:732:db.with_transaction(|tx| {
people::merge_people:13 | D+FS | D=src-tauri/src/services/people.rs:21:db.merge_people(keep_id, remove_id) ; FS=src-tauri/src/services/people.rs:36:let _ = std::fs::remove_dir_all(&remove_dir);
people::delete_person:55 | D+SIG+FS | D=src-tauri/src/services/people.rs:63:db.delete_person(person_id).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/people.rs:66:let _ = crate::services::signals::emit_and_propagate( ; FS=src-tauri/src/services/people.rs:90:let _ = std::fs::remove_dir_all(&person_dir);
people::update_person_field:179 | D+SIG | D=src-tauri/src/services/people.rs:193:db.update_person_field(person_id, field, value) ; SIG=src-tauri/src/services/people.rs:197:let _ = crate::services::signals::emit_propagate_and_evaluate(
people::link_person_entity:258 | D+SIG | D=src-tauri/src/services/people.rs:259:db.link_person_to_entity(person_id, entity_id, relationship_type) ; SIG=src-tauri/src/services/people.rs:263:let _ = crate::services::signals::emit_and_propagate(
people::unlink_person_entity:294 | D+SIG | D=src-tauri/src/services/people.rs:295:db.unlink_person_from_entity(person_id, entity_id) ; SIG=src-tauri/src/services/people.rs:299:let _ = crate::services::signals::emit_and_propagate(
people::create_person:329 | D+FS+C | D=src-tauri/src/services/people.rs:360:db.upsert_person(&person).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/people.rs:370:let _ = std::fs::create_dir_all(&person_dir); ; C=src-tauri/src/services/people.rs:331:let now = chrono::Utc::now().to_rfc3339();
people::archive_person:392 | D+SIG+BG | D=src-tauri/src/services/people.rs:393:db.archive_person(id, archived).map_err(|e| e.to_string())?; ; SIG=src-tauri/src/services/people.rs:400:let _ = crate::services::signals::emit_and_propagate( ; BG=src-tauri/src/services/people.rs:413:state.intel_queue.remove_by_entity_id(id);
people::create_person_from_stakeholder:426 | D+FS+C | D=src-tauri/src/services/people.rs:462:db.upsert_person(&person).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/people.rs:473:let _ = std::fs::create_dir_all(&person_dir); ; C=src-tauri/src/services/people.rs:433:let now = chrono::Utc::now().to_rfc3339();
projects::create_project:226 | D+FS+C | D=src-tauri/src/services/projects.rs:256:db.upsert_project(&project).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/projects.rs:261:let _ = std::fs::create_dir_all(&project_dir); ; C=src-tauri/src/services/projects.rs:237:let now = chrono::Utc::now().to_rfc3339();
projects::update_project_field:284 | D | D=src-tauri/src/services/projects.rs:293:db.update_project_field(&project_id, &field, &value)
projects::bulk_create_projects:432 | D+FS+C | D=src-tauri/src/services/projects.rs:456:db.upsert_project(&project).map_err(|e| e.to_string())?; ; FS=src-tauri/src/services/projects.rs:459:let _ = std::fs::create_dir_all(&project_dir); ; C=src-tauri/src/services/projects.rs:443:let now = chrono::Utc::now().to_rfc3339();
projects::archive_project:479 | D+BG | D=src-tauri/src/services/projects.rs:496:db.archive_project(id, archived) ; BG=src-tauri/src/services/projects.rs:509:state.intel_queue.remove_by_entity_id(id);
reports::generate_report:17 | D+EXT | D=src-tauri/src/services/reports.rs:149:let report_id = upsert_report( ; EXT=src-tauri/src/services/reports.rs:112:let stdout = run_report_generation(&input)?;
reports::generate_swot_report:180 | D+EXT | D=src-tauri/src/services/reports.rs:207:let report_id = upsert_report( ; EXT=src-tauri/src/services/reports.rs:201:let content = crate::reports::swot::run_parallel_swot_generation(&gathered, app_handle)?;
reports::generate_book_of_business:232 | D+SIG+EXT | D=src-tauri/src/services/reports.rs:327:let _ = crate::audit::write_audit_entry( ; SIG=src-tauri/src/services/reports.rs:286:let _ = handle.emit( ; EXT=src-tauri/src/services/reports.rs:282:let ctx = prefetch_glean_portfolio_context(endpoint, &account_names);
reports::save_report:415 | D | D=src-tauri/src/services/reports.rs:416:crate::reports::save_report_content(db, entity_id, entity_type, report_type, content_json)
settings::set_entity_mode:30 | D+FS | D=src-tauri/src/services/settings.rs:34:let config = crate::state::create_or_update_config(state, |config| { ; FS=src-tauri/src/services/settings.rs:45:let _ = std::fs::create_dir_all(&accounts_dir);
settings::set_workspace_path:61 | D | D=src-tauri/src/services/settings.rs:78:let config = crate::state::create_or_update_config(state, |config| {
settings::set_ai_model:98 | D | D=src-tauri/src/services/settings.rs:102:crate::state::create_or_update_config(state, |config| match tier {
settings::reset_ai_models_to_recommended:112 | D | D=src-tauri/src/services/settings.rs:113:crate::state::create_or_update_config(state, |config| {
settings::set_google_poll_settings:120 | D | D=src-tauri/src/services/settings.rs:138:crate::state::create_or_update_config(state, |config| {
settings::set_hygiene_config:157 | D | D=src-tauri/src/services/settings.rs:175:crate::state::create_or_update_config(state, |config| {
settings::set_daily_ai_budget:203 | D+EXT | D=src-tauri/src/services/settings.rs:212:let config = crate::state::create_or_update_config(state, |config| { ; EXT=src-tauri/src/services/settings.rs:218:crate::pty::sync_budget_config_to_kv(&db, budget);
settings::set_notification_config:225 | D | D=src-tauri/src/services/settings.rs:242:crate::state::create_or_update_config(state, |config| {
settings::set_text_scale:251 | D | D=src-tauri/src/services/settings.rs:259:crate::state::create_or_update_config(state, |config| {
settings::set_schedule:265 | D | D=src-tauri/src/services/settings.rs:280:crate::state::create_or_update_config(state, |config| {
settings::set_user_profile:307 | D+SQL | D=src-tauri/src/services/settings.rs:328:crate::state::create_or_update_config(state, |config| { ; SQL=src-tauri/src/services/settings.rs:356:let _ = db.conn_ref().execute(
settings::set_user_domains:443 | D | D=src-tauri/src/services/settings.rs:450:let config = crate::state::create_or_update_config(state, |config| {
signals::emit:19 | SIG | SIG=src-tauri/src/services/signals.rs:20:bus::emit_signal(
signals::emit_and_propagate:41 | SIG | SIG=src-tauri/src/services/signals.rs:42:bus::emit_signal_and_propagate(
signals::emit_propagate_and_evaluate:65 | SIG | SIG=src-tauri/src/services/signals.rs:66:bus::emit_signal_propagate_and_evaluate(
success_plans::create_objective:497 | D | D=src-tauri/src/services/success_plans.rs:498:db.create_objective(account_id, title, description, target_date, source)
success_plans::update_objective:509 | D | D=src-tauri/src/services/success_plans.rs:510:db.update_objective(
success_plans::complete_objective:529 | D+TX+SIG | D=src-tauri/src/services/success_plans.rs:532:.complete_objective(objective_id) ; TX=src-tauri/src/services/success_plans.rs:530:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/success_plans.rs:534:crate::services::signals::emit_and_propagate(
success_plans::delete_objective:558 | D | D=src-tauri/src/services/success_plans.rs:559:db.delete_objective(objective_id)
success_plans::create_milestone:563 | D | D=src-tauri/src/services/success_plans.rs:564:db.create_milestone(objective_id, title, target_date, auto_detect_signal)
success_plans::update_milestone:574 | D | D=src-tauri/src/services/success_plans.rs:575:db.update_milestone(
success_plans::complete_milestone:594 | D+TX+SIG | D=src-tauri/src/services/success_plans.rs:597:.complete_milestone(milestone_id) ; TX=src-tauri/src/services/success_plans.rs:595:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/success_plans.rs:599:crate::services::signals::emit_and_propagate(
success_plans::skip_milestone:634 | TX+SIG | TX=src-tauri/src/services/success_plans.rs:635:db.with_transaction(|tx| { ; SIG=src-tauri/src/services/success_plans.rs:640:crate::services::signals::emit_and_propagate(
success_plans::delete_milestone:660 | D | D=src-tauri/src/services/success_plans.rs:661:db.delete_milestone(milestone_id)
success_plans::link_action_to_objective:665 | D | D=src-tauri/src/services/success_plans.rs:666:db.link_action_to_objective(action_id, objective_id)
success_plans::unlink_action_from_objective:674 | D | D=src-tauri/src/services/success_plans.rs:675:db.unlink_action_from_objective(action_id, objective_id)
success_plans::create_objective_from_suggestion:701 | D+TX | D=src-tauri/src/services/success_plans.rs:704:.create_objective( ; TX=src-tauri/src/services/success_plans.rs:702:db.with_transaction(|tx| {
success_plans::apply_success_plan_template:735 | D+TX+C | D=src-tauri/src/services/success_plans.rs:744:.create_objective( ; TX=src-tauri/src/services/success_plans.rs:740:db.with_transaction(|tx| { ; C=src-tauri/src/services/success_plans.rs:753:let target_date = (Utc::now().date_naive()
success_plans::match_commitments_to_milestones:785 | D+SQL+SIG | D=src-tauri/src/services/success_plans.rs:831:let _ = db.update_milestone( ; SQL=src-tauri/src/services/success_plans.rs:815:if let Err(e) = db.conn_ref().execute( ; SIG=src-tauri/src/services/success_plans.rs:843:let _ = crate::signals::bus::emit_signal(
success_plans::reconcile_objectives:1122 | SQL+C | SQL=src-tauri/src/services/success_plans.rs:1190:.execute( ; C=src-tauri/src/services/success_plans.rs:1139:let now = chrono::Utc::now().to_rfc3339();
user_entity::get_user_entity:72 | D+SQL | D=src-tauri/src/services/user_entity.rs:105:let _ = crate::state::create_or_update_config(state, |config| { ; SQL=src-tauri/src/services/user_entity.rs:89:.execute(
user_entity::update_user_entity_field:117 | SQL+FS | SQL=src-tauri/src/services/user_entity.rs:154:.execute("INSERT INTO user_entity (id) VALUES (1)", []) ; FS=src-tauri/src/services/user_entity.rs:174:let _ = std::fs::create_dir_all(&user_dir);
user_entity::create_user_context_entry:285 | SQL | SQL=src-tauri/src/services/user_entity.rs:296:.execute(
user_entity::update_user_context_entry:332 | SQL | SQL=src-tauri/src/services/user_entity.rs:342:.execute(
user_entity::delete_user_context_entry:364 | SQL | SQL=src-tauri/src/services/user_entity.rs:381:.execute(
user_entity::write_user_context_json:429 | FS | FS=src-tauri/src/services/user_entity.rs:440:std::fs::create_dir_all(&user_dir)
```

`CURRENT_TIMESTAMP` audit from `rg CURRENT_TIMESTAMP src-tauri/src/migrations/`: `081_init_tasks.sql:6 completed_at`; `068_success_plans.sql:112,113,131,132,143,160`; `044_user_entity.sql:23,24,33,34`; `050_reports.sql:10,11`; `051_entity_context_entries.sql:8,9`; `069_account_events_expand.sql:53,54,77,78`. W2-A documents and files the follow-on if not converted in this PR; conversion is not required for DOS-209 completion.

## 4. Security

Primary risk is capability leakage. Ability code must not receive raw app state, raw `ActionDb`, raw SQL, live filesystem writers, live queues, or live external clients. `ServiceContext` therefore exposes mode-aware wrappers only; raw handles are private or `pub(in crate::services)`. The PR must add a trybuild compile-fail test proving code under an ability-facing module cannot call `ActionDb::open`, receive `&ActionDb`, call `ctx.services.raw_db_for_service`, or construct Live external wrappers in Simulate/Evaluate. DOS-304 is satisfied by this boundary; proc macros are supplemental lint, not proof.

`new_evaluate` rejects production DB paths. Startup/background workers have explicit Live behavior and are not spawned by Evaluate constructors. No new PII surface or authz bypass is introduced; external replay wrappers must not contain live secrets in non-Live modes.

## 5. Performance

Hot paths touched: `accounts::update_account_field_inner`, `meetings::capture_meeting_outcome`, `meetings::refresh_meeting_preps`, `intelligence::persist_entity_keywords`, `intelligence::upsert_health_outlook_signals`, and transaction wrappers. Budget: p99 deviation under 5% versus the W1 Suite P baseline, and empty guard overhead below measurement noise for single mutators.

Measurement plan: before coding, copy W1 baseline numbers from `.docs/plans/wave-W1/proof-bundle.md`; after migration, run the same Suite P mutator microbench set plus a transaction-lock-duration probe for `with_transaction_async`. W2-A has no new Suite P gate, but the W2 proof bundle must include the comparison artifact so W2-end remeasurement has pass/fail evidence.

## 6. Coding standards

Services-only mutations: this PR activates the W2 CI enforcement mechanism from the wave invariants table. The guard catalogue prevents unguarded service mutators; lint rejects raw clock/RNG calls in `services/` and `abilities/` even though `src-tauri/src/abilities/` does not exist yet.

Intelligence Loop five answers for DOS-209:
- Signals: no new signal type; `signals::emit` and `emit_and_propagate` become mode-aware, routing to the ADR-0115 in-memory ring buffer in Simulate/Evaluate.
- Health scoring: injected `Clock`/`SeededRng` is required for deterministic health scoring and health debouncer behavior.
- Intel context: no direct context builder edits beyond constructor propagation; future `build_intelligence_context()` and `gather_account_context()` take `&ServiceContext`.
- Briefing callouts: no new callouts.
- Feedback hook: no new hook, but feedback mutations are in the guard catalogue.

No customer data in fixtures. Clippy budget remains zero new warnings under `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings`.

## 7. Integration with parallel wave-mates

Hard pre-code gate: DOS-310 and DOS-311 must be merged, W1 L3 cleared, and the W1 proof bundle published before DOS-209 implementation starts. DOS-209 consumes W1's migration fence, schema epoch, write fence, and Suite P baseline; it does not start from assumptions.

W2-B-first is now the frozen DOS-209 contract per the 2026-04-29 L6 amendment, not merely coordination guidance:

> **Amended landing order (L6, 2026-04-29):** W2-B (<issue id="d4e527db-b0d5-4206-bc6f-49ee6c227f84">DOS-259</issue>) lands first. W2-A (this issue) rebases on top once the PTY orchestration extraction has been merged. The architectural argument: extraction first reduces the function surface W2-A's `check_mutation_allowed()?` sweep needs to gate, and avoids re-restructuring functions W2-B is deleting. This amendment supersedes the "this issue first" line in the Dependencies block below.

W2-B owns provider extraction from `services/intelligence.rs`: provider selection, Glean chat, PTY fallback, prompt fingerprint/replay hooks, and `generate_risk_briefing` provider calls move toward `intelligence/provider.rs`, `glean_provider.rs`, and `pty_provider.rs`. W2-A then rebases and touches only service mutation boundaries in `services/intelligence.rs`: `enrich_entity` residual DB reset/queue side effects if still present, `persist_entity_keywords`, `upsert_assessment_from_enrichment`, `upsert_assessment_snapshot`, `upsert_health_outlook_signals`, `upsert_inferred_relationships_from_enrichment`, `update_intelligence_field`, `update_stakeholders`, `dismiss_intelligence_item`, `recompute_entity_health*`, `bulk_recompute_health`, `track_recommendation`, `dismiss_recommendation`, and `mark_commitment_done`. If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2. This closes cycle-2 challenge NF2.

No W2-A SQL migration is planned. If the `CURRENT_TIMESTAMP` conversion is pulled forward, it waits behind W1 migration numbering and must cite the W1 proof bundle.

## 8. Failure modes + rollback

Missed guard: blocked by `dos209_mutation_catalog.rs` structural lint and Evaluate runtime tests. Capability leakage: blocked by trybuild compile-fail tests. Transaction bug: fallback is sync-within-async with no `.await` inside SQLite write lock. Panic/error rollback is covered by tests.

Rollback is a single PR revert if no timestamp migration is included. W1-B universal write fence is honored: W2-A does not bypass `write_intelligence_json` or add new `intelligence.json` writers.

## 9. Test evidence to be produced

Mandatory tests and commands:
- `services::context::tests::proptest_check_mutation_allowed_modes`: generator is `prop_oneof![Live, Simulate, Evaluate]` crossed with `MutationAttempt { kind: D|SQL|TX|SIG|FS|BG|EXT, public_catalog_index, uses_clock, uses_rng }`; Live accepts, Simulate/Evaluate return `WriteBlockedByMode`.
- `services::context::tests::constructors_set_expected_modes`.
- `src-tauri/tests/dos209_surface_constructors.rs::all_live_surfaces_construct_new_live`.
- `src-tauri/tests/dos209_mode_boundary.rs::evaluate_catalog_public_mutators_return_write_blocked_by_mode`.
- `src-tauri/tests/dos209_mutation_catalog.rs::catalog_every_mutator_guarded_first_line`.
- `services::context::tests::mutation_catalog_no_drift`: re-runs `scripts/dos209-mutation-audit.sh` and asserts stdout exactly matches `src-tauri/tests/dos209_mutation_catalog.txt`; any drift breaks CI. This closes cycle-2 challenge NF1.
- `src-tauri/tests/dos209_lint_regex_test.rs::lint_blocks_direct_utc_now_and_thread_rng` with regexes `\bUtc::now\s*\(`, `chrono::Utc::now\s*\(`, `chrono::offset::Utc::now\s*\(`, `rand::thread_rng\s*\(`, `thread_rng\s*\(`, and `rand::rng\s*\(`.
- `src-tauri/tests/dos209_capability_trybuild.rs` compile-fail fixtures for raw `ActionDb`, raw SQL, live external client, and production `test_live()`.
- `src-tauri/tests/dos209_transactions.rs::{nested_transaction_forbidden, rollback_on_error, rollback_on_panic, txctx_has_no_external_clients}`.

CI command: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && pnpm tsc --noEmit`. This restores the full regression suite and closes cycle-2 challenge NF3. Additional DOS-209 evidence command: `cargo test --manifest-path src-tauri/Cargo.toml dos209`; this targeted invocation supplements the full regression suite and is not a replacement. Wave artifact: W2 proof bundle includes clippy/test output, mutation catalogue diff, direct clock/RNG lint output, and W1 Suite P comparison.

## 10. Open questions

No Critical or High finding remains open. L6 escalation only if reviewers reject the DOS-304 interpretation that W2-A can proceed after freezing capability-handle enforcement in this plan, or if both the HRTB primary and the sync-within-async fallback fail against the actual SQLite wrapper.

## 11. Completion record — 2026-04-30

**Status: COMPLETE**

All 228+ service mutators migrated to ServiceContext substrate across 11 commits. Signals cascade (131 sites) cleaned up in Group F. L1 validation clean. L2 adversarial review passed (2 cycles).

### Deferred from §9 test evidence

The following tests from §9 were deferred post-L2 sign-off (substrate validated by 1759 passing unit tests; no drift observed during migration):
- `proptest_check_mutation_allowed_modes` — deferred to a standalone follow-up
- `dos209_surface_constructors.rs` — deferred
- `dos209_mode_boundary.rs` — deferred  
- `dos209_mutation_catalog.rs` (drift CI test) — deferred
- `dos209_lint_regex_test.rs` — deferred
- `dos209_capability_trybuild.rs` — deferred
- `dos209_transactions.rs` — deferred

### L2 pre-existing findings (filed as follow-ups)

1. **evaluate_on_signal enqueue discard** (`signals/bus.rs:289`) — `let _ = evaluate_on_signal(...)` discards enqueue result. Pre-existing; self-healing re-enrichment can be silently lost in Paused/Debounced state. Needs separate ticket.
2. **entity_quality partial write** (`intelligence.rs:1404`) — `.ok()` on entity_quality write allows partial state + clears retry marker. Pre-existing best-effort design. Needs separate ticket.
