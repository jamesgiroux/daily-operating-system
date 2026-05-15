# DOS-282 W6-A L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-A.
- **Linear issue:** DOS-282 - "Testing: v1.4.0 edge-case regression suite for unit, integration, validation, and eval fixtures" (DOS-282 content supplied verbatim in the authoring prompt for this packet).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-A-L0-packet.md`.
- **W6-A assignment:** the wave plan names W6-A as "DOS-282 v1.4.0 edge-case regression suite" and assigns it `tests/edge_cases/` extension covering unit + integration + validation layers, with done-condition "all 5 validation axes from DOS-289-293 have at least 1 regression each." Source: `.docs/plans/v1.4.1-waves.md:621-625`.
- **W6 merge gate:** W6 requires L0 plan approvals, L2 approvals, L3 Suite E final with bundles 1-13 and 14-18 mandatory green, L4 `/qa`, L5 drift check, retro, and proof bundle. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Reviewer contract:** W6 L0 requires `qa-expert` for all six W6 agents, with `security-auditor` only for DOS-292 (W6-E). W6-A's L0 panel is `qa-expert`-only. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; current corpus is `bundle-1` through `bundle-13`; harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Runtime contract:** synthesized user-facing and agent-facing context must go through abilities; surfaces invoke abilities through the registry/typed imports; every ability output carries provenance once; Transform outputs cannot authorize mutation on their own. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`, `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`.
- **Meta role:** W6-A is the **meta agent** of Wave 6. It does not own any of bundles 14-18 (those are owned by W6-B/C/D/E/F per the bundle-numbering contract); it owns the `tests/edge_cases/` extension and the per-ability fixture-path coverage (happy/empty/stale/revoked/contradiction) for the migrated abilities.

## 2. Load-Bearing User Outcome

DOS-282 frames the systemic failure W6-A must prevent:

> "DailyOS has repeatedly shipped user-facing intelligence bugs because implementation got ahead of scenario planning. v1.4.0 is a large runtime refactor; without an explicit regression suite, it can make the system cleaner internally while preserving or reintroducing daily-use failures."

The load-bearing outcome for W6-A is therefore not "more tests exist." It is: **the v1.4.0 + v1.4.1 substrate is proven against the user-facing intelligence loop, not just the runtime contract.** A migration that compiles, that passes the substrate fixture bundles, and that still re-introduces a known daily-use bug must fail W6-A.

Required behavior from DOS-282 spans three test layers and one fixture layer:

> "Expand unit tests, integration tests, validation tests, and ability eval fixtures so v1.4.0 proves the user-facing intelligence loop, not just the substrate."

This outcome consumes existing Intelligence Loop substrate, not display-only assertions:

- **Claim model:** unit-test coverage names `derive_commitment_id` determinism, claim canonicalization, tombstone pre-gate, source taxonomy parsing, and stale claim handling — every assertion lives on the claim substrate, not on rendered strings. Sources: DOS-282 Required-unit-tests block (in §2 quote below).
- **Provenance and trust:** unit-test coverage names trust factor math, provenance builder field-attribution invariants, and ability category enforcement (`Read`/`Transform` cannot mutate). Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `.docs/decisions/0105-provenance-as-first-class-output.md:206-241`.
- **Signals and invalidation:** integration-test coverage names "user correction → enrichment rerun → correction still applies," "revoked source masking through provenance render," and "Activity Log populated from invocation/claim/signal events" — these are signal-propagation tests, not display-layer tests.
- **Runtime and surfaces:** integration coverage names "calendar → linked entities → meeting briefing," "account vs meeting parity," "project vs meeting parity," and "old-path vs new-ability parity until cutover" — all exercise the abilities runtime and surface composition.
- **Feedback loop:** "user correction → enrichment rerun → correction still applies" and the per-ability `contradiction` fixture path test the feedback loop.

DOS-282's required ability-fixture matrix (the **5-path contract**):

> "Each migrated ability has fixtures for happy path, empty path, stale path, revoked-source path, and contradiction path where applicable."

Named abilities (DOS-282 ticket body):

