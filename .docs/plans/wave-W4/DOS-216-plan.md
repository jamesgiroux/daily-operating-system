# Implementation Plan: DOS-216

## Revision history
- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-216 scaffolds the ability evaluation harness, not migrated ability behavior. The Linear body is explicit: "Implement the evaluation harness: fixture layout, scoring per category, regression classification via prompt fingerprint, CI integration, and fixture governance (anonymization, retention, purge)." The fixture contract is also explicit: "Fixture contents: `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `clock.txt`, `seed.txt`, `expected_output.json`, `expected_provenance.json`." The runner must build `ServiceContext::new_evaluate` from the fixture clock/seed/replay data, run hermetically, and classify regressions as `ProviderDrift`, `PromptChange`, `InputChange`, `CanonicalizationBug`, or `LogicChange`.

All four 2026-04-24 amendments apply, with the last one setting W4's blocking slice. The production-readiness addendum requires Golden Daily Loop/user-loop fixture support and says "Golden Daily Loop fixture subset runs as a release-check command." The catalogue addendum says "report coverage by scenario bundle, not only by ability" with metadata such as `scenario_id`, `invariant`, `expected_render_policy`, and `surfaces_exercised`. The prompt/eval addendum says the harness should measure "query count, latency, repair fanout, cache hit rate" and scoped correction behavior. The plan-hardening amendment narrows W4: "The eval harness should land minimal replay-first infrastructure before broader capture/anonymization/live-judge work" and names one Read fixture, one Transform fixture, replay-only provider, exact output diff, provenance/source-warning diff, fixture loading from bundle directories, and hermetic CI as the first landing bar.

Current code already has the W2 seams but not the harness. `ReplayProvider` is explicitly W4-B's consumer surface (`src-tauri/src/intelligence/provider.rs:10-12`), `IntelligenceProvider::complete` returns `Completion` with fingerprint metadata (`src-tauri/src/intelligence/provider.rs:121-126`, `:193-203`), and a replay miss returns `ProviderError::ReplayFixtureMissing` without live fallback (`src-tauri/src/intelligence/provider.rs:227-295`). `ServiceContext` already exposes `ExecutionMode::Evaluate` (`src-tauri/src/services/context.rs:35-47`), deterministic `FixedClock`/`SeedableRng` (`:83-180`), and `new_evaluate` (`:368-390`). `src-tauri/src/abilities/` is absent in the current tree, so this plan depends on W3-A/DOS-210's registry plan and W3-B/DOS-211's provenance plan rather than inventing ability shapes.

## 2. Approach

Create the harness as Rust integration-test infrastructure, centered on `src-tauri/tests/harness.rs`, so the command is `cd src-tauri && cargo test --test harness`. This matches existing integration-test patterns: DOS-311 loads SQL fixtures into in-memory SQLite from `src-tauri/tests/dos311_fixtures/` (`src-tauri/tests/dos311_reconcile_test.rs:26-43`), while DOS-259 already proves replay-provider behavior in an integration test (`src-tauri/tests/dos259_provider_selection_test.rs:130-166`).

Add fixture directories under `src-tauri/tests/abilities/{ability}/fixtures/fixture_N/` with the Linear/ADR required files. Keep the ADR flat file names as required by ADR-0110 (`.docs/decisions/0110-evaluation-harness-for-abilities.md:22-39`), and add one per-fixture `metadata.json` only for the 2026-04-24 catalogue fields and anonymization certificate. Do not replace `inputs.json` with an `inputs/` directory or `expected_output.json` with an `expected_output/` directory unless L0 reviewers amend the ticket. Ability-level metadata stays in `fixtures/manifest.toml`, `evals/quality.toml`, and `evals/regression_baseline.json` per ADR-0110.

