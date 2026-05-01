# Implementation Plan: DOS-283

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-283 turns mock data into an executable fixture product for v1.4.0 validation, not a demo workspace. Load-bearing ticket lines: "Mock data is too often happy-path demo material." "Create a seeded v1.4.0 mock workspace that powers unit fixtures, ability evals, validation tests, and manual QA for the Golden Daily Loop." Acceptance requires "both positive and negative canonicalization cases", "stale vs current source timestamp cases", "at least one user correction/tombstone", "project detail parity cases", and documentation mapping "every seeded edge case to a test or validation assertion."

All 2026-04-24 amendments apply, narrowed to this W6-A slice. The hostile-workspace comment requires "two accounts sharing the same domain but different business units", "a prior user correction removing a wrong link/claim, which must survive refresh", and data that makes "cross-account bleed easy to reproduce and impossible to ignore." The catalogue comment says the workspace must seed eight bundles and that "Mock data that only looks realistic is insufficient." The feedback addendum requires claims across daily briefing, meeting briefing, account detail, and project detail, including a "recent but wrong-account claim" and "expected repaired display states." The plan-hardening amendment is the W6-A contract: "mock data is not just seeded demo richness; it is executable fixture input with expected assertions" and the minimum first bundles are "bundle 1: cross-entity / same-domain ambiguity" and "bundle 5: Golden Daily Loop correction resurrection."

The wave doc scopes W6-A to bundle files: `src-tauri/tests/fixtures/bundles/`, README row/source mappings, W3 schema, W4 harness, and done-when "bundles loadable via `cargo test`; manual QA load works in dev workspace" (`.docs/plans/v1.4.0-waves.md:606-612`). Suite E makes bundles 1+5 mandatory through W6 and blocking at ship (`.docs/plans/v1.4.0-waves.md:175-183`). This plan does not create other catalogue bundles.

## 2. Approach

Create a fixture corpus under W6-A-owned paths:

- `src-tauri/tests/fixtures/bundles/bundle_1/state.sql`
- `src-tauri/tests/fixtures/bundles/bundle_1/metadata.json`
- `src-tauri/tests/fixtures/bundles/bundle_1/expected_claims.json`
- `src-tauri/tests/fixtures/bundles/bundle_1/expected_render_policy.json`
- `src-tauri/tests/fixtures/bundles/bundle_1/expected_provenance.json`
- `src-tauri/tests/fixtures/bundles/bundle_1/external_replay.json`
- The same six files under `bundle_5/`
- `src-tauri/tests/fixtures/bundles/README.md`
- `src-tauri/tests/dos283_bundle_fixtures_test.rs`
- `src-tauri/tests/fixtures/bundles/load_dev_workspace.sh`

Use pure SQL `state.sql` files plus JSON expectation artifacts. The SQL is the only state source; JSON files describe expected assertions for W4-B/W6-C rather than driving inserts. This follows the DOS-311 fixture loader shape: open in-memory SQLite, read SQL from disk, `execute_batch` (`src-tauri/tests/dos311_reconcile_test.rs:26-43`), but unlike DOS-311's test-only `schema.sql` scaffold (`src-tauri/tests/dos311_fixtures/schema.sql:1-10`), DOS-283 loads against the production migrated schema so the same `state.sql` can be used for manual QA.

Test loader algorithm: open in-memory DB, run `migrations::run_migrations`, assert the W3-C tables/columns exist, then apply `bundle_1/state.sql` and `bundle_5/state.sql` inside a transaction. Current migration registration ends at version 125 with DOS-311 (`src-tauri/src/migrations.rs:572-590`), and the runner records migration versions only after successful `execute_batch` (`src-tauri/src/migrations.rs:1028-1085`), so W6-A must not include local DDL for `intelligence_claims`; it waits for W3-C.

Manual load algorithm: `load_dev_workspace.sh --bundle bundle_1 --db <dev.sqlite>` refuses to run unless `DAILYOS_ALLOW_DOS283_FIXTURES=1`, verifies the target has W3-C schema, creates a temp backup path for local dev only, and applies the same `state.sql` with `sqlite3` or the existing DB-open path. IDs are deterministic and namespaced (`dos283-b1-*`, `dos283-b5-*`); claim rows use `INSERT OR IGNORE` so reruns are idempotent and never `DELETE FROM intelligence_claims`.

