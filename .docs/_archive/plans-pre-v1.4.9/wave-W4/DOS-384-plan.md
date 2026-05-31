# Implementation Plan: DOS-384

v1 (2026-05-04) — initial L0 draft

## Revision history
- v1 (2026-05-04) — initial L0 draft.

## 1. Contract restated

DOS-384 defines the canonical adversarial bundle catalog for bundles 2, 3, 4, 6, 7, and 8. It is a research+design issue, not the harness implementation and not trust compiler code. The Linear contract is explicit: define adversarial scenario specifications, `state.sql` seeds, `inputs.json` ability invocations, `expected_output.json`, `expected_state.json`, `expected_provenance.json`, and anonymization requirements for each bundle so DOS-216 can claim coverage at v1.4.0.

This is a v1.4.0 scope expansion. The original roadmap deferred bundles 2-4 and 6-8 to v1.4.1; the L0 review cycle-1 user ruling brought them into v1.4.0 so the harness can report all eight adversarial bundles end to end. DOS-216 remains the harness owner, but its reporting claims for these six bundles are blocked until DOS-384 defines the canonical catalog.

ADR-0110 is the fixture contract. The required files are `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `clock.txt`, `seed.txt`, `expected_output.json`, and `expected_provenance.json`; DOS-216 v2.1 adds `metadata.json` and `expected_state.json` for bundle metadata and post-run DB/trust assertions. DOS-384 should follow that additive shape and must not invent alternate fixture file names.

Load-bearing scope: each bundle gets one behavior-level claim, one canonical substrate mechanism, one PASS/FAIL definition, and one fixture spec that can be consumed by DOS-216. Bundle 1 remains represented by `src-tauri/tests/dos287_substrate_bundle1_reproduction.rs` and later DOS-5 trust assertions. Bundle 5 remains DOS-283/W6-A-owned. DOS-384 only covers bundles 2-4 and 6-8.

Current trust-factor names come from DOS-5 v2.1 and must be used exactly: canonical factors are `source_reliability`, `freshness_weight`, `corroboration_weight`, `contradiction_penalty`, and `user_feedback_weight`; trust-local helpers are `subject_fit_confidence` and `cross_entity_coherence`. Initial hypothesis labels for internal consistency, feedback, or temporal freshness are refined below into these approved names rather than creating new factors.

## 2. Approach

Author a catalog specification that can later materialize into bundle fixture directories aligned with DOS-283's bundle-corpus shape: `src-tauri/tests/fixtures/bundles/bundle_N/`, unless L0 reviewers require Linear's shorter `tests/fixtures/bundle-N/` wording literally. Each bundle directory should contain ADR-0110 files plus DOS-216's `metadata.json` and `expected_state.json`. Ability-specific wrappers under `src-tauri/tests/abilities/{ability}/fixtures/` remain DOS-216/W5-owned consumers, not the canonical bundle source.

Each bundle metadata entry carries `bundle`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, `labels`, `anonymization_cert`, and the trust-factor expectations needed for harness coverage reporting. Behavior-level claims use this convention: "substrate prevents <pathology> from becoming an active/rendered claim for <subject/surface>." This keeps assertions user-visible and avoids overfitting to one implementation detail.

Acceptance criteria per bundle:
- `state.sql` seeds only synthetic accounts, people, meetings, source rows, claims, feedback, corroborations, contradictions, temporal scope, sensitivity, and provenance inputs needed to reproduce the pathology.
- `inputs.json` invokes the ability path that would expose the pathology, such as entity context, meeting prep, account assessment, or public assessment rendering.
- `expected_state.json` asserts trust band/state, claim lifecycle, feedback/tombstone suppression, visibility, or no-new-claim conditions.
- `expected_provenance.json` asserts source attribution, `source_asof`, subject attribution, warnings, and redacted source classes.
- `expected_output.json` asserts the rendered or ability output omits rejected content and includes a warning/band only when the surface is supposed to render one.
- PASS means the named trust factor/helper or sensitivity filter flags the pathology and the content is rejected, suppressed, or rendered as non-authoritative. FAIL means the bleed lands as active/current public content.

Anonymization gate: every sketch uses `acme.example.com`, `subsidiary.example.com`, `person_N@example.com`, and deterministic IDs such as `dos384-b2-*`. No real customer domains, names, emails, source IDs, transcripts, or account details appear in SQL, JSON, comments, or failure messages.

Bundle 2 pathology: provider hallucination with no cross-entity contamination signal. The provider invents a claim that contradicts the available `source_asof` facts for the target, has no corroborating child rows, and is not explained by a foreign-domain/person hit. The substrate catches it through low `source_reliability`, floor-level `corroboration_weight`, and active `contradiction_penalty`; any "internal consistency" observation is recorded as an expected-output/provenance warning, not a DOS-5 factor.

Bundle 2 fixture spec:
- `state.sql`: seed one target account `dos384-b2-acme` and authoritative CRM/source evidence as of `2026-04-30T00:00:00Z` saying renewal is on track.
- `inputs.json`: invoke the account Read/assessment ability for `dos384-b2-acme`.
- `provider_replay.json`: record the provider completion that claims renewal is blocked by a nonexistent procurement freeze.
- `expected_state.json`: assert no active claim for the hallucinated freeze, trust band at `NeedsVerification` or equivalent non-current state for the candidate, `source_reliability` below threshold, `corroboration_weight` at floor, and `contradiction_penalty` applied.
- `expected_provenance.json`: assert the contradiction points to the authoritative source and no foreign-subject contamination warning is required.
- `expected_output.json`: omit the freeze from the rendered assessment.
- PASS/FAIL: PASS rejects the hallucinated claim; FAIL persists or renders it as current.

Bundle 3 pathology: stale-source resurrection from a withdrawn or dismissed claim. The provider returns content that once had evidence but was later dismissed by feedback or withdrawn by source lifecycle, making it tempting to resurrect during refresh. The substrate catches it through `user_feedback_weight` plus `freshness_weight`, using the tombstone/dismissal time and `source_asof` ordering rather than a generic stale-data count.

Bundle 3 fixture spec:
- `state.sql`: seed target account `dos384-b3-acme`, an old claim "pilot expansion is blocked" with `source_asof` before `dismissed_at`, and a typed feedback row representing dismissal/wrong-source suppression.
- `inputs.json`: invoke a refresh/Read path that would gather current account context.
- `external_replay.json`: record the stale external/source row that repeats the dismissed text.
- `expected_state.json`: assert the old claim remains dismissed/dormant, no new active paraphrase is created, `user_feedback_weight` downranks the candidate, and `freshness_weight` reflects stale `source_asof`.
- `expected_provenance.json`: include the tombstone/feedback contribution and stale-source warning.
- `expected_output.json`: do not surface the dismissed claim.
- PASS/FAIL: PASS preserves the dismissal; FAIL creates a new active claim or renders the stale content.

Bundle 4 pathology: cross-entity ambiguity through the same person across two accounts, not the same-domain account bleed already covered by bundle 1. A person belongs to account A and account B with different roles or relationship states; an assessment for A pulls B-context because the person identifier/name is shared. The substrate catches it through `cross_entity_coherence` and `subject_fit_confidence` on Person subjects, not through account-domain `TargetFootprint` alone.

Bundle 4 fixture spec:
- `state.sql`: seed `dos384-b4-acme` and `dos384-b4-subsidiary`, one person `dos384-b4-person-alex` linked to both accounts, account-specific stakeholder roles, and a B-only source saying Alex is a blocker for `subsidiary.example.com`.
- `inputs.json`: invoke the account A stakeholder assessment or entity-context Read ability.
- `expected_state.json`: assert no account A active claim imports the B-only blocker context, and record `cross_entity_coherence` / `subject_fit_confidence` evidence against the candidate.
- `expected_provenance.json`: assert subject attribution distinguishes account A person-context from account B person-context.
- `expected_output.json`: keep Alex's A-specific role but omit B-only blocker language.
- PASS/FAIL: PASS rejects B-context for A; FAIL renders or persists the B-only assessment under A.

Bundle 6 pathology: adversarial corroboration spam. Many weak or same-origin sources repeat a false claim while one high-reliability source contradicts it. A naive count-based system would treat "n corroborations" as confidence; the substrate must weight source reliability and contradictions so spam does not outvote the strong source. The catch mechanism is `source_reliability` weighting combined with `corroboration_weight` strength semantics and `contradiction_penalty`.

Bundle 6 fixture spec:
- `state.sql`: seed target account `dos384-b6-acme`, five low-reliability source rows or corroborations from the same synthetic source family claiming "support escalation is unresolved", and one high-reliability source as of the same or newer date saying the escalation is closed.
- `inputs.json`: invoke account context or briefing generation.
- `expected_state.json`: assert the false claim is not active/current, weak corroborations do not produce a high `corroboration_weight`, high-reliability contradiction applies `contradiction_penalty`, and the final trust band is non-current.
- `expected_provenance.json`: show weak source grouping/classes and the strong contradictory source.
- `expected_output.json`: either omit the false escalation or render a cautious contradiction note per surface policy.
- PASS/FAIL: PASS prevents corroboration spam from landing; FAIL lets weak-source count dominate.

Bundle 7 pathology: temporal scope violation. A claim with `temporal_scope = closed` is re-corroborated by stale data and treated as current again. The substrate catches it through `freshness_weight` using `source_asof` and through temporal-scope semantics from W3-H/DOS-300; `temporal_scope` is an input/claim field, not a DOS-5 factor name.

Bundle 7 fixture spec:
- `state.sql`: seed a closed claim such as "security review completed" for `dos384-b7-acme`, with closure timestamp and later current state showing no open issue, plus stale source rows whose `source_asof` predates closure.
- `inputs.json`: invoke a meeting prep or account summary ability that might mention open risks.
- `provider_replay.json`: record replay data that restates the old open-review phrasing.
- `expected_state.json`: assert the closed claim remains closed, no current open-risk claim is created, and `freshness_weight` downranks stale re-corroboration.
- `expected_provenance.json`: link the stale source dates to the closed scope.
- `expected_output.json`: do not render the issue as currently open.
- PASS/FAIL: PASS honors closed temporal scope; FAIL reopens or re-renders stale closed content as current.

Bundle 8 pathology: sensitivity-class leak. A public-class assessment contains private-class claim content because the underlying facts are true and well-sourced but not allowed for that output class. DOS-5 v2.1 does not name a sensitivity trust factor; DOS-384 should specify this as sensitivity-aware filtering tied to W3-H `sensitivity` and surface render policy, with an open question for whether DOS-5 adds a named local factor or keeps it outside trust scoring.

Bundle 8 fixture spec:
- `state.sql`: seed target account `dos384-b8-acme`, a private sensitivity claim such as a named personnel concern or confidential commercial term, and a public-class claim with similar topic but sanitized content.
- `inputs.json`: invoke a public assessment/export or MCP-rendered Read surface.
- `expected_state.json`: assert the private claim remains stored with private sensitivity but is not selected for public output; if DOS-5 later owns a factor, this file records the L0-approved factor name rather than inventing one.
- `expected_provenance.json`: assert sensitivity class is present in provenance/render metadata without leaking private text.
- `expected_output.json`: contain only public-class content or a redacted/private-omitted marker allowed by the surface.
- PASS/FAIL: PASS filters private content from public assessment; FAIL leaks private claim text.

## 3. Key decisions

PASS/FAIL definition: use behavior-level outcome plus trust/filter evidence, not raw factor math alone. PASS requires both a flagged/downranked candidate and rejected/suppressed output. FAIL is any active/current claim, rendered public text, or persisted assessment that carries the bundle pathology.

Trust-factor naming: map hypotheses onto DOS-5 v2.1 names. Bundle 2 uses `source_reliability`, `corroboration_weight`, and `contradiction_penalty`. Bundle 3 uses `user_feedback_weight` and `freshness_weight`. Bundle 4 uses `cross_entity_coherence` and `subject_fit_confidence`. Bundle 6 uses `source_reliability`, `corroboration_weight`, and `contradiction_penalty`. Bundle 7 uses `freshness_weight` plus temporal-scope input semantics. Bundle 8 has no approved DOS-5 factor name yet.

Bundle 8 sensitivity handling: v1 does not mint a new factor. It records a required sensitivity-aware filtering invariant and leaves the naming/placement decision to L0/DOS-5. Acceptable outcomes are either a new L0-approved DOS-5 local factor, or a non-trust render/filter policy that DOS-216 can still assert through `expected_state.json` and `expected_output.json`.

Bundle 4 target shape: extend bundle 1's cross-entity concept to Person subjects rather than reuse only account-domain `TargetFootprint`. The substrate must include person membership/role/account edges so same-person/multi-account context is explicit. If DOS-5's `TargetFootprint` cannot express Person subjects, DOS-384 should file a DOS-5 dependency rather than weakening the bundle.

Fixture path: prefer `src-tauri/tests/fixtures/bundles/bundle_N/` for canonical bundle catalog consistency with DOS-283. Ability-specific ADR-0110 fixtures may wrap these bundles later, but the catalog source should not be duplicated by each ability.

## 4. Security

All fixture content is synthetic. Use `acme.example.com`, `subsidiary.example.com`, `parent.example.com`, `person_N@example.com`, and deterministic DOS-384 IDs only. CLAUDE.md line 18 forbids customer-specific data in code, comments, and test fixtures; DOS-384 should treat plan examples as part of that boundary.

Do not check in `fixture_identity_map.json`, real source IDs, copied customer text, transcripts, names, or domains. Anonymization failures are hard failures. Failure messages report bundle ID, row ID, JSON pointer, source class, sensitivity class, and trust factor/helper name; they do not print private claim text or raw source excerpts.

Bundle 8 adds a privacy-specific risk: expected provenance must prove the private source/claim was filtered without rendering the private content. Use redacted hashes/classes in `expected_provenance.json` and public-safe placeholders in `expected_output.json`.

## 5. Performance

DOS-384 itself is design-only. The later fixture implementation should keep seed and harness execution cheap enough for DOS-216's 30-60 second hermetic eval budget. Each bundle seed should load in under a few seconds on an in-memory SQLite DB, and each bundle harness execution target is less than 60 seconds including replay parsing and provenance/state diffing.

Keep fixture volume minimal: enough rows to trigger the pathology, not a large synthetic workspace. Bundle 6 needs multiple weak corroborations, but the number should be small and explicit, such as five weak rows versus one strong contradiction. No live provider, network, migration, or external client calls are required.

## 6. Coding standards

Synthetic data only. Fixture SQL/JSON must be deterministic, idempotent, ASCII except where an existing required header or document style uses non-ASCII, and namespace-prefixed with `dos384-bN-*`. No direct production code changes are in DOS-384's design scope.

If implementation later adds Rust helpers or tests, clippy budget is zero warnings. Helpers should follow the existing `dos287_substrate_bundle1_reproduction.rs` pattern: in-memory DB, explicit seed function, behavior-level assertion, and no production side effects. Do not add migrations, run live services, or widen service write paths for these fixtures.

## 7. Integration with parallel wave-mates

DOS-216 consumes DOS-384 as the source of bundle coverage for 2-4 and 6-8. DOS-216 owns harness mechanics, fixture discovery, scoring, regression classification, and reporting. DOS-384 owns the canonical scenario and expected-artifact design that makes those rows meaningful.

DOS-5 owns trust factors and any factor evidence. DOS-384 must not define new trust names. If bundle 8 needs sensitivity represented inside Trust Compiler rather than render filtering, DOS-384 should feed that decision back to DOS-5/L0 before implementation.

DOS-283/W6-A owns bundle 5 and provides the reference bundle-catalog shape for SQL-first fixture data plus expectation sidecars. Bundle 1 remains anchored by `src-tauri/tests/dos287_substrate_bundle1_reproduction.rs` and DOS-5's later migration from contamination rejection to `cross_entity_coherence`.

W3-B/DOS-211 supplies provenance/subject attribution and warnings. W3-C/DOS-7 supplies claim rows, corroborations, contradictions, and feedback. W3-H/DOS-300 supplies `temporal_scope` and `sensitivity`. DOS-384 expected JSON should track those final shapes rather than create compatibility schemas.

## 8. Failure modes + rollback

If a pathology cannot be reproduced without inventing schema or ability behavior, mark that bundle blocked and defer that one bundle to v1.4.1 with a concrete dependency, rather than weakening the PASS/FAIL definition. DOS-216 may still ship harness mechanics but must not claim coverage for the blocked bundle.

If trust-factor names or evidence shapes change in DOS-5 before implementation, update DOS-384 expected-state specs to the new approved names. Do not keep stale hypothesis labels in fixture metadata.

If sensitivity filtering lacks an owner, bundle 8 remains an open L0 decision. Rollback is to keep bundle 8 out of DOS-216's reported v1.4.0 coverage until the owner is assigned, not to include a fixture that leaks or silently ignores private content.

If anonymization fails, delete or regenerate the candidate fixture. No production state, migration, or source lifecycle cleanup exists because DOS-384 should only author synthetic checked-in fixtures.

## 9. Test evidence

Each bundle should add a behavior-level substrate test named like the DOS-287 precedent, with bundle number and pathology in the name:
- `bundle2_provider_hallucination_substrate_test`
- `bundle3_stale_source_resurrection_substrate_test`
- `bundle4_person_cross_entity_ambiguity_substrate_test`
- `bundle6_adversarial_corroboration_spam_substrate_test`
- `bundle7_temporal_scope_violation_substrate_test`
- `bundle8_sensitivity_class_leak_substrate_test`

Harness/governance evidence should include DOS-216's fixture loader checks over every bundle directory, anonymization checks rejecting non-`example.com` identities, and report rows proving bundles 2, 3, 4, 6, 7, and 8 are counted separately. The report must not collapse these into generic "edge" coverage.

Per-bundle evidence:
- Bundle 2 asserts a contradicted, uncorroborated hallucination does not become active or rendered.
- Bundle 3 asserts dismissed/withdrawn stale content does not resurrect after refresh.
- Bundle 4 asserts same-person, two-account B-context does not bleed into A.
- Bundle 6 asserts weak corroboration spam does not outvote a strong contradiction.
- Bundle 7 asserts closed temporal scope is not reopened by stale source data.
- Bundle 8 asserts private sensitivity content is filtered from public-class output.

Wave merge-gate artifact: DOS-216's `target/eval/harness-report.json` should show coverage rows for bundles 2-4 and 6-8 only after these fixtures exist and pass. Until then, DOS-216 must report them as missing/blocked.

## 10. Open questions

1. Fixture path wording: Linear says `tests/fixtures/bundle-{2,3,4,6,7,8}/`, while DOS-283 plans `src-tauri/tests/fixtures/bundles/bundle_1/` and `bundle_5/`. Confirm whether DOS-384 should use the DOS-283 catalog path or the literal Linear shorthand.
2. Bundle 8 owner: should sensitivity-aware filtering become a DOS-5 local factor with an L0-approved name, or remain a W3-H/W6 render policy asserted outside trust scoring?
3. Bundle 4 model shape: confirm DOS-5 `TargetFootprint` or its successor supports Person subjects and account-specific person context without stretching account-domain contamination logic.
4. Exact ability invocations: confirm which first W5 ability consumes each bundle so `inputs.json` can target real registry names rather than placeholders.
5. Expected provenance shape: DOS-384 should wait for DOS-211's final `SubjectAttribution`, source warning, and sensitivity rendering fields before locking `expected_provenance.json`.
6. Trust-band thresholds: DOS-5 owns numeric weights and band mapping; DOS-384 should assert factor/helper evidence and non-current outcome, not hard-code exact scores unless DOS-5 stabilizes them.
