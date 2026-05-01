# Implementation Plan: DOS-5

## Revision history
- v1 (2026-05-01) — initial L0 draft.

## 1. Contract restated
DOS-5 ships the numeric Trust Compiler for claim rows, and this W4-A slot folds DOS-326 into it by retiring the standalone contamination guard. Load-bearing DOS-5 lines: "`trust_score = exp(Σ w_i × log(factor_i))`"; "`corr` reads from `claim_corroborations.strength`"; "must be clock-injectable"; "No `Utc::now()` in compiler code"; "Writes `trust_score`, `trust_computed_at`, `trust_version` on `intelligence_claims`"; "trust recompute is still a pure-compute module."

Load-bearing DOS-326 lines: "becomes one factor in the Trust Compiler, not a separate gate"; "The standalone `intelligence/contamination.rs` module + `DAILYOS_CONTAMINATION_VALIDATION` env var + the `intel_queue.rs:2182` guard are deleted"; "Prompt-builder API includes a `cross_entity_context_expected: bool` flag"; "The three heuristics (foreign domain match, foreign WP VIP host, foreign company name) are reimplemented as inputs to the coherence factor — same detection logic, different output (score, not boolean)."

The 2026-04-24 amendments apply as follows: ADR-0114 integration applies for five canonical factors and deterministic inputs; the production-readiness addendum applies for separating source reliability, freshness, corroboration independence, contradiction visibility, and user correction behavior; the subject-correctness amendment applies as `subject_fit_confidence` composer-local helper, not a sixth canonical factor; the typed-feedback amendment applies through ADR-0123's `ClaimFeedback`; the 2026-04-24 PM corr amendment applies through W3-C's `claim_corroborations.strength`. DOS-326 has no 2026-04-24 comment; it was created on 2026-04-25 and is folded in whole.

## 2. Approach
Create `src-tauri/src/abilities/trust/` after W3-A creates the abilities root. Current code has no `src-tauri/src/abilities/` or `src-tauri/src/scoring/`; `src-tauri/src/lib.rs:43-67` exports `intelligence`, `services`, and `signals`, but no abilities/scoring root. W4-A-owned files: `trust/mod.rs`, `trust/types.rs`, `trust/config.rs`, and `trust/tests.rs` or equivalent under the directory; module-root export is a W3-A coordination line.

Core API:

```rust
pub fn compile_trust(claims: &[ClaimRow], context: TrustContext) -> TrustScore;
```

`ClaimRow` is the W3-C service DTO, not a new table. `TrustContext` contains injected `now`, validated scoring config, factor inputs already extracted from DB, and `CrossEntityCoherenceInput`; no `ActionDb`, `EntityId` lookup, global clock, env read, or signal emission crosses into the pure compiler.

Algorithm: compute five ADR-0114 R1.4 canonical factor values (`source_reliability`, `freshness_weight`, `corroboration_weight`, `contradiction_penalty`, `user_feedback_weight`), plus composer-local `subject_fit_confidence` and `cross_entity_coherence`. Clamp every factor to `[0.05, 1.0]`, aggregate by weighted geometric mean, clamp final score to `[0, 1]`, then map to `TrustBand::{LikelyCurrent, UseWithCaution, NeedsVerification, Unscored}`. ADR-0114 pins the geometric mean at `.docs/decisions/0114-scoring-unification.md:120-126` and canonical factors at `:290-294`; current scattered primitive code is in `signals/bus.rs:60-100`, `signals/decay.rs:8-29`, and `signals/fusion.rs:31-50`.

Freshness uses W3-G's `source_asof` contract. `ItemSource` already carries `confidence` and `sourced_at` at `src-tauri/src/intelligence/io.rs:29-37`; `effective_confidence()` defaults missing source confidence to `0.5` at `src-tauri/src/intelligence/io.rs:1227-1230`. Trust input prep uses W3-G's fallback chain from ADR-0105 `.docs/decisions/0105-provenance-as-first-class-output.md:424-433`, with `FreshnessContext { timestamp_known: bool, ... }`. `unknown_timestamp_penalty` is applied inside `freshness_weight`, not modeled as a factor.

Corroboration reads W3-C's `claim_corroborations.strength` child rows, not a raw count. W3-C's plan pins `claim_corroborations` fields at `.docs/plans/wave-W3/DOS-7-plan.md:23-25`, says W4 consumes `claim_corroborations.strength`, `claim_feedback`, `agent_trust_ledger`, and trust columns at `:87`, and explicitly does not implement Trust Compiler. The noisy-OR formula remains blocked by DOS-7 open question `.docs/plans/wave-W3/DOS-7-plan.md:113-115`; W4-A tests must be co-authored with the W3-C author once that formula is resolved.