Bundle 1 seeds the golden entity-context adversary: parent account, two child/same-domain accounts, a cross-account renewal meeting, same-first-name stakeholders, linked project rows, email/transcript/Glean evidence, wrong-subject tombstones, six paraphrases of one account claim, two related-but-distinct claims, and trust-band diversity. It uses existing operational tables for accounts (`src-tauri/src/migrations/001_baseline.sql:30-45`), people (`:141-159`), account domains (`src-tauri/src/migrations/002_internal_teams.sql:6-11`, provenance column at `src-tauri/src/migrations/118_account_domains_source.sql:10-18`), meetings/transcripts (`src-tauri/src/migrations/055_schema_decomposition.sql:16-51`), meeting links (`:100-123`, confidence at `src-tauri/src/migrations/095_meeting_entities_confidence.sql:17-21`), emails (`src-tauri/src/migrations/027_email_threads.sql:3-15`, `src-tauri/src/migrations/034_emails.sql:4-37`), and linked-entity audit rows (`src-tauri/src/migrations/110_linked_entities_raw.sql:21-51`, `src-tauri/src/migrations/112_entity_linking_evaluations.sql:19-39`).

Bundle 5 seeds correction resurrection: a `WrongSubject` per-subject tombstone, a user-edited superseding claim, duplicate/paraphrase claims collapsed through corroborations, a stale legacy projection row that must remain suppressed, a newer-evidence row that may reappear, and an expired/dormant claim that must not resurrect during meeting-prep refresh. It explicitly mirrors the DOS-311 ghost-resurrection contract: stale projection is `(dedup_key OR item_hash)` match with `sourced_at <= dismissed_at`, while newer evidence is allowed (`scripts/reconcile_ghost_resurrection.sql:7-16`, `:31-85`).

End-state alignment: W6-A gives W5-A exact bundle-1 parity inputs, W5-B correction-resurrection inputs, W6-B deterministic bleed-detection inputs, and W6-C a release-gate fixture product. It forecloses demo-only mock data, schema-less fixture builders, and fixtures that cannot be replayed in a real dev workspace.

## 3. Key decisions

Seed format: pick SQL-first with JSON expectation sidecars, not a Rust builder or TOML manifest. ADR-0110 requires fixture `state.sql` loaded into in-memory SQLite (`.docs/decisions/0110-evaluation-harness-for-abilities.md:20-39`, `:93-105`), and DOS-311 proves current tests already use SQL batch loading (`src-tauri/tests/dos311_reconcile_test.rs:26-43`). SQL is also the only format that manual QA can apply to a dev workspace without compiling a custom generator.

Bundle README format: use one Markdown table with columns `(bundle, row_id, table, subject_ref, claim_type, invariant, expected_state, suites, consumer_tests)`. This matches W4-B's metadata labels for `bundle`, `scenario_id`, `invariant`, `expected_render_policy`, and `surfaces_exercised` (`.docs/plans/wave-W4/DOS-216-plan.md:51-61`) while staying reviewable in a code diff.

Bundle 1 invariants:

- Same-domain subsidiary: two child accounts share `subsidiary.com`; only the entity with user-confirmed primary meeting link may surface the renewal risk. Current `meeting_entities` supports confidence and primary markers (`src-tauri/src/migrations/095_meeting_entities_confidence.sql:1-21`).
- Parent/child account: parent-level portfolio claim is allowed only when `SubjectRef::Multi` or explicit parent context is expected; child-only claims remain child-scoped.
- Cross-account meeting attendee: a renewal sync includes attendees from `parent.com`, `subsidiary.com`, and `partner.example.com`; W6-B can assert foreign-domain evidence lowers or blocks subject fit.
- Wrong-subject suppression: a claim first attached to parent is tombstoned for parent and optionally proposed to child, following ADR-0123 `WrongSubject { corrected_to }` semantics (`.docs/decisions/0123-typed-claim-feedback-semantics.md:45-54`, `:146-153`).
- Trust-band diversity: rows cover user-confirmed, fresh transcript/email corroborated, stale CRM, unknown timestamp, and revoked/opaque Glean source. ADR-0114 keeps five canonical trust factors (`.docs/decisions/0114-scoring-unification.md:290-316`); W4-A maps them to `TrustBand` (`.docs/plans/wave-W4/DOS-5-plan.md:24-31`, `:74-79`).

Bundle 5 invariants:

- WrongSubject tombstone survives refresh: tombstone PRE-GATE must run before dedup/commit, per ADR-0113 (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:114-122`).
- User-edited claim override: user-authored claim is active/current; AI paraphrases become corroborations or dormant alternatives, not replacements.
- Duplicate/paraphrase dedup: six same-meaning claims share a canonical `dedup_key` or map through `claim_corroborations`, while two related but distinct claims intentionally have separate keys. ADR-0113 defines dedup as hash over entity, claim type, field path, and normalized text (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:187-197`).
- Expired claim not resurrected: an old claim with expired/dormant lifecycle stays hidden unless new evidence has `source_asof` after the tombstone/expiry; ADR-0105 makes `source_asof` must-populate-when-knowable and conservative on unknown (`.docs/decisions/0105-provenance-as-first-class-output.md:391-437`, `:450-466`).

Schema dependency: W6-A seeds only tables created by W3-C/W3-D/W3-E/W3-H: `intelligence_claims`, `claim_corroborations`, `claim_contradictions`, `claim_feedback`, `agent_trust_ledger`, `claim_repair_job`, `claim_projection_status`, and columns `claim_state`, `surfacing_state`, `dedup_key`, `item_hash`, `source_asof`, `thread_id`, `temporal_scope`, `sensitivity`. DOS-7 pins the base table and child table intent (`.docs/plans/wave-W3/DOS-7-plan.md:19-27`, `:35-47`), DOS-301 pins projection status (`.docs/plans/wave-W3/DOS-301-plan.md:21-40`), and DOS-300 pins temporal/sensitivity defaults (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:48-87`).

Source/provenance shapes: use ADR-0107 `DataSource` and `SourceIdentifier` taxonomy (`.docs/decisions/0107-source-taxonomy-alignment.md:24-60`, `:70-131`), including `LegacyUnattributed` only for backfill-like legacy rows (`:223-241`). Expected provenance sidecars assert field attribution and `SourceTimestampUnknown` warnings, because ADR-0105 requires field-level attribution for every output field (`.docs/decisions/0105-provenance-as-first-class-output.md:199-241`) and W3-B adds subject-fit hard failures (`.docs/plans/wave-W3/DOS-211-plan.md:57-61`).

## 4. Security

The main attack surface is checked-in fixture data. All names, domains, emails, thread IDs, source IDs, and transcripts are synthetic and use generic examples only, per `CLAUDE.md:16-18`. ADR-0110 fixture governance requires placeholder identities, `example.com` emails, named-entity redaction, and source IDs marked anonymized (`.docs/decisions/0110-evaluation-harness-for-abilities.md:132-153`). The governance test must reject non-`example.com` emails, phone-like strings, real customer names/domains, and any committed identity map.

The second risk is cross-entity exposure becoming normalized by fixtures. Bundle 1 intentionally contains wrong-subject evidence, but every expected artifact marks whether the row is blocked, ambiguous, dormant, or visible. W3-B's `SubjectAttribution` and W6-B's validator consume those expectations; failures must report row IDs and invariant IDs, not claim text, account names, source excerpts, prompt text, or opaque Glean payloads. ADR-0108 requires actor-filtered provenance rendering and forbids internal IDs/prompt hashes to agents (`.docs/decisions/0108-provenance-rendering-and-privacy.md:31-72`).

Manual load is opt-in and dev-only. The script refuses paths that look like production unless the caller passes an explicit DB path and env guard; it does not open OAuth clients, Glean, Gmail, Slack, Linear, or filesystem source roots. Revoked-source rows are synthetic lifecycle cases; ADR-0098 says revoked source data is purged or masked while user-entered data persists (`.docs/decisions/0098-data-governance-source-aware-lifecycle.md:57-75`).

## 5. Performance

No production hot path changes. Cargo-test load cost is two small SQL files plus JSON metadata parsing. ADR-0110 budgets the full hermetic ability suite at 30-60 seconds and per-fixture parallel isolation (`.docs/decisions/0110-evaluation-harness-for-abilities.md:91-105`); W6-A's target is under 2 seconds for `dos283_bundle_fixtures_test` after migrations.

SQL inserts should use existing indexes rather than force new ones: account/domain lookup already has `idx_account_domains_domain` (`src-tauri/src/migrations/002_internal_teams.sql:11`) and source index (`src-tauri/src/migrations/118_account_domains_source.sql:17-18`), meetings use start/calendar indexes (`src-tauri/src/migrations/055_schema_decomposition.sql:87-92`), emails use thread/entity indexes (`src-tauri/src/migrations/034_emails.sql:31-37`), and DOS-7 owns claim read/suppression indexes (`.docs/plans/wave-W3/DOS-7-plan.md:59-65`). Manual load wraps a transaction to avoid slow per-row autocommit.

Fixture volume stays intentionally small: enough rows to exercise each invariant, not a synthetic 100K-claim benchmark. Suite P should measure release-gate runtime over bundles 1+5, not treat W6-A as a backfill performance test.

## 6. Coding standards

Services-only mutations: production app code remains untouched. Test/manual seed SQL writes directly only in fixture setup, analogous to DOS-311 test scaffolding; no command handler or ability module gets a new direct write path. Future implementation must not edit W2 frozen seams `src-tauri/src/services/context.rs` or `src-tauri/src/intelligence/provider.rs`; W2 provider replay is consumed as-is (`src-tauri/src/intelligence/provider.rs:10-22`, `:227-295`).

Intelligence Loop 5-question check (`CLAUDE.md:7-14`): bundle rows are test data, but they intentionally exercise signal/health/context/briefing/feedback paths. The fixtures do not add a product table or signal type; they verify that claim invalidation, trust bands, prep/entity context reads, render policies, and feedback semantics already wired by W3-W5 behave correctly.

No direct `Utc::now()` or `thread_rng()` in any new Rust test helper; fixture clocks are fixed literals. `ServiceContext` already provides deterministic clock/RNG seams for Evaluate mode (`src-tauri/src/services/context.rs:35-80`, `:109-180`, `:368-422`) and ADR-0104 requires fixture-supplied clock/RNG/provider in Evaluate (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:256-265`).

