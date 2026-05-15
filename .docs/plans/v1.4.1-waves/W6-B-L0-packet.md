# DOS-289 W6-B L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-B.
- **Linear issue:** DOS-289 - "Validation - stale-current contradiction and temporal truth" (DOS-289 content supplied verbatim in the authoring prompt for this packet).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-B-L0-packet.md`.
- **W6-B assignment:** the wave plan names W6-B as "DOS-289 stale-current contradiction validation" and assigns it "validation bundle + assertions for fresh-supersedes-stale; cross-time temporal tests." Source: `.docs/plans/v1.4.1-waves.md:627-630`.
- **W6 merge gate:** W6 requires L0 plan approvals, L2 approvals, L3 Suite E final with bundles 1-13 and 14-18 mandatory green, L4 `/qa`, L5 drift check, retro, and proof bundle. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Reviewer contract:** W6 L0 requires `qa-expert` for all six W6 agents, with `security-auditor` only for DOS-292. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Validation-suite numbering contract:** the wave-plan clarification resolves five new bundles as 14, 15, 16, 17, and 18, one per DOS-289 through DOS-293, all mandatory in the v1.4.1 release gate. Source: `.docs/plans/v1.4.1-waves.md:829-831`.
- **Fixture catalogue:** the committed fixture corpus is documented at `src-tauri/tests/fixtures/bundle-README.md`; it currently describes `bundle-1` through `bundle-13`, requires hyphenated `bundle-N` directories, and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Runtime contract:** synthesized user-facing and agent-facing context must go through abilities, surfaces invoke abilities through the registry/typed imports, every ability output carries provenance once, and Transform outputs cannot authorize mutation on their own. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`, `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`.

## 2. Load-Bearing User Outcome

DOS-289 frames the user failure this bundle must prevent:

> "A briefing can be well-sourced and still be wrong if it presents historical state as current state. v1.4.0 already has freshness decay and trust scoring, but that does not explicitly prove newer contradictory evidence wins over older claims."

The user harm is also explicit:

> "Walks into a meeting and asks about an issue resolved yesterday, references a stakeholder who left months ago, or treats a mitigated risk as open."

The load-bearing outcome for W6-B is therefore not "stale evidence gets a warning." It is: **DailyOS must prove that current-state advice cannot be driven by stale historical claims when fresher contradictory evidence exists.** Historical claims may remain visible as history, provenance, or timestamped disagreement, but they cannot render as current talking points, open risks, current stakeholder facts, or current urgency without qualification.

Required behavior from DOS-289:

> "current-state claims must encode state open or mitigated or resolved or historical or unknown; newer contradictory evidence suppresses, supersedes, or qualifies older; freshness considers downstream source semantics not only ingestion time; suggestions and talking points cannot use stale claims as current advice; user outdated feedback prevents future use unless re-validated."

This user outcome depends on the existing Intelligence Loop substrate, not a display-only test:

- **Claim model:** W6-B tests claim lifecycle state, surfacing state, `source_asof`, `temporal_scope`, contradiction edges, supersession, and user feedback state. The claims service has explicit supersession behavior that demotes the superseded claim and writes a `claim_contradictions` edge with `branch_kind = 'supersession'`. Source: `src-tauri/src/services/claims.rs:5810-5995`.
- **Provenance and trust:** ability outputs carry a provenance envelope with temporal context, trust classification, source attribution, field attribution, and warnings. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
- **Temporal truth:** `source_asof` is must-populate-when-knowable, and freshness uses `source_asof` before ingestion time so old downstream evidence cannot masquerade as fresh just because it was ingested recently. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`.
- **Freshness and contradiction scoring:** ADR-0114 identifies shared trust factors including freshness, contradiction, and user-feedback weighting; it also states that the Trust Compiler owns claim trust score and stitches corroboration, user feedback, and contradiction inputs together. Source: `.docs/decisions/0114-scoring-unification.md:1-49`, `.docs/decisions/0114-scoring-unification.md:312-316`.
- **Feedback loop:** `MarkOutdated` means "Was true, no longer," uses `FreshnessRefresh`, and renders `HiddenFromCurrent`; `NeedsNuance` can render a superseder. Source: `src-tauri/abilities-runtime/src/abilities/feedback.rs:26-62`, `src-tauri/abilities-runtime/src/abilities/feedback.rs:82-107`, `src-tauri/abilities-runtime/src/abilities/feedback.rs:222-320`.

The W6-B proof must cover these concrete DOS-289 edge cases as fixture rows or explicit assertions:

- Six-month-old escalation resolved yesterday, but prep must not flag it as active.
- Old call transcript says "happy" while fresher CRM or support evidence says risk, or vice versa.
- Changed renewal date must not leave old urgency in current advice.
- Stakeholder role/title changes must not drive current talking points from the old title.
- Recent fetch time from Clay or Glean cannot hide stale downstream object semantics.
- User `MarkOutdated` feedback prevents future current-state use unless a newer validating source re-establishes the claim.
- Historical case-study content remains historical, not current state.

## 3. Pre-Work

- **Read W6 source of truth.** W6 is the validation suite, six agents fan out, and W6-B owns stale-current contradiction validation. Source: `.docs/plans/v1.4.1-waves.md:617-630`.
- **Read W6 merge gate.** W6 cannot land partially: L3 Suite E final requires bundles 1-13 plus bundles 14-18 mandatory pass, all 18 green, no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Acknowledged Amendment 1.** Amendment 1 recategorizes W3 stage-3b as `instrumentation-complete, data-sufficiency-pending`, relaxes W6's hard precondition to stage-3b instrumentation-complete, and says W6 starts against the partial baseline. This does not reduce W6 scope; it only narrows what "stage-3b closure" means for unblocking W6. Sources: `.docs/plans/v1.4.1-waves-amendments.md:15-23`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`, `.docs/plans/v1.4.1-waves-amendments.md:71-75`.
- **Mapped bundle number.** `ls src-tauri/tests/bundle*.rs` currently finds bundle tests for 2, 3, 4, 6, 7, and 8 only; no `bundle14` sibling exists. The fixture catalogue documents committed `bundle-1` through `bundle-13`, and the harness currently expects bundles 1 through 13. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/tests/harness.rs:42-65`.
- **Assigned W6-B to bundle 14.** The lowest free bundle number starting at 14 is 14, and the wave plan assigns five new bundles 14-18 to DOS-289 through DOS-293. Sources: `.docs/plans/v1.4.1-waves.md:829-831`, `src-tauri/tests/fixtures/bundle-README.md:29-45`.
- **Found the fixture catalogue.** Search terms used across `.docs/plans/`, `.docs/design/`, `src-tauri/tests`, and `src-tauri/tests/fixtures`: `bundle catalogue`, `bundle catalog`, `fixture catalogue`, `fixture catalog`, `seed-bundle`, `seed bundle`, `validation bundle`, and `bundle [0-9]+`. The catalogue is `src-tauri/tests/fixtures/bundle-README.md`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-27`.
- **Read sibling bundle shape.** `bundle3_stale_source_resurrection_substrate_test.rs` loads a numbered fixture through `bundle_fixture_path`, asserts fixture metadata, runs `run_with_synthetic_enrich_stub`, compares expected and actual post-action state, and asserts warnings/trust fields. Source: `src-tauri/tests/bundle3_stale_source_resurrection_substrate_test.rs:1-91`.
- **Read loader shape.** Fixtures require `clock.txt`, `seed.txt`, `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `expected_output.json`, `expected_provenance.json`, and `metadata.json`; `expected_state.json` is optional. Source: `src-tauri/src/harness/loader.rs:11-23`, `src-tauri/src/harness/loader.rs:136-184`.
- **Read bundle discovery.** The loader discovers only directories whose names match `bundle-` followed by digits and contain `metadata.json`. Source: `src-tauri/src/harness/loader.rs:186-238`.
- **Read ADRs.** Binding ADRs for this packet are ADR-0102 abilities runtime, ADR-0105 provenance plus `source_asof`, ADR-0114 scoring/freshness/trust factors, ADR-0124 thread allowance, and ADR-0125 temporal scope. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `.docs/decisions/0114-scoring-unification.md:1-49`, `.docs/decisions/0124-longitudinal-topic-threading.md:29-49`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:50-54`.
- **Freshness ADR search result.** Grepping `.docs/decisions/` for freshness/trust/supersession/contradiction found no standalone "freshness decay ADR." The applicable freshness anchors are ADR-0105's `source_asof` amendment, ADR-0114's scoring unification consumed by DOS-10 Freshness decay, and ADR-0125's temporal-scope tie-in to freshness and supersession. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`, `.docs/decisions/0114-scoring-unification.md:1-49`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:156-198`.
- **Read implementation surfaces.** `prepare_meeting` composes `get_entity_context`, carries evidence `source_asof`, trust band, temporal scope, and source warnings; `get_entity_context` reads claim-backed entries and attributes every entry to claim provenance; `get_daily_readiness` composes both surfaces and carries `source_asof` through snapshots. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:14-32`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:107-146`, `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:14-37`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:141-195`.
- **Read trust runtime.** Trust bands are `likely_current`, `use_with_caution`, `needs_verification`, and `unscored`; triggered gates force `NeedsVerification`, and caveats include stale source and unresolved contradiction. Sources: `src-tauri/abilities-runtime/src/abilities/trust/types.rs:23-94`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:39-79`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:204-260`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:320-405`.
- **No fork-point diff in this packet authoring pass.** The authoring request explicitly prohibited git operations. This L0 packet is grounded by direct file reads and command checks, and L2 remains the pre-merge diff review stage after implementation.

