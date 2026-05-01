# Implementation Plan: DOS-219

## Revision history
- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-219 migrates meeting prep into the first real Transform ability. The load-bearing split is a pure LLM-calling brief builder plus non-pure publish/cache/maintenance behavior outside that builder. Linear pins the contract: "Meeting prep is the most visible ability output"; "Validates the whole Transform pattern"; "**Reads claims only**"; "**Writes claims only**"; "`SubjectAttribution` end-to-end with composition"; "`source_asof` populated through composition"; "`temporal_scope` distinguishes meeting-event vs state claims"; and "Subject-bleed gate exercised."

The 2026-04-24 PM amendments all apply. The single-source-of-truth amendment forbids reads from `entity_intelligence` columns or `intelligence.json`, and forbids direct writes to legacy AI surfaces. The prompt/eval amendment applies because this Transform must use `prepare_meeting_prep.v{n}.txt`, record `PromptFingerprint`, and pass judge-scored fixtures. The production-readiness comments apply: "`prepare_meeting` should not remain one giant narrative prompt"; expected behavior "must not render confident advice from stale, wrong-subject, restricted, or contradicted evidence"; and "every material topic and suggested outcome has source refs + source timestamp."

Current legacy reality is a bundled side-effect path. `refresh_meeting_briefing` builds a live service context, calls `refresh_meeting_briefing_full`, emits `entity-updated`, and returns `MeetingBriefingRefreshResult` (`src-tauri/src/commands/core.rs:199-214`). The service path mutates immediately (`src-tauri/src/services/meetings.rs:2974-2980`), snapshots current prep and linked entities (`:3014-3042`), refreshes each linked entity (`:3068-3103`), rebuilds mechanical prep (`:3189-3200`), may enqueue a prep rebuild (`:3236-3244`), and updates intelligence state metadata (`:3282-3299`). The prep queue writes `prep_context_json` and `prep_frozen_json` directly (`src-tauri/src/meeting_prep_queue.rs:592-614`), while PTY enrichment later reads and writes `prep_frozen_json` (`:793-803`, `:857-864`). W5-B separates those behaviors.

## 2. Approach

Create `src-tauri/src/abilities/prepare_meeting/` with `mod.rs`, `build_brief.rs`, `prompts.rs`, `synthesis.rs`, `publish.rs`, and `maintenance.rs`. `mod.rs` registers external ability name `prepare_meeting`; the pure entry in `build_brief.rs` is `build_meeting_brief(ctx, PrepareMeetingInput) -> AbilityResult<MeetingBrief>`. This reconciles ADR-0102's public ability naming with the W5 prompt's file split.

`build_brief.rs` owns only the Transform: load operational meeting identity/attendee seed, load intelligence content from active claims and composed children, call `ctx.intelligence.complete()` through W2's provider trait (`src-tauri/src/intelligence/provider.rs:196-202`), parse structured output, validate, build provenance, and return `AbilityOutput<MeetingBrief>`. It never calls `ctx.check_mutation_allowed()`, never opens DB writes, never emits Tauri events, never queues work, and never writes `meeting_prep`, `intelligence.json`, or claim rows.

Input shape: keep ADR-0102's `PrepareMeetingInput { meeting_id, depth, include_open_loops, schema_version }` (`.docs/decisions/0102-abilities-as-runtime-contract.md:140-149`). Internally build a `MeetingBriefContext` value from the meeting seed, deduped subjects, child `get_entity_context` outputs, per-meeting active claims, and open-loop claim queries. The caller should not pass a preassembled context because that would create a second ability contract and make MCP/Tauri parity weaker. Testability comes from Evaluate-mode fixtures and a private context-builder seam, not from public raw-context input.

The new context builder is claim-backed, not a wrapper around legacy `gather_account_context`. The legacy builder is useful only as a parity map: it currently resolves `linked_entities` (`src-tauri/src/prepare/meeting_context.rs:117-123`), reads account files and DB context (`:277-357`), dispatches by primary entity (`:724-745`), and injects entity intelligence. DOS-219 replaces that intelligence content with claims/composition so the ability does not read legacy entity-intelligence columns or `intelligence.json`.

