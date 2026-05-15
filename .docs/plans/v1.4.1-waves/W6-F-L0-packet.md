# DOS-293 W6-F L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-F.
- **Linear issue:** DOS-293 - "Validation: sync, refresh, concurrency, and partial-failure behavior" (DOS-293 content supplied verbatim in the authoring prompt for this packet).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-F-L0-packet.md`.
- **W6-F assignment:** the wave plan names W6-F as "DOS-293 sync, refresh, concurrency, partial-failure" and assigns it "validation bundle + assertions for partial-failure recovery, concurrent enrichment, refresh idempotency." Source: `.docs/plans/v1.4.1-waves.md:648-651`.
- **W6 merge gate:** W6 requires L0 plan approvals, L2 approvals, L3 Suite E final with bundles 1-13 and 14-18 mandatory green, L4 `/qa`, L5 drift check, retro, and proof bundle. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Reviewer contract:** W6 L0 requires `qa-expert` for all six W6 agents, with `security-auditor` only for DOS-292. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Validation-suite numbering contract:** five new bundles 14-18, one per DOS-289 through DOS-293, all mandatory in the v1.4.1 release gate. W6-F maps to **bundle 18**. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/tests/fixtures/bundle-README.md:29-45`.
- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; bundles 1-13 are present; the harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Runtime contract:** synthesized user-facing and agent-facing context must go through abilities; every ability output carries provenance once; Transform outputs cannot authorize mutation. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`, `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`.

## 2. Load-Bearing User Outcome

DOS-293 frames the user failure this bundle must prevent:

> "DailyOS often fails in real usage when work happens out of order — sync partially fails, refresh runs while the user edits, old retries commit after newer output, or child abilities fail but the parent surface renders as complete. The abilities runtime needs correctness under lifecycle pressure, not only clean synchronous invocation."

The user harm is also explicit:

> "User corrections disappear, stale briefings look ready, agendas get wiped, duplicate actions appear, or the app says refresh completed while the visible state is still old."

The load-bearing outcome for W6-F is therefore not "concurrent refresh has a warning state." It is: **the user-authored layer wins over the generated layer, generated outputs are versioned and monotonic, partial failures are explicit, refresh is idempotent for claims and commitments, and offline/stale state is visible.**

Required behavior from DOS-293:

> "User-authored layer wins over generated layer; generated outputs are versioned/monotonic, stale retries cannot overwrite newer state; partial failures are explicit in ability output and render state; refresh is idempotent for claims, commitments, and user-layer data; offline/stale mode is visible; eval mode uses injected clock/RNG/provider replay."

This user outcome depends on existing Intelligence Loop substrate:

- **Claim model:** generated claim outputs must carry a version/monotonic identifier so a late-arriving retry cannot overwrite newer state; user-authored layer rows have their own provenance and survive enrichment cycles.
- **Provenance and trust:** partial-failure state must be visible in the provenance envelope — child ability failures must propagate to parent provenance with a `degraded`/`partial` warning, not silent success.
- **Signals and invalidation:** refresh must coalesce safely; signal coalescing cannot drop a required invalidation; duplicate refresh cannot create duplicate claims.
- **Runtime and surfaces:** ability outputs must include a partial-failure indicator that surface render policy can act on; offline/stale state must be exposed not silently swallowed.
- **Feedback loop:** user corrections, dismissals, and agenda edits survive concurrent enrichment; eval mode replays deterministically via injected clock/RNG/provider.

The W6-F proof must cover these concrete DOS-293 edge cases:

- User correction races with enrichment; enrichment must not overwrite correction.
- Retry job commits older generated output after newer output already exists.
- Refresh wipes user agenda, notes, hidden attendees, or dismissed topics.
- One child ability fails during daily readiness but parent surface still shows complete.
- Offline mode uses stale data without disclosure.
- Signal coalescing drops a required invalidation; briefing stays stale.
- Duplicate refreshes create duplicate claims/actions.
- Eval replay differs from live because clock/random/provider behavior is not injected.

## 3. Pre-Work

- **Read W6 source of truth.** W6-F owns sync, refresh, concurrency, partial-failure. Source: `.docs/plans/v1.4.1-waves.md:648-651`.
- **Read W6 merge gate.** L3 Suite E final requires bundles 1-13 + bundles 14-18 mandatory green, no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Acknowledged Amendment 1.** Amendment 1 recategorizes W3 stage-3b as `instrumentation-complete, data-sufficiency-pending`, relaxes W6's hard precondition, and says W6 starts against the partial baseline. W6-F scope is unaffected. Sources: `.docs/plans/v1.4.1-waves-amendments.md:15-23`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Mapped bundle number.** Bundles 14-18 are new to W6, one per DOS-289 through DOS-293 in spec order. W6-F = bundle 18.
- **Read sibling W6-B packet.** W6-B (bundle 14) covers stale-current contradiction; W6-F's overlap with W6-B is bounded to the case where stale data renders during a failed refresh — bundle 18 asserts the failure shape, bundle 14 asserts the temporal-truth shape. Source: `.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:43-50`.
- **Read sibling W6-C packet.** W6-C (bundle 15) covers cross-surface consistency. W6-F adjacency: the "activity log says refresh completed but visible surface shows old data" scenario from DOS-290 is exercised at the consistency layer by bundle 15; W6-F bundle 18 owns the refresh-completion-vs-render-state assertion at the sync layer itself.
- **Prior art:** `bundle-5` covers "first-person meeting-prep parity; attendee `WrongSubject` tombstone; user-edited superseding claim; duplicate/paraphrase corroboration; expired dormant claim; double-refresh no-resurrection guard." The double-refresh no-resurrection guard is the closest sibling assertion; W6-F bundle 18 extends to the full DOS-293 set including partial-failure propagation. Source: `src-tauri/tests/fixtures/bundle-README.md:37`.
- **Read ADRs.** Binding ADRs are ADR-0102 abilities runtime, ADR-0105 provenance + warnings, and ADR-0112 parallel-run-and-cutover (eval-vs-live behavior). Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
- **Read harness shape.** Loader requires `clock.txt`, `seed.txt`, `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `expected_output.json`, `expected_provenance.json`, `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.

## 4. Architecture

### 4.1 Bundle Assignment

W6-F owns **bundle 18**.

- **New fixture directory:** `src-tauri/tests/fixtures/bundle-18/`.
- **New substrate test file:** `src-tauri/tests/bundle18_sync_refresh_concurrency_substrate_test.rs`.
- **Naming rationale:** project convention `bundleN_<topic>_substrate_test.rs`.
- **Discovery rationale:** fixture directories must be hyphenated `bundle-N` with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Release-gate rationale:** W6/W7 requires bundles 14-18 mandatory green. Source: `.docs/plans/v1.4.1-waves.md:653-663`.

### 4.2 Fixture Invariants

Bundle 18 proves **five invariants**:

1. **User-authored wins.** A user correction or edit cannot be silently overwritten by a concurrent enrichment cycle.
2. **Generated output is monotonic.** A retry commit with an older generation timestamp cannot overwrite newer state.
3. **Partial failures are explicit.** When a child ability fails, the parent ability output exposes a `partial`/`degraded` warning, not silent success.
4. **Refresh is idempotent.** Two refreshes back-to-back cannot create duplicate claims or commitments.
5. **Offline/stale visible; eval determinism.** Offline mode discloses stale state; eval mode uses injected clock/RNG/provider replay so replays are deterministic.

The bundle does not satisfy these invariants by adding warnings to clean-path output; it seeds the failure mode and asserts on the resulting substrate.

### 4.3 Fixture Shape

Bundle 18 seeds an end-to-end workspace exercising all eight DOS-293 edge cases. Generic synthetic identifiers only.

Required fixture files follow the loader contract:

- `clock.txt` fixes the test clock.
- `seed.txt` fixes randomization.
- `state.sql` seeds: a user-corrected claim row + an enrichment cycle pending; a generated output row at generation N + a retry job carrying generation N-1; a user agenda row + a refresh job pending; a daily-readiness parent invocation with one child ability scheduled to fail; an offline-mode flag + stale claim rows; a signal coalescing queue with a dropped invalidation; a duplicate-refresh case (two refresh jobs targeting the same subject); an eval-mode case using injected clock/RNG/provider replay.
- `inputs.json` drives `prepare_meeting`, `get_entity_context`, `get_daily_readiness`, and `extract_commitments` through the harness.
- `provider_replay.json` pins provider outputs for the parallel cycles to ensure deterministic replay.
- `external_replay.json` pins external-source outputs.
- `expected_output.json` asserts: user correction persists; older retry commit is rejected; daily readiness parent carries `partial` warning when child fails; duplicate refresh produces single claim row.
- `expected_provenance.json` asserts: partial-failure warning visible in parent provenance; `source_asof` and generation versions exposed.
- `expected_state.json` asserts post-action state for all eight scenarios. **Note:** `expected_state.json` is optional in the loader contract per `src-tauri/src/harness/loader.rs:23`; bundle 18 documents this file as a required **bundle-18 extension** in `metadata.json.fixture_design_notes` so the harness picks it up explicitly.
- `metadata.json` includes `bundle: 18`, `scenario_id: sync-refresh-concurrency-partial-failure`, `surfaces_exercised` covering all four affected abilities, dominant factors including user-authored precedence + generation monotonicity + partial-failure propagation, and a pass/fail definition that fails if any invariant breaks. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.

### 4.4 Seeded Scenario Coverage

The bundle's eight scenario branches map directly to DOS-293's concrete edge cases:

1. **User correction races with enrichment.** User edits a claim; concurrent enrichment cycle runs. Expected: user edit persists; enrichment defers or merges, never overwrites.
2. **Retry commits older generation (substrate row rejection asserted).** Generation N output exists; retry job commits with generation N-1 marker. Expected: the retry's attempted write to the claim row is rejected at the substrate level — `expected_state.json` asserts the claim row's `generation` column remains at N AND a rejection record exists at `generated_output_rejections` (or the equivalent rejection log). Output-level absence alone is not sufficient — generation monotonicity must be proven at the persisted row.
3. **Refresh wipes user agenda.** User adds private agenda notes; refresh runs. Expected: user agenda rows are not touched by refresh.
4. **Child ability fails, parent renders complete.** Daily readiness composes one child that fails. Expected: parent output carries `partial` warning; render policy reflects degraded state; parent does not render `complete`.
5. **Offline mode hides staleness (named output field).** Offline-mode flag set with stale claim rows. Expected: the ability output's provenance envelope `warnings` array contains an entry of class `OfflineStale` with a `stale_age_hours` field carrying N. The render layer reads this field; bundle 18 asserts on the substrate field, not on rendered display strings. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:55-63`.
6. **Signal coalescing drops invalidation.** Two invalidation signals queued; coalescing reduces to one. Expected: dependent claims are still invalidated; briefing reflects fresh state, not stale.
7. **Duplicate refresh produces single claim.** Two refresh jobs for the same subject queued. Expected: one claim row created, not two; no duplicate commitments or open loops.
8. **Eval replay deterministic.** Eval mode with injected clock/RNG/provider replay produces identical output across replays. Expected: bit-for-bit identical `expected_output.json` across two runs.