## 4. Architecture

### 4.1 Bundle Assignment

W6-B owns **bundle 14**.

- **New fixture directory:** `src-tauri/tests/fixtures/bundle-14/`.
- **New substrate test file:** `src-tauri/tests/bundle14_stale_current_contradiction_substrate_test.rs`.
- **Naming rationale:** user instruction requires `src-tauri/tests/bundleN_TOPIC_substrate_test.rs`; existing siblings use that shape, including `bundle3_stale_source_resurrection_substrate_test.rs`. Source: `src-tauri/tests/bundle3_stale_source_resurrection_substrate_test.rs:1-18`.
- **Discovery rationale:** fixture directories must be hyphenated `bundle-N`, and the loader only recognizes `bundle-` plus digits with `metadata.json`. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/src/harness/loader.rs:205-238`.
- **Release-gate rationale:** W6/W7 requires bundles 14-18 mandatory green; current release-gate defaults still list only bundles 1, 5, and 13 as mandatory, so W6-B implementation must either wire bundle 14 into the mandatory set directly or coordinate with the W7 release-gate owner before L3/L5. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`, `src-tauri/src/release_gate.rs:720-766`, `src-tauri/src/release_gate.rs:1509-1543`.

### 4.2 Fixture Invariant

**Invariant:** A stale historical/current-state claim with older `source_asof` must not drive current advice when fresher contradictory or resolving evidence exists. The old claim may remain as history, provenance, or timestamped disagreement; current surfaces must render the fresh state, suppress the stale state, or explicitly qualify disagreement.

The invariant is not satisfied by age-only scoring. The bundle must prove **four orthogonal substrate behaviors**, each asserted independently so that one passing does not mask another failing:

1. **`source_asof` independence from `observed_at`.** Old `source_asof` + recent `observed_at` (recent fetch of an old downstream object) must produce stale freshness/trust/provenance **even before** any contradiction or supersession applies. This is asserted with a single-claim fixture row (no fresh contradicting evidence) where the stale claim's age is computed from `source_asof` and the resulting trust band is not `likely_current`. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:156-180`.
2. **Contradiction path.** Two same-subject/same-claim-type/different-text claims write `claim_contradictions` and **both remain active** until reconciliation; trust band drops with `UnresolvedContradiction` caveat. Source: `src-tauri/src/services/claims.rs:5810-5995`.
3. **Supersession path.** A `supersedes` commit demotes the old claim with `demotion_reason = 'superseded'`; the superseded row is dormant and not current. Source: `src-tauri/src/services/claims.rs:6150-6295`.
4. **Feedback path.** User feedback (`MarkOutdated`) triggers `FreshnessRefresh` or `HiddenFromCurrent`; a **post-feedback render attempt** must prove the old active claim is hidden absent newer revalidation, even when the claim is NOT already superseded by other evidence. This catches the case where supersession masks feedback vacuity. Sources: `src-tauri/abilities-runtime/src/abilities/feedback.rs:238-251`, `src-tauri/src/services/claims.rs:8102-8175`.

All four must be exercised in bundle 14, not "either/or."

### 4.3 Fixture Shape

Bundle 14 should seed a single synthetic account and meeting using generic domains and people only. No real customer names, domains, or email addresses belong in the fixture.

Required fixture files follow the loader contract:

- `clock.txt` fixes the test clock. Use a May 2026 timestamp so "resolved yesterday" and "six months old" have deterministic ages.
- `seed.txt` fixes randomization.
- `state.sql` seeds account, meeting, subject, old claim, fresh resolving claim/evidence, source rows, feedback rows, and any claim contradiction/supersession rows needed for the test.
- `inputs.json` drives `prepare_meeting`, `get_entity_context`, `get_daily_readiness`, and the post-action feedback/enrichment step through the same harness shape used by sibling bundles.
- `provider_replay.json` must include an attempted stale-current talking point so the assembler validation can prove stale advice is rejected, suppressed, or timestamp-qualified.
- `external_replay.json` pins the downstream object semantics: recent fetch/ingestion can point at an old downstream source, but `source_asof` remains old.
- `expected_output.json` asserts current rendering.
- `expected_provenance.json` asserts source attribution, source age, trust caveats, warning posture, and child composition.
- `expected_state.json` asserts post-action claim states, supersession/contradiction edges, feedback effects, and no resurrection.
- `metadata.json` includes `bundle: 14`, a stable scenario id such as `stale-current-contradiction`, `surfaces_exercised` covering all required surfaces, dominant factors including freshness, contradiction, source semantics, and user feedback, and a pass/fail definition that fails if stale current advice renders as current state. Sources: `src-tauri/src/harness/loader.rs:11-23`, `src-tauri/src/harness/types.rs:39-79`, `src-tauri/tests/fixtures/bundle-README.md:8-27`.

### 4.4 Seeded Scenario

The bundle should model the DOS-289 fixture requirement:

> "seed stale-current bundle: old open risk, fresh resolving evidence, generated briefing attempt, account and meeting surfaces, user outdated feedback."

Minimum rows:

- **Old open risk:** an active claim for `account:example-account` with `claim_type` and `field_path` appropriate for current risk/open-loop state, `temporal_scope = state`, `source_asof` about six months before the fixed clock, and source metadata showing the downstream object date is old even if observed/ingested later. Source-asof semantics are required by ADR-0105. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:391-437`.
- **Fresh resolving evidence:** a newer same-subject/same-field claim or evidence row with `source_asof` yesterday and a resolved/mitigated state. If it is committed through `supersedes`, the old claim must become dormant with `demotion_reason = 'superseded'`; if it is a cross-source contradiction, the fixture must include or exercise the reconciliation path so current surfaces do not silently pick stale evidence. Sources: `src-tauri/src/services/claims.rs:5810-5995`, `src-tauri/src/services/claims.rs:6150-6295`.
- **Briefing attempt:** provider replay attempts to produce a current talking point from the old open risk. `prepare_meeting` must reject, omit, or qualify that candidate because it only accepts candidates with known source ids, subject fit, source lifecycle checks, and field attribution to the source. Source: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:959-1026`.
- **Account/entity surface:** `get_entity_context` must show the current resolved/mitigated state as current and move the stale open state to history/qualification or omit it from current entries. It reads claim-backed entries and sets source trust band per claim. Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`.
- **Meeting surface:** `prepare_meeting` reads claim-backed context, composes `get_entity_context`, carries evidence `source_asof`, trust band, temporal scope, and emits stale-source warnings for old evidence. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:222-299`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:526-545`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1061-1104`.
- **Daily readiness surface:** `get_daily_readiness` composes `prepare_meeting` and `get_entity_context`, carries `source_asof` on overnight changes, risk shifts, and open loops, and attributes the final narrative to children/context sources. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:14-37`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:80-100`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:141-195`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:593-665`.
- **User outdated feedback:** mark the old claim outdated and assert it becomes hidden from current state or freshness-refreshed to the fresher claim. If no fresher claim exists, it must still leave current surfaces dormant/pending refresh instead of reusing old evidence. Sources: `src-tauri/abilities-runtime/src/abilities/feedback.rs:238-251`, `src-tauri/src/services/claims.rs:8102-8175`.

### 4.5 Cross-Surface Agreement Rule

Bundle 14 should not duplicate DOS-290's general cross-surface consistency work, but it must assert agreement for this stale-current scenario because DOS-289 explicitly requires account-detail, meeting-briefing, and daily-readiness agreement.

**Comparison oracle (normalized field-level diff, not string containment):**

For the seeded subject (`account:example-account`), bundle 14 extracts the following fields from each of the three abilities and asserts equality across surfaces, with explicit allowed-divergences listed:

