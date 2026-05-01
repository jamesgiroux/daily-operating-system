# Implementation Plan: DOS-288

## Revision history
- v1 (2026-05-01) — initial L0 draft.

## 1. Contract restated

DOS-288 is the W6 validation layer for the cross-entity bleed invariant. The ticket names the missing safety property: "none of those automatically prove that content attached to an entity actually belongs to that entity" and "This is the missing invariant behind cross-account content bleed." The product contract is broader than the old DOS-287 detector: "Every rendered claim, summary, talking point, action, risk, health reason, stakeholder insight, and briefing section must pass a **content ownership check** before it is shown as attached to an entity or meeting."

The ownership questions are load-bearing: "What entity or meeting is this block about?", "Which source evidence supports that subject?", "Why is this evidence allowed to attach to this subject?", "What competing entities could this evidence belong to?", and "If the answer is ambiguous, how is that ambiguity shown instead of silently rendered as fact?" Acceptance is also explicit: "Bleed detector catches wrong-subject content before it reaches confident UI rendering", "Cross-entity canonicalization is rejected unless subject identity is the same or explicitly merged", and "Golden Daily Loop release gate fails if any seeded bleed fixture renders as confident fact."

There are no Linear comments on DOS-288; the 2026-04-24 issue body is the amendment source. The applicable 2026-04-24 substrate amendments are W3-B SubjectAttribution / subject-fit finalization (`.docs/plans/wave-W3/DOS-211-plan.md:13`, `:57-61`), ADR-0105 `source_asof` and provenance warnings (`.docs/decisions/0105-provenance-as-first-class-output.md:391-420`, `:435-448`), ADR-0123 `WrongSubject` per-subject tombstones (`.docs/decisions/0123-typed-claim-feedback-semantics.md:45-54`, `:146-153`), and ADR-0110 fixture/eval governance (`.docs/decisions/0110-evaluation-harness-for-abilities.md:20-39`, `:91-105`).

Current code still has the DOS-287 runtime detector: `intelligence::contamination` scans foreign domains, WP-VIP hosts, and company names (`src-tauri/src/intelligence/contamination.rs:10-19`, `:109-185`); `intel_queue.rs` calls it before persistence and can emit/reject in strict mode (`src-tauri/src/intel_queue.rs:2460-2554`); devtools commands expose audit/cleanup (`src-tauri/src/lib.rs:671-673`, `src-tauri/src/commands/app_support.rs:1478-1648`). W4-A/DOS-5 retires that path by folding the same heuristics into `cross_entity_coherence`, so W6-B proves the new substrate blocks the DOS-287 reproduction without preserving `contamination.rs`.

## 2. Approach

Add a pure subject-ownership validator under W3-B's provenance module: `src-tauri/src/abilities/provenance/ownership.rs`, exported by `abilities/provenance/mod.rs` once W3-A/W3-B create the module root. The validator is not a production render/write gate; it is a static/eval assertion layer used by tests and the W6 release gate. It compiles in the normal library but no Live render, queue, or command path calls it.

Core API shape:

```rust
pub fn validate_subject_ownership(
    output: &AbilityOutput<impl Serialize>,
    rendered_paths: &[FieldPath],
    policy: OwnershipPolicy,
) -> Result<OwnershipReport, OwnershipError>;
```

`OwnershipReport` records subject, rendered field paths checked, source refs resolved, competing subjects, cross-entity coherence hits, and render policy (`confident | ambiguous | suppressed | needs_verification`). `OwnershipError` is structural and non-content-bearing: missing subject, missing field attribution, source ref without entity-link evidence, ambiguous/blocked subject fit, cross-subject canonical merge, or confident render despite low `cross_entity_coherence`.