Algorithm:
1. Resolve the meeting subject and related attendee/account/person `SubjectRef`s through `abilities/common/` helpers shared with W5-A.
2. Compose `get_entity_context` once per deduped subject; store children under stable `composition_id`s.
3. Query active `intelligence_claims` for per-meeting and open-loop claims until `list_open_loops` lands in DOS-221.
4. Compute `what_changed_since_last` from claim history / claim sequence diffs.
5. Build a source-ref-preserving prompt input with trust bands, sensitivity ceilings, temporal scope, and source lifecycle flags.
6. Invoke the prompt template registry entry `prepare_meeting_prep.v1.txt` and record the ADR-0106 fingerprint fields.
7. Parse `MeetingBrief { meeting, topics, attendee_context, open_loops, what_changed_since_last, suggested_outcomes, schema_version }` from structured JSON.
8. Deterministically validate subject fit, source access/revocation, freshness, duplicate/paraphrase merge, tombstone pre-gate, sensitivity, and contradiction/user-correction state before returning.
9. Run targeted repair only for failed, low-trust, or feedback-affected candidate items; do not regenerate the whole brief when one talking point fails.

`publish.rs` is not an ADR-0102 Publish ability. It is the internal persistence half of this feature: it converts accepted `MeetingBrief` sections into `ClaimProposal`s and calls `services::claims::commit_claim`. Claim types must be DOS-300 registry values: `meeting_topic`, `attendee_context`, `open_loop`, `meeting_change_marker`, `suggested_outcome`, plus `meeting_event_note`/`meeting_readiness` where the registry owner confirms names (`.docs/plans/wave-W3/DOS-300-plan.md:23-27`). It writes no legacy surfaces; DOS-301 projection owns compatibility.

`maintenance.rs` owns scheduled refresh orchestration invoked through W4-C `WorkerAbilityBridge` as `Actor::System`. It selects stale/dirty meetings, invokes the Transform, and then calls `publish.rs` only under explicit policy preauthorization. Cache invalidation is claim invalidation plus DOS-301 projection repair, not direct file/column mutation.

End-state alignment: W5-B makes the flagship meeting brief a registry-discovered, fixture-scored ability with provenance, trust, subject fit, and source freshness. It forecloses the old path where a manual refresh bundles entity refresh, LLM synthesis, cache writes, DB writes, and UI events in one command.

## 3. Key decisions

Transform boundary: `build_brief.rs` is category `Transform`, `mutates = []`, `allowed_actors = [User, Agent, System]`, `allowed_modes = [Live, Simulate, Evaluate]`. ADR-0102 defines Transform as no service mutation and may invoke the provider (`.docs/decisions/0102-abilities-as-runtime-contract.md:78-94`), with Transform output untrusted for mutation authorization (`:304-323`). `System` is allowed because scheduled maintenance needs to re-run synthesis, but only `maintenance.rs` may persist after policy preauthorization.

About-this shape: no inline `SourceAttribution` fields in `MeetingBrief`. ADR-0102 says provenance lives once on `AbilityOutput<T>` (`.docs/decisions/0102-abilities-as-runtime-contract.md:166-179`), and ADR-0105 requires `field_attributions: BTreeMap<FieldPath, FieldAttribution>` (`.docs/decisions/0105-provenance-as-first-class-output.md:199-241`). Each topic/bullet/section gets field-path attribution with `DerivationKind::LLMSynthesis`, direct child refs, subject attribution, trust band, and `source_asof` freshness. UI "About this" reads the top-level provenance map.

Context ownership: public input is `MeetingId`; private context is derived inside. This keeps Tauri, MCP, worker, and eval surfaces invoking the same ability. The one ambiguity is whether the meeting row read is allowed under "reads claims only"; this plan treats operational meeting metadata as non-intelligence seed data while all intelligence content comes from claims/composed abilities.

Source freshness: every source emitted by the prompt parser is normalized through DOS-299's chain: `itemSource.sourcedAt` -> Glean document timestamps -> source-class date -> `SourceTimestampUnknown`. The legacy entity prompt already requires `itemSource.sourcedAt` (`src-tauri/src/intelligence/dimension_prompts.rs:748-764`), and `ItemSource.sourced_at` is the existing evidence-time field (`src-tauri/src/intelligence/io.rs:29-37`). DOS-219 fixtures must prove one populated `source_asof` and one unknown warning, matching ADR-0105's amendment (`.docs/decisions/0105-provenance-as-first-class-output.md:391-446`).

