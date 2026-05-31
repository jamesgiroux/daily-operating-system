# Implementation Plan: DOS-211

## Revision history

- v2 (2026-05-01) - L0 cycle-1 revision pass. Closed all 5 findings: thread_ids on envelope (ADR-0124), children: Vec<Provenance> matches ADR-0105 §51 (ComposedProvenance internal newtype with transparent serde), finalize-time failure clarified (ADR-0105 amendment follow-up), FeedbackEvent/SubjectFitGate deferred (out of scope), W3-A co-land seam concretized (W3-B owns all types).
- v1 (2026-04-30) - initial L0 draft.

## 1. Contract restated

DOS-211 creates the provenance substrate every ability returns through `AbilityOutput<T>`. The Linear contract is explicit: "Implement the `Provenance` envelope shape, `TrustAssessment`, composition merge semantics, and `ProvenanceBuilder` helper. Every ability output is wrapped in `AbilityOutput<T>` with provenance." It also says the builder "fails at build time if any output field lacks attribution", `sources[]` is "own direct only", `children[]` carries transitive sources, and "Stable `composition_id` per declared `composes` entry (not positional `child_idx`)."

The load-bearing ADR pins are ADR-0105 envelope fields at `.docs/decisions/0105-provenance-as-first-class-output.md:24-58` including the `children: Vec<Provenance>` wire field at `:51-52`, trust shape and authorized contribution source shape at `:106-158` and `:129-136`, source attribution at `:165-195`, field attribution at `:206-241`, composition merge at `:257-266`, builder behavior at `:305-311`, and size budget at `:331-333`. ADR-0102 requires `AbilityOutput<T>` to carry provenance exactly once (`.docs/decisions/0102-abilities-as-runtime-contract.md:166-170`, `:298-305`) and makes Transform output untrusted for mutation authorization (`:313-323`).

The 2026-04-24 amendments apply. The SubjectAttribution addendum says provenance must answer "what subject entity/meeting this field is about" and whether subject ownership is "direct, inherited, inferred, or user-confirmed"; it also asks for competing subjects and whether ownership is "confident, ambiguous, or blocked." The split notes make this normative: "`SubjectAttribution` becomes a typed substrate primitive on every `FieldAttribution` and on the `Provenance` envelope itself" and "ProvenanceBuilder enforces structural subject coherence" (`.docs/plans/roadmap-renumbering-2026-04-24.md:54-57`). ADR-0124 requires `thread_ids: Vec<ThreadId>` on `Provenance` now, defaulting to an empty vec, with `provenance_schema_version == 1` (`.docs/decisions/0124-longitudinal-topic-threading.md:37-49`). The source-time amendment changes `source_asof` to "must-be-populated-when-knowable" (`.docs/decisions/0105-provenance-as-first-class-output.md:391-401`) and says DOS-211 must carry the field while DOS-299 consumes/populates it (`:437-448`).

Current-code snapshot: `src-tauri/src/abilities/` does not exist; `src-tauri/src/lib.rs:43-66` exports `intelligence` and `services` but no abilities module. W2-B's frozen provider seam is readable context only: `src-tauri/src/intelligence/provider.rs:16-22` says `AbilityContext` lands in W3-A and provider routing later uses it; `FingerprintMetadata` lives at `:95-126`. W2-A's frozen context already owns `ExecutionMode`, `Clock`, and `SeededRng` (`src-tauri/src/services/context.rs:35-69`, `:109-121`) and explicitly says the provider seam is not `ServiceContext` (`:21-28`).

## 2. Approach

Create W3-B-owned provenance files under `src-tauri/src/abilities/provenance/`, plus the shared manifest dependency line if W3-A has not already pinned it:

- `mod.rs` - public exports and schema helpers.
- `envelope.rs` - `AbilityOutput<T>`, `Provenance`, `ComposedProvenance`, `CompositionId`, `ThreadId`, `InputsSnapshot`, and `ProvenanceWarning`.
- `builder.rs` - `ProvenanceBuilder`, `ProvenanceError`, field coverage walk, budget/truncation logic.
- `trust.rs` - `TrustAssessment`, `EffectiveTrust`, `TrustContribution`, trust merge.
- `source.rs` - `SourceAttribution`, `DataSource`, `SourceIdentifier`, `ScoringClass`, source indexes.
- `field.rs` - `FieldAttribution`, `FieldPath`, `SourceRef`, `DerivationKind`, `Confidence`, `SanitizedExplanation`.
- `subject.rs` - `SubjectAttribution`, `SubjectRef`, subject-fit enums, competing-subject shape.
- `src-tauri/Cargo.toml` - pin `schemars = "0.8.22"` as a non-optional dependency, not behind the `mcp` feature, matching the version already present through `rmcp` in `Cargo.lock`.

