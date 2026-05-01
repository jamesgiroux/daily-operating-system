# Implementation Plan: DOS-281

## Revision history
- v1 (2026-05-01) -- initial L0 draft.

## 1. Contract restated
DOS-281 is the v1.4.0 release blocker that turns the Golden Daily Loop into an executable ship criterion. The command must prove the seeded loop: bundle 1 establishes entity/context ownership; bundle 5 proves user corrections and tombstones survive repeated enrichment; W5 pilots run through the ability bridge; W6-B bleed detection is green; and one machine-readable + human-readable artifact records the result.

Load-bearing Linear lines:
- "Create and run a Golden Daily Loop validation suite as a release blocker for v1.4.0."
- "If v1.4.0 ships with green substrate tests but this loop is still wrong, stale, noisy, or untrustworthy, the release fails the user."
- "The gate passes only if:" followed by stable meeting identity, correct or visibly uncertain account/person/project links, source-grounded topics, material-claim attribution and timestamps, stale-claim demotion, correction/tombstone non-resurrection, duplicate-claim collapse, commitment dedupe, structured owners, surface agreement, usable provenance, and actionable failure states.
- Acceptance criteria include "Scripted validation path exists and can boot a seeded workspace", "Validation includes at least one user correction followed by refresh/enrichment and verifies the correction sticks", "Validation runs on seeded mock corpus in CI or a documented release-check command", and "Manual/recorded run against >=20 real-dev meetings is attached before release."

All 2026-04-24 amendments apply. The 12:44 tightening makes cross-entity/content bleed a hard invariant. The 12:52 catalogue amendment requires bundle coverage with seeded data, regression test, eval fixture, render behavior, and lint/activity-log expectations. The 13:08 addendum adds DOS-294/DOS-295 blockers and explicit feedback scenarios: outdated, wrong entity, wrong source, unverifiable, and partly right. The 19:30 plan-hardening amendment says "This issue should not remain only a checklist. It needs a runnable release command for the v1.4 subset, starting with bundle 1 and bundle 5, plus a separate manual evidence artifact for real-dev meetings / seven-day parallel runs" and separates hermetic CI from real-dev release evidence.

## 2. Approach
Pick a Rust binary plus pnpm alias: create `src-tauri/src/bin/release_gate.rs`, add `[[bin]] name = "release-gate" path = "src/bin/release_gate.rs"` beside current explicit bin declarations (`src-tauri/Cargo.toml:92-113`), and add `release-gate` to the existing script block (`package.json:14-22`). The command becomes `pnpm release-gate -- --mode hermetic` or `cargo run --manifest-path src-tauri/Cargo.toml --bin release-gate -- --mode hermetic`.

Do not implement as only `scripts/release-gate.sh`. Current binary patterns already return precise `ExitCode`s and operator summaries (`src-tauri/src/bin/reconcile_post_migration.rs:23-47`, `:110-147`; `src-tauri/src/bin/repair_entity_linking.rs:7-52`), while manual mode needs Rust-native read-only DB access through `ActionDb::open_readonly_at` (`src-tauri/src/db/core.rs:226-258`). A shell wrapper can remain the pnpm convenience layer, not the source of truth.

Core data structures in the binary:
- `GateConfig { mode, mandatory_bundles, tracked_bundles, output_dir, harness_report, db_path, manual_evidence, run_tests, git_sha }`.
- `GateEvidenceV1 { schema_version, run_id, mode, generated_at, git_sha, db_schema_version, suites, invariants, mandatory_bundles, tracked_bundles, manual, latency, summary_markdown }`.
- `SuiteResult { name, source, command_or_report, status, mandatory, duration_ms, failures }`.
- `InvariantResult { id, bundle, surface, status, evidence_ref, failure_summary }`.
- `ManualDogfoodEvidence { meeting_count, date_range, operator, redaction_level, seven_day_parallel_run_ref, attached_artifacts }`.