Algorithm:
1. Walk the `AbilityOutput<T>` provenance tree. ADR-0102 pins provenance to the wrapper only (`.docs/decisions/0102-abilities-as-runtime-contract.md:166-179`, `:298-305`), so domain output types must not carry a second provenance copy.
2. For every renderable claim-bearing field path, require W3-B field attribution and `SubjectAttribution`. W3-B already plans finalize rejection for `Ambiguous` / `Blocked` subject fit (`.docs/plans/wave-W3/DOS-211-plan.md:57-59`); W6-B asserts no render surface regresses around that guard.
3. Resolve each field's source refs to `SourceAttribution` / child provenance. Source attribution must include entity-link evidence, not only raw source IDs, matching the DOS-288 Scope §1 contract and ADR-0105 source/field rules (`.docs/decisions/0105-provenance-as-first-class-output.md:165-173`, `:206-241`).
4. Reject canonical duplicate/corroboration groups when subject refs differ unless the provenance explicitly declares `SubjectRef::Multi` or an explicit user-confirmed merge. Current `SubjectRef` variants and deterministic Multi behavior exist in `src-tauri/src/db/claim_invalidation.rs:57-78`, `:168-228`.
5. For generated text and claim rows that survive subject fit, call W4-A's real `CrossEntityCoherenceInput` API, not a mock. If W4-A has not landed when W6-B starts, compile against the planned interface from `.docs/plans/wave-W4/DOS-5-plan.md:31-55` and rebase to the landed names before PR.

Add `src-tauri/tests/dos288_bleed_detection_test.rs`. It follows the SQL-fixture pattern from `src-tauri/tests/dos311_reconcile_test.rs:22-43`: open an in-memory SQLite DB, load W6-A bundle SQL, invoke the relevant ability/eval path or trust compiler, and assert ownership/trust outcomes. W6-A owns the seed directories under `src-tauri/tests/fixtures/bundles/` and specifically calls out bundle-1 as same-domain/cross-entity coverage (`.docs/plans/wave-W6/_prompts/DOS-283.md:15-23`, `:46-57`).

Document "structural impossibility" inline in the module/test docs rather than creating a separate `.docs/` artifact. The proof is: subject-fit prevents wrong-subject `AbilityOutput` from finalizing or rendering confidently; cross-entity-coherence prevents stored/canonicalized wrong-subject claims from retaining a confident trust band even if valid provenance exists for another subject. This moves v1.4.0 from heuristic post-generation cleanup to pre-render provenance + trust invariants, and forecloses reviving `DAILYOS_CONTAMINATION_VALIDATION` as the release gate.

## 3. Key decisions

Validator placement: choose `src-tauri/src/abilities/provenance/ownership.rs`. Extending `ProvenanceBuilder` would blur W3-B's construction invariant with W6's audit/eval invariant; a standalone `tests/` helper would make W6-C unable to call the same logic from the release gate. A pure provenance sibling keeps the invariant close to `SubjectAttribution` while remaining non-production-gating.

Structural impossibility claim: it is conditional, not magic. Layer 1 is W3-B finalization: `SubjectFitStatus::Ambiguous | Blocked` for claim-bearing output returns before rendering (`.docs/plans/wave-W3/DOS-211-plan.md:57-59`). Layer 2 is W4-A scoring: `cross_entity_coherence` reuses the old foreign domain / infrastructure / company-name heuristics as a Trust Compiler factor (`.docs/plans/wave-W4/DOS-5-plan.md:30-55`, `:76-81`). Assumptions: all W5/W6 render paths consume `AbilityOutput<T>`; render policy respects `TrustBand`; bundle footprints include aliases/domains/project/person evidence; and legacy `intelligence.json` surfaces are not used as confident facts without DOS-301 projection.

Test suite shape: deterministic bundle-1 scenarios are mandatory; property tests are supplemental only. The ticket's enumerated cases map to named rows in bundle-1: same-domain accounts, similar names/stakeholders, parent/child/project boundary, same first name/role, recurring series entity change, user link correction, Account B evidence in Account A briefing, cross-subject canonical duplicate, and stale historical source. Property tests cover symmetric subject-pair invariants and Multi ordering, but cannot replace bundle rows.

CI integration: implement as ordinary Rust integration tests so existing CI runs them through `.github/workflows/test.yml:77-79` (`cargo test`). Add a narrow selector for W6-C: `cargo test --test dos288_bleed_detection`. No workflow edit is needed unless W6-C wants a separate release-gate step.

Devtools command: W6-B does not replace `devtools_audit_cross_contamination`. W4-A owns deletion or a debug-only trust-coherence audit; its open question is already captured in `.docs/plans/wave-W4/DOS-5-plan.md:126-131`. W6-B tests must fail if they import `crate::intelligence::contamination`, because that would keep the retired guard alive.

## 4. Security

The attack surface is false confidence: valid source provenance attached to the wrong account, project, person, or meeting. The validator must fail closed on missing or ambiguous subject ownership and must never downgrade a blocked subject into a warning that can render as fact. `WrongSubject` feedback tombstones only the asserted subject and proposes corrected-subject claims through the normal gate (`.docs/decisions/0123-typed-claim-feedback-semantics.md:148-153`), so resurrection tests must check both the wrong subject and the corrected subject.