- `get_entity_context` — rich Glean account; local-only account; revoked Glean source; ambiguous entity identity; duplicate claims; stale high-confidence claim.
- `prepare_meeting` — first meeting with new person; recurring 1:1; multi-account meeting; stale Glean; revoked source; user correction; recent contradiction; project-led meeting.
- `get_daily_readiness` — typical day; no meetings; risk shift; stale Glean; duplicate open loops; post-meeting commitments.
- `detect_risk_shift` — degrading account; stable account; active escalation; revoked source; old champion vs recent silence.
- `list_open_loops` / `extract_commitments` — many loops; empty result; transcript-only commitment; email-only commitment; transcript + email corroboration; ambiguous owner; no owner; repeated enrichment must not create duplicate row.

## 3. Pre-Work

- **Read W6 source of truth.** W6 has six agents fanned out across 5 validation axes; W6-A is the meta agent and W6-B/C/D/E/F own per-axis bundles 14-18. Source: `.docs/plans/v1.4.1-waves.md:617-651`.
- **Read W6 merge gate.** L3 Suite E final requires bundles 1-13 plus bundles 14-18 mandatory pass, all 18 green, no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Acknowledged Amendment 1.** Amendment 1 recategorizes W3 stage-3b as `instrumentation-complete, data-sufficiency-pending`, relaxes W6's hard precondition to stage-3b instrumentation-complete, and says W6 starts against the partial baseline. W6-A scope is unaffected; the stage-3b residual does not become W6-A's work. Sources: `.docs/plans/v1.4.1-waves-amendments.md:15-23`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Read sibling assignments.** W6-B = DOS-289 stale-current = bundle 14; W6-C = DOS-290 cross-surface = bundle 15; W6-D = DOS-291 ambiguous identity = bundle 16; W6-E = DOS-292 source lifecycle = bundle 17 (security-auditor lane); W6-F = DOS-293 sync/refresh = bundle 18. Source: `.docs/plans/v1.4.1-waves.md:627-651`.
- **Read fixture catalogue.** Current bundles 1-13 are documented at `src-tauri/tests/fixtures/bundle-README.md:31-45`. Bundles 1, 3, 5, 11, 12, and 13 already exercise ability-level paths (`get_entity_context`, `prepare_meeting`, stale-source, revoked-source); W6-A's 5-path-per-ability matrix must cite which existing bundle covers each cell and which cells are new. Source: `src-tauri/tests/fixtures/bundle-README.md:29-45`.
- **Read W6-B sibling packet.** Bundle 14 owns stale-current contradiction depth; W6-A's "stale path" cells consume that bundle, not duplicate it. Source: `.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:74-79`, `.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:153-170`.
- **No existing `tests/edge_cases/` dir.** `ls src-tauri/tests/edge_cases` returns "No such file or directory" in this tree, so W6-A creates the directory and contributes the first files. The wave-plan files-owned line at `.docs/plans/v1.4.1-waves.md:624` is the source of the directory path.
- **Read ADRs.** Binding ADRs are ADR-0102 abilities runtime, ADR-0105 provenance + field attribution + `source_asof`, ADR-0114 scoring unification (trust factor math), ADR-0124 thread allowance, and ADR-0125 temporal scope. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `.docs/decisions/0114-scoring-unification.md:1-49`, `.docs/decisions/0124-longitudinal-topic-threading.md:29-49`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:50-54`.
- **Read harness shape.** The harness loader requires `clock.txt`, `seed.txt`, `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `expected_output.json`, `expected_provenance.json`, `metadata.json`, with `expected_state.json` optional. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
- **Substrate test convention.** Sibling tests use `bundleN_<topic>_substrate_test.rs` and `bundle_fixture_path(...)` + harness asserts. W6-A's edge-case files do not own a bundle number; they use a different naming pattern that signals "regression assertion, not fixture-bundle entry point."

## 4. Architecture

### 4.1 Files Owned

W6-A owns the meta extension layer. Three file families:

1. **Per-axis regression assertion files** under `src-tauri/tests/edge_cases/`:
   - `src-tauri/tests/edge_cases/dos289_stale_current_regression.rs` — at least one regression per DOS-289 edge case that consumes bundle 14.
   - `src-tauri/tests/edge_cases/dos290_cross_surface_regression.rs` — consumes bundle 15.
   - `src-tauri/tests/edge_cases/dos291_ambiguous_identity_regression.rs` — consumes bundle 16.
   - `src-tauri/tests/edge_cases/dos292_source_lifecycle_regression.rs` — consumes bundle 17.
   - `src-tauri/tests/edge_cases/dos293_sync_refresh_regression.rs` — consumes bundle 18.

