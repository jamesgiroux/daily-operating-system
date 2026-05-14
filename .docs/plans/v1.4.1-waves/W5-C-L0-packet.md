# W5-C L0 Packet - DOS-222 migrate detect_risk_shift

## 1. Header

- Date: 2026-05-13.
- Project: v1.4.1 Abilities Runtime Completion.
- Wave: Wave 5 - Capability migrations.
- Issue: DOS-222 - Capability: Migrate detect_risk_shift to Abilities.
- Linear URL: https://linear.app/a8c/issue/DOS-222.
- Working branch: `dos-280-w5-c-l0-prep`.
- Worktree path: `/Users/jamesgiroux/Documents/dailyos-repo/worktrees/dos-280-w5-c-l0-prep`.
- Packet version: V1.
- Packet status: L0 authoring packet for review.
- Boundary: documentation-only.
- Boundary: no code edits.
- Boundary: no migrations.
- Boundary: no fixtures.
- Boundary: no runtime config changes.
- Boundary: no commit.
- Boundary: no PR.
- Target implementation owner path from the wave plan: `src-tauri/abilities-runtime/src/abilities/detect_risk_shift/`.
- Target implementation shape from the W5 pilot pattern: directory ability with `mod.rs`, `prompts.rs`, and `synthesis.rs`.
- Linear issue path note: DOS-222 currently says Stage 1 is `src-tauri/src/abilities/transform/detect_risk_shift.rs`.
- Wave authority note: `.docs/plans/v1.4.1-waves.md` assigns the new ability inside `abilities-runtime`, so this packet uses the wave-owned runtime path.
- Grounding gap: the requested ADR filename `.docs/decisions/0106-prompt-fingerprinting.md` does not exist in this worktree.
- Grounding resolution: the real ADR-0106 file is `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md`.
- Grounding gap: no exact `detect_risk_shift` command, service, or legacy module symbol appears in the current worktree.
- Grounding resolution: the current adjacent legacy risk path is `generate_risk_briefing` / `get_risk_briefing`, implemented through `src-tauri/src/commands/planning_reports.rs`, `src-tauri/src/services/intelligence.rs`, and `src-tauri/src/risk_briefing.rs`.
- This packet does not pretend the exact legacy `detect_risk_shift` implementation exists.
- This packet treats DOS-222 as the new ability contract plus parity harness against the risk-shift behavior that the issue names.

## 2. Load-bearing user outcome

- Risk shifts become substrate-backed rather than ad-hoc prompt output.
- A risk assessment for an account is no longer just narrative text.
- It is an ability output with explicit provenance.
- It is a Transform output with `Trust::Untrusted` for mutation authorization.
- It carries field-level attribution for every material field.
- The user can open "why this risk" on every indicator.
- "Why this risk" resolves to sources, child ability provenance, and trajectory points.
- Risk direction is explainable as a computed result, not as model opinion.
- The direction is backed by `trajectory_delta_v1`.
- The computed direction points at engagement trajectory data.
- The computed direction points at role trajectory data when role data exists.
- The model may synthesize indicator language.
- The model may synthesize the evidence summary.
- The model does not decide the overall direction.
- The user sees trajectory data points behind each risk indicator.
- The expected data points include engagement curve movement.
- The expected data points include role progression movement when the subject has role data.
- The expected evidence also includes recent signals and claim history diffs.
- Field-level source refs let the user distinguish fresh evidence from stale evidence.
- `source_asof` lets the user understand whether evidence is current.
- Stale Glean evidence is downweighted instead of being presented as current truth.
- Revoked Glean evidence is masked or suppressed before it can support an indicator.
- Same-domain account ambiguity does not leak adjacent account risk into the target account.
- Subject-fit failure is a hard error or a suppressed candidate, not a confident risk.
- Daily readiness can compose the result without inventing a second risk model.
- MCP and Tauri callers get the same ability contract once cutover happens.
- Parallel run lets the product compare ability output against the legacy behavior before users see the new path.

## 3. Pre-work