Clippy budget is zero warnings. SQL and JSON fixtures must remain ASCII, deterministic, idempotent, and namespace-prefixed. Do not use real customer data; do not copy the existing `glean_financial_company_response.json` naming style as customer-like fixture content even though it shows item-source shape (`src-tauri/tests/fixtures/glean_financial_company_response.json:1-47`).

## 7. Integration with parallel wave-mates

W3-C/DOS-7 is the hard dependency. Bundle SQL must use its final `intelligence_claims` schema and child tables. Coordinate column names before coding: `id` vs `claim_id`, `claim_state`, `surfacing_state`, `item_hash`, `dedup_key`, `source_asof`, `thread_id`, `temporal_scope`, `sensitivity`, and TEXT vs BLOB claim IDs are still open in DOS-7 (`.docs/plans/wave-W3/DOS-7-plan.md:111-118`).

W3-B/DOS-211 owns `Provenance`, `SourceAttribution`, `SubjectAttribution`, warnings, and field attribution; W6-A only writes expected JSON matching that shape (`.docs/plans/wave-W3/DOS-211-plan.md:19-45`, `:57-67`). W3-D/DOS-301 owns projection status/repair; bundle 5 asserts claims/projections do not resurrect stale content (`.docs/plans/wave-W3/DOS-301-plan.md:106-132`). W3-H/DOS-300 owns final claim-type strings and default temporal/sensitivity semantics (`.docs/plans/wave-W3/DOS-300-plan.md:21-29`, `:69-77`).

W4-B/DOS-216 consumes the bundle metadata labels. `metadata.json` must expose `bundle_1` / `bundle_5`, `@edge`, `@golden-daily-loop`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, and anonymization certificate fields so harness reports can show bundle coverage (`.docs/plans/wave-W4/DOS-216-plan.md:16-31`, `:53-61`, `:107-115`).

W5-A/DOS-218 consumes bundle 1 for exact Read parity and wrong-subject entity context tests (`.docs/plans/wave-W5/DOS-218-plan.md:87-95`). W5-B/DOS-219 consumes bundle 5 for meeting-prep parity, user corrections, duplicate claims, stale Glean, and correction resurrection (`.docs/plans/wave-W5/DOS-219-plan.md:91-99`). W6-B/DOS-288 consumes bundle 1 for deterministic `assert_bleed_blocked` tests; W6-C/DOS-281 consumes both bundles for the release gate (`.docs/plans/v1.4.0-waves.md:614-626`).

No migration numbering coordination belongs to W6-A. If implementation discovers missing schema, stop and update §10 rather than adding a migration in this PR.

## 8. Failure modes + rollback

If W3-C schema is absent or changed, the test helper fails before applying fixture SQL with a missing-schema message. It must not create compatibility tables, because that would hide schema drift and break manual QA load.

