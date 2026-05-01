# Implementation Plan: DOS-7

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-7 keeps `intelligence_claims` as a durable, append-only commit substrate; it does not derive current truth on demand from Provenance envelopes. The design-review answer is: Provenance envelopes remain the evidence payload, while `intelligence_claims` is the queryable commit log, tombstone gate, trust/feedback target, and per-claim identity layer. Load-bearing ticket lines: "The `intelligence_claims` table is load-bearing for the entire v1.4.0 substrate." "Atomic backfill consolidating **all 9 dismissal/tombstone mechanisms** into `intelligence_claims` rows with `claim_state='tombstoned'`." "No worker drains = no migration. No reconcile pass = no migration complete."

The 2026-04-24 PM rewrite applies in full: schema for `intelligence_claims`, `claim_corroborations`, `claim_contradictions`, `agent_trust_ledger`, `claim_feedback`, and `claim_repair_job`; all 9 tombstone mechanisms; hard-delete role refactor; read-only enforcement on legacy dismissal stores; per-entity invalidation instead of `entity_graph_version`; DOS-311 schema-epoch fence. Memory-substrate amendments A-F all apply: corroboration strength, surfacing lifecycle, contradiction branches, immutability allowlist, terminology, and closed duplicate scope.

The 2026-04-26 cycle-2 amendment also applies: "This issue's scope expands" to absorb DOS-308 implementation work: `SuppressionDecision`, shared canonicalization, writer-side `item_hash` population, top-N suppression candidate resolution, and the claims-side covering lookup index. DOS-308 remains a hard precondition for the design contract, audit script, quarantine table, remediation script, and zero-quarantine migration gate.

Hard PR-open preconditions: DOS-308 design/audit/quarantine, DOS-309, DOS-310, and DOS-311 merged. The prompt says DOS-309/DOS-310/DOS-311 are already merged; DOS-308 must still provide the quarantine gate before this branch opens.

## 2. Approach

Create `src-tauri/src/services/claims.rs` and register it in `src-tauri/src/services/mod.rs:5-28`. This module owns `commit_claim`, `record_corroboration`, `reconcile_contradiction`, `load_claims_active`, `load_claims_including_dormant`, `load_claims_dormant_only`, and the suppression reader. It must be the only production writer for claim rows, corroboration strength bumps, contradiction reconciliation, feedback rows, and tombstone-equivalent operations.

Add a DOS-7 cutover migration after the current tail `src-tauri/src/migrations.rs:588-590` (last registered version is 125). Tentative slot: `src-tauri/src/migrations/125_dos_7_claims_consolidation.sql` registered as version 126, unless W3 migration sequencing reserves intervening slots first. The current runner only does `conn.execute_batch(migration.sql)` in order (`src-tauri/src/migrations.rs:1028-1085`), so DOS-7 also needs a Rust cutover hook around this migration: pre-flight log -> `write_fence::bump_schema_epoch` (`src-tauri/src/intelligence/write_fence.rs:136-155`) -> pause/drain queue -> run schema/backfill -> requeue -> run reconcile -> resume.

Schema in the migration: `intelligence_claims` with immutable assertion columns (`id`, `subject_ref`, `claim_type`, `field_path` or `topic_key`, `text`, `dedup_key`, `item_hash`, `actor`, `data_source`, `source_ref`, `source_asof`, `observed_at`, `created_at`, `provenance_json`, `metadata_json`), lifecycle columns (`claim_state`, `surfacing_state`, `demotion_reason`, `reactivated_at`, `retraction_reason`, `expires_at`, `superseded_by`, `trust_score`, `trust_computed_at`, `trust_version`, `thread_id`, `temporal_scope`, `sensitivity`), and indexes for default reads plus suppression lookup. Include `thread_id` from ADR-0124 (`.docs/decisions/0124-longitudinal-topic-threading.md:33-48`), `source_asof` from ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:391-466`), and `temporal_scope` / `sensitivity` from ADR-0125 (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:51-87`) in the initial create-table if W3-C lands first, to avoid later ALTER churn.

Child tables: `claim_corroborations` with amendment A fields (`strength`, `reinforcement_count`, `last_reinforced_at`, `data_source`, `source_asof`, `source_mechanism`); `claim_contradictions` with amendment C branch/reconciliation fields; `agent_trust_ledger`; `claim_feedback` with ADR-0123 row shape (`.docs/decisions/0123-typed-claim-feedback-semantics.md:91-106`); and `claim_repair_job` skeleton for `CannotVerify` budgets (`.docs/decisions/0123-typed-claim-feedback-semantics.md:112-118`, `:159-165`).