| Field | get_entity_context | prepare_meeting | get_daily_readiness | Allowed divergence |
| --- | --- | --- | --- | --- |
| primary subject id | `subject.entity_id` | `attendee_context[].entity_id` (matching subject) | `surfaces.account[].entity_id` | None |
| current-state claim id | `entries[].claim_id` where `claim_type='current_state'` | `topics[].source_claim_ids` | `overnight_changes[].source_claim_ids` | None |
| current-state trust band | `entries[].trust_band` | `topics[].trust_band` | `overnight_changes[].trust_band` | None |
| `source_asof` of current-state claim | `entries[].source_asof` | `topics[].source_asof` | `overnight_changes[].source_asof` | Format only — Unix vs ISO |
| claim_state of stale row | `entries[].claim_state` | (not surfaced directly; assert via `source_claim_ids` set) | (not surfaced directly) | Absence vs explicit dormant is equivalent |

Assertion: for every shared field above, the three ability outputs return equal values. Bundle 14 fails if any cell diverges silently. String containment is **not** an acceptable substitute.

Grounded in the ability composition model: `prepare_meeting` composes `get_entity_context`, and `get_daily_readiness` composes both. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:14-32`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:14-37`, `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`.

### 4.6 Trust And Lint Assertions

Bundle 14 must include two classes of assertions:

- **Trust assertions (tightened):** the stale open claim's effective trust band on the rendered output must satisfy **one of two substrate conditions**, asserted directly on the band enum value (NOT on caveat strings):
  - **(a) Band downgrade:** `trust_band != TrustBand::LikelyCurrent` when an active unresolved contradiction exists; band is `UseWithCaution` or `NeedsVerification` carrying the `UnresolvedContradiction` cause.
  - **(b) Lifecycle demotion:** if supersession won, the stale row's `claim_state` is dormant/superseded and the stale row does not appear in current-state output at all.
  - An implementation that adds a caveat string while keeping `band = LikelyCurrent` and the stale row active **must fail this assertion**. Sources: `src-tauri/abilities-runtime/src/abilities/trust/types.rs:23-94`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:39-79`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:320-405`.
- **Lint assertions:** release-gate/harness lint must fail when stale and fresh contradictory current-state claims both render as current without suppression, supersession, or timestamped disagreement. The existing release gate already converts bundle status and invariant failures into mandatory gate failures; W6-B implementation must add bundle-14 status/invariant coverage so the lint participates in the W6/W7 mandatory gate. Sources: `src-tauri/src/release_gate.rs:720-766`, `src-tauri/src/release_gate.rs:1509-1543`, `.docs/plans/v1.4.1-waves.md:653-663`.

### 4.7 Intelligence Loop Check