Cross-entity coherence replaces `src-tauri/src/intelligence/contamination.rs`, whose current detector runs three heuristics at `src-tauri/src/intelligence/contamination.rs:68-185`, collects narrative text at `:188-253`, reads `DAILYOS_CONTAMINATION_VALIDATION` at `:256-305`, and uses subdomain ownership logic at `:307-319`. The W4 shape is:

```rust
pub struct CrossEntityCoherenceInput {
    pub claim_text: String,
    pub target_footprint: TargetFootprint,
    pub portfolio_footprints: Vec<EntityFootprint>,
    pub cross_entity_context_expected: bool,
}

pub struct TargetFootprint {
    pub subject: SubjectRef,
    pub names: Vec<String>,
    pub domains: Vec<String>,
    pub related_subjects: Vec<SubjectRef>,
    pub allowed_aliases: Vec<String>,
}

pub struct CrossEntityHit {
    pub token: String,
    pub kind: CrossEntityHitKind, // Domain | InfrastructureId | CompanyName
    pub source_subject: Option<SubjectRef>,
}
```

The extractor builds the expanded footprint from account domains, target subdomains, parent/child accounts, aliases/DBA names where available, and portfolio-domain relationships; the pure factor receives only these values. If `cross_entity_context_expected` is true, coherence returns 1.0 plus evidence that the skip was intentional. Otherwise it runs the same three heuristics and converts hits to a factor score, never a persistence veto.

Retire call sites: remove `pub mod contamination` from `src-tauri/src/intelligence/mod.rs:1-4`; remove the pre-persistence guard from `src-tauri/src/intel_queue.rs:2460-2554`; remove/rebase devtools contamination structs and command internals in `src-tauri/src/commands/app_support.rs:1478-1648`; remove command registration for audit/clear in `src-tauri/src/lib.rs:671-673`. The `intel_queue` write fence immediately after the old guard (`src-tauri/src/intel_queue.rs:2556-2565`) stays intact.

Persist through W3-C only. Trust Compiler returns `TrustScore`; a W4-A recompute operation calls W3-C's read API, compiles, then calls W3-C's allowlisted updater for `intelligence_claims.trust_score`, `trust_computed_at`, and `trust_version`. No direct SQL update against claim rows in `abilities/trust`.

End-state alignment: W4-A turns trust from an absent/zero placeholder into deterministic per-claim substrate for W6-A rendering and W5 ability outputs, while retiring the false-positive-prone binary contamination write gate. It forecloses row-creation-age freshness, six-canonical-factor drift, W3-C calling W4-A, and env-var-driven hard rejection in enrichment.

## 3. Key decisions
Pure compiler boundary: pick `compile_trust(claims: &[ClaimRow], context: TrustContext) -> TrustScore`; DB reads, clock reads, footprint construction, feedback consumption, signal emission, and row updates live outside. This follows ADR-0114's no hidden state rule at `.docs/decisions/0114-scoring-unification.md:24-30` and ADR-0104's injected clock model at `.docs/decisions/0104-execution-mode-and-mode-aware-services.md:49-63`.

Persistence: score lives on W3-C's claim row, matching ADR-0114 R1.7 `.docs/decisions/0114-scoring-unification.md:312-316` and W3-C schema `.docs/plans/wave-W3/DOS-7-plan.md:23`. W4-A does not add a table and does not mutate `provenance_json` or `metadata_json`.

Aggregation: geometric mean, not arithmetic mean. Arithmetic would let a 1.0 factor average away a floor-level contradiction or cross-entity hit; geometric mean preserves "one weak factor matters" while still letting strong provenance distinguish weak foreign-name hints from broad unsupported contamination.

Canonical factor count: exactly five shared canonical factor names from ADR-0114 R1.4. `subject_fit_confidence` and `cross_entity_coherence` are Trust Compiler local inputs. Source diversity is represented inside `claim_corroborations.strength`, not a separate `source_count` factor.

Cross-entity factor input shape: use `CrossEntityCoherenceInput` above. It preserves DOS-326's prompt-aware semantics, expanded target identity, and soft-signal philosophy without keeping `intelligence::contamination` as a public module or gate.

Bands: export `TrustBand::{LikelyCurrent, UseWithCaution, NeedsVerification, Unscored}` with stable serde strings. W6-A may render labels such as "trust this", "be careful", "verify first"; the compiler owns stable machine bands.