2. **DOS-282 unit-test files** under `src-tauri/tests/edge_cases/unit/`:
   - `dos282_derive_commitment_id_determinism.rs`
   - `dos282_owner_resolution.rs` (exact email, alias, fuzzy name, ambiguous, unassigned)
   - `dos282_claim_canonicalization.rs` (exact, near, negative-non-collapse)
   - `dos282_trust_factor_math.rs` (freshness, reliability, corroboration, contradiction, feedback)
   - `dos282_tombstone_pregate.rs`
   - `dos282_provenance_field_attribution.rs`
   - `dos282_ability_category_enforcement.rs` (Read/Transform cannot mutate)
   - `dos282_source_taxonomy_parsing.rs` (incl. Glean downstream lineage)
   - `dos282_stale_claim_source_age_vs_index_age.rs`
   - `dos282_duplicate_open_loop_commitment_collapse.rs`

3. **DOS-282 integration-test files** under `src-tauri/tests/edge_cases/integration/`:
   - `calendar_to_briefing_integration.rs`
   - `account_meeting_claim_parity_integration.rs`
   - `project_meeting_claim_parity_integration.rs`
   - `transcript_to_commitment_to_work_integration.rs`
   - `user_correction_survives_enrichment_rerun_integration.rs`
   - `revoked_source_masking_integration.rs`
   - `glean_unavailable_fallback_integration.rs`
   - `activity_log_event_population_integration.rs`
   - `lint_seeded_corpus_integration.rs`
   - `old_path_new_ability_parity_integration.rs`

Files outside this list (per-ability fixture path coverage, harness extensions, release-gate wiring) are out of scope for W6-A and live with the sibling agents or W7 release-gate work.

### 4.2 5-Path Ability Fixture Coverage Matrix

The DOS-282 acceptance bar is "each migrated ability has fixtures for happy / empty / stale / revoked-source / contradiction paths where applicable." W6-A authors **the coverage matrix that maps each (ability × path) cell to an existing or sibling-owned bundle**, plus per-axis regression assertions. The matrix below is **the locked acceptance artifact**, not a template — implementation cannot leave cells in `(new)` or `(new or existing)` state. Every cell is one of three explicit states:

- A specific bundle reference (existing 1-13 or sibling-owned 14-18).
- A new W6-A-meta fixture (W6-A authors a minimal happy/empty/contradiction harness fixture under a `bundle-WA-NN-meta-<ability>-<path>` directory) — **W6-A scope expansion accepted**: where a cell has no existing or sibling-owned bundle, W6-A authors a minimum-viable fixture. This supersedes the prior "W6-A does not author new fixture bundles" statement.
- An explicit **N/A — does not apply** justification (e.g., `detect_risk_shift` empty path = "no risk signals yet" already covered by happy path with zero rows).

Locked matrix:

| ability | happy | empty | stale | revoked-source | contradiction |
| --- | --- | --- | --- | --- | --- |
| `get_entity_context` | bundle-1 | W6-A-meta-1 (zero claims for subject) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-2, bundle-6, bundle-14 (W6-B) |
| `prepare_meeting` | bundle-5, bundle-13 | W6-A-meta-2 (meeting with zero context) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-14 (W6-B), bundle-15 (W6-C) |
| `get_daily_readiness` | W6-A-meta-3 (typical day) | W6-A-meta-4 (no meetings) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-14 (W6-B) |
| `detect_risk_shift` | W6-A-meta-5 (degrading account) | N/A — happy path with zero signals doubles as empty | W6-A-meta-6 (stale champion silence) | bundle-12, bundle-17 (W6-E) | W6-A-meta-7 (contradicting risk signals) |
| `list_open_loops` / `extract_commitments` | bundle-9 | W6-A-meta-8 (empty workspace) | W6-A-meta-9 (commitments past TTL) | bundle-17 (W6-E) | W6-A-meta-10 (transcript vs email conflict) |