- **Claim model:** no display-only stale/current string is sufficient. Bundle 14 must assert claim lifecycle, `source_asof`, `temporal_scope`, contradiction/supersession edges, and feedback semantics. Sources: `src-tauri/src/services/claims.rs:5810-5995`, `src-tauri/src/services/claims.rs:6150-6295`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:50-54`.
- **Provenance and trust:** every output under test must expose source attribution and trust posture through the ability provenance wrapper; no domain output should invent a parallel provenance field. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:143-182`, `.docs/decisions/0105-provenance-as-first-class-output.md:103-158`.
- **Signals and invalidation:** supersession, contradiction reconciliation, and freshness refresh already bump subject/claim invalidation in the claims service; bundle 14 expected state should assert changed claim ids and no resurrection after feedback. Sources: `src-tauri/src/services/claims.rs:5954-5995`, `src-tauri/src/services/claims.rs:6268-6286`, `src-tauri/src/services/claims.rs:8171-8175`, `src-tauri/src/services/claims.rs:8527-8568`.
- **Runtime and surfaces:** required consumers are `prepare_meeting`, `get_entity_context`, and `get_daily_readiness`; Tauri and MCP parity follows ability registry invocation rather than per-surface custom logic. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`, `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`.
- **Feedback loop:** `MarkOutdated` must keep the old fact as historical but hidden from current surfaces unless a newer source re-validates it. Sources: `src-tauri/abilities-runtime/src/abilities/feedback.rs:238-251`, `src-tauri/src/services/claims.rs:8102-8175`.

## 5. Acceptance Criteria

DOS-289 Acceptance, quoted verbatim:

> "prepare_meeting does not generate current talking points from superseded historical claims; get_entity_context distinguishes current vs historical; TrustAssessment reflects contradiction or supersession not just age; lint flags stale-current contradictions; account-detail and meeting-briefing and daily-readiness agree on current state or show timestamped disagreement."

Testable decomposition:

1. **`prepare_meeting` stale-current suppression.** Given an old open-risk claim and fresh resolving evidence, provider replay may attempt a stale current talking point, but final `topics`, `attendee_context`, `open_loops`, `what_changed_since_last`, and `suggested_outcomes` must not present the old risk as current advice. Candidate validation and source attribution are mandatory. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:137-146`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:959-1026`.
2. **`prepare_meeting` source semantics.** The stale claim's age is computed from `source_asof`, not ingestion time; stale evidence may warn or qualify but cannot look fresh because it was observed recently. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1061-1104`.
3. **`get_entity_context` current vs historical split.** The current entry set must favor the fresh resolved/mitigated state or explicitly render timestamped disagreement; the stale open-risk row can appear only as historical/qualified context, not as current fact. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:522-606`.
4. **Claim lifecycle and contradiction state.** Expected post-action state asserts either old claim dormant/superseded by the fresh claim, or unresolved contradiction with current surfaces refusing to silently select the stale side. Sources: `src-tauri/src/services/claims.rs:5810-5995`, `src-tauri/src/services/claims.rs:6150-6295`.
5. **TrustAssessment reflects contradiction/supersession on the band enum, not on caveat strings.** The stale claim's effective `trust_band` enum value must satisfy condition (a) or (b) from §4.6: either band ∈ {`UseWithCaution`, `NeedsVerification`} with `UnresolvedContradiction` cause, OR the stale row is non-current/superseded and does not appear in current-state output. An implementation that adds a caveat string while keeping `band = LikelyCurrent` must fail. Sources: `src-tauri/abilities-runtime/src/abilities/trust/types.rs:23-94`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:204-260`, `src-tauri/abilities-runtime/src/abilities/trust/mod.rs:320-405`.
6. **Lint flags stale-current contradiction.** A harness or release-gate assertion fails if old-open and fresh-resolved current-state claims both render as current without suppression, supersession, or timestamped disagreement. Sources: `src-tauri/src/release_gate.rs:720-766`, `src-tauri/src/release_gate.rs:1509-1543`.
7. **Account-detail/entity, meeting briefing, and daily readiness agree.** For the seeded account and meeting, `get_entity_context`, `prepare_meeting`, and `get_daily_readiness` must all render the same current state or all show timestamped disagreement. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:14-32`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:14-37`.
8. **User outdated feedback prevents reuse (substrate effect, not masked by supersession).** The bundle includes a feedback-only scenario where the old claim is NOT already superseded by other evidence, so `MarkOutdated`'s effect is observable in isolation. After `MarkOutdated`, expected state asserts the feedback row exists, `FreshnessRefresh` or `HiddenFromCurrent` is applied, and a post-feedback render attempt of `prepare_meeting` or `get_entity_context` does NOT include the marked claim in current output. This prevents the failure mode where supersession independently demotes the claim and the feedback path is never actually exercised. Sources: `src-tauri/abilities-runtime/src/abilities/feedback.rs:238-251`, `src-tauri/src/services/claims.rs:8102-8175`.
9. **Historical case study remains historical.** If the fixture includes a historical example/source, it must use temporal scope or render policy so it cannot become current state. Sources: `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:50-54`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:173-180`.
10. **Bundle 14 is mandatory — the W6-B PR itself flips the mandatory bit.** Implementation does not "coordinate with W7" or defer the wiring. The W6-B PR includes the edit to `src-tauri/src/release_gate.rs:26-38` (and any other release-gate mandatory-set declaration) that promotes bundle 14 from tracked to mandatory. If that edit is missing, L2 review on the W6-B PR rejects. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-289 content is supplied verbatim in the authoring prompt for this packet. No Linear connector lookup was required to draft V1.
- **Upstream unblock:** W6 starts after the W3 stage-3b precondition, as amended to instrumentation-complete rather than full data-sufficiency closure. Sources: `.docs/plans/v1.4.1-waves.md:653-655`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Adjacent W6 coordination:** W6-B owns bundle 14; W6-C/D/E/F own bundles 15/16/17/18 by wave-plan numbering, with DOS-292 explicitly owning bundle 17. Sources: `.docs/plans/v1.4.1-waves.md:632-651`, `.docs/plans/v1.4.1-waves.md:829-831`.
- **Release-gate coordination:** W6-B must produce bundle metadata and invariant names early enough for W7/release-gate wiring because current release-gate defaults still only mark bundles 1, 5, and 13 mandatory. Sources: `src-tauri/src/release_gate.rs:26-38`, `.docs/plans/v1.4.1-waves.md:653-663`.
- **Not a DOS-290 takeover:** Cross-surface consistency is tested only for the stale-current scenario because DOS-289 Acceptance explicitly requires account-detail, meeting-briefing, and daily-readiness agreement. General cross-surface consistency remains W6-C. Source: `.docs/plans/v1.4.1-waves.md:632-635`.
- **Not a DOS-292 takeover:** Source lifecycle/privacy/actor-specific provenance remains W6-E, and W6-E is the security-auditor lane. Source: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** W6 merge gate requires L0 plan approvals with `qa-expert` for all six W6 agents. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Security reviewer:** not required for W6-B. The wave gate names `security-auditor` only for DOS-292, and DOS-292 is W6-E. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.
- **Review focus for `qa-expert`:**
  - Bundle 14 assignment and naming are unambiguous.
  - Fixture catalogue/harness shape is followed.
  - DOS-289 edge cases are represented by fixture state, replay, expected output, expected provenance, and expected state.
  - `prepare_meeting`, `get_entity_context`, and `get_daily_readiness` are all asserted.
  - Trust, contradiction/supersession, and user feedback are asserted, not just stale age.
  - Bundle 14 can become mandatory in the W6/W7 release gate with no quarantine.

## 8. L0 Acceptance Gate

L0 passes only if the reviewer accepts all of the following:

1. **Problem fit:** the plan tests stale-current contradiction and temporal truth, not generic stale-source warnings.
2. **Bundle lock:** W6-B is locked to bundle 14 and implementation path `src-tauri/tests/bundle14_stale_current_contradiction_substrate_test.rs`.
3. **Fixture lock:** bundle directory is `src-tauri/tests/fixtures/bundle-14/`, using the loader-required files and `metadata.json` fields. Sources: `src-tauri/src/harness/loader.rs:11-23`, `src-tauri/tests/fixtures/bundle-README.md:8-27`.
4. **Amendment acknowledgement:** Amendment 1 is acknowledged, and the packet does not treat stage-3b residual work as a W6-B blocker beyond the amended instrumentation-complete baseline. Source: `.docs/plans/v1.4.1-waves-amendments.md:37-75`.
5. **Acceptance coverage:** every clause of DOS-289 Acceptance is decomposed into a testable bundle assertion in Section 5.
6. **Runtime parity:** required assertions exercise ability outputs and provenance, not frontend-only display fixtures. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
7. **No invented ADRs:** no standalone freshness-decay ADR is cited; ADR-0105, ADR-0114, and ADR-0125 are the binding freshness/temporal anchors. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`, `.docs/decisions/0114-scoring-unification.md:1-49`, `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:156-198`.
8. **Reviewer panel:** `qa-expert` is the only required L0 reviewer; no `security-auditor` is listed for W6-B.
9. **No PII:** all fixture examples are synthetic and generic.

## 9. Out-Of-Scope

- Writing implementation files in this packet authoring turn.
- Committing changes.
- Creating schema migrations unless implementation proves no existing claim/lifecycle/trust field can encode the required state. If a schema/table/column is proposed later, the full Intelligence Loop check is mandatory before implementation.
- Building a new user-facing contradiction UI.
- General DOS-290 cross-surface consistency beyond the stale-current scenario.
- DOS-291 ambiguous identity and primary-context selection.
- DOS-292 source lifecycle, privacy, actor-specific provenance, and the security-auditor lane.
- DOS-293 sync, refresh, concurrency, and partial-failure recovery.
- Treating W6-B as a broad freshness-tuning effort. This bundle proves fresh contradictory/resolving evidence wins over stale current-state claims.
- Adding customer-specific names, domains, emails, or account details to fixtures.

## 10. Changelog

- **V1 - 2026-05-15:** Initial W6-B L0 packet. Assigned DOS-289 to bundle 14, cited the W6 plan and Amendment 1, grounded fixture architecture in the bundle catalogue/harness, mapped ability and trust runtime anchors, quoted DOS-289 problem/user-harm/required-behavior/acceptance, decomposed acceptance criteria, and listed `qa-expert` as the only required L0 reviewer.