`commit_claim` algorithm: `ctx.check_mutation_allowed()`; validate `subject_ref` JSON and claim type against the ADR-0125 registry; canonicalize text/key via trim -> NFC -> collapse whitespace -> SHA-256; acquire the ADR-0113 R2 app-level lock for `(subject_ref, claim_type, topic_key/field_path)` (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:396-430`); run tombstone PRE-GATE before dedup (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:114-122`); merge same-meaning claims into `claim_corroborations`; detect contradictions only after canonicalization; write trust/invalidation; call `db::claim_invalidation::bump_for_subject` (`src-tauri/src/db/claim_invalidation.rs:43-45`, `:180-228`) in the same transaction.

Backfill all 9 mechanisms into tombstoned claims with `source_mechanism` preserved: `suppression_tombstones` (`src-tauri/src/migrations/084_feedback_events.sql:18-29`), `account_stakeholder_roles.dismissed_at` (`src-tauri/src/migrations/107_stakeholder_role_dismissals.sql:1-18`), `email_dismissals` (`src-tauri/src/migrations/030_email_dismissals.sql:7-24`), `meeting_entity_dismissals` (`src-tauri/src/migrations/099_meeting_entity_dismissals.sql:18-28`), `linking_dismissals` (`src-tauri/src/migrations/111_linking_dismissals.sql:17-29`), `briefing_callouts.dismissed_at` (`src-tauri/src/migrations/020_signal_propagation.sql:27-41`), ticket-named `work_tab_actions.dismissed_at` (live migration 108 currently exposes `nudge_dismissals` at `src-tauri/src/migrations/108_work_tab_actions.sql:51-58`; see section 10), `triage_snoozes` (`src-tauri/src/migrations/109_triage_snoozes.sql:16-28`), and `DismissedItem` from `IntelligenceJson` (`src-tauri/src/intelligence/io.rs:65-76`). Source-time backfill must lift `ItemSource.sourced_at` where available (`src-tauri/src/intelligence/io.rs:24-41`).

Refactor current writers and readers: replace `ActionDb::is_suppressed`'s bool/string-time implementation (`src-tauri/src/db/intelligence_feedback.rs:70-99`) with the DOS-308 enum contract; change `intel_queue.rs:2375-2403` from `.unwrap_or(false)` to explicit fail-closed `Malformed`; move `create_suppression_tombstone` callers from `db/feedback.rs:99-114`, `services/accounts.rs:1234-1242`, and `services/intelligence.rs:1264-1288` through `commit_claim`; stop writing `DismissedItem` directly in `services/intelligence.rs:1214-1221` except via DOS-301 projection.

Refactor hard-delete CHAIN paths. `db/accounts.rs:set_team_member_role` currently deletes user roles at `src-tauri/src/db/accounts.rs:769-773`; `remove_account_team_member` deletes roles and stakeholder link at `:833-842`; service wrappers call both inside transactions at `src-tauri/src/services/accounts.rs:2545-2605`. These become tombstone claim writes plus row preservation. Existing service soft-delete precedent at `src-tauri/src/services/accounts.rs:3355-3369` informs semantics, but claim substrate becomes canonical.

## 3. Key decisions

Keep the table. Deriving from Provenance envelopes fails the product contract: it cannot enforce a synchronous tombstone PRE-GATE, cannot answer "all claims about subject X" without replaying historical ability outputs, cannot carry user feedback and repair jobs as first-class targets, and cannot preserve 9 legacy dismissal sources under one immutable identity. Provenance becomes `provenance_json` plus source/corroboration rows, not the storage root.

Use the latest ticket vocabulary, not older ADR state names: `claim_state = active | dormant | tombstoned | withdrawn`, with `surfacing_state = active | dormant`. ADR-0113's older `committed/superseded/retracted` text is superseded by the DOS-7 rewrite and amendment B; supersession remains represented by `superseded_by` and history reads.

Use `field_path` for compatibility with ADR-0113 and reconcile SQL (`scripts/reconcile_ghost_resurrection.sql:42-85`), plus `topic_key` if DOS-280 canonicalization needs it. Do not put `thread_id` in `dedup_key`; ADR-0124 explicitly forbids that (`.docs/decisions/0124-longitudinal-topic-threading.md:44-48`).

Use TEXT UUID claim IDs unless reviewers force the BLOB snippet in amendment C. This matches the existing SQLite/Rust codebase's TEXT id pattern and ADR-0113 examples, but the ticket's contradiction snippet says `winner_claim_id BLOB`; see section 10.