### 4.5 Partial-Failure Propagation Shape

For scenario 4 (child fails), the parent provenance envelope must carry the partial-failure information:

- A `warnings` entry using the **existing** ADR-0105 warning enum value `OptionalComposedReadFailed` (defined at `src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:421-455`), carrying the failing child ability name and reason. Bundle 18 does NOT introduce new vocabulary tokens; if the existing enum is insufficient for the partial-failure case, the W6-F PR scope expands to extend the enum (with an ADR amendment), but the default approach is to reuse `OptionalComposedReadFailed`.
- A trust band that reflects the degradation (no silent `likely_current` on a partial output).
- A render policy hint (downstream surfaces can render as degraded/partial).

This is asserted on the ADR-0105 provenance envelope, not on render strings. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.

### 4.6 Eval Determinism

For scenario 8, the bundle asserts:

- Two harness invocations with identical `clock.txt`, `seed.txt`, `provider_replay.json`, and `external_replay.json` produce **canonical-JSON-equal** output (object key-order tolerance per `src-tauri/src/harness/scoring.rs:148`, `:521`). The assertion uses the existing harness scoring path's canonical equality, not raw byte comparison — byte-identical was a misstatement; canonical equality is the correct invariant given the harness's JSON normalization.
- The substrate uses injected clock/RNG; no direct `std::time::SystemTime::now()` or unseeded `rand::random()` calls in the abilities path during eval mode.
- Provider responses come from `provider_replay.json`; no live provider call in eval mode.