- Upstream artifact read: `.docs/plans/v1.4.1-waves.md`.
- Relevant section: `# Wave 5 - Capability migrations`.
- W5 states that DOS-220, DOS-221, and DOS-222 migrate abilities following the v1.4.0 W5 pattern.
- W5 states that DOS-222 owns `abilities/detect_risk_shift/` inside `abilities-runtime`.
- W5 states that DOS-222 is done when parity vs legacy on bundle-11 is green and operations array is declared.
- Upstream artifact read: `.docs/plans/wave-W5/proof-bundle.md`.
- The W5 proof bundle records DOS-218 `get_entity_context` as the Read pilot.
- The W5 proof bundle records DOS-219 `prepare_meeting` as the Transform pilot.
- The W5 proof bundle records eight L2 closure cycles before approval.
- The W5 proof bundle's critical lesson is channel enumeration before patching individual leaks.
- Upstream artifact read: `.docs/plans/wave-W5/DOS-219-plan.md`.
- DOS-219 is the Transform pilot pattern DOS-222 should follow.
- DOS-219 separates pure Transform synthesis from persistence or maintenance behavior.
- DOS-219 composes `get_entity_context`.
- DOS-219 uses prompt template fingerprinting.
- DOS-219 validates source refs and subject fit before accepting LLM candidates.
- DOS-219 confirms the same-domain ambiguity fixture pattern that DOS-222 needs.
- Upstream artifact read: `.docs/decisions/0102-abilities-as-runtime-contract.md`.
- ADR-0102 defines abilities as named, typed, versioned functions.
- ADR-0102 defines Transform as no service mutation and may invoke the provider.
- ADR-0102 defines Transform outputs as untrusted for mutation authorization.
- ADR-0102 defines composition and registry metadata.
- Upstream artifact read: `.docs/decisions/0105-provenance-as-first-class-output.md`.
- ADR-0105 defines the `Provenance` envelope.
- ADR-0105 requires field-level attribution for every output field.
- ADR-0105 requires non-empty source refs for LLM synthesis.
- ADR-0105 defines `source_asof` semantics and stale-source warnings.
- Upstream artifact read: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md`.
- ADR-0106 defines `PromptFingerprint`.
- ADR-0106 defines `canonical_prompt_hash`.
- ADR-0106 requires replay lookup by canonical prompt hash.
- Upstream artifact read: `.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md`.
- ADR-0109 defines `TrajectoryBundle`.
- ADR-0109 names `detect_risk_shift` as a consumer of maintained temporal primitives.
- ADR-0109 says Transform abilities consume trajectory through `get_entity_context`.
- Upstream artifact read: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md`.
- ADR-0112 defines Stage 0 through Stage 5 capability migration.
- ADR-0112 places `detect_risk_shift` after `list_open_loops`.
- ADR-0112 allows LLM-as-judge for Transform parallel-run comparison.
- Linear DOS-222 description read.
- DOS-222 says `detect_risk_shift` is category Transform.
- DOS-222 says trust is Untrusted.
- DOS-222 says the ability consumes `TrajectoryBundle` from `get_entity_context`.
- DOS-222 says it composes `get_entity_context(Deep)` for trajectory data.
- DOS-222 names four fixture classes.
- DOS-222 requires judge-scored relevance >=0.85 and faithfulness >=0.90.
- DOS-222 requires 7-day parallel run with 3% regression tolerance for synthesis variance.
- DOS-222 requires every `RiskIndicator` to carry populated `source_refs`.
- Runtime source read: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/`.
- `prepare_meeting/mod.rs` registers a Transform ability with `composes = get_entity_context`.
- `prepare_meeting/prompts.rs` renders a template and builds fingerprint inputs.
- `prepare_meeting/synthesis.rs` builds live context, composes `get_entity_context`, gates sources, calls the provider, and assembles provenance.
- Runtime source read: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs`.
- `get_entity_context` returns `GetEntityContextOutput { entries, trajectory }`.
- `get_entity_context` maps `ContextDepth::Deep` to `TrajectoryQueryDepth::Weeks(DEEP_LIMIT_WEEKS)`.
- Runtime source read: `src-tauri/abilities-runtime/src/abilities/temporal/mod.rs`.
- Current runtime `TrajectoryBundle` contains `engagement_curve` and `role_progression`.
- Runtime grounding gap: current runtime `TrajectoryBundle` does not yet include ADR-0109's full `health_curve`, `risk_trajectory`, or `relationship_strength` fields.
- Packet implication: DOS-222 V1 should consume current `engagement_curve` and `role_progression` first, and must not claim missing runtime fields exist.
- Upstream artifact read: `.docs/plans/v1.4.1-waves.md` Wave 4 DOS-280 block.
- DOS-280 claim canonicalization just merged into the plan as a prerequisite substrate.
- DOS-280 matters because risk-shift claim history diffs must not double-count semantically duplicate claims.
- DOS-220 sibling W5-A composes this result for daily readiness.
- DOS-221 sibling W5-B is a pure Read ability and should be simpler than DOS-222.
- DOS-215 temporal primitives are marked as the source of `TrajectoryBundle` readiness in DOS-222 Stage 0.
- DOS-218 entity context pilot is the upstream Read ability DOS-222 composes.
- DOS-219 Transform pilot is Done and supplies the implementation pattern.

## 4. Inherited contracts from v1.4.0 W5 pilots

