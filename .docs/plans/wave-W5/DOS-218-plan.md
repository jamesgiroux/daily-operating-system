# Implementation Plan: DOS-218

## Revision history

- v1 (2026-05-01) — initial L0 draft.

## 1. Contract restated

DOS-218 is the first real Read ability on the v1.4.0 spine. The W5-A slot says this is a pilot, not a feature expansion: "Parity with the legacy command on bundle-1 is the bar, not improvement" (`.docs/plans/wave-W5/_prompts/DOS-218.md`). Current legacy behavior is narrow: `get_entity_context_entries(entity_type, entity_id)` returns `Vec<EntityContextEntry>` from `commands/workspace.rs:1237-1242`; the service reads `entity_context_entries` by `(entity_type, entity_id)` ordered `created_at DESC` (`src-tauri/src/services/entity_context.rs:20-45`); the DTO has only `id`, `entity_type`, `entity_id`, `title`, `content`, `created_at`, `updated_at` (`src-tauri/src/types.rs:2462-2473`).

Linear's pilot contract is broader and must be quoted because it conflicts with the W5 slot. It says: "**Stage 1:** write `get_entity_context` ability at `src-tauri/src/abilities/read/get_entity_context.rs`"; "**Reads claims only.** This ability reads from `intelligence_claims`"; and the output is `EntityContext { identity, state, relationships, recent_events, open_loops, trajectory, schema_version }` wrapped in `AbilityOutput<EntityContext>`. This plan follows the wave-owned path and parity surface, `src-tauri/src/abilities/get_entity_context/`, and treats the claims-only/structured-EntityContext shape as an L0 open question rather than silently changing scope.

2026-04-24 amendments that apply to this W5 slice: `SubjectAttribution` must prove each returned note belongs to the requested subject; `source_asof` must be populated when knowable; `thread_ids` are present but empty in provenance; `temporal_scope`/`sensitivity` are explicit if a claim row is emitted; hostile entity fixtures from the Linear comments apply. The comments are load-bearing: "success is suppressing or marking content whose subject does not match the requested entity" and "do not accept an implementation that only proves happy-path entity reads."

## 2. Approach

Create the W5-owned ability directory: `src-tauri/src/abilities/get_entity_context/{mod.rs,input.rs,output.rs,types.rs}`. Register one Read ability named `get_entity_context`, category `Read`, `mutates = []`, `composes = []`, `allowed_modes = [Live, Evaluate]`, and at minimum `allowed_actors = [User]`; add `Agent` only if reviewers accept user-authored notes over MCP with ADR-0108 filtering. The implementation reads through the existing service/query shape, but the ability code must accept an `AbilityContext`, not `AppState`; if W3-A exposes a read-capable service handle, use that. If not, add only an ability-local read helper over `ctx.services` and record the helper as temporary until `services::entity_context::get_entries` gains `ServiceContext`.

Input: `GetEntityContextInput { schema_version, entity_type, entity_id }`. Validate `entity_type` as `account | project | person | meeting` before query, normalize into W3/W1 `SubjectRef` (`src-tauri/src/db/claim_invalidation.rs:58-64` shows the current variants), and reject unsupported values with a typed ability error. Keep the frontend payload compatible with `useEntityContextEntries`, which already passes `entityType` and `entityId` to the old command (`src/hooks/useEntityContextEntries.ts:14-18`).

Output: return `AbilityOutput<Vec<EntityContextEntry>>`. The `data` JSON must equal the legacy command result byte-for-byte after canonicalization for bundle-1 fixtures. Do not add sensitivity, trust, or provenance fields to `EntityContextEntry`; ADR-0102 requires provenance to live exactly once on the wrapper (`.docs/decisions/0102-abilities-as-runtime-contract.md:166-179`, `:298-305`).

Provenance algorithm: for each row, add one direct `SourceAttribution` with `DataSource::User` because ADR-0107 defines `User` as "user-written context" (`.docs/decisions/0107-source-taxonomy-alignment.md:27-30`). Use the entry id as the source identifier. Parse `updated_at` as the current note author's source time and set both `source_asof` and `observed_at` from it; fall back to `created_at` if needed; if both are unparsable, set `source_asof = None` and emit `SourceTimestampUnknown`. Attribute every serialized leaf (`/0/title`, `/0/content`, etc.) as `Direct`, user-confirmed subject, implicit confidence. Top-level subject is the requested `SubjectRef`; field subject must match or finalize fails per DOS-211 subject-fit rules (`.docs/plans/wave-W3/DOS-211-plan.md:57-61`).