Hermetic algorithm:
1. Resolve bundle list. Defaults are blocking `bundle_1` and `bundle_5`; bundles 2-4 and 6-8 are parsed as tracked/non-blocking if present per Suite E (`.docs/plans/v1.4.0-waves.md:175-183`, `:640-642`).
2. Run or consume W4-B harness evidence. Preferred: invoke/parse `target/eval/harness-report.json` from DOS-216, whose report already covers bundle counts, fixture counts, runtime, and regression classes (`.docs/plans/wave-W4/DOS-216-plan.md:109-115`). If `--no-run-tests`, only parse an existing report.
3. Load W6-A bundles through the W6-A loader or SQL paths. The reference is DOS-311's in-memory fixture pattern: `Connection::open_in_memory`, `schema.sql`, and scenario SQL via `execute_batch` (`src-tauri/tests/dos311_reconcile_test.rs:26-43`).
4. Invoke W6-B bleed detection by stable test filter or library hook. Today the legacy guard is env-driven/shadow-capable (`src-tauri/src/intelligence/contamination.rs:256-305`) and called before persistence (`src-tauri/src/intel_queue.rs:2460-2552`); W6-B must replace this with subject-fit/coherence evidence, not keep the old env gate alive.
5. Invoke W5 pilots only through W4-C `EvalAbilityBridge` / registry path. Do not hardcode ability internals. W4-C's plan says bridges call `AbilityRegistry::invoke_by_name_json`, convert to `AbilityResponseJson`, and pass providers into `AbilityContext` rather than calling providers directly (`.docs/plans/wave-W4/DOS-217-plan.md:24-31`, `:52`).
6. Assert the release invariants as artifact-level predicates: bundle 1 `get_entity_context` parity/subject ownership, bundle 5 `prepare_meeting` correction-resurrection, W6-B bleed blocked, W4 harness bundle coverage, W5 p50/p99 no-regression, provenance/source timestamp coverage, and no PII fixture/governance failures.
7. Write `src-tauri/target/release-gate/evidence.json` and `evidence.md`. Print the markdown summary to stdout. Exit `0` only when all mandatory checks pass; exit `1` for mandatory validation failure; exit `2` for config, missing artifact, bundle load, DB open, or evidence write failure.

Manual dogfood mode is read-only and evidence-capture oriented: `pnpm release-gate -- --mode manual --db ~/.dailyos/dailyos-dev.db --manual-evidence path/to/manual.json`. It opens the dev DB read-only (`src-tauri/src/db/core.rs:233-258`), validates the submitted dogfood artifact shape, summarizes counts/statuses, and writes the same `evidence.json`/`evidence.md` with `mode = "manual"`. The actual real-workspace corrections, refreshes, and seven-day parallel run happen in the app before capture; the gate does not mutate the real workspace.

End-state alignment: DOS-281 becomes the coded merge signal for `.docs/plans/v1.4.0-waves.md:635-642`: L3/L4 evidence, Suite S/P/E final, bundle 1+5 mandatory pass, manual dogfood captured, and proof bundle written. This forecloses shipping v1.4.0 on substrate-only green tests.

## 3. Key decisions
Binary vs script: choose `release-gate` Rust binary with `pnpm release-gate` alias. Reason: it needs typed evidence parsing, read-only SQLite manual mode, structured exit codes, and reuse of Rust DTOs. The existing repo already uses explicit Rust utility bins (`src-tauri/Cargo.toml:101-113`).

Mode switch: `--mode hermetic` is default and uses in-memory SQLite + replay/eval evidence; `--mode manual` requires read-only DB access and manual evidence. ADR-0104 makes `Evaluate` replay/determinism structural (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:23-47`, `:256-263`), and the current code exposes `ExecutionMode::Evaluate` plus `ServiceContext::new_evaluate` (`src-tauri/src/services/context.rs:37-47`, `:368-390`).

Evidence artifact: version `release_gate_evidence_v1` contains suite names, pass/fail per invariant, ability latency p50/p99, bleed detection result, correction-resurrection result, timestamp, schema version, git SHA from `DAILYOS_GIT_SHA`/`GITHUB_SHA`/`option_env!` with `unknown` fallback, and a markdown summary. Do not shell out to `git`; this plan is consistent with the current no-git work rule.

Failure output: always print a human-readable summary and write partial evidence if possible. Reconcile and repair binaries already prefer operator summaries and distinct failure exit codes (`src-tauri/src/bin/reconcile_post_migration.rs:115-147`, `src-tauri/src/bin/repair_entity_linking.rs:37-52`); release gate follows that pattern.

Mandatory vs tracked: bundle 1 and bundle 5 failures block exit zero. Bundles 2-4 and 6-8 appear as `mandatory=false` suite/invariant rows and can fail without blocking unless the failure points to a substrate invariant that also affects bundles 1/5. This matches Suite E's pass rule (`.docs/plans/v1.4.0-waves.md:181-183`).

ADR-pinned shapes: ability calls must route through the registry and return `AbilityOutput<T>` with provenance exactly once (`.docs/decisions/0102-abilities-as-runtime-contract.md:26-34`, `:166-179`). Provenance requires source attribution and `source_asof` when knowable (`.docs/decisions/0105-provenance-as-first-class-output.md:165-173`, `:391-420`). The harness fixture shape is ADR-0110's flat files (`.docs/decisions/0110-evaluation-harness-for-abilities.md:18-35`), not a new release-gate-only fixture format.

## 4. Security
New attack surface is release evidence leaking customer data. The JSON/Markdown artifacts must store IDs, counts, invariant labels, source classes, hash prefixes, and redacted summaries only. No claim text, prompt text, completion text, transcript text, email bodies, customer names, or raw source excerpts. This matches DOS-216's provenance diff redaction (`.docs/plans/wave-W4/DOS-216-plan.md:65-69`) and CLAUDE's fixture/customer-data rule (`CLAUDE.md:16-18`).

Manual mode opens real/dev workspace DB read-only. It must reject write-intent flags, never call mutation services, never emit signals, never refresh abilities, and never trigger external clients. If a manual artifact includes real meeting titles or names, the binary redacts or refuses the field unless `redaction_level = "hash_only"`.

Hermetic mode must never fall through to live providers or external services. `ReplayProvider` returns `ReplayFixtureMissing` instead of live fallback (`src-tauri/src/intelligence/provider.rs:227-295`), and ADR-0104 requires service externals/provider replay in Evaluate (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:207-209`).