- DOS-222 inherits the W5 Transform contract.
- Because DOS-222 is a Transform, every DOS-219 pilot closure class applies.
- Contract: ADR-0106 canonical replay key parity.
- `ReplayProvider` lookup must match `PromptFingerprint.canonical_prompt_hash` byte-for-byte.
- Current source support: `src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs` exposes `replay_fixture_key()` and `prompt_fingerprint_from_completion()` over the same canonical hash helper.
- Current source support: `src-tauri/abilities-runtime/src/intelligence/provider.rs` says replay fixtures use `canonical_prompt_hash` and missing hashes fail.
- DOS-222 proof must include a test that the replay lookup key equals the provenance fingerprint hash.
- Contract: Cycle-7 9-channel sensitivity sweep.
- DOS-222 hits all 9 prompt-related channels.
- DOS-222 should enumerate the channels at L0, before L2 discovers them one at a time.
- Channel 1: subject-ref claims.
- Planned DOS-222 path: `detect_risk_shift/synthesis.rs::RiskShiftContext::from_entity_id`.
- Planned source: account subject claims read through a service handle backed by `services::claims::load_claims_active_for_surface`.
- Gate: `claim_allowed_for_prompt_input()` from `abilities-runtime/src/types.rs` or `services/claims.rs`.
- Specific ability use: only Public/Internal subject claims enter `PromptRiskContext.claim_history`.
- Channel 2: source-ref claims.
- Planned DOS-222 path: `detect_risk_shift/synthesis.rs::load_claims_for_trajectory_source_refs`.
- Planned source: claims recovered by source ref for the trajectory and recent-signal source refs.
- Repo source: `services::claims::load_claims_active_by_source_ref_for_surface`.
- Gate: same Public/Internal sensitivity gate plus same subject allowlist.
- Specific ability use: source-ref claims can corroborate a risk indicator but cannot broaden subject scope.
- Channel 3: snapshot claims to evidence-source mapping.
- Planned DOS-222 path: `detect_risk_shift/synthesis.rs::EvidenceSource::from_claim`.
- Pattern source: `prepare_meeting/synthesis.rs::evidence_from_claim`.
- Gate: `claim_allowed_for_prompt_input()` before `RiskEvidenceSource` is built.
- Specific ability use: `RiskEvidenceSource` feeds prompt context and source attribution.
- Channel 4: composed `get_entity_context` children.
- Planned DOS-222 path: `detect_risk_shift/synthesis.rs::compose_entity_context_deep`.
- Source: `get_entity_context` child output entries and child output trajectory.
- Gate: child `get_entity_context` already filters Agent reads and attributes trajectory, but DOS-222 must apply its own prompt boundary gate before serializing child entries.
- Specific ability use: `PromptRiskContext.entity_context.entries` and `PromptRiskContext.trajectory`.
- Channel 5: prebuilt private/eval seams.
- Planned DOS-222 path: private `DetectRiskShiftInput.context` or `fixture_context` seam, if added.
- Required serde: `#[serde(default, skip_deserializing, skip_serializing)]`.
- Required schema: `#[schemars(skip)]`.
- Pattern source: `PrepareMeetingInput.context` in `prepare_meeting/synthesis.rs`.
- Specific ability use: an MCP or Agent caller must not inject fabricated trajectory or claim JSON.
- Channel 6: rendered prompt plus canonical JSON inputs.
- Planned DOS-222 path: `detect_risk_shift/prompts.rs::render_prompt`.
- Pattern source: `prepare_meeting/prompts.rs::render_prompt`.
- Gate: canonical JSON inputs are built only from gated `PromptRiskContext`.
- Specific ability use: restricted source IDs and restricted claim text must be absent from rendered prompt text and canonical prompt JSON.
- Channel 7: template variables.
- Planned DOS-222 template: `detect_risk_shift.v{n}.txt`.
- Planned variables: `{{schema_version}}` and `{{risk_context_json}}`.
- Gate: template variables are downstream of gated canonical JSON.
- Specific ability use: no raw account/customer free text bypasses `PromptRiskContext`.
- Channel 8: output-only provenance fields.
- Planned DOS-222 path: `RiskAssembler::assemble` field attributions.
- Gate: output-only provenance is not sent to the provider, but source refs must stay subject-scoped and render-safe.
- Specific ability use: `field_attributions.*.source_refs`, `children`, and `prompt_fingerprint` must not expose restricted source text.
- Channel 9: non-claim prompt data.
- Planned DOS-222 path: `PromptRiskContext.entity` and deterministic `trajectory_delta_v1` scalar summaries.
- Data: entity id, entity type, schema version, trajectory window metrics, engagement deltas, role progression deltas.
- Gate: no sensitivity gate needed for mechanical scalar summaries, but any free text from signals or claims is not allowed in this channel.
- Specific ability use: account metadata must be minimal and sanitized; source text must arrive only through Channels 1-4 after gating.
- Contract: serde `skip_deserializing` on context and input fields.
- DOS-222 public input is `DetectRiskShiftInput { entity_id, schema_version }`.
- If private fixture fields exist, they must be skipped for deserialization and serialization.
- Contract: test-harness Cargo feature gating.
- Existing repo source: `src-tauri/src/lib.rs` has a `compile_error!` if `test-harness` is enabled in release builds.
- DOS-222 test-only helpers must stay behind the same feature posture.
- Contract: ServiceContext non-Live fail-closed reads.
- Existing source support: `read_prepare_meeting_context` and `read_entity_context_claims` return `FixtureReaderRequired` when no fixture reader is injected.
- Current caveat: `read_trajectory_bundle` returns an empty default when no trajectory reader is injected.
- DOS-222 L0 requirement: because risk shift depends on trajectory, fixtures must inject trajectory data and tests must fail if expected trajectory is absent.
- Contract: centralized sensitivity gate in `services/claims.rs`.
- Existing source support: `services::claims::claim_allowed_for_prompt_input` allows only Public/Internal.
- Runtime mirror: `abilities-runtime/src/types.rs::claim_allowed_for_prompt_input`.
- DOS-222 must not implement a local, diverging sensitivity allowlist.
- Contract: subject-fit hard-error on `entity_id` mismatch.
- DOS-222 must validate that every composed trajectory snapshot has the requested `entity_id`.
- DOS-222 must validate that every accepted indicator subject is the input account subject.
- If `RiskIndicator.source_refs` point to adjacent account evidence, the candidate is rejected or downgraded to ambiguity.
- Contract: `source_asof` propagation through composition.
- `get_entity_context` attributes trajectory data points.
- DOS-222 must preserve child source refs through its field attributions.
- DOS-222 must lift direct signal or claim `source_asof` into `SourceAttribution`.
- DOS-222 must emit `SourceStale` or freshness diagnostics when evidence age requires downweighting.
- Contract: subject-bleed defense.
- DOS-219 Bundle 1 pattern: same-domain entity ambiguity fixture.
- DOS-222 fixture should use generic same-domain accounts, e.g. `subsidiary.com` and `parent.com`.
- Fixture expected result: adjacent account evidence cannot support the target account's risk indicator.
- Fixture expected result: ambiguous same-domain subject fit blocks confident risk.