Reports, errors, test logs, and release-gate artifacts must not print raw claim text, account names, source excerpts, email addresses, domains, prompt text, or Glean payloads. Use field paths, synthetic row IDs, subject-ref type/id, hit kind, counts, and redacted token hashes. ADR-0108 says logs reference invocation IDs rather than provenance content (`.docs/decisions/0108-provenance-rendering-and-privacy.md:48-52`) and explanations are sanitized before rendering (`:74-84`).

Auth/authz stays with W3-A/W4-C registry and bridges. W6-B adds no user command, no MCP tool, no external API, and no source read beyond fixture/eval paths. Fixture data must remain synthetic per `CLAUDE.md:16-18` and ADR-0110 anonymization rules (`.docs/decisions/0110-evaluation-harness-for-abilities.md:132-141`).

## 5. Performance

No production hot path is added. The validator is O(rendered field paths + provenance source refs + child provenance refs + cross-entity hits). It performs no network calls, provider calls, embeddings, cache writes, or DB writes. In tests, DB work is fixture loading and footprint extraction only.

The cross-entity factor budget is owned by W4-A: trust math p99 < 5ms at claim volume (`.docs/plans/v1.4.0-waves.md:167-170`, `.docs/plans/wave-W4/DOS-5-plan.md:83-88`). W6-B should record elapsed time in the Suite S/E report but not introduce a new Suite P budget. If W6-C invokes this suite, it must stay small enough to fit ADR-0110's 30-60s hermetic eval window (`.docs/decisions/0110-evaluation-harness-for-abilities.md:91-105`).

## 6. Coding standards

Services-only mutations: W6-B writes no production state. `ownership.rs` is pure validation; `dos288_bleed_detection_test.rs` loads in-memory fixture SQL; no command handler, service mutation, signal emission, queue wake, `intelligence.json` write, or claim commit is introduced. The W1-B write fence is therefore honored by construction.

Intelligence Loop check (`CLAUDE.md:7-14`): no new table/column/signal/health rule/briefing callout/feedback UI is added. The validator verifies that W3/W4/W5 substrate outputs obey the existing loop before render. Feedback semantics are consumed through ADR-0123, not redefined.

No direct `Utc::now()` or `thread_rng()` in services or abilities. Use fixture clocks from ADR-0110 and W3-B's injected-clock provenance production (`.docs/decisions/0105-provenance-as-first-class-output.md:34-36`). Clippy budget is zero warnings under the standard gate (`CLAUDE.md:20-24`). Do not edit `src-tauri/src/services/context.rs` or `src-tauri/src/intelligence/provider.rs`.

## 7. Integration with parallel wave-mates

W3-B/DOS-211 owns the provenance envelope, field attribution, subject attribution, and finalize errors (`.docs/plans/wave-W3/DOS-211-plan.md:19-45`, `:57-67`). W6-B extends the provenance module with validator logic after W3-B lands; it does not change the envelope schema unless W3-B's final names differ from the plan.

W4-A/DOS-5 owns `abilities/trust`, `CrossEntityCoherenceInput`, `CrossEntityHit`, `TrustBand`, and retirement of `intelligence::contamination` (`.docs/plans/wave-W4/DOS-5-plan.md:30-57`, `:115-124`). W6-B imports the real API and asserts wrong-subject content cannot be `LikelyCurrent`.

W6-A/DOS-283 owns bundle SQL and README mapping rows to invariants (`.docs/plans/v1.4.0-waves.md:606-612`, `.docs/plans/wave-W6/_prompts/DOS-283.md:24-31`). This checkout does not yet contain `.docs/plans/wave-W6/DOS-283-plan.md`, so W6-B must coordinate on final bundle row IDs before coding.

W6-C/DOS-281 consumes W6-B's exact test selector and evidence artifact (`.docs/plans/wave-W6/_prompts/DOS-281.md:15-20`, `:48-53`). W5-A/DOS-218 and W5-B/DOS-219 supply the actual `get_entity_context` and `prepare_meeting` ability outputs, including provenance and trust bands (`.docs/plans/wave-W5/DOS-218-plan.md:87-95`, `.docs/plans/wave-W5/DOS-219-plan.md:91-99`). W6-D/DOS-320 may consume `TrustBand` render policy, but W6-B should not edit render surfaces.