Keep `src-tauri/src/intelligence/provider.rs` and `src-tauri/src/services/context.rs` read-only. W3-A owns the abilities module root and imports W3-B's public provenance surface; W3-A does not define stubs for W3-B-owned types.

Define `pub const PROVENANCE_SCHEMA_VERSION: u32 = 1;`. Define `ThreadId` as `pub struct ThreadId(pub String)` deriving `Serialize`, `Deserialize`, `JsonSchema`, `Hash`, `Eq`, `PartialEq`, `Clone`, and `Debug`. Define `Provenance` per ADR-0105 with serde/schema derives, `provenance_schema_version: u32` defaulting to `PROVENANCE_SCHEMA_VERSION`, identity fields, `produced_at` from `ctx.services.clock.now()`, `InputsSnapshot`, `actor`, `mode`, computed `TrustAssessment`, direct `sources: Vec<SourceAttribution>`, required public `thread_ids: Vec<ThreadId>` with `#[serde(default)]`, optional prompt fingerprint, `children: Vec<ComposedProvenance>`, `field_attributions: BTreeMap<FieldPath, FieldAttribution>`, top-level `subject: SubjectAttribution`, and warnings.

Define `ComposedProvenance { composition_id: CompositionId, provenance: Box<Provenance> }` as internal Rust scaffolding only. Its custom serde implementation emits and accepts the bare `Provenance` shape for each child so the JSON wire format matches ADR-0105 §51 verbatim: `children` serializes as `Vec<Provenance>` with no wrapper objects (`.docs/decisions/0105-provenance-as-first-class-output.md:51-52`). `composition_id` remains the stable in-process key required by the ticket and is used by `SourceRef::Child { composition_id, field_path }`; it is never a runtime vector position.

Define `SourceAttribution` with ADR-0105 fields including `source_asof: Option<DateTime<Utc>>` (`.docs/decisions/0105-provenance-as-first-class-output.md:165-173`). `DataSource`, `GleanDownstream`, `SourceIdentifier`, and `ScoringClass` follow ADR-0107 (`.docs/decisions/0107-source-taxonomy-alignment.md:27-60`, `:75-131`), including backfill-only `LegacyUnattributed` for DOS-299 (`:223-241`). W3-B carries the field and warning variants; W3-G owns semantic timestamp parsing, backfill, and coverage enforcement.

Define `FieldAttribution` with `subject: SubjectAttribution`, `derivation: DerivationKind`, source refs, confidence, and optional `SanitizedExplanation`. `ProvenanceBuilder::finalize<T: Serialize>` serializes the domain output to `serde_json::Value`, expands expected JSON-pointer leaf paths, applies any subtree helpers to concrete leaves, and rejects missing attribution before returning `AbilityOutput<T>`. Direct projections use `pass_through()` to auto-fill field attributions. LLM synthesis requires source refs. Declared confidence requires a sanitized explanation.

The check runs at `ProvenanceBuilder::finalize()` (i.e., when the ability returns), NOT at Rust compile-time. ADR-0105:305 says "fails at build time" which we interpret as "builder.build() time, i.e., finalize" (`.docs/decisions/0105-provenance-as-first-class-output.md:305-310`). A follow-up will amend ADR-0105:305 to read "fails at finalize-time, not Rust compile-time" for clarity. We do NOT attempt a macro-time field-attribution check; the runtime check has equivalent safety because every ability output passes through `finalize()` before returning to the caller.

Builder lifecycle:

1. `new(ctx, descriptor)` captures ability identity, actor, mode, clock time, and empty direct sources/children/field map.
2. `source(...)` adds a direct source, derives `scoring_class` from `DataSource`, records `synthesis_marker`, and returns a stable `SourceIndex`.
3. `compose(composition_id, child)` validates the id against the declared `composes` metadata and stores the child without flattening its sources.
4. `attribute(...)`, `pass_through(...)`, and `subject(...)` populate field and subject attribution.
5. `finalize(data)` validates coverage, computes trust, applies size policy, and wraps the result in `AbilityOutput<T>`.