W6-A's implementation deliverable is this locked matrix (also published in `.docs/plans/wave-W6/coverage-matrix.md`) plus the ten W6-A-meta-N fixtures plus per-axis regression assertions that fail if any cell loses its bundle reference at runtime.

### 4.3 Per-Axis Regression Files

Each of the five `dosNNN_*_regression.rs` files contains **at least one named-user-harm regression assertion per axis** (wave plan §625), consuming the sibling-owned bundle (14-18) via `bundle_fixture_path` and exercising the abilities runtime end-to-end. The bar is **not** "bundle is loadable" — that's too thin to catch a real bug. The bar is "the named user-harm scenario from the DOS-### ticket fails this assertion if reintroduced."

Concrete user-harm assertion per file:

- `dos289_stale_current_regression.rs` — asserts: given bundle-14's six-month-old escalation + fresh resolving evidence, `prepare_meeting.topics` does not contain a string referring to the stale escalation as "current" or "open." A render-string regex check on the rendered topic content; if the implementation ever resurrects stale content as a current talking point, this assertion fires.
- `dos290_cross_surface_regression.rs` — asserts: given bundle-15's seeded shared account, the primary entity id is identical across `get_entity_context`, `prepare_meeting`, `get_daily_readiness`, and the MCP bridge response. A direct equality check on the rendered subject_ref. If a future change causes one surface to silently diverge, this fires.
- `dos291_ambiguous_identity_regression.rs` — asserts: given bundle-16's same-domain twin accounts (scenario 1), the entity linker output `subject_state` is `ambiguous`, not `primary`. A substrate-level check on the linker return type. An implementation that silently picks a primary fails this.
- `dos292_source_lifecycle_regression.rs` — asserts: given bundle-17's revoked Glean source, none of the 9 ADR-0108 channel renderers includes the revoked content. Iterates the channel matrix; if any channel leaks, fires.
- `dos293_sync_refresh_regression.rs` — asserts: given bundle-18's user-correction + concurrent-enrichment scenario (scenario 1), the user-correction row persists in the substrate after the enrichment cycle. A direct query on the claim row. If enrichment overwrites, fires.

These assertions are **shallow but specific**: they catch the named regression cleanly without duplicating the sibling bundle's depth assertions (which test the full claim/trust/provenance chain). The sibling bundle proves correctness; the W6-A regression proves the user-facing bug does not return.

### 4.4 Intelligence Loop Check