App-level per-key mutex is the default commit lock per ADR-0113 R2 (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:420-426`). A DB lock table is overkill for this single-user app unless multi-process claim writers appear.

Legacy tables/columns stay for one release as read-only projection/parity surfaces. Direct writes are blocked by CI lints; DOS-301's `services/derived_state.rs` is the only allowed projection writer.

## 4. Security

The new cross-tenant risk is subject bleed: a claim or tombstone attached to the wrong `subject_ref` can suppress or resurrect data on another entity. `commit_claim` must validate `subject_ref`, `claim_type` allowed subject types, and correction scopes before insert. ADR-0123 `WrongSubject` must tombstone only the asserted subject and propose a corrected-subject claim through the same gate (`.docs/decisions/0123-typed-claim-feedback-semantics.md:45-54`, `:146-153`).

The migration processes customer intelligence and legacy JSON blobs. It must never log claim text, raw source excerpts, customer names, or PII; audit counts use mechanism, entity type, row id, timestamp class, and error code. Reconcile output currently logs subject/dedup/item_hash (`src-tauri/src/bin/reconcile_post_migration.rs:119-131`); DOS-7 repair mode should keep that non-content posture.

Read-path filtering must default to `claim_state='active' AND surfacing_state='active'`, and sensitivity defaults to `internal` per ADR-0125 (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:79-87`). Surface-specific sensitivity enforcement is later scope, but storing the column now prevents public-default leakage.

The quarantine gate is security-critical: DOS-7's first cutover step aborts if `suppression_tombstones_quarantine` is non-empty. Malformed suppression records fail closed for runtime reads and fail the migration until quarantined/remediated.

## 5. Performance

Hot paths: enrichment suppression filtering (`src-tauri/src/intel_queue.rs:2375-2403`), claim default reads, `commit_claim`, corroboration writes, and the one-time 9-mechanism backfill. The covering suppression index should support `(subject_ref, claim_type, field_path, claim_state, dedup_key, item_hash, expires_at, source_asof)` or equivalent; default reads need `(subject_ref, claim_state, surfacing_state, claim_type)`.

`record_corroboration` uses one indexed lookup and either insert or same-row strength update. The aggregate `1 - product(1 - strength)` can be computed from child rows for Trust Compiler; cache only if profiling shows repeated aggregation dominates.

Per-entity invalidation adds one synchronous counter bump inside the commit transaction through `db::claim_invalidation`, which W1 documented as the replacement for `entity_graph_version` thrash (`src-tauri/src/db/claim_invalidation.rs:1-32`). No claim write touches `entity_graph_version`.

The backfill should run in chunks inside one `BEGIN IMMEDIATE` cutover window where possible, but memory use must stream JSON rows rather than materializing all `IntelligenceJson` blobs. Drain timeout comes from DOS-311 primitives (`src-tauri/src/intelligence/write_fence.rs:112-121`); migration aborts on timeout rather than proceeding with live writers.

## 6. Coding standards

Services-only mutations: command handlers keep calling services; direct DB write helpers for legacy tombstones become private/read-only or lint-forbidden. `services/claims.rs` is the write path; `services/derived_state.rs` is the projection writer exception.

CLAUDE.md's Intelligence Loop 5-question check applies to each new table/column (`CLAUDE.md:7-14`): claim writes emit invalidation/signals where policy requires; health consumers can discover claim-version bumps; prompt/prep context reads route through default active loaders; briefing callouts consume projections, not raw legacy blobs; typed feedback updates Bayesian source/agent weights.

No direct `Utc::now()` or `thread_rng()` in `services/claims.rs`; time comes from `ServiceContext.clock`. Existing direct `datetime('now')` in legacy SQL (`src-tauri/src/db/accounts.rs:769-773`, `src-tauri/src/services/accounts.rs:3364-3366`) should not be copied into claim writes. Fixtures use generic ids and domains only.

Add CI lints: no `DELETE FROM intelligence_claims`; no direct `INSERT/UPDATE/DELETE` against legacy dismissal stores outside `services/derived_state.rs`; no forbidden `UPDATE` columns on `intelligence_claims`; no direct `UPDATE claim_corroborations` or `claim_contradictions` outside services; no `DataSource::LegacyUnattributed` outside migration/backfill paths per ADR-0107 amendment.

## 7. Integration with parallel wave-mates

W3-D / DOS-301 is the closest collaborator. W3-C ships schema and `commit_claim` first; W3-D owns `services/derived_state.rs`, `claim_projection_status`, file projection, and shared JSON validators. If `src-tauri/src/validators/json_columns.rs` is introduced by W3-D, W3-C calls into it and does not fork validation logic. W3-C must not write legacy AI columns except via projection contracts.

W3-E / DOS-294 consumes the `claim_feedback` skeleton table and typed feedback enum. W3-C creates the table shape; W3-E owns UI/service semantics beyond the skeleton.

W3-F / DOS-296 coordinates `thread_id`: if W3-C lands first, `thread_id` is in the base table; W3-F must not add a duplicate ALTER and instead owns assignment/provenance wiring. W3-G / DOS-299 consumes W3-C's `source_asof` column and owns source timestamp semantics. W3-H / DOS-300 coordinates `temporal_scope`, `sensitivity`, and the claim type registry; if W3-C includes the columns, W3-H owns registry enforcement and metadata defaults.