Implement a `FixtureRunner` data model inside the harness runner: `FixtureManifest`, `FixtureMetadata`, `EvalFixture`, `ReplayCompletionMap`, `ExpectedArtifacts`, `CategoryScorer`, `RegressionClassifier`, and `HarnessReport`. `EvalFixture` loads `clock.txt` into `FixedClock`, `seed.txt` into `SeedableRng`, `state.sql` into an in-memory SQLite connection, `provider_replay.json` into `ReplayProvider::new`, and `external_replay.json` into fixture-backed `ExternalClients`. The runner then calls the W3-A registry erased path (`invoke_by_name_json`) with `ExecutionMode::Evaluate`; if W4-C's `EvalAbilityBridge` lands first, the harness consumes that bridge rather than creating a parallel public bridge.

Runner algorithm:

1. Discover ability fixture roots under `src-tauri/tests/abilities/**/fixtures/manifest.toml`.
2. Apply label filters from CLI args/env (`@core` by default for PR fast path; full set for explicit/nightly).
3. For each selected fixture, validate governance metadata before loading any replay payload.
4. Build `ServiceContext::new_evaluate` and replay providers from fixture clock/seed/replay files.
5. Invoke the registry by ability name and JSON input.
6. Canonicalize actual/expected JSON, score by category, diff provenance, classify regressions, and append a redacted report entry.

Scoring is category-specific, following ADR-0110 (`.docs/decisions/0110-evaluation-harness-for-abilities.md:41-51`). Read uses exact canonical JSON equality against `expected_output.json`. Transform uses continuous rubric scores from `evals/quality.toml`, but W4 CI uses replayed judge completions, not a live judge. Maintenance compares `planned_mutations` snapshots. Publish compares planned/outbox entries only, never external side effects. Provenance comparison always runs: compare `expected_provenance.json` against the full W3-B envelope, including `sources[]`, `children[]`, field attributions, warnings, and `source_asof`/subject fields from DOS-211's plan (`.docs/plans/wave-W3/DOS-211-plan.md:31-35`, `:57-61`).

Regression classification uses ADR-0106's SHA-256 `canonical_prompt_hash` rules (`.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:40-47`) and evaluation mapping (`:106-114`). Store the current fingerprint in `expected_provenance.json` and duplicate the compact baseline in `evals/regression_baseline.json` for fast classification. Store replay completions keyed by `canonical_prompt_hash`; until DOS-213's production canonicalizer is available, the W2 `prompt_replay_hash` fallback is accepted only for W2-style fixtures and is tagged as `hash_algorithm = "w2_rendered_prompt_sha256"` in `metadata.json`.

Classifier precedence is deterministic:

1. If `inputs_hash` or `state_sql_hash` changed, label `InputChange`.
2. Else if `prompt_template_version` changed, label `PromptChange`.
3. Else if `canonical_prompt_hash` changed while template version and inputs stayed fixed, label `CanonicalizationBug`.
4. Else if replayed completion text differs while fingerprint fields match, label `ProviderDrift`.
5. Else if output/provenance differs with no explanatory fingerprint/input delta, label `LogicChange`.

Add governance tooling in the implementation PR: `scripts/check_eval_fixture_anonymization.sh`, `src-tauri/tests/eval_fixture_governance.rs`, and `.docs/evals/fixture-governance.md`. The script scans only checked-in fixtures and rejects non-`example.com` emails, phone-like numbers, known real-domain patterns, and `fixture_identity_map.json`. Capture mode writes a candidate fixture only when explicitly invoked with `--capture-fixture`; it tokenizes entity names, redacts email/phone/free text, writes the identity map out of tree, and never runs in CI.

The governance doc records retention and purge as executable policy, not prose-only guidance: quarterly stale-fixture review, rebaseline-on-prompt-template-version bump, regenerate/remove on source revocation, and no shared fixture corpus across developers without re-anonymization.

CI integration extends the existing workflow after the Rust test step (`.github/workflows/test.yml:74-79`) with `cargo test --test harness` and the anonymization script, plus a PR-time subset selector: `@core` runs fast fixtures, `@regression` adds accepted regressions, `@edge` adds edge cases, `@golden-daily-loop` runs the release subset. Nightly/full-suite wiring is tracked but not W4-blocking unless reviewers require it.

End-state alignment: DOS-216 gives W5 pilot abilities a deterministic fixture spine, gives W4-A trust work a way to prove bundles 1 and 5, gives W6 release-gate work a report format, and forecloses live-provider or live-DB evaluation in CI.