## 4. Security
The new risk is scoring a wrong-subject claim as reliable because its source is reliable. W3-B's `SubjectAttribution` fails ambiguous/blocked subject fit for claim-bearing output (`.docs/plans/wave-W3/DOS-211-plan.md:57-59`), ADR-0123 `WrongSubject` creates per-subject tombstone behavior (`.docs/decisions/0123-typed-claim-feedback-semantics.md:45-54`, `:129-142`), and W4-A additionally downranks `subject_fit_confidence` / `cross_entity_coherence`.

The expanded target footprint is cross-tenant sensitive. Extractor queries must scope to the user's local workspace DB only, and logs/test failures must not include customer names, raw claim text, source excerpts, domains, or Glean payloads. `CrossEntityHit` evidence stored or emitted should use token kind, redacted token hash, source subject id/type, and count; full tokens stay in debug-only fixtures.

No hard rejection by default. Deleting the env var removes a bypassable global policy knob; customer-facing export/compliance flows may later choose to suppress `NeedsVerification` claims, but that is a consumer policy, not a Trust Compiler write gate.

## 5. Performance
Budget is W4 Suite P: trust score p99 < 5ms at claim volume (`.docs/plans/v1.4.0-waves.md:169`, `:566`). The compiler itself is O(number of factors + hits) and should be <1ms per claim; DB extraction dominates.

Avoid the current contamination pattern of loading all accounts inside an enrichment write path (`src-tauri/src/intelligence/contamination.rs:83-93`) for each scan. The W4 extractor should build portfolio footprints once per recompute batch or per entity and pass value objects to the compiler. Corroboration aggregation reads indexed W3-C child rows; cache the noisy-OR result only if profiling shows repeated claim recomputes dominate.

Backfill/shadow run keeps DOS-5's stated <=30 min for 100K claims and distribution checks: no NaN, no negatives, <=5% at floor/ceiling. Floating point code uses `f64`, clamps before `ln`, never subtracts then `.sqrt()`s, and property-tests 10K random tuples.

## 6. Coding standards
Services-only mutations hold: `abilities/trust` computes; W3-C's service updater mutates. No command handler or queue path updates `trust_score` directly. The implementation must add or extend the W3-C lint that rejects direct `UPDATE intelligence_claims SET trust_score` outside the claims service allowlist.

Intelligence Loop 5-question check (`CLAUDE.md:7-14`): trust recompute emits/informs `ClaimTrustChanged`; trust feeds confidence bands, not health scores directly; trust belongs in prep/intel context only through default active claim loaders; briefing callouts consume bands later; typed feedback feeds Bayesian/source/agent weights through ADR-0123.

No direct `Utc::now()` or `rand::thread_rng()` in services or abilities. Do not reuse `signals::decay::age_days_from_now` for trust until it is made clock-injected; it calls `Utc::now()` at `src-tauri/src/signals/decay.rs:15-29`. Fixtures use generic accounts/domains only per `CLAUDE.md:18`. Clippy budget is zero warnings, including property tests.

## 7. Integration with parallel wave-mates
W3-B/DOS-211 provides `SourceAttribution { observed_at, source_asof, evidence_weight, scoring_class }` per `.docs/plans/wave-W3/DOS-211-plan.md:31-35` and deterministic provenance composition at `:67`. W4-A reads that shape; it does not change provenance schema.

W3-C/DOS-7 owns `intelligence_claims`, `claim_corroborations`, `claim_feedback`, `agent_trust_ledger`, and `services/claims.rs` (`.docs/plans/wave-W3/DOS-7-plan.md:19-27`). W4-A consumes those read/update APIs. W3-C must not call Trust Compiler during substrate creation; W4-A's recompute operation starts after W3-C lands.

W3-G/DOS-299 owns `source_asof` population and freshness fallback. If W3-G lands first, W4-A imports `FreshnessContext`; if W4-A lands first, define `FreshnessContext { timestamp_known: bool, ... }` under `abilities/trust` and W3-G adopts it. W3-G already plans trust tests for timestamp unknown at `.docs/plans/wave-W3/DOS-299-plan.md:85-93`.

W3-H/DOS-300 owns `temporal_scope`, `sensitivity`, and claim-type registry defaults. W4-A reads `temporal_scope` for freshness context but does not activate DOS-10 temporal decay policy beyond the W3-G fallback.

W6-A/DOS-320 consumes `TrustBand` for rendering/filtering. W4-A must stabilize enum names and factor evidence, but W6-A owns UI labels and surface policy.