This invariant is asserted via the harness comparing two replay runs, not via mock injection at unit-test level.

### 4.7 Intelligence Loop Check

- **Claim model:** user-authored layer rows have distinct provenance from generated rows; the bundle asserts on substrate state, not display.
- **Provenance and trust:** partial-failure propagation lives in the provenance envelope's warnings + trust band; bundle 18 asserts envelope shape.
- **Signals and invalidation:** signal coalescing must preserve all required invalidations even when collapsing duplicates; assertion is on the dependent claim's freshness state after coalescing.
- **Runtime and surfaces:** all four affected abilities (`prepare_meeting`, `get_entity_context`, `get_daily_readiness`, `extract_commitments`) carry partial-failure metadata when applicable.
- **Feedback loop:** user-authored corrections + dismissals + agenda edits survive concurrent enrichment; assertion is on substrate persistence, not on UI behavior.

## 5. Acceptance Criteria

DOS-293 Acceptance, quoted verbatim:

> "User correction survives concurrent enrichment; old retry output cannot overwrite newer output; user agenda/notes/dismissals survive refresh; daily readiness marks partial/degraded state when child ability fails; duplicate refresh does not create duplicate claims or commitments; eval fixture reproduces race outcome deterministically."

Testable decomposition:

1. **User correction survives concurrent enrichment.** Scenario 1: user edit + concurrent enrichment cycle. Expected substrate state asserts user edit persists.
2. **Old retry rejected.** Scenario 2: retry with generation N-1 marker is rejected when current is generation N.
3. **Refresh preserves user agenda, notes, AND dismissals.** Scenario 3 seeds three distinct user-layer rows: an agenda row at `user_agenda_items`, a note row at `user_notes`, and a dismissal row at `user_dismissals`. `expected_state.json` asserts all three rows are unchanged after the refresh cycle. DOS-293 explicitly names "agenda/notes/dismissals" — partial coverage on agenda alone fails. Source: DOS-293 Acceptance.
4. **Daily readiness marks partial.** Scenario 4: parent ability output carries `partial`/`degraded` warning in provenance envelope; render policy reflects.
5. **Duplicate refresh produces single output.** Scenario 7: only one claim row created from two refresh jobs.
6. **Eval determinism (canonical equality, not byte-identical).** Scenario 8: two replay runs produce canonical-JSON-equal output via the existing harness scoring path. Substrate also asserts: no `SystemTime::now()` or unseeded `rand::random()` calls fire in the abilities path during eval mode; provider responses come from `provider_replay.json`, not live. Source: `src-tauri/src/harness/scoring.rs:148-521`.
7. **Offline state visible.** Scenario 5: offline-mode output explicitly discloses staleness with age.
8. **Signal coalescing preserves invalidations.** Scenario 6: dependent claims are refreshed after coalescing, not stale.
9. **Bundle 18 is mandatory — the W6-F PR itself flips the mandatory bit.** Implementation does not "coordinate with W7" or defer wiring. The W6-F PR includes the edit promoting bundle 18 to mandatory in `src-tauri/src/release_gate.rs:26-38`. If that edit is missing, L2 review rejects. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`.

11. **Deterministic race shape (sequential, not real concurrency).** Scenarios 1 (user-correction race) and 7 (duplicate refresh) use the bundle-5 prior-art shape: a current-thread runtime with explicit first/second step ordering — NOT real scheduler concurrency. Bundle 18 is reproducible test-to-test; flaky real-concurrency assertions are not acceptable. Source: `src-tauri/tests/dos283_bundle5_double_refresh_resurrection_test.rs:54-99`.

12. **Per-scenario subtests for diagnostics.** Bundle 18's substrate test file uses one `#[test]` function per scenario (or a parameterized test with `#[test_case(scenario_id)]`), not one monolithic test. Diagnostic output names which scenario_id failed.
10. **No PII in seeded state.** All identifiers are synthetic generic.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-293 content is supplied verbatim in the authoring prompt for this packet.
- **Upstream unblock:** W6 starts after the W3 stage-3b precondition, as amended to instrumentation-complete. Sources: `.docs/plans/v1.4.1-waves.md:653-655`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Adjacent W6 coordination:** W6-F owns bundle 18; W6-B (bundle 14) covers stale-current temporal truth, W6-C (bundle 15) covers cross-surface consistency. The "activity log says refresh completed but surface shows old data" case is shared between bundle 15 (consistency assertion) and bundle 18 (refresh-completion assertion).
- **Prior art:** `bundle-5` already has the double-refresh-no-resurrection guard; bundle 18 extends to the full DOS-293 set including partial-failure propagation and user-layer durability.
- **Not a W6-A takeover:** W6-A's `dos293_sync_refresh_regression.rs` consumes bundle 18; W6-F produces the bundle.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** W6 merge gate requires `qa-expert` for all six W6 agents. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Security reviewer:** not required for W6-F.
- **Review focus for `qa-expert`:**
  - All eight DOS-293 scenarios seeded in `state.sql` with explicit failure mode.
  - Partial-failure propagation asserted on provenance envelope shape, not display strings.
  - User-authored layer durability asserted across concurrent enrichment + refresh + retry cases.
  - Generation monotonicity asserted on retry-commit rejection.
  - Eval determinism asserted via two-replay comparison.
  - Bundle 18 can become mandatory in the W6/W7 release gate.
  - No PII in any seeded state.