## 5. Composition + synthesis shape

- Ability name: `detect_risk_shift`.
- Category: Transform.
- Trust: Untrusted.
- Public input: `DetectRiskShiftInput`.
- Required input field: `entity_id`.
- Required input field: `schema_version`.
- Scope: account-scoped per DOS-222.
- Stage 1 location per wave plan: `src-tauri/abilities-runtime/src/abilities/detect_risk_shift/`.
- Directory shape: `mod.rs`.
- Directory shape: `prompts.rs`.
- Directory shape: `synthesis.rs`.
- Optional internal test seam: private fixture context, skipped by serde.
- `mod.rs` registers the ability.
- `mod.rs` declares `category = Transform`.
- `mod.rs` declares `allowed_actors = [User, Agent, System]` unless reviewer panel narrows it.
- `mod.rs` declares `allowed_modes = [Live, Simulate, Evaluate]` if Simulate remains available in current runtime policy.
- `mod.rs` declares `requires_confirmation = false`.
- `mod.rs` declares `may_publish = false`.
- `mod.rs` declares `composes = get_entity_context`.
- `mod.rs` declares no mutations.
- `prompts.rs` owns `TEMPLATE_ID = "detect_risk_shift"`.
- `prompts.rs` owns `TEMPLATE_VERSION`.
- `prompts.rs` includes `detect_risk_shift.v{n}.txt` from the prompt template registry.
- `prompts.rs` renders prompt text from a canonical `PromptRiskContext`.
- `prompts.rs` creates canonical JSON inputs.
- `prompts.rs` builds `PromptFingerprint` from provider completion metadata.
- `synthesis.rs` owns `build_risk_shift`.
- `synthesis.rs` composes `get_entity_context` at `ContextDepth::Deep`.
- Child input: `GetEntityContextInput { schema_version: 2, entity_type: "account", entity_id, depth: Deep }`.
- Child output consumed: `entries`.
- Child output consumed: `trajectory`.
- Current runtime trajectory consumed: `engagement_curve`.
- Current runtime trajectory consumed: `role_progression`.
- Direct reads: recent signals.
- Direct reads: claim history.
- Direct reads: claim sequence diff for change over time.
- Direct reads must flow through narrow service handles.
- Direct reads must not open raw DB handles from ability code.
- Direct reads must be mode-aware.
- Direct reads must fail closed in Evaluate when fixture readers are absent.
- Synthesis input contains only gated fields.
- Synthesis input includes trajectory data point ids or source refs.
- Synthesis input includes engagement deltas.
- Synthesis input includes role progression facts when available.
- Synthesis input includes recent signals that passed subject and sensitivity gates.
- Synthesis input includes claim-history diffs that passed subject and sensitivity gates.
- Output: `RiskShiftResult`.
- Output field: `entity_id`.
- Output field: `direction`.
- Output field: `indicators`.
- Output field: `evidence_summary`.
- Output field: `schema_version`.
- `direction` is computed.
- `direction` is not LLM-selected.
- `direction` uses deterministic `trajectory_delta_v1`.
- `trajectory_delta_v1` compares engagement curve windows.
- `trajectory_delta_v1` considers bidirectional ratio changes.
- `trajectory_delta_v1` considers meeting count changes.
- `trajectory_delta_v1` considers email count changes.
- `trajectory_delta_v1` considers role progression changes when role data exists.
- Direction enum: `Improving`.
- Direction enum: `Stable`.
- Direction enum: `DegradingMinor`.
- Direction enum: `DegradingMajor`.
- Indicators are synthesized by LLM.
- Each indicator carries source refs.
- Each indicator points to trajectory data points and signals.
- Each indicator has enough attribution for "why this risk".
- `evidence_summary` is synthesized by LLM.
- `evidence_summary` aggregates source refs from accepted child indicators.
- The provider call is one synthesis call unless reviewers approve targeted repair.
- No publish or persistence happens in the Transform.
- Tauri write cutover is out of scope for this packet.
- Legacy fallback remains available through ADR-0112 Stage 3.

## 6. Field-level attribution per DOS-222 field-level attribution section