## 8. Failure modes + rollback
If W3-C schema/updater is absent, W4-A cannot persist and must fail PR open rather than direct-SQL around it. If trust recompute fails for one claim in batch, leave the prior `trust_score`/`trust_version` unchanged, record a non-content error count, and continue or abort per batch mode. If factor config is malformed or weights do not sum, boot/test fails fast rather than producing silently distorted scores.

If the cross-entity factor over-penalizes legitimate parent/child, portfolio, DBA, or peer-benchmark mentions, rollback is config/feature scoped: disable the local coherence weight or hold affected recompute jobs, not resurrect the old `DAILYOS_CONTAMINATION_VALIDATION` gate. Prior scores remain last-written derived values and can be recomputed.

If a migration/projection from W3 is running, W4-A honors W1-B/W3 write fence by running recompute after DOS-7 cutover and through service writes only. `FenceCycle` captures/rechecks `schema_epoch` at `src-tauri/src/intelligence/write_fence.rs:67-109`, drain/bump primitives at `:112-155`; W4-A does not bypass them.

## 9. Test evidence to be produced
Unit tests: `trust_geometric_mean_all_floor_05`, `trust_geometric_mean_all_one`, `trust_geometric_mean_mixed_08`, `trust_feedback_boost_clamped_to_ceiling`, `trust_contradiction_present_downranks`, `trust_nan_never_emitted_for_random_factor_tuples`, `trust_factor_count_is_five_canonical_plus_local_helpers`, `freshness_context_timestamp_unknown_applies_penalty`, `freshness_context_timestamp_known_uses_source_asof_age`.

Corroboration tests, co-authored with W3-C after DOS-7 open question resolves: `corroboration_noisy_or_prefers_three_independent_sources`, `corroboration_same_source_reinforcement_saturates_below_diverse_sources`, `corroboration_zero_rows_clamps_to_floor`.

Cross-entity tests: `cross_entity_coherence_clean_claim_scores_one`, `cross_entity_coherence_foreign_domain_scores_low_without_rejecting`, `cross_entity_coherence_foreign_vip_host_scores_low`, `cross_entity_coherence_company_name_suppressed_when_target_name_present`, `cross_entity_coherence_allows_target_subdomain`, `cross_entity_coherence_skips_when_context_expected`, `peer_benchmark_claim_sets_cross_entity_context_expected`.

Integration tests: `trust_recompute_updates_claim_trust_columns_via_claims_service`, `trust_recompute_does_not_update_direct_sql`, `claim_trust_changed_emits_on_policy_threshold`, `bundle1_cross_entity_bleed_lowers_band`, `bundle5_user_correction_tombstone_not_averaged_away`, `trust_compiler_p99_under_5ms_claim_volume`.

Wave merge-gate artifact: score distribution on representative dev DB, trust math benchmark report, `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` (`CLAUDE.md:20-24`), plus W4 Suite P proof for p99 <5ms and Suite E proof for bundle-1 coherence and bundle-5 correction-resurrection. Suite S contribution is redacted evidence/log assertions for cross-entity footprints and no customer data in fixtures.

## 10. Open questions
1. Corroboration noisy-OR: DOS-7 open question says amendment A may saturate wrong. Confirm formula before W4-A locks expected Trust Compiler tests.
2. `ClaimTrustChanged` threshold: DOS-5 says `|delta| > CHANGE_EPSILON (0.01)`, while ADR-0115 says emit only on trust-band boundary crossings (`.docs/decisions/0115-signal-granularity-audit.md:147-157`). Which rule wins for W4?
3. Factor weights after ADR-0114 R1.4: DOS-5's old six weights include both `src` and `rel`; W4 has five canonical factors plus local `subject_fit_confidence` and `cross_entity_coherence`. Confirm default weights and whether local factors live in `[trust.local_factors]`.
4. Persistence of factor evidence: W3-C schema names `trust_score`, `trust_computed_at`, and `trust_version` but no dedicated factor-breakdown column. Should W4-A persist `cross_entity_coherence` evidence in an agreed trust-evidence JSON field, or is it returned only in recompute output/provenance until W6-A?
5. Retired devtools commands: should `devtools_audit_cross_contamination`/`devtools_clear_contaminated_enrichment` be deleted with the module, or replaced by a debug-only trust-coherence audit that never clears data?
6. ADR-0114/Linear says Trust Compiler calls `scoring::factors`, but `.docs/plans/v1.4.0-waves.md:730-733` puts the scoring factor library refactor outside v1.4.0 and current code has no `src-tauri/src/scoring/`. Confirm W4-A uses trust-local pure factors with ADR-0114 names until the later shared-library refactor, rather than expanding this slot's file ownership.
