# DOS-292 W6-E L0 Packet V1

## 1. Context and Scope

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-E.
- **Linear issue:** DOS-292 - "Validation: source lifecycle, privacy, and actor-specific provenance."
- **Bundle:** validation bundle 17.
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md`.
- **No implementation in this pass:** no wave-plan edits, no ADR edits, no fixture edits, no test edits, no commit.
- **W6-E assignment:** the wave plan names W6-E as "DOS-292 source lifecycle + privacy + actor-specific provenance" and assigns "validation bundle 17 + assertions for revoked/restricted source rejection, actor-specific provenance routing." Source: `.docs/plans/v1.4.1-waves.md:642-646`.
- **W6-E release gate:** W6 cannot land partially; L3 requires all bundles 1-13 and 14-18 green, including bundle 17, with no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **W6 reviewer exception:** W6 L0 requires `qa-expert` for all six agents and `security-auditor` for DOS-292. W6-E is the DOS-292 lane. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.
- **Amendment posture:** W6 starts against the W3 stage-3b `instrumentation-complete` baseline, not full data-sufficiency closure. This amendment does not reduce W6 validation scope. Source: `.docs/plans/v1.4.1-waves-amendments.md:37-47`, `.docs/plans/v1.4.1-waves-amendments.md:71-75`.
- **Fixture catalogue:** committed fixture directories are hyphenated `bundle-N`; current catalogue documents bundles 1-13 and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Fixture manifest contract:** metadata must include bundle id, scenario id, invariant, expected render policy, surfaces exercised, source lifecycle refs, anonymization certificate, trust factors, pass/fail definition, and design notes. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
- **Existing lifecycle adjacent fixtures:** bundle 11 already covers stale Glean evidence and bundle 12 already covers revoked source evidence; W6-E must broaden this into actor-aware, multi-channel, lifecycle-state validation rather than duplicate those single cases. Source: `src-tauri/tests/fixtures/bundle-README.md:41-45`.
- **Harness baseline:** the release-gate harness currently expects bundles 1-13 in `loader_loads_all_committed_bundles`; bundle 17 implementation must update coverage expectations as part of the implementation PR, not this packet. Source: `src-tauri/tests/harness.rs:42-65`.
- **Anonymization baseline:** fixture governance rejects phone-like tokens and identity-map files; W6-E fixtures must stay synthetic and must not encode customer-specific identifiers. Source: `src-tauri/tests/harness.rs:1703-1749`.
- **ADR-0108 filename:** the requested `0108-sensitivity-and-render-policy.md` file is absent; the actual decision file is `.docs/decisions/0108-provenance-rendering-and-privacy.md`.
- **ADR binding:** ADR-0108 governs per-surface rendering, privacy, `ProvenanceMasked`, actor filtering, sanitization, size budgets, and rendering-required boundaries. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:9-17`.

W6-E is not an implementation feature, a new product surface, or a schema design exercise by default. It is a validation bundle and assertion set that proves the already-designed source lifecycle, actor filtering, provenance masking, and sensitivity policy behave correctly when sources disconnect, revoke, expire, restrict, or become stale.

The load-bearing scope is **privacy-preserving explainability under changing source access**:

- Claims retain enough lifecycle and provenance semantics to degrade safely.
- Tauri and MCP use the same ability/provenance policy, with actor-specific rendering.
- Customer-facing or external outputs cannot receive internal-only material.
- Redacted provenance remains useful without leaking sensitive identifiers.
- Every policy channel named by W6 is tested as a class, not as one-off output strings.

## 2. Problem Statement

DOS-292 ticket text, supplied verbatim in the authoring request:

> **Title:** Validation: source lifecycle, privacy, and actor-specific provenance.

> **Problem:** v1.4.0 handles provenance and source taxonomy, but user-visible behavior for disconnected, revoked, unavailable, restricted, or stale downstream sources is not explicit enough.

> **User Harm:** User sees claims sourced from data they can no longer access, cannot verify a claim, or gets sensitive/internal content in a customer-facing talking point.