Auth/authz is inherited from W4-C bridge and W3 registry. The gate must not construct `AbilityContext` directly in a way that bypasses actor policy; ADR-0102 says surfaces do not bypass the registry (`.docs/decisions/0102-abilities-as-runtime-contract.md:252-258`, `:289-299`).

## 5. Performance
No product hot path is touched. Hot path is the warm release command. Budget: artifact aggregation under 15s after prerequisite harness/tests exist; if the gate invokes the golden subset itself, bundle 1+5 harness execution should stay inside DOS-216's 30-60s hermetic target (`.docs/plans/wave-W4/DOS-216-plan.md:73-77`). The full W6 Suite P still ratifies release-gate latency (`.docs/plans/v1.4.0-waves.md:163-171`).

DB behavior: hermetic runs use one in-memory SQLite connection per bundle/test scope, avoiding live writer contention. Manual mode uses read-only SQLite flags and `busy_timeout` (`src-tauri/src/db/core.rs:245-255`), so it cannot block the Tauri writer lane except for normal read sharing.

Evidence parsing is O(report size + invariant count). Provenance payloads are summarized, not embedded, so `evidence.json` remains small enough for CI and release proof bundles. Ability p50/p99 values are consumed from W5/W4 reports; the release gate does not benchmark abilities independently.

## 6. Coding standards
Services-only mutations: the binary writes only `target/release-gate/*` artifacts. Any hermetic correction/assertion setup that requires writes must use fixture DBs and service APIs; manual mode is read-only. It must not edit `services/context.rs` or `intelligence/provider.rs`, both frozen seams.

Intelligence Loop 5-question check (`CLAUDE.md:7-14`): evidence artifacts are release/CI metadata, not a product data surface; they emit no signals, do not feed health scoring, do not enter `build_intelligence_context()` or `gather_account_context()`, do not trigger briefing callouts, and do not feed Bayesian source weights. If an implementation stores release evidence in app DB, that is a new data surface and must return to L0.

No direct `Utc::now()`/`thread_rng()` in services or abilities. The binary may stamp artifact `generated_at`, but all ability/harness assertions use injected fixture clock/RNG per current `Clock`, `FixedClock`, and `SeedableRng` seams (`src-tauri/src/services/context.rs:64-120`).

No customer data in fixtures or committed example artifacts. Tests use `example.com`, `parent.com`, `subsidiary.com`, and synthetic meeting IDs per `CLAUDE.md:18`. Clippy/test bar stays `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` (`CLAUDE.md:20-24`).

## 7. Integration with parallel wave-mates
W6-A / DOS-283 supplies bundle 1 + 5 seed data. The W6-A prompt says bundle 1 is golden entity context and bundle 5 is correction resurrection, loaded under `src-tauri/tests/fixtures/bundles/bundle_1/` and `bundle_5/` (`.docs/plans/wave-W6/_prompts/DOS-283.md:15-22`). There is no checked-in `.docs/plans/wave-W6/DOS-283-plan.md` in this workspace; final loader API remains a coordination dependency.

W6-B / DOS-288 supplies bleed detection evidence and test names. Its prompt says the gate must consume tests proving W3-B subject-fit plus W4-A cross-entity-coherence make bleed structurally impossible, and that W6-C needs invokable W6-B test names (`.docs/plans/wave-W6/_prompts/DOS-288.md:16-20`, `:50-53`). There is no checked-in `.docs/plans/wave-W6/DOS-288-plan.md` yet; use §10 to block coding until names/API settle.

W5-A / DOS-218 supplies `get_entity_context` parity on bundle 1. Its plan promises bundle-1 fixtures, exact Read equality, provenance diff, and p95 overhead budget (`.docs/plans/wave-W5/DOS-218-plan.md:89-95`).

W5-B / DOS-219 supplies `prepare_meeting` parity/correction resurrection on bundle 5. Its plan names `prepare_meeting_bundle5_wrong_subject_tombstone_no_resurrection`, user-edited override, duplicate-claim render, double-refresh idempotence, and W5 Suite P p50/p99 evidence (`.docs/plans/wave-W5/DOS-219-plan.md:93-99`).