- Pin exactly: `direction` -> Computed algorithm `trajectory_delta_v1`.
- `direction` source refs point to engagement trajectories.
- `direction` source refs point to role trajectories when role trajectories contributed.
- `direction` field path: `/direction`.
- `direction` derivation kind: `Computed { algorithm = "trajectory_delta_v1" }`.
- `direction` confidence kind: Computed.
- `direction` must not use `LLMSynthesis`.
- Pin exactly: `indicators` array -> LLMSynthesis per indicator.
- Each `indicators/{index}` field attribution is `LLMSynthesis`.
- Each indicator source refs point to recent signals.
- Each indicator source refs point to trajectory points.
- Each indicator source refs point to claim-history diffs when they contributed.
- An indicator with no source refs is invalid.
- An indicator with only unsupported model text is invalid.
- An indicator whose source refs are not for `entity_id` is invalid.
- Pin exactly: `evidence_summary` -> LLMSynthesis.
- `evidence_summary` source refs aggregate accepted child source refs.
- `evidence_summary` field path: `/evidence_summary`.
- `evidence_summary` cannot cite a source rejected from every indicator.
- `entity_id` attribution is Direct or Constant.
- `schema_version` attribution is Constant.
- Empty `indicators` array still receives an attribution entry.
- Empty `indicators` attribution should be Constant with diagnostics explaining no supported risk shift.
- Provenance lives once on `AbilityOutput<RiskShiftResult>`.
- `RiskShiftResult` must not contain a duplicated provenance field.
- The provenance child for `get_entity_context` must remain nested.
- Source refs into child provenance must use stable `CompositionId`.
- Expected composition id pattern: `get_entity_context:{entity_id}` or an equivalent stable declared id.
- Field-level attribution must survive Tauri and MCP serialization.
- Field-level attribution must survive judge fixture evaluation.

## 7. Prompt fingerprinting

- Prompt template id: `detect_risk_shift`.
- Prompt template file: `detect_risk_shift.v{n}.txt`.
- Prompt template registry location from ADR-0106: `src-tauri/src/abilities/prompts/`.
- Current pilot include pattern: `prepare_meeting/prompts.rs` includes `src/abilities/prompts/prepare_meeting_prep.v1.0.0.txt`.
- DOS-222 should use the same registry approach.
- Every provider invocation captures `PromptFingerprint` in provenance.
- `PromptFingerprint.provider` comes from provider metadata.
- `PromptFingerprint.model` comes from provider metadata.
- `PromptFingerprint.prompt_template_id` is `detect_risk_shift`.
- `PromptFingerprint.prompt_template_version` is the registered template version.
- `PromptFingerprint.canonical_prompt_hash` is computed by the shared ADR-0106 helper.
- `ReplayProvider` lookup uses the same helper through `replay_fixture_key`.
- Provenance uses the same helper through `prompt_fingerprint_from_completion`.
- L0 proof requirement: `canonical_prompt_hash` in replay lookup equals `PromptFingerprint.canonical_prompt_hash` byte-for-byte.
- L0 proof requirement: fixture lookup by legacy `prompt_replay_hash` is not accepted.
- L0 proof requirement: missing replay fixture returns `FixtureMissingCompletion`.
- Canonical JSON inputs must include the gated risk context.
- Canonical JSON inputs must not include restricted claim text.
- Canonical JSON inputs must not include source refs for rejected candidates.
- The rendered provider prompt must match the canonical JSON source of truth.
- Prompt changes do not bump ability schema version.
- Prompt changes do require eval review and fixture updates.
- Judge variance must be classified by prompt version and canonical prompt hash.

## 8. Acceptance criteria

- [ ] Registered with category=Transform, trust=Untrusted.
- [ ] Consumes `TrajectoryBundle` from `get_entity_context`.
- [ ] ≥4 eval fixtures; judge-scored ≥0.85 relevance, 0.90 faithfulness.
- [ ] Parallel-run divergence ≤3% over 7 days (judge-scored).
- [ ] Each `RiskIndicator` has populated source_refs; users can trace "why this risk."
- [ ] 9-channel sensitivity audit at L0 - Transform hits all 9; per-channel gate citation included in Section 4.
- [ ] ADR-0106 replay-key parity assertion byte-for-byte.
- [ ] Subject-bleed defense fixture - same-domain entity ambiguity.
- [ ] Stale-trajectory fixture - Glean evidence 6+ months old triggers freshness_weight downweight via DOS-5.
- [ ] Fixture 1: account trending down.
- [ ] Fixture 2: account stable.
- [ ] Fixture 3: account in active escalation.
- [ ] Fixture 4: account with revoked Glean.
- [ ] Judge-scored relevance >=0.85.
- [ ] Judge-scored faithfulness >=0.90.
- [ ] Parallel-run divergence <=3% over 7 days.
- [ ] Synthesis variance allowance follows DOS-222, not ADR-0112's generic <=1% Read-like default.
- [ ] Bundle-11 parity green.
- [ ] Each `RiskIndicator` source_refs populated.
- [ ] Users can trace why this risk.
- [ ] `direction` generated by `trajectory_delta_v1`, not LLM.
- [ ] `direction` field attribution is Computed.
- [ ] `indicators` field attribution is LLMSynthesis per indicator.
- [ ] `evidence_summary` field attribution is LLMSynthesis.
- [ ] `source_asof` propagates from composed trajectories and direct sources.
- [ ] Revoked Glean source cannot support an accepted risk indicator.
- [ ] Restricted-sensitivity claim text is absent from prompt evidence.
- [ ] Restricted-sensitivity claim ids are absent from canonical prompt JSON.
- [ ] Private fixture context cannot be injected through erased Agent/MCP input.
- [ ] Same-domain adjacent account evidence is rejected or marked ambiguous.
- [ ] Non-Live fixture readers are required for direct claim/signal history reads.
- [ ] Trajectory absence is explicit in diagnostics when a fixture expects trajectory.
- [ ] Current runtime `TrajectoryBundle` limitation is respected: only engagement and role trajectories are assumed in V1.
- [ ] No direct DB writes from ability code.
- [ ] No Tauri event emission from Transform code.
- [ ] No claim commits from Transform code.
- [ ] No raw customer data in fixtures.