- **Claim model:** unit tests assert claim derivation, canonicalization, tombstone, source taxonomy, and stale-claim handling on real substrate, not on display strings.
- **Provenance and trust:** unit tests assert provenance builder field-attribution invariants and trust factor math; integration tests assert revoked-source masking propagates through provenance render.
- **Signals and invalidation:** integration tests assert user correction triggers enrichment rerun, the correction survives, and Activity Log captures invocation/claim/signal events.
- **Runtime and surfaces:** integration tests assert calendar → linked entities → meeting briefing, account/meeting/project parity, and old-path-vs-new-ability parity until cutover. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`, `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`.
- **Feedback loop:** per-ability contradiction-path fixtures plus the user-correction integration test cover the feedback contribution to claim state.

## 5. Acceptance Criteria

DOS-282 Acceptance, quoted verbatim:

> "Test inventory is added to the v1.4.0 implementation plan or linked doc. Each migrated ability has fixtures for happy path, empty path, stale path, revoked-source path, and contradiction path where applicable. Golden Daily Loop validation suite depends on these fixtures. Mock data issue provides the seeded workspace state needed by these tests. CI or release-check command runs the fast subset. Full validation run is documented before v1.4.0 cutover."

The DOS-282 ticket was authored for v1.4.0; under v1.4.1 W6 it carries forward as the meta-suite + edge-case regression covering bundles 14-18 + the 5-path coverage matrix. The cutover gate is now v1.4.1 release.

Testable decomposition:

1. **Test inventory exists.** W6-A produces `.docs/plans/wave-W6/coverage-matrix.md` (or equivalent linked doc) listing every unit test, integration test, and per-ability fixture-path cell with file path + bundle reference. Acceptance fails if a DOS-282 required test is missing from the inventory.
2. **Per-axis regression coverage.** Each of `dos289_*`, `dos290_*`, `dos291_*`, `dos292_*`, `dos293_*` regression files exists under `src-tauri/tests/edge_cases/`, contains at least one passing assertion that consumes the sibling-owned bundle, and fails loudly if the sibling bundle is removed or renamed. Source: `.docs/plans/v1.4.1-waves.md:625`.
3. **Unit-test layer complete.** All ten DOS-282 unit-test families are present under `src-tauri/tests/edge_cases/unit/` and assert against the real substrate (claims service, trust factors, provenance builder, ability category gate, source taxonomy, tombstone gate). Source: DOS-282 Required-unit-tests block.
4. **Integration-test layer complete.** All ten DOS-282 integration-test families are present under `src-tauri/tests/edge_cases/integration/` and exercise the abilities runtime end-to-end. Source: DOS-282 Required-integration-tests block.
5. **Ability fixture coverage matrix locked.** Every (ability × 5-path) cell in §4.2 either references an existing/sibling-owned bundle or names the new fixture that fills it. No cell is empty without a documented "N/A — does not apply" justification. Source: DOS-282 Required-ability-fixtures block.
6. **Golden Daily Loop dependency.** The release-check command at `src-tauri/src/release_gate.rs` (the function reachable by `cargo run --bin release-check` or the equivalent gate runner) directly invokes the W6-A fast subset. The W6-A implementation PR wires the per-axis regression files into the release-gate runner; it does not depend on W7 to remember. Source: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs`.
7. **No PII.** All seeded workspace state uses generic synthetic identifiers (`example.com`, `Alex Example`, etc.) per the project's no-customer-data-in-source-code rule. Source: `/Users/jamesgiroux/Documents/dailyos-repo/CLAUDE.md` Critical Rules.
8. **CI fast subset wall-clock budget.** The fast subset is the 5 per-axis regression files (`dos289_*`, `dos290_*`, `dos291_*`, `dos292_*`, `dos293_*`) plus the 10 W6-A unit tests, all under a single cargo test target `edge_cases_fast` (created by W6-A). **Wall-clock budget: under 90 seconds on the CI runner** measured by `cargo test --test edge_cases_fast --release -- --test-threads 4`. If the suite exceeds 90s, slow tests get moved to a separate `edge_cases_full` target run only on the W6 merge gate, not per-PR. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
9. **Parallel-run comparator located.** The old-path/new-ability parity integration test uses the ADR-0112 `ParallelRunValidator` infrastructure. W6-A implementation must locate the existing validator (search `src-tauri/src` for `ParallelRun` or "parallel_run") and, if absent, scope a thin comparator that compares legacy command output to new ability output for a defined field set (subject id, primary entity, claim count, trust band) with documented tolerance for inherent provenance differences. Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:31-41`.
10. **Mock-data dependency named.** DOS-282 names a "Mock data issue provides the seeded workspace state needed by these tests." W6-A implementation depends on the existing v1.4.0 mock-data seed at `src-tauri/tests/fixtures/` and the harness loader (`src-tauri/src/harness/loader.rs`); if a new seed is needed for a W6-A-meta-N fixture, W6-A authors it inline rather than blocking on an external mock-data deliverable.
9. **Old-path/new-ability parity.** The `old_path_new_ability_parity_integration.rs` test asserts that for each migrated ability, the legacy path output and the new ability output remain in parity until the cutover commit removes the legacy path. Source: ADR-0112 parallel-run-and-cutover.
10. **Cross-link to sibling packets.** W6-A's implementation references the bundle owned by each sibling: bundle 14 → W6-B, bundle 15 → W6-C, bundle 16 → W6-D, bundle 17 → W6-E, bundle 18 → W6-F. Source: `.docs/plans/v1.4.1-waves.md:627-651`.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-282 content is supplied verbatim in the authoring prompt for this packet. No Linear connector lookup was required to draft V1.
- **Upstream unblock:** W6 starts after the W3 stage-3b precondition, as amended to instrumentation-complete rather than full data-sufficiency closure. Sources: `.docs/plans/v1.4.1-waves.md:653-655`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Sibling dependencies:** W6-A's regression files consume bundles 14-18; W6-A's implementation should not block on those bundles existing before merging, but L3 Suite E final cannot pass until all 18 bundles are green. W6-A may merge before sibling bundles land if the regression files are `#[ignore]`-gated with a tracking comment until the bundle exists; preferred is to merge after at least the sibling bundle skeleton has landed.
- **W7 release-gate coordination:** W6-A must coordinate with W7 (release gate hardening) to ensure the per-axis regression files participate in the mandatory CI fast subset and the Golden Daily Loop validation. Source: `.docs/plans/v1.4.1-waves.md:667-715` (W7 scope).
- **Not a sibling takeover:** W6-A does not author bundle 14-18 contents; that work is owned by W6-B/C/D/E/F. W6-A authors `tests/edge_cases/`, the coverage matrix, and the per-axis regression layer that consumes the sibling bundles.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** W6 merge gate requires L0 plan approvals with `qa-expert` for all six W6 agents. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Security reviewer:** not required for W6-A. The wave gate names `security-auditor` only for DOS-292 (W6-E). Source: `.docs/plans/v1.4.1-waves.md:642-646`.
- **Review focus for `qa-expert`:**
  - The 5-path coverage matrix is exhaustive across the migrated abilities and either points to an existing/sibling bundle or names a new fixture for every cell.
  - The per-axis regression files contain real assertions, not placeholder bodies.
  - Unit tests target real substrate (claims, trust factors, provenance, ability gate, source taxonomy), not mocks.
  - Integration tests exercise the abilities runtime end-to-end, not internal helpers.
  - `old_path_new_ability_parity_integration.rs` is wired and survives until cutover.
  - The CI fast subset is wall-clock fast enough to gate every PR, and the full validation run is documented.
  - No PII in any seeded workspace state.