W4-B / DOS-216 owns `target/eval/harness-report.json` and bundle coverage metadata (`.docs/plans/wave-W4/DOS-216-plan.md:49-61`, `:109-115`). W4-C / DOS-217 owns `EvalAbilityBridge` and all registry/surface invocation. W3-C/DOS-7 and W3-E/DOS-294 own `services::claims`, `claim_feedback`, typed feedback semantics, and tombstone pre-gates (`.docs/plans/wave-W3/DOS-7-plan.md:19-31`; `.docs/plans/wave-W3/DOS-294-plan.md:37-53`).

Potential shared-file collisions: W6-C edits `src-tauri/Cargo.toml` and `package.json`. W6-B may edit CI; W6-C should not edit `.github/workflows/test.yml` unless reviewers explicitly assign release-gate CI wiring. Current test workflow only runs Rust tests generically at `.github/workflows/test.yml:74-79`.

## 8. Failure modes + rollback
Bundle load failure, harness report missing, W6-B test missing, replay miss, provenance diff failure, correction-resurrection failure, or manual evidence schema failure all write partial evidence and exit non-zero. Missing upstream artifacts are config/infrastructure failures (`2`), not product invariant failures (`1`).

Tracked bundle failures are recorded and printed but do not block exit zero unless the failure is classified as a substrate bug affecting bundles 1/5. Mandatory bundle failures block `v1.4.0` tag by making the release gate non-zero.

Manual mode failure does not modify the user's DB because it opens read-only and has no mutation path. If the DB cannot be opened read-only, the operator still has the app-run manual artifact; the binary reports the DB-open failure separately.

Rollback is migration-free: remove the `release-gate` bin entry, package script, and tests; no schema, projections, claims, signals, or source data are touched. W1-B universal write fence is honored because the binary never writes `intelligence.json`, claim rows, legacy projections, or external systems; all durable output is under `src-tauri/target/release-gate/`.

## 9. Test evidence to be produced
Unit tests: `release_gate_cli_defaults_to_bundle1_bundle5`, `release_gate_cli_rejects_manual_without_evidence`, `release_gate_evidence_schema_v1_roundtrips`, `release_gate_markdown_summary_redacts_raw_claim_text`, `release_gate_tracked_bundle_failure_non_blocking`, `release_gate_mandatory_failure_exit_one`, `release_gate_infra_failure_exit_two`, and `release_gate_manual_db_uses_readonly_open`.

Integration tests: `release_gate_parses_harness_report_bundle_coverage`, `release_gate_requires_bleed_suite_green`, `release_gate_requires_get_entity_context_bundle1_parity`, `release_gate_requires_prepare_meeting_bundle5_no_resurrection`, `release_gate_records_latency_p50_p99_from_suite_report`, `release_gate_writes_evidence_json_and_md`, and `release_gate_manual_evidence_twenty_meetings_and_seven_day_parallel_run_required`.

Wave merge-gate artifact: `pnpm release-gate -- --mode hermetic --bundle bundle_1 --bundle bundle_5`, producing `src-tauri/target/release-gate/evidence.json` and `evidence.md`; plus `pnpm release-gate -- --mode manual --manual-evidence <redacted-json>` for the dogfood run. This PR contributes Suite S aggregation (bleed blocked, no PII in fixtures/evidence, secrets not read), Suite P aggregation (release-gate duration + W5 p50/p99), and Suite E final (bundle 1+5 mandatory, tracked bundles filed).

## 10. Open questions
1. W6-A final loader API is absent in this workspace. Should W6-C call a Rust helper, read `state.sql` paths directly, or invoke `cargo test --test bundle_loader -- --bundle ...`?
2. W6-B final test names/API are absent. Confirm whether the release gate invokes a subprocess test filter or calls a Rust function exported from non-`#[cfg(test)]` code.
3. DOS-281 names Daily Briefing / next Daily Briefing, but the W5 wave only migrates `get_entity_context` and `prepare_meeting`; DOS-220 is related but not a W5 pilot. Should v1.4.0 block on a `get_daily_readiness` ability fixture, a legacy daily-readiness assertion, or manual dogfood evidence only?
4. Confirm manual evidence storage: is a redacted JSON+Markdown artifact under `target/release-gate/` enough for the tag proof bundle, or should implementation add a committed `.docs/release-gate/manual-dogfood.md` template?
5. Confirm exact Suite P budget for the full release-gate command. This plan proposes <=15s aggregation and <=60s golden harness subset warm, but `.docs/plans/v1.4.0-waves.md:171` leaves release-gate latency budget provisional.
6. Confirm git SHA source for local manual runs: environment-only avoids running `git`, but local artifacts may show `git_sha = "unknown"` unless release tooling exports `DAILYOS_GIT_SHA`.