## 9. Linear dependency edges

- DOS-222 Stage 0 requires DOS-215 Done.
- DOS-222 Stage 0 requires DOS-218 Done.
- Linear DOS-222 relation list confirms DOS-215 is related.
- Linear DOS-222 relation list confirms DOS-218 is related.
- DOS-215 supplies temporal primitives.
- DOS-218 supplies `get_entity_context`.
- DOS-218 supplies the `TrajectoryBundle` consumption path.
- DOS-219 supplies the Transform pilot pattern but is not a direct Linear blocker in DOS-222.
- DOS-280 claim canonicalization just merged and should be treated as available substrate.
- No remaining Linear blockers were visible on DOS-222 at packet authoring time.
- DOS-222 status at read time: Backlog.
- DOS-222 priority at read time: High.
- Sibling W5-A DOS-220 composes this result for daily readiness.
- W5-A can use a shim while DOS-222 is in Stage 3.
- W5-A can use legacy fallback until DOS-222 cutover.
- W5-A should not invent a separate risk-shift synthesis path.
- Sibling W5-B DOS-221 is independent and simpler.
- Recommended execution order: DOS-221, then DOS-220, then DOS-222.
- If W5-A lands before DOS-222, its acceptance should name the fallback boundary.
- If DOS-222 lands before W5-A, it should expose the typed output W5-A can compose.

## 10. L0 reviewer panel

- Required reviewer: `/plan-eng-review`.
- `/plan-eng-review` focus: architecture.
- `/plan-eng-review` focus: ability registration and category.
- `/plan-eng-review` focus: Composition with `get_entity_context`.
- `/plan-eng-review` focus: Transform synthesis boundary.
- `/plan-eng-review` focus: trajectory consumption.
- `/plan-eng-review` focus: `trajectory_delta_v1` deterministic direction.
- `/plan-eng-review` focus: direct reads through service handles only.
- `/plan-eng-review` focus: acceptance fixture sufficiency.
- Required reviewer: `/codex challenge`.
- `/codex challenge` focus: channel leaks.
- `/codex challenge` focus: restricted sensitivity crossing provider boundary.
- `/codex challenge` focus: subject bleed.
- `/codex challenge` focus: same-domain account ambiguity.
- `/codex challenge` focus: replay/provenance canonical hash parity.
- `/codex challenge` focus: judge-scoring confidence.
- `/codex challenge` focus: stale trajectory evidence.
- `/codex challenge` focus: revoked source masking.
- Conditional reviewer: `/cso`.
- `/cso` trigger: LLM trust boundary.
- `/cso` trigger: source refs surviving verification.
- `/cso` trigger: MCP/Agent invocation exposure.
- `/cso` trigger: untrusted Transform output consumed by another ability.
- `/cso` expected question: can prompt-injected source text affect mutation or publication?
- `/cso` expected answer: no mutation in Transform; downstream action requires separate trust signal per ADR-0102.
- Optional domain reviewer: `/plan-devex-review` only if MCP schema/discovery details change.
- Optional domain reviewer: `/plan-design-review` only if surface rendering changes are added, which this packet excludes.

## 11. L0 acceptance gate

- All reviewer panels approve.
- Required approval: eng.
- Required approval: codex challenge.
- Conditional approval: cso if reviewer panel decides the trust-boundary scope requires it.
- DOS-222 Linear description links to this packet.
- The packet is V1 and documentation-only.
- The packet names grounding gaps instead of fabricating missing state.
- 9-channel audit complete.
- 9-channel audit includes per-channel gate citation.
- 9-channel audit includes the specific planned DOS-222 path.
- ADR-0106 parity proof is shown in packet.
- ADR-0106 parity proof cites shared helper use in `prompt_fingerprint.rs`.
- Subject-bleed defense fixture is named.
- Subject-bleed defense fixture uses same-domain entity ambiguity.
- Stale trajectory fixture is named.
- Revoked Glean fixture is named.
- Bundle-11 parity is an explicit acceptance gate.
- Judge thresholds are explicit.
- Parallel-run 7-day window is explicit.
- DOS-222-specific 3% tolerance is explicit.
- Current runtime `TrajectoryBundle` limitation is explicit.
- The plan does not claim missing trajectory fields exist.
- The plan does not claim an exact legacy `detect_risk_shift` module exists.
- Cycle-7 class-pattern memo applied.
- This packet pre-enumerates channels rather than discovering channels in L2.
- L0 can close only after reviewers accept the current runtime caveats.

## 12. Out-of-scope

- Tauri write cutover.
- Tauri command replacement.
- MCP tool replacement.
- Old-path removal.
- Claim persistence for new risk indicators.
- Risk-indicator projection into legacy tables.
- Daily readiness composition code changes.
- Surface rendering audit beyond prompt-input boundary.
- User-facing "About this" rendering implementation.
- Bundle expansion beyond the four named fixture classes plus bundle-11.
- Adding missing ADR-0109 fields to runtime `TrajectoryBundle`.
- HealthCurve runtime implementation.
- RiskTrajectory runtime implementation.
- RelationshipStrength runtime implementation.
- New migrations.
- New claim types.
- New design-system work.
- New UI copy.
- New prompt quality rubric beyond DOS-222 thresholds.
- Changing DOS-220.
- Changing DOS-221.
- Changing DOS-218.
- Changing DOS-219.
- Committing this packet.
- Opening a PR for this packet.