> **Concrete Edge Cases:** Glean result points to downstream object actor cannot access; Google calendar/email source disconnected after claim creation; Slack/Gong/Zendesk source revoked or expires per retention policy; source available to Tauri user but not MCP actor; provenance redaction hides so much user cannot verify the claim; internal-only note becomes customer-facing suggestion; Clay/Glean fetch fresh but underlying CRM/Zendesk object stale.

> **Required Behavior:** claims know source lifecycle state active/unavailable/revoked/expired/restricted/stale; trust and render policy change when source lifecycle changes; provenance rendering is actor and surface aware; customer-facing suggestions cannot include internal-only content; MCP and Tauri surfaces apply same actor-filtered policy.

> **Fixture Requirements:** claims from sources that become disconnected, revoked, actor-restricted, stale at downstream object layer.

> **Acceptance Criteria:** source lifecycle changes trigger trust downgrade, invalidation, or visible degraded state; restricted provenance does not leak sensitive details; redacted provenance still gives enough source summary to be useful; MCP cannot expose source detail the app would hide; internal-only content cannot become customer-facing talking point.

The user failure is not "a source label is missing." The failure is **DailyOS continues to act as if a source is safe, current, explainable, and actor-visible after the access or lifecycle facts that made it safe have changed.**

This matters because provenance is a trust feature. ADR-0105 requires every ability output to carry a provenance envelope with temporal context, trust classification, source attribution, composition tree, and field-level attribution. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:17-58`.

This also matters because actor filtering is not optional decoration. ADR-0102 says abilities receive a declared actor so they can enforce policy, and registry enumeration is actor-scoped for MCP discovery and invocation. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:120-139`, `.docs/decisions/0102-abilities-as-runtime-contract.md:282-290`.

ADR-0108 makes the surface split concrete:

- The Tauri app can render rich provenance details for the owning user. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:21-30`.
- MCP responses include provenance by default, but the MCP wrapper strips internal graph identifiers and collapses deeper child provenance for `Actor::Agent`. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:31-40`.
- External publications carry source-class footnotes only, with no identifiers, entity names, or attribution text. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:41-47`.
- Logs reference invocation ids and do not include provenance content. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:48-53`.

The W6-E bundle must prove all of that remains true when lifecycle states change after claim creation.

## 3. Data Model and Substrate Touchpoints

### 3.1 Claim lifecycle states under test

Bundle 17 must seed or exercise claims whose source lifecycle moves across these DOS-292 states:

- `active`
- `unavailable`
- `revoked`
- `expired`
- `restricted`
- `stale`

The implementation should use existing lifecycle and provenance fields if they can encode the requirement. If implementation later needs a new table, schema column, claim field, or render-policy field, it must satisfy the Intelligence Loop integration check before shipping.

Minimum Intelligence Loop answers for any new substrate field:

- **Claim model:** the field must attach to a claim, source attribution, or provenance record rather than exist as frontend-only display state.
- **Provenance and trust:** it must describe how `source_asof`, source attribution, field attribution, and trust-band rendering change.
- **Signals and invalidation:** lifecycle changes must refresh or invalidate derived state rather than wait for a manual rerender.
- **Runtime and surfaces:** abilities, Tauri, MCP, telemetry, replay, and eval surfaces must consume the same canonical policy.
- **Feedback loop:** user corrections, dismissals, corroborations, contradictions, and access changes must feed back into claim or source state.

### 3.2 Source attribution and time

ADR-0105 defines `SourceAttribution` with `data_source`, identifiers, `observed_at`, `source_asof`, evidence weight, scoring class, and optional synthesis marker. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:160-198`.

ADR-0105 later amends `source_asof` from optional metadata to "must-be-populated-when-knowable" and defines a conservative freshness fallback chain. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`.

Bundle 17 must therefore distinguish:

- **DailyOS observed time:** when DailyOS or a provider last fetched the source.
- **Source authored time:** when the downstream object was actually authored or modified.
- **Downstream object lifecycle:** whether the underlying CRM/Zendesk/Gong/Slack/Calendar/Email object is active, unavailable, revoked, expired, restricted, or stale.
- **Actor visibility:** whether the current actor may see identifiers, summaries, details, or no content.

Clay or Glean returning fresh fetch results is not enough. The bundle must include a case where the fetch is fresh but the underlying CRM/Zendesk object is stale, and the render/trust outcome follows the downstream object semantics rather than the provider fetch timestamp.

### 3.3 Provenance masking and redaction

ADR-0108 defines `ProvenanceMasked` for revoked sources with `original_invocation_id`, original ability name, original produced timestamp, masked timestamp, mask reason, and sources masked. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:86-108`.

Masking is irreversible: once provenance is masked, original content is discarded; a reconnect requires a new invocation with new provenance. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:108-110`.

Bundle 17 must assert both sides of redaction:

- Restricted provenance does not leak sensitive details.
- Redacted provenance still provides enough source-class, lifecycle, and summary detail for the actor to understand why the output degraded.

The second bullet is not a loophole. It means "useful source summary" such as source class, lifecycle state, last-known timestamp, and redaction reason, not raw identifiers or hidden content.

### 3.4 Actor and surface policy

ADR-0102's ability context carries `actor`, and abilities use services rather than direct DB access. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:120-139`.

ADR-0102 also says external callers use erased invocation through the registry with schema validation and actor-policy enforcement. Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:264-271`.

ADR-0108 defines a universal renderer:

> `render_provenance_for(prov: &Provenance, actor: Actor, surface: Surface) -> RenderedProvenance`

and states that it consults actor and surface to determine what to include. Sources: `.docs/decisions/0108-provenance-rendering-and-privacy.md:54-72`.

Bundle 17 must cover at least these actor/surface combinations:

- `Actor::User` on the Tauri app: highest first-party detail, still respecting revocation and sensitivity.
- `Actor::Agent` through MCP default response: summary provenance, actor-filtered identifiers, no internal graph IDs.
- `Actor::Agent` through MCP detail retrieval: fuller but still actor-filtered detail.
- External or customer-facing output channel: source classes and safe summary only.
- Logs/telemetry/replay/eval: no raw source content leakage through operational channels.

### 3.5 Sensitivity-class convention

Bundle 8 is the committed sensitivity-class precedent. It loads a fixture, asserts metadata, runs through the eval bridge, and checks that a public output excludes confidential content. Source: `src-tauri/tests/bundle8_sensitivity_class_leak_substrate_test.rs:16-24`.

Bundle 8 also asserts claim sensitivity and `public_render_allowed` metadata for public and confidential claims. Source: `src-tauri/tests/bundle8_sensitivity_class_leak_substrate_test.rs:30-39`.

Bundle 8 further checks the confidence evidence records the source claim sensitivity, target surface, required max sensitivity, and score. Source: `src-tauri/tests/bundle8_sensitivity_class_leak_substrate_test.rs:41-60`.

Bundle 17 should follow this convention but widen it:

- sensitivity class alone is not enough;
- lifecycle state must be part of render and trust decisions;
- actor-specific policy must be part of render decisions;
- all nine W6 channels must be checked.

## 4. Architecture and Flow

### 4.1 Bundle assignment

W6-E owns **bundle 17**.

- **New fixture directory:** `src-tauri/tests/fixtures/bundle-17/`.
- **Expected substrate test file:** `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs`.
- **Scenario id:** `source-lifecycle-actor-provenance`.
- **Invariant:** source lifecycle, actor visibility, and sensitivity policy jointly govern trust, provenance rendering, and customer-facing output eligibility.
- **Release-gate reason:** W6 L3 requires bundles 14, 15, 16, 17, and 18 mandatory pass with no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.

### 4.2 Required wave-plan sweep

The wave plan explicitly defines W6-E done-when as:

> "validation bundle covers the full **9-channel ADR-0108 sensitivity sweep** baseline established by v1.4.0 W5 cycle-7 + DOS-412 (callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, error logs) - parameterized so any new channel introduced in v1.4.1 (Wave 8 eval/benchmark artifacts, telemetry W7-E) is automatically covered; revoked/restricted source rejection green for each channel."

Source: `.docs/plans/v1.4.1-waves.md:642-646`.

Bundle 17 must treat the nine channels as a policy registry or generated matrix, not as nine handwritten one-off assertions that future channels can bypass.

The required baseline channels are:

1. **Callouts:** user-facing generated callouts, warnings, and surfaced advice.
2. **Prep outputs:** meeting prep fields such as topics, attendee context, open loops, changes, and suggested outcomes.
3. **MCP responses:** default and detail MCP provenance/render responses for `Actor::Agent`.
4. **Tauri renders:** first-party app ability renders and provenance affordances for `Actor::User`.
5. **Signal payloads:** emitted or propagated signal bodies and metadata.
6. **Telemetry:** operational event payloads and metric dimensions.
7. **Eval fixtures:** fixture JSON, expected output, expected provenance, expected state, and reports.
8. **Replay:** provider and external replay material.
9. **Error logs:** hard errors, soft-degradation warnings, structured error output, and log lines.

The implementation gate **locks the source of truth as `RenderPolicyChannel::all()`** — a Rust enum in `src-tauri/src/bridges/types.rs` (or co-located with the render policy module) whose `all()` iterator returns every variant. The enum gets a `#[non_exhaustive]` attribute and W6-E's test imports the iterator, so adding a new variant in W7-E or W8 forces a compile-time exhaustiveness check at every test site using the iterator. The alternative "test-only matrix" approach is rejected because a test-only list rots when production code adds a channel that the test never iterated. When W7-E telemetry or W8 eval/benchmark adds a new variant, the gate fails until the new variant is classified in the render-policy map.