## 3. Key decisions

Fixture format: pick ADR/Linear flat required files plus additive `metadata.json`. Reason: ADR-0110's fixture shape is concrete (`state.sql`, `inputs.json`, replay files, clock/seed, expected artifacts), while the 2026-04-24 catalogue amendment needs per-fixture scenario fields not covered by `expected_output.json`. `metadata.json` holds `bundle`, `labels`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, `source_lifecycle_refs`, `anonymization_cert`, `retention_policy`, `prompt_fingerprint_baseline`, and optional post-action state descriptors.

Scoring: output is continuous internally but CI resolves to pass/fail per category. Read, Maintenance, and Publish are exact/structural pass-fail. Transform has continuous judge dimensions from `evals/quality.toml` (`.docs/decisions/0110-evaluation-harness-for-abilities.md:107-121`) but fails CI only when thresholds are below the configured floor or a hard provenance/source-warning diff occurs. Prompt changes are fail-soft per ADR-0110 (`:81-89`): they block pending explicit reviewer rebaseline, not because prompt edits are inherently wrong.

Hermetic isolation: use in-memory SQLite, `ExecutionMode::Evaluate`, `FixedClock`, `SeedableRng`, `ReplayProvider`, and fixture-backed external handles. The service layer already rejects non-Live mutations through `check_mutation_allowed()` (`src-tauri/src/services/context.rs:412-422`), and ADR-0104 requires evaluation mode to inject clock/RNG and replay providers (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:256-265`). The harness must also add a guard that no fixture runner opens production DB paths or network clients.

Prompt fingerprint hash: canonical SHA-256 per ADR-0106 is the target. Baseline storage is git-tracked beside the fixture: full fingerprint in `expected_provenance.json`, replay key in `provider_replay.json`, and compact run baseline in `evals/regression_baseline.json`. W2's rendered-prompt hash (`src-tauri/src/intelligence/provider.rs:211-225`) remains a temporary replay lookup only, never the final regression-classification hash.

Bundle governance: W4 blocks on Bundle 1 and Bundle 5 coverage reportability, not on perfect final corpus volume. Bundles 2-4 and 6-8 are recorded as tracked/non-blocking per the wave gate (`.docs/plans/v1.4.0-waves.md:175-183`). Trust scores may be `null`/`None` in W4 fixtures until W4-A lands non-trivial Trust Compiler scores, but the report schema reserves `trust_score`, `trust_band`, and `trust_missing_reason`.

## 4. Security

The main new attack surface is checked-in fixtures containing personal or customer data. CLAUDE.md forbids real customer domains, names, email addresses, and account details in source and fixtures (`CLAUDE.md:16-18`). ADR-0110 requires placeholder names, `example.com` emails, NER redaction, and source IDs marked anonymized (`.docs/decisions/0110-evaluation-harness-for-abilities.md:132-153`). The anonymization script and governance test must fail closed before fixtures enter the repo.

The second surface is accidental live evaluation. The harness must be structurally unable to hit PTY, HTTP, Google/Glean/Slack/Gmail/Salesforce, or a live DB. Missing replay data is a typed harness failure, matching `ProviderError::ReplayFixtureMissing` (`src-tauri/src/intelligence/provider.rs:183-190`, `:290-295`), not a fallback to Live. Capture mode is opt-in local tooling, emits no secrets, and writes `fixture_identity_map.json` outside the repo.

Provenance diffing must not leak raw source excerpts in failure messages. Diffs use JSON pointer paths, source IDs/classes, warning variants, bundle IDs, and hash prefixes; they do not print prompt text, completion text, email bodies, account names, or unredacted `SourceAttribution` payloads. Revoked-source fixtures are regenerated or removed according to ADR-0098's source-aware lifecycle and ADR-0107's mask/purge behavior.

## 5. Performance

Hot path is CI fixture execution. Target budget is Linear's full suite `<=60 seconds` and ADR-0110's 30-60 second expectation for hermetic evals (`.docs/decisions/0110-evaluation-harness-for-abilities.md:93-105`). Use `tokio::task::JoinSet` or bounded worker tasks with existing Tokio (`src-tauri/Cargo.toml:35`) rather than adding Rayon. Each fixture gets its own in-memory SQLite connection and replay maps, so there is no shared DB lock contention.

The regression classifier hashes small canonical JSON documents with existing `sha2`/`hex` deps (`src-tauri/Cargo.toml:41-43`). Provenance comparison can be O(serialized JSON size); W3-B already budgets provenance and elides oversize trees (`.docs/plans/wave-W3/DOS-211-plan.md:77-83`). Report aggregation is append-only in memory and writes one CI artifact under `src-tauri/target/eval/`, not a committed file.

Judge scoring is the cost risk. W4 uses replayed judge completions, so Transform scoring adds JSON parsing and rubric threshold checks but no network/model latency. Live judge drift detection remains non-blocking until the replay runner is stable, per the 2026-04-24 plan-hardening amendment.

## 6. Coding standards

No source code in `services/context.rs` or `intelligence/provider.rs` changes; those W2 seams are read-only for this issue. Harness code must not introduce production mutations, direct DB writes from commands, or direct `Utc::now()`/`thread_rng()` in services/abilities. Existing W2 guidance puts deterministic clock/RNG behind `ServiceContext` (`src-tauri/src/services/context.rs:64-80`, `:109-180`), and the provider lint pattern shows the clock/RNG grep shape to reuse (`scripts/check_no_direct_clock_rng_in_provider_modules.sh:47-48`).

Intelligence Loop 5-question check: DOS-216 adds evaluation artifacts, not a product data surface. No new user-facing table, schema column, health-scoring rule, briefing callout, or feedback learning hook is added. If implementation adds `HarnessReport` serialization, it is CI evidence only and must not become app telemetry without a separate ADR/DOS issue.

Clippy budget is zero new warnings. Fixture data uses synthetic IDs and generic domains only. Any temporary sample ability must be `#[cfg(test)]` or experimental per W3-A's registry plan, not production exposed.

## 7. Integration with parallel wave-mates

W2-B/DOS-259 is merged and provides the replay provider, but the current trait method is `IntelligenceProvider::complete`, not `execute` (`src-tauri/src/intelligence/provider.rs:193-203`). The W4 prompt's `execute` wording should be treated as stale unless reviewers require a new method.

W3-A/DOS-210 provides registry discovery and `invoke_by_name_json`; its plan explicitly says DOS-216 consumes registry enumeration for fixture harness discovery (`.docs/plans/wave-W3/DOS-210-plan.md:76-84`). If W3-A does not ship stable sample Read/Transform test abilities, W4-B needs an approved test-only sample seam before satisfying the sample-fixture acceptance criterion.

W3-B/DOS-211 owns provenance. W4-B reads the final `AbilityOutput<T>`/`Provenance`/`SourceAttribution` shape and must not fork schema types. Fixture expected provenance captures the full envelope, not raw text, including field attribution and warnings (`.docs/plans/wave-W3/DOS-211-plan.md:31-35`, `:57-61`).

W4-A/DOS-5 consumes Bundle 1 and Bundle 5 harness evidence for Trust Compiler. Harness reports may record `trust = None` in early W4, but the schema must accept non-trivial trust scores when W4-A lands. W4-C/DOS-217 owns public `EvalAbilityBridge`; W4-B should either consume it or keep its runner-local adapter private to avoid two bridge APIs.

W5-A/DOS-218 and W5-B/DOS-219 generate the first real get-entity-context and meeting-brief fixtures. W6-A/DOS-283 owns production-grade seeded mock workspace bundles, but W4-B's metadata schema must already report bundle/scenario coverage.

## 8. Failure modes + rollback

Fixture load failure, replay miss, provenance mismatch, or hard regression class fails the harness before any production side effect. PromptChange and LogicChange can be fail-soft with explicit reviewer rebaseline; CanonicalizationBug and InputChange are hard until fixed or intentionally updated. Capture-mode anonymization failure must delete the candidate fixture directory and leave the out-of-tree identity map intact.

If CI wiring is wrong, the rollback is mechanical: remove the workflow step and the harness files; no migration or production state cleanup exists. If a fixture corrupts expected artifacts, rebaseline that fixture or remove it from the manifest while preserving the failure report for review. If manifest parsing fails because of TOML dependency placement, switch manifest/quality loading to a parser already approved by the repo or add a dev dependency with L0 reviewer approval.

W1-B universal write fence is honored: DOS-216 does not write `intelligence.json`, claim rows, projections, signals, or external systems. Test output goes to temp dirs/in-memory DBs and optional CI artifacts. Capture mode writes checked-in fixture candidates only by explicit developer command, not during app runtime or normal CI.

## 9. Test evidence to be produced

Required harness tests: `harness_loads_fixture_manifest_and_metadata`, `harness_constructs_evaluate_context_from_clock_seed`, `harness_loads_provider_replay_by_prompt_hash`, `harness_replay_provider_missing_hash_is_hard_failure`, `harness_read_exact_output_diff_fails`, `harness_provenance_warning_diff_fails`, `harness_transform_replayed_judge_threshold_passes`, `harness_maintenance_planned_mutation_snapshot_diff`, `harness_publish_compares_outbox_only`, `harness_fixture_labels_core_regression_edge_subset`, and `harness_reports_bundle_coverage_1_and_5`.

Regression classifier tests: `harness_classifies_provider_drift`, `harness_classifies_prompt_change_fail_soft`, `harness_classifies_input_change`, `harness_classifies_canonicalization_bug_hard_fail`, and `harness_classifies_logic_change_fail_soft`.

Governance tests: `eval_fixture_governance_rejects_non_example_email`, `eval_fixture_governance_rejects_phone_like_tokens`, `eval_fixture_governance_rejects_identity_map_in_tree`, `capture_fixture_redacts_entity_names`, `capture_fixture_redacts_free_text_ner_tokens`, and `revoked_source_fixture_requires_regeneration_or_removal`.

Wave merge-gate artifact: `cargo test --test harness`, `cargo test --test eval_fixture_governance`, `scripts/check_eval_fixture_anonymization.sh`, and a `target/eval/harness-report.json` CI artifact showing Bundle 1 and Bundle 5 coverage, tracked/non-blocking bundles 2-4/6-8, fixture counts by ability/category/label, runtime, and regression-class counts. Suite S contribution: PII/no-network/revoked-source evidence. Suite P contribution: parallel run timing under 60s. Suite E contribution: Bundle 1 cross-entity/same-domain coverage and Bundle 5 correction-resurrection coverage.

## 10. Open questions

1. The W4 prompt says the harness must work with `mode = Replay`, but current code has `ExecutionMode::Evaluate` plus `ReplayProvider`. Confirm W4-B should use `Evaluate` mode and provider kind `Other("replay")`, not add an `ExecutionMode::Replay`.
2. The wave doc references an "Adversarial Edge Case Catalogue", but the checked-in `.docs/plans/v1.4.0-waves.md` only exposes bundle gate lines (`:175-183`) and bundle 1/5 target lines (`:538-569`). Where is the authoritative definition of bundles 2-4 and 6-8?
3. Confirm the fixture-format conflict: Linear/ADR require flat files; the W4 prompt asks to record `inputs/` + `expected_output/` + `metadata.json`. This plan keeps flat required files and adds `metadata.json`; reviewers should approve or amend before coding.
4. W4-C owns `EvalAbilityBridge`, but DOS-216's ticket says `EvalAbilityBridge` constructs `ServiceContext::new_evaluate`. Should W4-B create only a private `FixtureRunner` adapter and consume W4-C's bridge when present?
5. W3-A sample ability availability is load-bearing for DOS-216 acceptance. If W3-A does not ship sample Read/Transform abilities, can W4-B add test-only sample abilities inside the harness test crate, or does W3-A need a small follow-up seam?
6. Current CI triggers PRs only against `trunk` (`.github/workflows/test.yml:9-11`), while this wave targets `dev`. Should DOS-216 update the workflow branch list as part of CI integration, or is that owned by release engineering outside W4-B?