## 8. L0 Acceptance Gate

L0 passes only if the reviewer accepts all of the following:

1. **Problem fit:** the plan tests sync/refresh/concurrency/partial-failure invariants, not generic refresh testing.
2. **Bundle lock:** W6-F is locked to bundle 18 and implementation path `src-tauri/tests/bundle18_sync_refresh_concurrency_substrate_test.rs`.
3. **Fixture lock:** bundle directory `src-tauri/tests/fixtures/bundle-18/` using loader-required files and `metadata.json` fields. Sources: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
4. **Amendment acknowledgement:** Amendment 1 is acknowledged.
5. **Acceptance coverage:** every clause of DOS-293 Acceptance is decomposed into a testable assertion in §5.
6. **Eight edge-case scenarios seeded:** the bundle covers all eight DOS-293 concrete cases.
7. **Five invariants asserted:** user-authored wins, generation monotonic, partial visible, refresh idempotent, offline/eval visible/deterministic.
8. **Provenance-envelope shape:** partial-failure is asserted on the ADR-0105 envelope, not the render layer.
9. **Reviewer panel:** `qa-expert` is the only required L0 reviewer.
10. **No PII:** all fixture identifiers are synthetic.

## 9. Out-Of-Scope

- Cross-surface consistency beyond the refresh-completion-vs-render-state case (W6-C bundle 15).
- Stale-current temporal-truth beyond the failed-refresh-still-renders case (W6-B bundle 14).
- Subject-selection ambiguity (W6-D bundle 16).
- Source lifecycle + 9-channel sensitivity (W6-E bundle 17).
- Writing implementation files in this packet authoring turn.
- Committing changes.
- Building a user-facing offline-mode UI banner (the bundle asserts on substrate state; UI is downstream).
- Adding new ADRs (existing ADR-0102 + ADR-0105 are sufficient).
- Treating W6-A's regression file as W6-F scope.
- Customer-specific identifiers anywhere in the bundle.

## 10. Changelog

- **V1 - 2026-05-15:** Initial W6-F L0 packet. Assigned DOS-293 to bundle 18; locked the eight DOS-293 edge-case scenarios; mapped the five invariants (user-authored wins, generation monotonic, partial visible, refresh idempotent, offline/eval deterministic); asserted partial-failure shape on ADR-0105 envelope; named `qa-expert` as the only required L0 reviewer.