Keep `get_entity_context_for_prompt` behavior in view but do not migrate it in this PR. It returns only `(title, content)` and silently returns empty on SQL errors (`src-tauri/src/intelligence/user_context.rs:89-108`); `build_intelligence_context` formats those notes into "### title\ncontent" (`src-tauri/src/intelligence/prompts.rs:1171-1179`) and prompt rendering wraps them as user data (`:1931-1934`). The parity fixture should assert the ability does not change this prompt-context path until a later reader migration intentionally consumes the ability.

End-state alignment: this proves Read-category registration, schema invocation, provenance, subject-fit, source time, trust band, and exact eval parity on the smallest useful user-authored read. It forecloses a hidden feature expansion of entity context into synthesized account state during W5.

## 3. Key decisions

Input shape: choose `(entity_type, entity_id)` in the public schema, then convert to `SubjectRef` internally. Reason: legacy Tauri and frontend already use this shape (`src-tauri/src/commands/workspace.rs:1237-1242`, `src/hooks/useEntityContextEntries.ts:14-18`), while raw `SubjectRef` would leak Rust enum shape to TypeScript before W3-A/W4-C stabilize schema ergonomics. Subject validation still uses `SubjectRef` so provenance and cross-entity tests are typed.

Output shape: choose `AbilityOutput<Vec<EntityContextEntry>>`, not Linear's claims-oriented `EntityContext`. Reason: W5 prompt says parity with `get_entity_context_entries` is the bar, and Read evals require exact output equality (`.docs/decisions/0110-evaluation-harness-for-abilities.md:43-46`). Any richer `EntityContext` projection must be a separate accepted delta or a contract change.

Claims: this ability reads `entity_context_entries`, not `intelligence_claims`, for W5 parity. It does not create claims and therefore does not require a DOS-300 `CLAIM_TYPE_REGISTRY` entry. If reviewers decide the Linear "Reads claims only" amendment wins, this plan needs revision because bundle-1 parity against the current command cannot be exact without a projection/backfill dependency.

Source attribution: user-authored notes use `DataSource::User` and provenance actor `Actor::User`; they are not LLM synthesis and do not get `DataSource::Ai`, `Glean`, or `LegacyUnattributed`. `source_asof` for the current note content is `updated_at`, with `created_at` retained only as fallback because edits change the authored content.