## 13. Why DOS-222 is the most complex of W5

- DOS-222 is a Transform.
- DOS-222 composes a Read ability.
- DOS-222 consumes temporal primitives.
- DOS-222 invokes LLM synthesis.
- DOS-222 mixes deterministic computation with model-written indicators.
- DOS-222 must preserve source refs across child provenance.
- DOS-222 must preserve source refs across direct reads.
- DOS-222 must support judge-scored fixtures.
- DOS-222 must support 7-day parallel run.
- DOS-222 must tolerate synthesis variance without accepting unsupported output.
- DOS-222 has the highest subject-bleed risk in W5.
- DOS-222 has the highest channel-leak risk in W5.
- DOS-222 has stale-source risk because trajectory points can summarize old evidence.
- DOS-222 has revoked-source risk because Glean-derived data can disappear after capture.
- DOS-222 has canonicalization risk because duplicate claims can distort trend deltas.
- DOS-222 has output attribution risk because every indicator needs traceability.
- DOS-222 has user trust risk because "risk" language carries decision weight.
- DOS-220 is moderately complex.
- DOS-220 composes reads and may synthesize daily readiness.
- DOS-220 can consume DOS-222 after cutover or use fallback during Stage 3.
- DOS-221 is simplest.
- DOS-221 is a pure Read migration.
- DOS-221 should ship first to prove simple read migration.
- DOS-220 should ship second to prove composed readiness migration.
- DOS-222 should ship last because it combines Transform, Composition, LLM synthesis, and trajectory consumption.
- This order minimizes cycle risk.
- This order minimizes fixture cost.
- This order limits how much W5-A depends on an unreviewed risk substrate.
- Natural W5 order: DOS-221, then DOS-220, then DOS-222.

## 14. Changelog

- V1 2026-05-13 initial L0 packet.
- V1 grounded on v1.4.0 W5 pilot lessons.
- V1 grounded on DOS-222 Linear description.
- V1 grounded on `.docs/plans/v1.4.1-waves.md`.
- V1 grounded on `.docs/plans/wave-W5/proof-bundle.md`.
- V1 grounded on `.docs/plans/wave-W5/DOS-219-plan.md`.
- V1 grounded on ADR-0102.
- V1 grounded on ADR-0105.
- V1 grounded on actual ADR-0106 filename in this repo.
- V1 grounded on ADR-0109.
- V1 grounded on ADR-0112.
- V1 grounded on shipped `prepare_meeting` Transform source.
- V1 grounded on shipped `get_entity_context` trajectory consumption.
- V1 grounded on current runtime `TrajectoryBundle` shape.
- V1 notes missing exact `detect_risk_shift` legacy symbol.
- V1 notes missing requested ADR-0106 short filename.

## V2 Cycle-1 Fold (2026-05-14)

Cycle-1 L0 panel raised 11 findings (4 architect APPROVE-WITH-COMMENTS + 7 codex BLOCK). This section folds them.

### ADR-0106 sampling capture (folds Architect F1 + Codex F2)

PromptFingerprint captures non-default temperature, top_p, seed from provider completion when present; defaults are elided per ADR-0106 contract. The same canonicalization helper computes the hash both for ReplayProvider lookup AND for PromptFingerprint provenance so the bytes match.

Fixture: replay_parity_risk_shift_sampling.json
- Seeds: deterministic risk-shift input with non-default sampling params
- Asserts: PromptFingerprint.canonical_prompt_hash == ReplayProvider lookup hash byte-for-byte
- Includes: golden replay across temperature=0.7, top_p=0.9, seed=42

### Untrusted-output downstream-handoff (folds Architect F2)

AbilityOutput trust is Untrusted. Any downstream ability — e.g., DOS-220 daily readiness, future surfaces — that elevates a RiskIndicator to a persisted claim MUST re-verify source_refs against the gated input source registry. Trust does NOT inherit through composition. AC: downstream callers receive a TrustEnvelope::Untrusted discriminant that hard-errors on persist_as_claim paths without explicit re-verification.

### evidence_summary.source_asof = oldest contributing (folds Architect F3)

evidence_summary.source_asof is set to the OLDEST contributing source_asof across indicators[].source_refs. Freshness diagnostics then surface the weakest link rather than averaging away staleness. Behavior fixture: indicator A from a 30-day-old signal + indicator B from a 6-month-old Glean doc → evidence_summary.source_asof = the 6-month-old timestamp.

### Empty-indicators path (folds Architect F4)

Fixture 2 — stable account — acceptance extended:
- Asserts: indicators[] is empty
- Asserts: direction = RiskDirection::Stable
- Asserts: diagnostic emission for no supported risk shift path — via ProvenanceWarning::InsufficientEvidence or equivalent
- Asserts: evidence_summary text reflects stability — no material risk shift indicators detected over window — rather than fabricating risk

### Channel-audit citations (folds Codex F1)