## 8. L0 Acceptance Gate

L0 passes only if the reviewer accepts all of the following:

1. **Scope fit:** the packet is the meta layer of W6, not a duplicate of any sibling bundle.
2. **File ownership:** `src-tauri/tests/edge_cases/` directory and its named subfiles are unambiguous; no overlap with `src-tauri/tests/bundle14_*` through `bundle18_*` (those are sibling-owned).
3. **Amendment acknowledgement:** Amendment 1 is acknowledged; W3 stage-3b residual is not pulled into W6-A scope.
4. **Acceptance coverage:** every clause of DOS-282 Acceptance is decomposed into a testable assertion in §5.
5. **Coverage matrix locked:** every (ability × 5-path) cell in §4.2 has a bundle reference or a documented N/A.
6. **No PII:** all seeded workspace state is generic synthetic identifiers.
7. **Runtime parity:** integration tests exercise the abilities runtime end-to-end, not display-only display.
8. **No new ADRs invented:** ADR-0102, ADR-0105, ADR-0112, ADR-0114, ADR-0124, and ADR-0125 are the binding anchors; no fictional ADR is cited.
9. **Reviewer panel:** `qa-expert` is the only required L0 reviewer; no `security-auditor` is listed for W6-A.

## 9. Out-Of-Scope

- Authoring bundles 14-18 (owned by W6-B/C/D/E/F).
- Authoring the bundle-14 stale-current fixture rows (W6-B owns).
- Authoring the cross-surface bundle-15 fixture (W6-C owns).
- Authoring the ambiguous identity bundle-16 fixture (W6-D owns).
- Authoring the source lifecycle bundle-17 fixture or the 9-channel sensitivity sweep (W6-E owns; security-auditor lane).
- Authoring the sync/refresh bundle-18 fixture (W6-F owns).
- Writing release-gate mandatory-set wiring for bundles 14-18 (W7 release gate coordination).
- Re-running the W3 stage-3b shadow trust closure work (Amendment 1 deferred to v1.4.2 spike outcome).
- Building a user-facing edge-case explorer or test-coverage dashboard.
- Adding customer-specific names, domains, emails, or account details to any seeded state.

## 10. Changelog

- **V1 - 2026-05-15:** Initial W6-A L0 packet. Located meta scope at `src-tauri/tests/edge_cases/`; locked sibling bundle mapping (W6-B=14, W6-C=15, W6-D=16, W6-E=17, W6-F=18); decomposed DOS-282 unit-test, integration-test, and 5-path fixture acceptance into testable assertions; flagged W7 release-gate coordination; acknowledged Amendment 1; named `qa-expert` as the only required L0 reviewer.