### 4.3 Seeded fixture scenarios

Bundle 17 should seed a compact synthetic corpus using only generic names, domains, and account ids:

- **Disconnected Google source:** calendar/email source created a claim, then disconnects after claim creation. Expected result: trust downgrade, invalidation, or visible degraded state; provenance shows a useful redacted summary.
- **Revoked Slack/Gong/Zendesk source:** source is revoked or expires per retention policy. Expected result: provenance masked or source detail suppressed; content cannot render as current prep fact or customer-facing suggestion.
- **Restricted Glean downstream object:** Glean result cites a downstream object available to the Tauri user but unavailable to the MCP actor. Expected result: Tauri can show permitted summary/detail; MCP cannot expose hidden identifiers or detail.
- **Stale downstream object behind fresh fetch:** Clay/Glean fetch is recent, but CRM/Zendesk object is stale by `source_asof` or downstream modified time. Expected result: freshness/trust follows object semantics, not fetch recency.
- **Internal-only note near public suggestion:** internal note contains plausible customer-facing wording. Expected result: public/customer-facing channel excludes it and records why.
- **Over-redacted provenance:** a case where raw details are restricted but source class, lifecycle state, timestamp posture, and redaction reason remain visible enough to verify degradation.

### 4.4 Fixture files

Bundle 17 must use the loader-required shape demonstrated by the harness minimal fixture:

- `clock.txt`
- `seed.txt`
- `state.sql`
- `metadata.json`
- `inputs.json`
- `provider_replay.json`
- `external_replay.json`
- `expected_output.json`
- `expected_provenance.json`
- `expected_state.json`

The harness minimal writer creates this exact family of required files and optional expected state in test helpers. Source: `src-tauri/tests/harness.rs:2060-2119`.

Bundle 17 metadata should include:

- `bundle: 17`
- `scenario_id: "source-lifecycle-actor-provenance"`
- `invariant` naming actor-filtered lifecycle-safe provenance
- `expected_render_policy` covering degrade, mask, suppress, or show-summary
- `surfaces_exercised` including the nine channels or the matrix name that expands to them
- `source_lifecycle_refs` for each seeded source/claim state
- `anonymization_cert: "synthetic"`
- `trust_factors_dominant` including source lifecycle, actor restriction, sensitivity, freshness, and provenance redaction
- `pass_fail_definition` failing on any restricted/revoked/internal-only leak
- `fixture_design_notes` mapping each scenario to each channel

### 4.5 Ability and renderer flow

The validating flow should run through abilities and the shared provenance renderer:

1. Seed claims, source attributions, lifecycle changes, actor policies, sensitivity classes, and expected post-action state.
2. Invoke the relevant ability path in Evaluate mode with a fixed actor and fixed clock.
3. Render provenance through actor/surface policy, not through ad-hoc string checks.
4. Compare domain output, rendered provenance, warnings, diagnostics, signal payloads, telemetry/replay/log artifacts, and expected state.
5. Repeat across the channel matrix so every channel proves revoked/restricted source rejection.

Ability outputs must carry provenance exactly once on `AbilityOutput<T>`, and domain outputs must not re-declare provenance. Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:141-182`.

Synthesized user-facing and agent-facing outputs go through abilities, and surfaces do not bypass the registry for erased invocation. Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-360`.

## 5. Acceptance Criteria

DOS-292 Acceptance, supplied verbatim in the authoring request:

> "source lifecycle changes trigger trust downgrade, invalidation, or visible degraded state; restricted provenance does not leak sensitive details; redacted provenance still gives enough source summary to be useful; MCP cannot expose source detail the app would hide; internal-only content cannot become customer-facing talking point."

Wave-plan W6-E Acceptance, quoted for the 9-channel sweep:

> "validation bundle covers the full **9-channel ADR-0108 sensitivity sweep** baseline established by v1.4.0 W5 cycle-7 + DOS-412 (callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, error logs) - parameterized so any new channel introduced in v1.4.1 (Wave 8 eval/benchmark artifacts, telemetry W7-E) is automatically covered; revoked/restricted source rejection green for each channel."

Source: `.docs/plans/v1.4.1-waves.md:642-646`.

Testable decomposition:

1. **Bundle 17 is present and mandatory.** Fixture directory `src-tauri/tests/fixtures/bundle-17/` and substrate test `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs` exist in the implementation PR, and bundle 17 participates in the mandatory W6/W7 gate. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:653-663`.
2. **Lifecycle states are represented.** Claims or source attributions in expected state cover `active`, `unavailable`, `revoked`, `expired`, `restricted`, and `stale`; the states are not frontend-only strings.
3. **Lifecycle changes degrade trust or render state.** A disconnected, revoked, expired, restricted, unavailable, or stale source causes trust downgrade, invalidation, visible degraded state, suppression, or masking according to policy.
4. **Source time beats fetch time (downstream object `source_asof` named).** A fresh Clay/Glean fetch over a stale downstream CRM/Zendesk object does not render as fresh/current. The trust-path input is the **downstream object's `source_asof`** (the CRM/Zendesk row's update timestamp), NOT the Clay/Glean wrapper's fetch time. The assertion compares the downstream-object `source_asof` from the fixture against the trust band; if the implementation uses Clay/Glean fetch time, the assertion fails. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`.
5. **Restricted provenance does not leak sensitive detail.** MCP and operational channels do not expose internal graph ids, hidden source identifiers, raw attribution text, raw prompt hashes, or internal watermarks. Sources: `.docs/decisions/0108-provenance-rendering-and-privacy.md:31-40`, `.docs/decisions/0108-provenance-rendering-and-privacy.md:70-72`.
6. **Redacted provenance remains useful.** The rendered summary includes safe source class, lifecycle/degradation state, timestamp posture, and redaction reason so the user or agent can understand the claim's degraded trust without seeing hidden content.
7. **MCP cannot expose details the actor cannot see (assertion at renderer level, not endpoint-conditional).** The assertion is on the MCP **render wrapper** (`src-tauri/src/bridges/mcp.rs:385-426`) — for `Actor::Agent`, the wrapper output cannot include fields that the same actor's default MCP response redacts. If `get_provenance(invocation_id)` endpoint exists today, it is asserted to apply the same wrapper; if the endpoint does not exist, the assertion is satisfied by the wrapper-level check alone. The assertion does not depend on the endpoint existing. Additionally, **actor identity is carried in the provenance envelope itself** (an `actor_scope` field on the envelope or attached `Actor` reference), not only in ambient request context, so the actor cannot be lost across async/MCP boundaries. Sources: `.docs/decisions/0108-provenance-rendering-and-privacy.md:31-40`, `src-tauri/src/bridges/mcp.rs:385-426`.
8. **Tauri and MCP share policy.** Differences between Tauri and MCP are produced by actor/surface rendering policy, not divergent source reads or custom surface branches. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:264-290`, `.docs/decisions/0108-provenance-rendering-and-privacy.md:54-72`.
9. **Internal-only content cannot become customer-facing.** A seeded internal-only note cannot appear in callouts, prep suggestions, external publication summaries, or any public/customer-facing channel. Source: `src-tauri/tests/bundle8_sensitivity_class_leak_substrate_test.rs:30-60`.
10. **All nine ADR-0108 channels are enumerated.** The implementation defines or uses a channel list covering callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, and error logs.
11. **The nine-channel gate is parameterized.** Adding a new render/output channel in W7-E telemetry, W8 eval/benchmark artifacts, or a later v1.4.1 surface fails the gate until the new channel is classified and tested for lifecycle and sensitivity policy.
12. **Revoked/restricted rejection is green for each channel.** Every channel in the matrix has at least one assertion proving revoked or restricted source detail cannot leak and cannot drive unsafe output.
13. **Provenance masking shape is exercised.** Revoked source provenance produces `ProvenanceMasked` or equivalent masked placeholder with mask reason and masked source class, not raw source content. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:86-110`.
14. **Sanitization is shared.** LLM-generated explanation text in provenance is sanitized before rendering in any channel; no raw HTML/Markdown links, executable content, prompt-injection instructions, or URLs survive in rendered explanations. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:74-84`.
15. **Logs and telemetry are low-detail.** Error logs and structured telemetry reference invocation ids, lifecycle state, and safe source class but not provenance content or PII. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:48-53`.
16. **Fixture data stays synthetic.** No real customer names, domains, email addresses, phone numbers, or identity maps appear in fixture artifacts. Sources: `src-tauri/tests/fixtures/bundle-README.md:19-20`, `src-tauri/tests/harness.rs:1703-1749`.
17. **Expected post-action state is assertive.** `expected_state.json` records lifecycle, invalidation, trust/render degradation, and no-leak outcomes rather than only comparing rendered text.
18. **No frontend-only pass.** A passing implementation must exercise ability output, provenance, renderer policy, and harness artifacts; hiding content in a React component while leaving MCP/replay/log channels exposed is a failure.

## 6. Review Ladder

- **L0 reviewers:** `qa-expert` plus `security-auditor`.
- **W6 default:** all six W6 agents require `qa-expert` at L0.
- **W6-E exception:** W6-E is the only W6 agent that adds `security-auditor`, because W6-E is DOS-292 and the merge gate names `security-auditor` specifically for DOS-292. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.
- **L0 approval bar:** both reviewers must accept the fixture scope, actor/surface matrix, lifecycle-state modeling, 9-channel parameterization, and no-leak assertions before implementation.
- **L1 self-validation:** implementation agent validates real fixture execution, expected output/provenance/state comparison, and channel matrix coverage.
- **L2 diff review:** local pre-merge review of code and fixtures; security-auditor findings that are literal acceptance violations, ADR-named contract violations, or PR-introduced regressions must be fixed before merge.
- **L3 wave:** integrated W6 adversarial review includes full Suite S re-run, including the 9-channel ADR-0108 sweep, and all 18 bundles green. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **L4 surface:** full `/qa` against the v1.4.1 build after W6 integration because W6-E covers user-visible and agent-visible privacy behavior. Source: `.docs/plans/v1.4.1-waves.md:660-661`.
- **L5 drift:** integrated state must match the v1.4.1 validation-suite DoD, including per-bundle ownership and no release with a quarantined bundle. Source: `.docs/plans/v1.4.1-waves.md:659-663`.