## 8. Failure modes + rollback

If W3-B subject-fit types are absent, W6-B cannot prove ownership; block implementation rather than invent parallel subject shapes. If W4-A has not landed, tests may compile against the planned interface temporarily but must not merge without the real API. If W6-A bundle-1 rows are missing or renamed, the integration suite fails hard with fixture-missing errors.

If the validator is too strict, legitimate comparisons, parent/child rollups, partner mentions, or peer benchmarks may be marked ambiguous. Rollback is removing or adjusting the validator/tests, not restoring `DAILYOS_CONTAMINATION_VALIDATION`. Legitimate cross-entity content must pass via explicit `cross_entity_context_expected`, related subjects, `SubjectRef::Multi`, or user-confirmed binding.

If a projection/render path bypasses `AbilityOutput<T>`, the structural proof fails: the suite should report `legacy_surface_bypass` and W6-C should block release. The safe fallback is to keep Stage 3 parallel run visible on the old path until the bypass is migrated or filtered.

Rollback has no migration concern: remove the validator module and integration tests, and W6-C drops the selector. No persisted data, schema version, or write fence state changes. The universal write fence is honored because W6-B has no write path and fixture tests use in-memory SQLite.

## 9. Test evidence to be produced

Unit tests in `abilities/provenance`: `ownership_validator_rejects_blocked_field_subject`, `ownership_validator_rejects_ambiguous_competing_subjects`, `ownership_validator_requires_source_ref_entity_link_evidence`, `ownership_validator_allows_user_confirmed_subject_override`, `ownership_validator_rejects_cross_subject_canonical_merge`, and `ownership_validator_allows_explicit_multi_subject_when_declared`.

Integration tests in `src-tauri/tests/dos288_bleed_detection_test.rs`: `bundle1_same_domain_accounts_only_primary_subject_confident`, `bundle1_similar_account_names_foreign_company_needs_verification`, `bundle1_parent_child_project_boundary_blocks_wrong_level`, `bundle1_same_first_name_person_action_blocks_wrong_person`, `bundle1_recurring_series_account_change_blocks_inherited_old_account`, `bundle1_wrong_link_tombstone_not_resurrected_after_reenrichment`, `bundle1_account_b_evidence_never_confident_in_account_a_brief`, `bundle1_duplicate_claims_not_canonicalized_across_subjects`, `bundle1_stale_account_a_source_does_not_contaminate_account_b`, and `bundle1_dos287_vip_host_repro_blocked_without_contamination_module`.

Trust/coherence assertions: `cross_entity_coherence_foreign_domain_lowers_trust_band`, `cross_entity_coherence_foreign_vip_host_lowers_trust_band`, `cross_entity_coherence_foreign_company_name_lowers_trust_band`, `cross_entity_context_expected_allows_explicit_comparison`, and `target_owned_subdomain_remains_confident`.

Wave artifact: `cargo test --test dos288_bleed_detection`, `cargo test ownership_validator`, standard `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`, and `src-tauri/target/dos288/bleed-report.json` generated by the test/release-gate run with row IDs, invariant IDs, pass/fail, redacted hit kinds, and elapsed time. Suite S contribution: DOS-287 reproduction blocked and no PII/secrets in fixture output. Suite E contribution: bundle-1 ownership invariant pass. Suite P contribution: non-blocking elapsed-time evidence only.

## 10. Open questions

1. W6-A plan absent in this checkout: what are the final bundle-1 row IDs / invariant IDs that W6-B tests should target?
2. W3-B final type names: is the landed enum `SubjectFitStatus::{Confident, Ambiguous, Blocked}`, `SubjectFitConfidence`, or another name? The plan uses the DOS-211 contract, not final Rust spelling.
3. W4-A threshold: what exact `cross_entity_coherence` factor value maps to `NeedsVerification` vs `UseWithCaution`, and does W6-B assert the numeric value or only the resulting `TrustBand`?
4. DOS-288 acceptance names `get_daily_readiness`, but current code has readiness fragments rather than a W5/W6 ability. Which owner supplies that fixture surface for W6, or is it deferred to W6-C release-gate aggregation?
5. Lint mode scope: should suspected bleed be exposed only as eval/release-gate diagnostics, or does v1.4.0 need a user/devtools surface after W4-A removes `devtools_audit_cross_contamination`?