Sensitivity: treat these notes as `Internal` by default if materialized as claims later, never `Public`. `Internal` is the ADR-0125 conservative default for internal system/user surfaces and is MCP-safe (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:64-85`). Person-specific or legal/HR notes may need `Confidential`, but adding that to the legacy DTO would break parity; surface it in §10.

Trust: user-authored direct rows should map to `TrustBand::LikelyCurrent` when timestamps parse and subject fit is confident. This does not mean the user's note is globally true; it means the returned field is a direct current copy of saved user content. Malformed timestamps, blocked subject fit, or revoked/masked source state lower or block the band according to W4-A (`.docs/plans/wave-W4/DOS-5-plan.md:24-26`, `:74-79`).

Composition: declare no composed children for this ability. The provenance envelope still exercises composition shape with `children = []`; W5-B can consume this ability later from Transform composition.

## 4. Security

Primary risk is cross-entity bleed. The query already scopes by `(entity_type, entity_id)` and uses bind params (`src-tauri/src/services/entity_context.rs:20-31`), but W5 must add input enum validation and subject-fit assertions so same-domain accounts, parent/child/project overlap, duplicate claims, and user corrections from the Linear comments cannot be rendered as if they belonged to the requested subject.

Second risk is privacy of user-written notes. Logs, eval diffs, provenance errors, and bridge errors must not print `title` or `content`; use entry id, source class, field path, and subject only. Fixtures must use generic companies/domains per `CLAUDE.md:16-18`, and W4-B provenance diffs must avoid raw source excerpts (`.docs/plans/wave-W4/DOS-216-plan.md:63-69`).

Auth/authz is registry/bridge-owned. Tauri invokes as `Actor::User` in Live mode per ADR-0111 (`.docs/decisions/0111-surface-independent-ability-invocation.md:21-27`, `:33-66`). If `Actor::Agent` is allowed for MCP, the response must rely on W4-C/ADR-0108 actor-filtered provenance and should be reviewed by accessibility-tester + architect-reviewer before exposing user notes to agents.

## 5. Performance

The hot path is the same indexed read as today: `entity_context_entries` has `idx_entity_context_entity(entity_type, entity_id)` (`src-tauri/src/migrations/051_entity_context_entries.sql:12-13`) and orders by `created_at DESC` without a covering order column. Expected query cost is unchanged from the command; added cost is JSON schema validation plus provenance construction O(rows × fields).

Budget target: p95 ability overhead under 5ms beyond the legacy query for bundle-1 fixture sizes. No provider calls, Glean calls, embeddings, cache writes, signals, or DB writer locks are allowed. If note volume exceeds provenance soft budget, W3-B's size policy warns at 100KB and hard-elides over 1MB (`.docs/decisions/0105-provenance-as-first-class-output.md:327-333`); W5 should add a high-row fixture only if reviewers request it.

## 6. Coding standards

Services-only mutations: the ability is pure Read and must not call `create_entry`, `update_entry`, `delete_entry`, signal emission, embedding generation, projection, or claim commit. Current mutation methods in `entity_context.rs` call `ctx.check_mutation_allowed()` and emit signals (`src-tauri/src/services/entity_context.rs:50-95`, `:122-183`, `:186-238`); W5 must not route through them.

Intelligence Loop check (`CLAUDE.md:7-14`): no new table/column, signal type, health scoring rule, briefing callout, or feedback hook is added. The ability exposes an existing data surface through the spine. It should not change `build_intelligence_context()` prompt injection behavior in this PR; that path remains at `src-tauri/src/intelligence/prompts.rs:1163-1180`.

No direct `Utc::now()` or `thread_rng()` in ability code; use `ctx.services.clock.now()` only for `produced_at`/fallback diagnostics. Fixture data must be synthetic. Clippy budget is the standard `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` from `CLAUDE.md:20-24`.

## 7. Integration with parallel wave-mates

W3-A/DOS-210 owns the registry, `AbilityContext`, actor/mode policy, and erased `invoke_by_name_json` (`.docs/plans/wave-W3/DOS-210-plan.md:24-36`, `:76-84`). W5-A consumes those only. W3-B/DOS-211 owns `AbilityOutput<T>`, `ProvenanceBuilder`, `SourceAttribution`, `SubjectAttribution`, and finalize errors (`.docs/plans/wave-W3/DOS-211-plan.md:19-45`, `:57-67`).

W3-G/DOS-299 owns source-time helpers and warnings; W5-A should reuse those for `updated_at`/`created_at` parsing instead of hand-rolling timestamp semantics (`.docs/plans/wave-W3/DOS-299-plan.md:17-23`, `:85-93`). W3-H/DOS-300 is read-only here because no claim rows are emitted (`.docs/plans/wave-W3/DOS-300-plan.md:71-75`).

W4-A/DOS-5 owns `TrustBand` names and score/band mapping (`.docs/plans/wave-W4/DOS-5-plan.md:63-75`). W4-B/DOS-216 owns fixture layout; W5-A fixture roots go under `src-tauri/tests/abilities/get_entity_context/fixtures/` with `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `clock.txt`, `seed.txt`, `expected_output.json`, `expected_provenance.json`, and additive `metadata.json` (`.docs/plans/wave-W4/DOS-216-plan.md:16-31`, `:53-61`). W4-C/DOS-217 invokes via `TauriAbilityBridge`; W5-A should not add an ability-specific bridge (`.docs/plans/wave-W4/DOS-217-plan.md:18-31`).

W5-B/DOS-219 may create `abilities/common/`; W5-A should put only genuinely shared `SubjectRef` parsing there if both branches agree. Otherwise keep local helpers in `get_entity_context/` to avoid premature shared abstractions.

## 8. Failure modes + rollback

If ability registration fails, the old Tauri command remains registered at `src-tauri/src/lib.rs:641-648`; rollback is removing the W5 ability registration and leaving existing command/hook behavior untouched. If schema validation fails on legacy-compatible inputs, the parity test catches it before cutover.

If provenance finalization fails for a row because timestamps or subject attribution are invalid, return a hard ability error rather than data without provenance. For timestamp-only uncertainty, return data with `SourceTimestampUnknown` warning, matching ADR-0105's warning path (`.docs/decisions/0105-provenance-as-first-class-output.md:391-420`).

Parallel-run/cutover should follow ADR-0112: old and new paths run together, user receives old output, Read output exact-match is required, and rolling divergence target is <=1% (`.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:31-47`). W5's implementation PR should not remove old command handling until that evidence exists.

W1-B universal write fence is honored by construction: this Read ability performs no DB writes, file writes, signal propagation, queueing, projection, claim writes, or external calls. Evaluate fixtures run in-memory through W4-B and cannot fall through to Live providers.

## 9. Test evidence to be produced

Ability/unit tests: `get_entity_context_input_rejects_unknown_entity_type`, `get_entity_context_subject_ref_account_project_person_meeting`, `get_entity_context_empty_returns_empty_vec_with_subject_provenance`, `get_entity_context_orders_created_at_desc`, `get_entity_context_field_attribution_covers_every_entry_leaf`, `get_entity_context_user_source_sets_source_asof_from_updated_at`, `get_entity_context_unparseable_timestamp_warns_source_timestamp_unknown`, and `get_entity_context_wrong_subject_fixture_blocks_or_marks`.

Parity/eval fixtures: `fixture_001_bundle1_rich_user_notes`, `fixture_002_bundle1_empty_entity`, `fixture_003_bundle1_same_domain_wrong_entity_suppressed`, `fixture_004_bundle1_parent_child_project_boundary_labeled`, `fixture_005_bundle1_user_correction_tombstone_survives_refresh_stub`, and `fixture_006_bundle1_duplicate_subject_entities_not_merged`. Read scoring is exact output equality plus full provenance diff per ADR-0110 (`.docs/decisions/0110-evaluation-harness-for-abilities.md:41-51`, `:91-105`).

Bridge/parallel evidence: `tauri_bridge_invokes_get_entity_context_as_user_live`, `mcp_get_entity_context_agent_policy_redacts_or_rejects_user_notes` if Agent is allowed, `parallel_get_entity_context_matches_legacy_bundle1`, and a seven-day or simulated rolling-window artifact showing <=1% divergence before cutover.

Wave merge-gate artifact: `cargo test get_entity_context`, `cargo test --test harness get_entity_context`, `scripts/check_eval_fixture_anonymization.sh`, standard clippy/test/tsc, and `target/eval/harness-report.json` showing Bundle 1 coverage. Suite S contribution is subject-bleed/privacy evidence; Suite P contribution is read overhead vs legacy query; Suite E contribution is hostile entity ownership and exact parity fixtures.

## 10. Open questions

1. Contract conflict: should W5-A follow the wave prompt's legacy `Vec<EntityContextEntry>` parity surface, or Linear's broader claims-only `AbilityOutput<EntityContext>` surface? This plan picks parity; reviewer approval should make that explicit.
2. Module path conflict: Linear says `src-tauri/src/abilities/read/get_entity_context.rs`; W5 wave/prompt says `src-tauri/src/abilities/get_entity_context/`. This plan uses the wave-owned directory.
3. Actor policy: should user-authored entity notes be MCP-visible (`allowed_actors=[User, Agent, System]` per Linear) or User-only until render/sensitivity gates mature?
4. Sensitivity: is ADR-0125 `Internal` sufficient for all entity context entries, or should person/stakeholder notes be `Confidential` even if that prevents MCP/default-agent exposure?
5. Timestamp semantics: for edited notes, should `source_asof` be `updated_at` for current content as this plan proposes, or should provenance preserve original `created_at` plus a separate current-version timestamp?
6. Claims-only future: if entity context entries must eventually become `intelligence_claims`, which issue owns backfill/projection from `entity_context_entries` into claim rows without breaking the current note CRUD surface?