`qa-expert` review focus:

- Bundle 17 assignment and fixture naming are unambiguous.
- Fixture shape follows the catalogue and harness conventions.
- DOS-292 edge cases are represented by state, replay, output, provenance, and post-action assertions.
- The 9-channel sweep is parameterized, not manually duplicated.
- Bundle 17 can become mandatory without quarantine.

`security-auditor` review focus:

- Restricted/revoked/internal-only content cannot leak through any channel.
- MCP actor filtering cannot be bypassed by detail retrieval.
- Logs, telemetry, replay, and eval artifacts do not become side channels.
- Redaction usefulness does not reintroduce sensitive identifiers or internal graph structure.
- Sanitization handles untrusted explanation text before any render.

## 7. Risks and Mitigations

- **Risk: channel drift.** A new telemetry, eval, replay, or render channel ships after bundle 17 and bypasses sensitivity policy.
  **Mitigation:** use a parameterized channel registry/matrix that fails when a new channel is unclassified.

- **Risk: frontend-only privacy.** Tauri hides a detail but MCP, replay, telemetry, or logs still expose it.
  **Mitigation:** assertions run at ability/provenance/render-policy level and across all nine channels.

- **Risk: useful redaction becomes leakage.** Redacted provenance tries to be helpful by showing hidden object identifiers, names, or raw source text.
  **Mitigation:** useful summary is constrained to source class, lifecycle state, timestamp posture, redaction reason, and safe counts.

- **Risk: over-redaction makes trust unverifiable.** User or agent sees "hidden" with no way to understand why the claim degraded.
  **Mitigation:** expected provenance must include enough safe summary to explain source class and lifecycle reason.

- **Risk: stale downstream objects look fresh.** Provider fetch time masks source age.
  **Mitigation:** seeded case proves freshness uses `source_asof` or downstream object time before observed/fetch time. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:391-437`.

- **Risk: MCP detail endpoint bypasses default filter.** `get_provenance(invocation_id)` returns raw detail unavailable in default MCP response.
  **Mitigation:** test both default MCP response and detail retrieval through the same actor-filtered renderer. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:31-40`.

- **Risk: logs and telemetry become the leak path.** Operational channels include raw provenance or source content for debugging.
  **Mitigation:** channel matrix asserts logs and telemetry carry invocation ids, safe lifecycle/source class, and counts only. Source: `.docs/decisions/0108-provenance-rendering-and-privacy.md:48-53`.

- **Risk: fixture contains real customer data.** Privacy validation accidentally embeds examples from production.
  **Mitigation:** generic examples only; rely on fixture anonymization governance that rejects PII-like artifacts. Source: `src-tauri/tests/harness.rs:1703-1749`.

- **Risk: bundle 17 duplicates bundle 8, 11, or 12.** Existing fixtures already cover sensitivity, stale Glean, and revoked source in narrower forms.
  **Mitigation:** bundle 17 proves the cross-product of lifecycle state, actor policy, provenance masking, and nine channels; it can reference existing conventions but must not shrink to one case. Source: `src-tauri/tests/fixtures/bundle-README.md:39-45`.

## 8. Rollout and Test Plan

Implementation should proceed in one PR owned by W6-E:

1. Add `src-tauri/tests/fixtures/bundle-17/` with deterministic clock, seed, SQL state, inputs, provider replay, external replay, expected output, expected provenance, expected state, and metadata.
2. Add `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs`.
3. Add or reuse a channel matrix that enumerates the nine ADR-0108 channels and fails on unclassified additions.
4. Add assertions for each lifecycle scenario: disconnected, revoked, expired, restricted, unavailable, and stale downstream object.
5. Add actor-specific assertions for Tauri user, MCP default agent, MCP detail agent, external/customer-facing, telemetry/log/replay/eval channels.
6. Wire bundle 17 into harness/release-gate expectations required by W6/W7 once implementation begins.
7. Run focused tests for bundle 17 and neighboring sensitivity/lifecycle bundles.
8. Run required L1 commands before PR: `cargo clippy -- -D warnings`, `cargo test`, and `pnpm tsc --noEmit`.

Proof artifacts expected at L1:

- Bundle 17 fixture metadata and scenario map.
- Test output showing bundle 17 passes.
- Negative assertions or failure snapshots proving restricted/revoked detail is rejected per channel.
- A short matrix mapping each DOS-292 edge case to each channel class.
- A note confirming no customer-specific data and no identity-map artifact.

Minimum targeted test assertions:

- `source_lifecycle_changes_trigger_degraded_state`
- `restricted_mcp_provenance_redacts_source_details`
- `redacted_provenance_retains_safe_summary`
- `mcp_detail_does_not_bypass_actor_filter`
- `internal_only_content_cannot_feed_customer_facing_output`
- `fresh_fetch_stale_downstream_object_uses_source_asof`
- `all_adr0108_channels_have_revoked_restricted_rejection`
- `new_channel_requires_policy_classification`

No L1 claim should rely on "compiles clean" alone. Done means bundle 17 proves the DOS-292 behavior with real fixture state flowing through the Intelligence Loop to rendered or emitted artifacts.

## 9. Out of Scope

- Writing implementation files in this packet authoring turn.
- Committing changes.
- Editing `.docs/plans/v1.4.1-waves.md` or the amendments file.
- Editing ADR-0102, ADR-0105, or ADR-0108 in this packet turn.
- Creating new schema by default; implementation should first use existing claim, provenance, lifecycle, trust, and render-policy substrate.
- General source taxonomy redesign.
- General sensitivity policy redesign outside the DOS-292 validation bundle.
- Broad cross-surface consistency beyond source lifecycle/privacy/actor-specific provenance.
- DOS-289 stale-current contradiction except where stale downstream source semantics overlap.
- DOS-290 broad cross-surface agreement.
- DOS-291 ambiguous identity and primary-context selection.
- DOS-293 sync, refresh, concurrency, and partial-failure recovery.
- Publishing customer-facing content.
- Using real customer names, domains, email addresses, phone numbers, object ids, or account details in fixtures.

## 10. Open Questions

1. **Where should the nine-channel matrix live?** Preferred implementation is a shared test helper or policy enum close to the renderer/harness so W7-E and W8 additions cannot bypass it. L0 should confirm ownership.
2. **What is the canonical channel name for future W7-E telemetry additions?** The acceptance gate should fail closed until the telemetry owner registers any new channel class.
3. **What is the canonical channel name for W8 eval/benchmark artifacts?** The wave plan requires automatic coverage; W8 should consume the same matrix rather than defining a separate list.
4. **Which existing lifecycle field encodes `unavailable` versus `restricted`?** If no existing substrate field cleanly distinguishes them, implementation must run the Intelligence Loop check before adding a field.
5. **How much summary is "useful enough" after redaction?** Proposed minimum is source class, lifecycle state, timestamp posture, redaction reason, and safe counts, with no source identifiers or raw text.
6. **Does MCP detail retrieval exist in current implementation or only ADR contract?** If absent, bundle 17 should still include the expected policy assertion at the wrapper/renderer level and mark the concrete endpoint assertion pending on the implementation owner, not silently skip MCP detail risk.
7. **Should lifecycle changes emit invalidation signals immediately in this bundle or assert expected state after a simulated refresh?** L0 should confirm the most faithful path to the current harness.
8. **Should bundle 17 reuse bundle 8 sensitivity helpers or introduce source-lifecycle-specific helpers?** Reuse is preferred if it preserves clear failure messages for lifecycle and actor-specific policy.