Temporal/sensitivity assignment: meeting-event facts use `PointInTime { occurred_at: meeting.start }`; persistent context/risk/outcome claims use `State`; trend/change summaries use `Trend` only when a bounded window is present. Sensitivity is explicit per claim: `Internal` default, `Confidential` for personal stakeholder context, and `Public` only when source-class inheritance makes it safe. ADR-0125 pins these enums and conservative defaults (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:21-87`).

Snapshot/cache strategy: authoritative writes are claims only. Legacy meeting-prep compatibility must be a DOS-301 projection target; current DOS-301 targets list `entity_intelligence`, `success_plans`, `accounts_columns`, and `intelligence_json` (`.docs/plans/wave-W3/DOS-301-plan.md:21-40`), so this plan does not direct-write `meeting_prep` as a workaround.

## 4. Security

The primary risks are prompt injection, subject bleed, revoked/restricted source exposure, and internal notes leaking into agent/customer-facing suggestions. Prompt inputs must wrap untrusted source text and never let source text choose actor, subject, sensitivity, or claim type. Transform output is untrusted; `publish.rs` needs a separate user/policy trust signal before claims are committed.

Subject-fit is a hard gate for claim-bearing output. W3-B finalization rejects ambiguous/blocked subject fit (`.docs/plans/wave-W3/DOS-211-plan.md:57-59`), DOS-7 `commit_claim` validates subject refs and tombstone pre-gates before dedup (`.docs/plans/wave-W3/DOS-7-plan.md:27`), and ADR-0123 `WrongSubject` tombstones only the asserted subject (`.docs/decisions/0123-typed-claim-feedback-semantics.md:45-54`, `:146-153`). Same-domain and multi-account meetings render `SubjectAmbiguous` warnings or suppress confident claims.

No prompt, completion, claim text, account name, source excerpt, access token, Glean opaque payload, or customer domain appears in logs or test failure messages. Fixture content follows ADR-0110 anonymization rules and CLAUDE.md's no-customer-data rule.

## 5. Performance

Current manual refresh does serial linked-entity enrichment before prep rebuild (`src-tauri/src/services/meetings.rs:3068-3103`) and blocking mechanical prep (`:3189-3200`). W5-B should be no slower at p50/p99 in Suite P by batching claim retrieval, deduping subject composition calls, and avoiding entity refresh inside the Transform path.

Budgets: one meeting seed read, one batched active-claim query per subject group, one open-loop query, N deduped `get_entity_context` child invocations, one provider call for initial synthesis, and bounded targeted repair only for invalid candidates. Provenance size should stay below W3-B's normal Transform budget (~50KB) and never exceed the builder cap. Cache keys for any in-memory child results include user, mode, schema version, provider fingerprint inputs, and claim invalidation epochs; never key only on `meeting_id`.

## 6. Coding standards

Services-only mutations: `build_brief.rs` has no mutations; `publish.rs` mutates only by calling `services::claims.rs`; `maintenance.rs` invokes services and honors `ServiceContext` mode gates. No direct DB/file/signal writes from ability modules. Do not edit `src-tauri/src/services/context.rs` or `src-tauri/src/intelligence/provider.rs`.

Intelligence Loop 5-question check: new meeting-brief claims need claim invalidation/projection; trust bands can feed briefing render but not health scoring directly; claim-backed context replaces legacy `gather_account_context` intelligence inputs; meeting callouts consume projections, not raw Transform output; user correction feedback feeds DOS-294/DOS-7 claim feedback paths.

No direct `Utc::now()` or `thread_rng()` in services or abilities. Use `ctx.services.clock.now()`, fixed fixture clocks, and replay providers. No real customer data in fixtures. Clippy budget remains zero warnings under the standard gate.

## 7. Integration with parallel wave-mates

W5-A/DOS-218 coordinates on `src-tauri/src/abilities/common/`: `SubjectRef` resolution, entity/attendee subject loading, claim-backed context DTOs, and child-composition helpers. If W5-A lands helpers first, import them; if W5-B lands first, keep helpers generic and documented for W5-A adoption.

W3-A/DOS-210 supplies registry metadata, `AbilityContext`, actor/mode policy, and category checks (`.docs/plans/wave-W3/DOS-210-plan.md:24-36`). W3-B/DOS-211 supplies `AbilityOutput<T>`, field attribution, source attribution, and subject fit (`.docs/plans/wave-W3/DOS-211-plan.md:31-45`). W3-C/DOS-7 supplies `services::claims::commit_claim` and tombstone semantics (`.docs/plans/wave-W3/DOS-7-plan.md:19-31`). W3-D/DOS-301 must add/confirm the meeting-prep projection target before cutover. W3-G/DOS-299 supplies source-time parsing. W3-H/DOS-300 must include all meeting-brief claim types. W4-A/DOS-5 supplies `TrustBand`. W4-B/DOS-216 owns fixture runner shape. W4-C/DOS-217 invokes Tauri as `Actor::User`, MCP as `Actor::Agent`, and worker/eval paths without ability-specific branches.

## 8. Failure modes + rollback

If Transform synthesis fails, return a typed ability error with no writes; old legacy path remains available during ADR-0112 Stage 3. If validation rejects some items, return a partial brief only when the remaining fields have complete provenance and diagnostics explain suppressed/ambiguous items; otherwise fail closed. If `publish.rs` claim commit fails, the brief can still render ephemerally but no legacy projection is attempted.

If DOS-301 projection fails after claims commit, the claim remains authoritative and `claim_projection_status` records failure per DOS-301 (`.docs/plans/wave-W3/DOS-301-plan.md:48-66`); repair recomputes from claims. If bundle-5 tombstone/correction handling fails, block cutover and keep Stage 3 parallel run off or old-path-visible. Rollback is ADR-0112 Stage 3/4 rollback: disable new invocation or wrap the Tauri command back to legacy (`.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:128-144`).

W1-B write fence is honored because W5-B never writes `intelligence.json` directly. Legacy file projection, if any, goes through DOS-301 and `fenced_write_intelligence_json`.

## 9. Test evidence to be produced

Unit tests: `prepare_meeting_transform_descriptor_has_no_mutations`, `prepare_meeting_build_brief_returns_ability_output_with_provenance`, `prepare_meeting_every_leaf_has_field_attribution`, `prepare_meeting_llm_fields_require_source_refs`, `prepare_meeting_prompt_fingerprint_recorded`, `prepare_meeting_source_asof_from_item_source_sourced_at`, `prepare_meeting_source_timestamp_unknown_warns`, `prepare_meeting_subject_multi_for_multi_account`, `prepare_meeting_ambiguous_subject_blocks_confident_claim`, and `prepare_meeting_publish_uses_claims_service_only`.

Integration/parity tests: `prepare_meeting_parallel_legacy_bundle5_parity`, `prepare_meeting_bundle5_wrong_subject_tombstone_no_resurrection`, `prepare_meeting_same_domain_account_bleed_ambiguous_not_confident`, `prepare_meeting_stale_glean_downweighted_not_current_truth`, `prepare_meeting_revoked_source_masks_rendered_provenance`, `prepare_meeting_user_edited_claim_overrides_ai`, `prepare_meeting_duplicate_claims_render_once_with_corroboration`, `prepare_meeting_double_refresh_idempotent_no_duplicate_claims`, and `refresh_meeting_briefing_command_wraps_invoke_ability_after_cutover`.

Fixtures under `src-tauri/tests/abilities/prepare_meeting/fixtures/`: first external meeting with no entity, recurring 1:1 with rich history, recurring instance with attendee change, multi-account same-domain ambiguity, project-led meeting, stale Glean >180 days, Glean unavailable, revoked source, user-edited claim override, recent transcript contradicts old summary, duplicate/paraphrased claims, ambiguous primary entity, and bundle-5 correction resurrection. Each fixture includes `provider_replay.json`, `expected_output.json`, and `expected_provenance.json` per ADR-0110 (`.docs/decisions/0110-evaluation-harness-for-abilities.md:20-39`) and Transform thresholds relevance >=0.85, faithfulness >=0.90, attribution completeness >=0.95 (`:107-121`).

Wave gate artifact: W5 proof bundle with `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`, `cargo test --test harness prepare_meeting`, Suite P p50/p99 comparison vs legacy, Suite E bundle-5 parity/correction-resurrection report, accessibility-tester "About this" pass, architect-reviewer provenance/claim contract pass, and codex-consult prompt/fingerprint review.

## 10. Open questions

1. Path conflict: Linear says `src-tauri/src/abilities/transform/prepare_meeting/`, while the W5 prompt and wave doc assign `src-tauri/src/abilities/prepare_meeting/`. This plan uses the wave-owned path; confirm before coding.
2. DOS-301 projection target: current plan targets do not name `meeting_prep`/`MeetingBrief`. Should DOS-301 add a `meeting_prep` target, map W5-B through `intelligence_json`, or keep Stage 3 parallel-only until a projection manifest covers it?
3. Claim type names: are `meeting_topic`, `meeting_event_note`, `attendee_context`, `meeting_change_marker`, `suggested_outcome`, and `meeting_readiness` the final DOS-300 canonical strings?
4. Context seed read: is reading operational `meetings`/`meeting_entities` metadata acceptable under "reads claims only", or must Tauri/worker pass a trusted `MeetingSummarySeed` into the ability?
5. Targeted repair budget: should the first implementation allow one repair pass per invalid item, or should DOS-216 `quality.toml` own the exact repair fanout limit?