W3-B / DOS-211 Provenance Builder feeds `source_asof`, `SubjectRef`, and Provenance warnings. W4 / DOS-5 Trust Compiler consumes `claim_corroborations.strength`, `claim_feedback`, `agent_trust_ledger`, and trust columns; W3-C does not implement Trust Compiler.

Linear currently lists DOS-302 and DOS-294 as blockers of DOS-7, while the prompt's hard-dependency list says DOS-308/309/310/311. Treat DOS-302/DOS-294 as coordination/blocker ambiguity until clarified in section 10.

## 8. Failure modes + rollback

If schema creation succeeds but backfill fails, no migration version should be recorded. Current migration runner records only after successful `execute_batch` (`src-tauri/src/migrations.rs:1078-1085`); the DOS-7 Rust cutover hook must preserve that atomicity and avoid a half-recorded version.

If drain times out, abort before backfill. If reconcile finds ghost resurrection rows, migration does not complete; `scripts/reconcile_ghost_resurrection.sql:14-16` defines zero findings as clean. `reconcile_post_migration --repair` must switch from the current no-op skeleton (`src-tauri/src/bin/reconcile_post_migration.rs:135-147`) to re-tombstoning through `commit_claim`.

Rollback is restore-from-pre-migration backup for failed production cutover; current migration infrastructure already creates a safety backup before pending migrations (`src-tauri/src/migrations.rs:1019-1026`). After successful cutover, rollback should leave legacy tables read-only and revert readers to legacy paths only if the reconcile report is clean enough to avoid ghost resurrection.

W1-B universal write fence honored: intel workers capture/recheck `schema_epoch` via `FenceCycle` and `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:67-109`, `:223-238`). DOS-7 bumps epoch, drains, and rejects stale file writes rather than letting legacy `intelligence.json` overwrite claim-backed state.

## 9. Test evidence to be produced

Unit/integration tests: `commit_claim_rejects_tombstoned_subject_pre_gate`, `commit_claim_same_actor_supersedes_different_actor_contradicts`, `commit_claim_concurrent_same_field_path_routes_deterministically`, `record_corroboration_same_source_strength_saturates`, `record_corroboration_noisy_or_aggregate_prefers_source_diversity`, `reconcile_contradiction_preserves_both_branches`, `is_suppressed_returns_malformed_fail_closed_contract`, `is_suppressed_claim_lookup_hash_beats_exact_key_beats_keyless`, `writer_side_item_hash_populated_for_dismiss_paths`.

Backfill tests: one named test per mechanism, plus duplicate-pair tests for `suppression_tombstones` latest-wins with prior rows as corroborations and `meeting_entity_dismissals` plus `linking_dismissals` preserving both `source_mechanism` values. Include `triage_snooze_expires_to_withdrawn`, `dismissed_item_backfill_uses_item_source_sourced_at`, and `suppression_quarantine_nonempty_aborts_migration`.

Lint tests/scripts: `check_claim_writer_allowlist`, `check_legacy_dismissal_writes_read_only`, `check_intelligence_claims_no_delete`, `check_claim_immutability_allowlist`, `check_claim_type_registry_covers_written_types`, and the existing `scripts/check_write_fence_usage.sh`.

Wave merge-gate artifact: attach DOS-7 migration audit counts, DOS-308 quarantine count = 0, reconcile report = 0 ghost-resurrection findings, and commands `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` per `CLAUDE.md:20-24`. Suite S: tombstone/security/PII/lint evidence. Suite P: indexed lookup/backfill/drain timing budget. Suite E: 5-run ghost-resurrection simulation and default-read parity.

## 10. Open questions

1. Claim id storage conflict: ADR examples and project patterns point to TEXT UUIDs; DOS-7 amendment C snippets use `BLOB` for `winner_claim_id` / `merged_claim_id`. Confirm TEXT vs BLOB before coding.
2. Corroboration formula conflict: amendment A's formula appears to saturate from 0.5 to 1.0 on the first same-source reinforcement, while its prose expects about 0.7. Confirm formula or expected tests.
3. Migration hook placement: should DOS-7 extend `migrations.rs` with a version-specific Rust hook, or add a separate startup cutover command that records schema version after SQL/backfill/reconcile?
4. Linear blocker ambiguity: should DOS-302 and DOS-294 truly block DOS-7 PR open, or are they stale relations now that W3-E consumes `claim_feedback` after W3-C?
5. `field_path` vs `topic_key`: use both, or normalize `topic_key` as a registry-derived alias of `field_path`? DOS-280 wording uses `topic_key`; ADR-0113/reconcile use `field_path`.
6. `work_tab_actions.dismissed_at` source mismatch: DOS-7 names this as mechanism 7, but current migration 108 shows `nudge_dismissals`, not a `work_tab_actions` table. Confirm the live source table/column before coding the backfill.