If SQL load fails midway, the transaction rolls back and no fixture rows remain in the in-memory DB or dev workspace. Manual load emits the bundle, table, statement class, and row id; it does not print claim text or source excerpts. Rerun is idempotent because deterministic IDs use `INSERT OR IGNORE` and no claim-table deletes.

If expectation JSON drifts from seeded SQL, `dos283_bundle_expected_claims_match_state_sql` fails and points to `(bundle, row_id, claim_type)`. If a downstream ability changes legitimate render policy, W4-B/W6-C rebaseline the expectation sidecars with reviewer approval, not by editing SQL casually.

Rollback is remove or skip the bundle fixture from the W4 harness manifest while preserving the failure report. No production migration or irreversible data rewrite exists. W1-B universal write fence is honored: W6-A does not write `intelligence.json`; any later legacy file projection remains DOS-301's responsibility through `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:223-238`). Schema epoch/drain primitives stay untouched (`src-tauri/src/intelligence/write_fence.rs:67-155`).

## 9. Test evidence to be produced

Cargo tests in `src-tauri/tests/dos283_bundle_fixtures_test.rs`:

- `dos283_bundle1_state_sql_loads_after_w3_schema`
- `dos283_bundle5_state_sql_loads_after_w3_schema`
- `dos283_bundle_metadata_labels_bundle_1_and_5`
- `dos283_readme_maps_every_seeded_row_to_invariant`
- `dos283_bundle1_same_domain_accounts_have_distinct_subject_refs`
- `dos283_bundle1_wrong_subject_tombstone_targets_only_asserted_subject`
- `dos283_bundle1_trust_band_diversity_present`
- `dos283_bundle1_paraphrase_dedup_and_distinct_claims_both_present`
- `dos283_bundle5_wrong_subject_tombstone_survives_refresh_fixture`
- `dos283_bundle5_user_edited_claim_overrides_ai_claim`
- `dos283_bundle5_duplicate_paraphrases_collapse_to_corroborations`
- `dos283_bundle5_expired_claim_is_not_active_or_renderable`
- `dos283_manual_load_script_requires_explicit_dev_opt_in`
- `dos283_fixture_governance_rejects_non_example_email`

Harness/release-gate consumers should add or run: `harness_reports_bundle_coverage_1_and_5`, `get_entity_context_fixture_bundle1_same_domain_wrong_entity_suppressed`, `prepare_meeting_bundle5_wrong_subject_tombstone_no_resurrection`, `assert_bleed_blocked_bundle1_cross_account_meeting`, and `release_gate_loads_bundles_1_and_5`.

Wave merge-gate artifact: `cargo test --test dos283_bundle_fixtures_test`, `cargo test --test harness bundle_1 bundle_5`, `scripts/check_eval_fixture_anonymization.sh`, manual dev-load dry-run transcript, and `target/eval/harness-report.json` showing Bundle 1 and Bundle 5 coverage. Suite S contribution: no PII/secrets, revoked-source masking expectations, cross-entity exposure assertions. Suite P contribution: fixture load and release-gate runtime. Suite E contribution: mandatory bundle 1 bleed/ownership and bundle 5 correction-resurrection invariants.

## 10. Open questions

1. W3-C final schema: confirm exact claim table/child-table names, `id` vs `claim_id`, TEXT vs BLOB IDs, and allowed `claim_state`/`surfacing_state` values before writing SQL.
2. DOS-300 final claim-type strings: confirm canonical names for account current state, renewal risk/date, stakeholder role/engagement, project status/detail, meeting topic, open loop, and suggested outcome.
3. Project-account relationship: current `projects` table has no `account_id` column even though entity-graph comments reference one (`src-tauri/src/migrations/113_entity_graph_version.sql:9-15`, `:63-65`), and project links are otherwise represented through actions, meetings, entity members, or future claims. Which table/claim shape should bundle 1 use for "project linked to one/multiple accounts"?
4. W4-B layout conflict: should global bundle dirs be consumed directly by the harness, or should W6-A also materialize ability-specific fixture wrappers under `src-tauri/tests/abilities/{ability}/fixtures/`?
5. Expected provenance shape: DOS-211 still has open questions on exact `SubjectAttribution` fields and feedback contributions (`.docs/plans/wave-W3/DOS-211-plan.md:115-121`). Confirm before locking `expected_provenance.json`.
6. Manual load script scope: should it support rollback/removal of namespace-prefixed fixture rows in dev workspaces, or is load-only safer for the first W6-A PR?