Channel-by-channel gate citations:
- Channel 1 — subject-ref claims via load_claims_active: services/claims.rs — grep apply_central_sensitivity_gate for the function; cite line range
- Channel 2 — source-ref claims via load_claims_active_by_source_ref: same gate, same file
- Channel 3 — snapshot.claims to EvidenceSource: grep prepare_meeting/synthesis.rs for the helper that filters EvidenceSource by subject; cite line range
- Channel 4 — composed get_entity_context children: gate at get_entity_context own boundary — DOS-218; cite get_entity_context.rs Agent-actor filter line range
- Channel 5 — PrepareMeetingInput.context.evidence test seam: serde skip_deserializing enforces; cite the struct definition
- Channel 6 — rendered prompt + canonical JSON: gate applied at prepare_prompt_inputs — or similar; grep for the helper that assembles canonical JSON; cite line range
- Channel 7 — template variables: same upstream gate; cite the template-substitution call site
- Channel 8 — output-only provenance fields: no gate needed; documents what is not sent to provider
- Channel 9 — non-claim prompt data: no gate needed; documents trivially-safe metadata

If exact line numbers can not be determined from grep, leave the function name + file path as the citation and note exact line at implementation time.

### TrajectoryBundle subject-fit (folds Codex F3)

Input-boundary verification: BEFORE PromptRiskContext.trajectory serialization, every trajectory point + signal source_ref subject is verified to belong to the caller input entity_id. Cross-stakeholder leakage defense — if a trajectory point references a person who is also a stakeholder at a different account, that point is dropped — not just the cross-account ref.

Fixture: bundle-11-trajectory-subject-bleed
- Seeds: Account A trajectory includes a person who is also a stakeholder at Account B
- Asserts: prompt input for Account A risk-shift call does NOT include the cross-account person signal text
- Asserts: source_ref to that person claim is masked or dropped

### Judge config pinned (folds Codex F4)

Judge model: same as DOS-219 prepare_meeting judge — grep .docs/plans/wave-W5/DOS-219-plan.md and .docs/plans/wave-W5/proof-bundle.md for the judge harness config; bind same provider + model + prompt template here.

Judge prompt template: judge/detect_risk_shift.v1.txt — new template; ground in DOS-219 judge prompt shape.

Sample unit: one risk-shift invocation per entity_id per day at 100% sampling — low-frequency; per DOS-222 spec line, risk shifts are bounded by tracked-account count.

Dimensions: relevance >= 0.85, faithfulness >= 0.90, attribution-completeness >= 0.95 — matches DOS-219.

Divergence definition: judge score variance between legacy and new ability over 7-day window, weighted equally across dimensions. Drift threshold <= 3%. Drift-failure rule: alert + investigate; do NOT auto-cutover until divergence trends down for 7 days.

### Post-synthesis source_ref membership check (folds Codex F5)

After the LLM returns RiskIndicator[], every source_ref in every indicator is verified against the GATED input source registry — the trajectory points + signals + claims that were sensitivity-gated at input. Refs that do not match a gated input source ID are rejected: the indicator is dropped — not the whole output. ProvenanceWarning::HallucinatedSourceRef increments.

Fixture: bundle-11-hallucinated-source-ref
- Seeds: input with N trajectory points; LLM returns an indicator with a fabricated source_ref ID
- Asserts: indicator dropped; warning counter incremented; rest of output preserved

### Revoked-Glean revalidation (folds Codex F6)

BEFORE prompt assembly, cached trajectory source_refs are revalidated against current Glean revocation state. If a source backing a cached trajectory point has been revoked since the cache write, that point is dropped from prompt input. Pair with ProvenanceWarning::SourceRevoked shape from W5-B V2.

Fixture: bundle-11-revoked-cached-trajectory
- Seeds: trajectory cached at T1; source revoked at T2; risk-shift call at T3
- Asserts: revoked source trajectory point dropped from prompt
- Asserts: ProvenanceWarning::SourceRevoked emitted

### trajectory_delta_v1 contract pinned (folds Codex F7)

Algorithm version: trajectory_delta_v1
- Input contract: TrajectoryBundle with engagement_curve, role_progression — current runtime shape
- Window sizes: 30-day vs 90-day delta windows
- Thresholds:
  - Improving: 30-day engagement up greater than 10% AND no role-progression downward signal
  - Stable: absolute 30-day engagement delta less than or equal to 10% AND no role-progression downward
  - DegradingMinor: 30-day engagement down 10-25% OR single role-progression downward
  - DegradingMajor: 30-day engagement down greater than 25% OR multiple role-progression downward signals
- Null handling: insufficient data — less than 7 days of engagement curve — yields RiskDirection::InsufficientEvidence + ProvenanceWarning::InsufficientEvidence
- Output mapping: direction enum, deterministic, golden-testable

Golden tests in Stage-2 fixtures:
- account-trending-down yields DegradingMajor + indicators
- account-stable yields Stable + empty indicators
- account-active-escalation yields DegradingMajor or DegradingMinor depending on signal magnitude
- account-with-revoked-Glean yields InsufficientEvidence — Glean revocation drops enough trajectory points that the 7-day floor is not met

### Changelog V2 (2026-05-14)

- V2 folds 11 cycle-1 panel findings — 4 architect + 7 codex.
- Architect F1 + Codex F2 maps to ADR-0106 sampling capture
- Architect F2 maps to Untrusted-output downstream-handoff
- Architect F3 maps to evidence_summary.source_asof = oldest contributing
- Architect F4 maps to Empty-indicators path
- Codex F1 maps to Channel-audit citations
- Codex F3 maps to TrajectoryBundle subject-fit
- Codex F4 maps to Judge config pinned
- Codex F5 maps to Post-synthesis source_ref membership check
- Codex F6 maps to Revoked-Glean revalidation
- Codex F7 maps to trajectory_delta_v1 contract pinned