End-state alignment: this gives W3-C/DOS-7 a canonical provenance JSON payload, W3-F/DOS-296 the already-present thread-id slot to populate later, W3-G/DOS-299 a `source_asof` carrier, W4-A/DOS-5 deterministic trust inputs, and W5 pilots a single "About this" substrate. It forecloses author-set trust, positional child references, and un-attributed claim-bearing fields.

## 3. Key decisions

`TrustAssessment` shape is exactly `effective: EffectiveTrust`, `contributions: Vec<TrustContribution>`, and `contains_stored_synthesis: bool` per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:106-118`). The builder computes it; authors never set it. `EffectiveTrust` is only `Trusted | Untrusted`.

Trust contribution sources are limited to `DirectSource { source_index }`, `ComposedChild { composition_id }`, and `StoredSynthesisField { entity_id, field }`, matching ADR-0105 §129's authorized source categories (`.docs/decisions/0105-provenance-as-first-class-output.md:129-136`). Feedback and subject-fit effects are not encoded as trust contribution variants in DOS-211.

Read + structured trusted direct sources is `Trusted`; any prompt fingerprint, stored synthesis marker, unbounded external free text, or untrusted child makes the effective trust `Untrusted`; Maintenance/Publish merge the weakest child/direct contribution bottom-up. This follows ADR-0105's rules (`.docs/decisions/0105-provenance-as-first-class-output.md:149-158`) and ADR-0102's prompt-injection boundary.

`DerivationKind` variants ship as `Direct`, `Composed { composition_id: CompositionId }`, `Computed { algorithm: &'static str }`, `LLMSynthesis`, and `Constant`. This is ADR-0105's set (`.docs/decisions/0105-provenance-as-first-class-output.md:213-219`) with the child reference corrected to stable `composition_id` per the ticket. `SourceRef` mirrors that correction: `Source { source_index } | Child { composition_id, field_path }`.

`SubjectAttribution` ships as a typed primitive on both the envelope and every field attribution. Shape: `subject: SubjectRef`, `binding: SubjectBindingKind`, `supporting_source_refs: Vec<SourceRef>`, `competing_subjects: Vec<CompetingSubject>`, and `fit: SubjectFitAssessment { status, confidence, method }`. `SubjectBindingKind` covers direct/input-bound, inherited, inferred, source-matched, and user-confirmed. `SubjectFitStatus` is `Confident | Ambiguous | Blocked`; finalize rejects `Ambiguous` and `Blocked` for claim-bearing output. This implements the SubjectAttribution amendment and closes cross-entity bleed before render.

Finalize-time failure conditions are closed: missing top-level subject; missing field attribution on any serialized output leaf; non-constant field with no source or child ref; `LLMSynthesis` with empty refs; invalid source index; invalid `composition_id`; declared confidence without sanitized explanation; field subject not coherent with envelope subject unless explicitly `Multi`; manual trust override attempts; serialized provenance still >1MB after deepest-first child elision. These are runtime `ProvenanceBuilder::finalize()` errors, not Rust compile-time errors.

Warnings are explicit and serializable: existing ADR-0105 variants plus `SourceTimestampUnknown`, `SourceTimestampImplausible`, `SubjectFitQualified`, and `DepthElided`. Soft size over 100KB emits a warning; >1MB collapses deepest composition subtrees and emits `ProvenanceWarning::DepthElided` per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:331-333`).

JSON Schema uses non-optional `schemars = "0.8.22"` (`src-tauri/Cargo.toml:81-90` today has it behind `mcp`, and W3-B/W3-A must remove that feature gate). Provenance types derive `Serialize`, `Deserialize`, `JsonSchema`, and `PartialEq` directly; schema generation is ability-substrate behavior, not MCP-only behavior.

Forward compatibility: do not use `#[serde(deny_unknown_fields)]`. The required DOS-296 `thread_ids` field uses `#[serde(default)]` and keeps `provenance_schema_version = 1`, matching ADR-0124 (`.docs/decisions/0124-longitudinal-topic-threading.md:37-49`).

Composition merge order for W4-A is deterministic and follows ADR-0105's merge rules (`.docs/decisions/0105-provenance-as-first-class-output.md:257-263`): compute child provenance first, sort/validate children by declared `composition_id` order from W3-A registry metadata, keep `sources[]` direct-only, keep transitive sources under `children[]`, then build root trust by appending own direct contributions followed by child effective summaries in declared order. W4-A can read a sparse tree and reproduce weakest-trust propagation without flattening sources.

## 4. Security

The new attack surface is provenance becoming trusted explanation UI and trust input. The builder fails closed on missing or incoherent subject attribution so sourced content cannot appear on the wrong account merely because it has good source attribution. `SanitizedExplanation` is constructor-gated; no raw prompt text, source snippets, secrets, customer names, access tokens, or Glean opaque payloads go into explanations or error messages. `SourceIdentifier::OpaqueGleanSource` carries opaque refs only, matching ADR-0107 (`.docs/decisions/0107-source-taxonomy-alignment.md:95-109`).

Trust is data trust, not authorization. ADR-0102 still requires confirmation, policy, or safe schema range before untrusted Transform output authorizes mutations (`.docs/decisions/0102-abilities-as-runtime-contract.md:313-323`). The provenance builder only computes the marker consumed by registry/services. Manual construction with mismatched trust is rejected by avoiding public constructors for computed fields and by finalize recomputation.

Validation errors must be structural and non-content-bearing: `MissingFieldAttribution { field_path }` is acceptable; `Missing attribution for "Acme renewal risk..."` is not. Schema generation must not expose ability names an actor cannot enumerate; W4-C bridge work still owns actor-filtered MCP discovery.

## 5. Performance

Finalize is O(serialized output leaves + direct sources + children). The heaviest normal path is `serde_json::to_value` plus JSON-pointer flattening; expected provenance remains ADR-0105's ~500B-50KB range (`.docs/decisions/0105-provenance-as-first-class-output.md:327-333`). Composition does not flatten transitive sources, so deep trees avoid quadratic source copying. Size checks serialize provenance once near finalize; soft >100KB warns, hard >1MB performs deepest-first child elision before returning.

No DB queries, locks, network, provider calls, or cache writes are introduced in W3-B. Subject-fit checks operate on already-declared refs. JSON Schema generation is test/build-time via non-optional `schemars`, not on hot ability invocation paths.

Budget tests should cover a realistic wide output and a synthetic deep A->B->C->... chain. The synthetic case verifies deepest-first elision preserves root direct sources, root field attribution, aggregate trust, and a count of skipped levels.

## 6. Coding standards

W3-B adds substrate types and pure builder logic only. It performs no service mutation and therefore honors services-only mutation boundaries. Ability code using the builder must use `ctx.services.clock.now()` for `produced_at`; no `Utc::now()` or `thread_rng()` is introduced in `src-tauri/src/abilities/`, matching `ServiceContext` guidance (`src-tauri/src/services/context.rs:64-69`, `:109-121`).

Intelligence Loop check: no new schema write path, signal type, health-scoring rule, briefing surface, or feedback UI is introduced. Provenance is the substrate that later claims/feedback/trust work consumes. Fixtures must use synthetic IDs and generic entity names only. Clippy budget is zero new warnings; schema tests must run without requiring the `mcp` feature because `schemars = "0.8.22"` is non-optional.

Public constructors should encode invariants: `Confidence::declared` requires a `SanitizedExplanation`, `Confidence::implicit` is only used by direct projection helpers, and `SourceAttribution::new` derives scoring class instead of accepting it from author code. Any test-only bypasses live under `#[cfg(test)]`.

## 7. Integration with parallel wave-mates

W3-A/DOS-210 owns the abilities module root, registry metadata, category validation, and declared `composes` list. W3-B consumes that metadata shape for `CompositionId` validation but does not implement the macro or registry.

W3-B owns: `AbilityOutput<T>` (struct), `Provenance` (struct), `ComposedProvenance` (newtype wrapper), `CompositionId` (newtype), `ProvenanceBuilder`, `ProvenanceError`, `TrustAssessment`, `EffectiveTrust`, `TrustContribution`, `SourceAttribution`, `DataSource`, `FieldAttribution`, `Confidence`, `SanitizedExplanation`, `ThreadId`.

W3-A imports these via `use crate::abilities::provenance::*;` and does NOT redefine any of them.

Co-land path: W3-B PR lands first OR both land on a shared integration branch. W3-A's CI does not need a stub because the W3-B types must exist before W3-A compiles.

W3-C/DOS-7 stores provenance JSON on claims and consumes `SubjectRef`, `source_asof`, and warnings; its current plan already expects W3-B to feed those fields (`.docs/plans/wave-W3/DOS-7-plan.md:87`). W3-C owns the `intelligence_claims` table and write path; W3-B only supplies serializable payloads.

W3-E/DOS-294 owns `FeedbackAction` and `claim_feedback`; W3-B does not represent feedback as trust contribution variants in this issue. W3-F/DOS-296 populates the `thread_ids: Vec<ThreadId>` field that W3-B defines now with `#[serde(default)]` and no schema-version bump; W3-B must not add `deny_unknown_fields` anywhere that would reject later compatible fields. W3-G/DOS-299 owns `source_asof` population helpers and semantic bounds; W3-B defines the field and warning carrier. W4-A/DOS-5 consumes the deterministic trust tree; W3-B does not implement numeric trust scoring.

## 8. Failure modes + rollback

Missing attribution, invalid subject fit, invalid composition refs, or bad declared confidence are hard finalize errors and return no `AbilityOutput<T>`. Unknown source timestamps are not hard errors; they produce `SourceTimestampUnknown` warnings per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:401-420`). Implausible timestamps are carried as warnings for DOS-299 freshness handling; W3-B does not quarantine or migrate data.

Oversized provenance degrades by eliding deepest children; if still over 1MB, finalize fails so a raw oversize tree never leaves the ability. If schema generation fails, the implementation fails CI because ability schemas and downstream MCP tool descriptions would be incomplete.

Rollback is mechanical because W3-B has no migration and no runtime write path: revert the provenance module and its tests, then W3-A/W5 callers stop compiling until they drop imports. The W1-B universal write fence is honored by construction: this module does not write DB rows, files, signals, or external APIs. If a later ability writes after using provenance, that write remains behind ServiceContext/W1-B gates.

## 9. Test evidence to be produced

Unit tests in `src-tauri/src/abilities/provenance/`: `provenance_finalize_rejects_missing_field_attribution`, `provenance_finalize_rejects_ambiguous_subject_fit`, `provenance_direct_projection_autopopulates_field_attribution`, `provenance_llm_synthesis_requires_source_refs`, `provenance_declared_confidence_requires_explanation`, `provenance_read_structured_sources_trusted`, `provenance_transform_with_prompt_fingerprint_untrusted`, `provenance_publish_maintenance_inherit_weakest_child`, `provenance_contains_stored_synthesis_from_source_marker`, `composition_a_b_c_preserves_child_grandchild_tree`, `composition_refs_use_composition_id_not_position`, `source_asof_roundtrip_preserves_known_unknown_warning`, `json_roundtrip_preserves_equality`, `schemars_schema_for_provenance_is_valid_shape`, and `provenance_size_budget_warns_then_depth_elides`.

New L0 cycle-1 regression tests: `dos211_thread_ids_test.rs::thread_ids_default_empty_roundtrip`, `dos211_thread_ids_test.rs::thread_ids_two_ids_roundtrip`, `dos211_thread_ids_test.rs::provenance_schema_version_is_one`, `dos211_composition_wire_format_test.rs::children_serialize_as_bare_provenance_array_per_adr_0105`, and `dos211_finalize_unattributed_field_fails_test.rs::ability_emitting_unattributed_field_fails_at_finalize`.

Gate artifact for this PR: `cargo test provenance`, `cargo test --all-features provenance`, and `cargo clippy --all-targets --all-features -- -D warnings` from `src-tauri/`, plus the wave gate's broader W3 Suite S/P/E evidence. Suite S contribution: subject-fit reject tests for cross-entity bleed. Suite P contribution: budget/truncation test with deterministic size assertions. Suite E contribution: A->B->C composition + JSON roundtrip + schema generation fixture.

## 10. Out of scope

FeedbackEvent and SubjectFitGate trust contribution variants are deferred to a follow-up ADR amendment + ticket. ADR-0105 §129 only authorizes direct source, composed child, and stored synthesis field (`.docs/decisions/0105-provenance-as-first-class-output.md:129-136`).

## 11. Open questions

1. ADR-0105's checked-in file does not contain the concrete `SubjectAttribution` Rust shape even though the 2026-04-24 split notes and Linear comment make it required. Architect-reviewer should confirm the field names above before coding.
