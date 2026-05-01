# Implementation Plan: DOS-211

## Revision history

- v1 (2026-04-30) - initial L0 draft.

## 1. Contract restated

DOS-211 creates the provenance substrate every ability returns through `AbilityOutput<T>`. The Linear contract is explicit: "Implement the `Provenance` envelope shape, `TrustAssessment`, composition merge semantics, and `ProvenanceBuilder` helper. Every ability output is wrapped in `AbilityOutput<T>` with provenance." It also says the builder "fails at build time if any output field lacks attribution", `sources[]` is "own direct only", `children[]` carries transitive sources, and "Stable `composition_id` per declared `composes` entry (not positional `child_idx`)."

The load-bearing ADR pins are ADR-0105 envelope fields at `.docs/decisions/0105-provenance-as-first-class-output.md:24-58`, trust shape at `:106-158`, source attribution at `:165-195`, field attribution at `:206-241`, composition at `:257-266`, builder behavior at `:305-311`, and size budget at `:331-333`. ADR-0102 requires `AbilityOutput<T>` to carry provenance exactly once (`.docs/decisions/0102-abilities-as-runtime-contract.md:166-179`, `:298-305`) and makes Transform output untrusted for mutation authorization (`:313-323`).

The 2026-04-24 amendments apply. The SubjectAttribution addendum says provenance must answer "what subject entity/meeting this field is about" and whether subject ownership is "direct, inherited, inferred, or user-confirmed"; it also asks for competing subjects and whether ownership is "confident, ambiguous, or blocked." The split notes make this normative: "`SubjectAttribution` becomes a typed substrate primitive on every `FieldAttribution` and on the `Provenance` envelope itself" and "ProvenanceBuilder enforces structural subject coherence" (`.docs/plans/roadmap-renumbering-2026-04-24.md:54-57`). The source-time amendment changes `source_asof` to "must-be-populated-when-knowable" (`.docs/decisions/0105-provenance-as-first-class-output.md:391-401`) and says DOS-211 must carry the field while DOS-299 consumes/populates it (`:437-448`). DOS-296 adds `thread_ids: Vec<ThreadId>` additively with `provenance_schema_version = 1` (`.docs/decisions/0124-longitudinal-topic-threading.md:37-49`).

Current-code snapshot: `src-tauri/src/abilities/` does not exist; `src-tauri/src/lib.rs:43-66` exports `intelligence` and `services` but no abilities module. W2-B's frozen provider seam is readable context only: `src-tauri/src/intelligence/provider.rs:16-22` says `AbilityContext` lands in W3-A and provider routing later uses it; `FingerprintMetadata` lives at `:95-126`. W2-A's frozen context already owns `ExecutionMode`, `Clock`, and `SeededRng` (`src-tauri/src/services/context.rs:35-69`, `:109-121`) and explicitly says the provider seam is not `ServiceContext` (`:21-28`).

## 2. Approach

Create only W3-B-owned provenance files under `src-tauri/src/abilities/provenance/`:

- `mod.rs` - public exports and feature-gated schema helpers.
- `envelope.rs` - `Provenance`, `ComposedProvenance`, `InputsSnapshot`, `ProvenanceWarning`, `AbilityOutput<T>` if W3-A has not already supplied it.
- `builder.rs` - `ProvenanceBuilder`, `ProvenanceError`, field coverage walk, budget/truncation logic.
- `trust.rs` - `TrustAssessment`, `EffectiveTrust`, `TrustContribution`, trust merge.
- `source.rs` - `SourceAttribution`, `DataSource`, `SourceIdentifier`, `ScoringClass`, source indexes.
- `field.rs` - `FieldAttribution`, `FieldPath`, `SourceRef`, `DerivationKind`, `Confidence`, `SanitizedExplanation`.
- `subject.rs` - `SubjectAttribution`, `SubjectRef`, subject-fit enums, competing-subject shape.

Keep `src-tauri/src/intelligence/provider.rs` and `src-tauri/src/services/context.rs` read-only. Module exposure (`src-tauri/src/abilities/mod.rs` and `src-tauri/src/lib.rs`) is a W3-A coordination point because the abilities root does not exist today.

Define `Provenance` per ADR-0105 with serde derives, `provenance_schema_version: u32` defaulting to `1`, identity fields, `produced_at` from `ctx.services.clock.now()`, `InputsSnapshot`, `actor`, `mode`, computed `TrustAssessment`, direct `sources: Vec<SourceAttribution>`, optional prompt fingerprint, `children: Vec<ComposedProvenance>`, `field_attributions: BTreeMap<FieldPath, FieldAttribution>`, top-level `subject: SubjectAttribution`, and warnings. `ComposedProvenance { composition_id: CompositionId, provenance: Box<Provenance> }` preserves the JSON `children[]` contract while making references stable. `SourceRef::Child` uses `composition_id`, never runtime vector position.

Define `SourceAttribution` with ADR-0105 fields including `source_asof: Option<DateTime<Utc>>` (`.docs/decisions/0105-provenance-as-first-class-output.md:165-173`). `DataSource`, `GleanDownstream`, `SourceIdentifier`, and `ScoringClass` follow ADR-0107 (`.docs/decisions/0107-source-taxonomy-alignment.md:27-60`, `:75-131`), including additive `LegacyUnattributed` for DOS-299 backfill only (`:223-241`). W3-B carries the field and warning variants; W3-G owns semantic timestamp parsing, backfill, and coverage enforcement.

Define `FieldAttribution` with `subject: SubjectAttribution`, `derivation: DerivationKind`, source refs, confidence, and optional `SanitizedExplanation`. `ProvenanceBuilder::finalize<T: Serialize>` serializes the domain output to `serde_json::Value`, expands expected JSON-pointer leaf paths, applies any subtree helpers to concrete leaves, and rejects missing attribution before returning `AbilityOutput<T>`. Direct projections use `pass_through()` to auto-fill field attributions. LLM synthesis requires source refs. Declared confidence requires a sanitized explanation.

Builder lifecycle:

1. `new(ctx, descriptor)` captures ability identity, actor, mode, clock time, and empty direct sources/children/field map.
2. `source(...)` adds a direct source, derives `scoring_class` from `DataSource`, records `synthesis_marker`, and returns a stable `SourceIndex`.
3. `compose(composition_id, child)` validates the id against the declared `composes` metadata and stores the child without flattening its sources.
4. `attribute(...)`, `pass_through(...)`, and `subject(...)` populate field and subject attribution.
5. `finalize(data)` validates coverage, computes trust, applies size policy, and wraps the result in `AbilityOutput<T>`.

End-state alignment: this gives W3-C/DOS-7 a canonical provenance JSON payload, W3-F/DOS-296 an additive thread-id slot, W3-G/DOS-299 a `source_asof` carrier, W4-A/DOS-5 deterministic trust inputs, and W5 pilots a single "About this" substrate. It forecloses author-set trust, positional child references, and un-attributed claim-bearing fields.

## 3. Key decisions

`TrustAssessment` shape is exactly `effective: EffectiveTrust`, `contributions: Vec<TrustContribution>`, and `contains_stored_synthesis: bool` per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:106-118`). The builder computes it; authors never set it. `EffectiveTrust` is only `Trusted | Untrusted`.

Trust contribution sources include `DirectSource { source_index }`, `ComposedChild { composition_id }`, `StoredSynthesisField { entity_id, field }`, `FeedbackEvent { feedback_id }`, and `SubjectFitGate { subject }`. The last two let DOS-294/DOS-5 explain feedback and subject-fit effects without adding a separate top-level event log in this issue.

Read + structured trusted direct sources is `Trusted`; any prompt fingerprint, stored synthesis marker, unbounded external free text, or untrusted child makes the effective trust `Untrusted`; Maintenance/Publish merge the weakest child/direct contribution bottom-up. This follows ADR-0105's rules (`.docs/decisions/0105-provenance-as-first-class-output.md:149-158`) and ADR-0102's prompt-injection boundary.

`DerivationKind` variants ship as `Direct`, `Composed { composition_id: CompositionId }`, `Computed { algorithm: &'static str }`, `LLMSynthesis`, and `Constant`. This is ADR-0105's set (`.docs/decisions/0105-provenance-as-first-class-output.md:213-219`) with the child reference corrected to stable `composition_id` per the ticket. `SourceRef` mirrors that correction: `Source { source_index } | Child { composition_id, field_path }`.

`SubjectAttribution` ships as a typed primitive on both the envelope and every field attribution. Shape: `subject: SubjectRef`, `binding: SubjectBindingKind`, `supporting_source_refs: Vec<SourceRef>`, `competing_subjects: Vec<CompetingSubject>`, and `fit: SubjectFitAssessment { status, confidence, method }`. `SubjectBindingKind` covers direct/input-bound, inherited, inferred, source-matched, and user-confirmed. `SubjectFitStatus` is `Confident | Ambiguous | Blocked`; finalize rejects `Ambiguous` and `Blocked` for claim-bearing output. This implements the SubjectAttribution amendment and closes cross-entity bleed before render.

Finalize failure conditions are closed: missing top-level subject; missing field attribution on any serialized output leaf; non-constant field with no source or child ref; `LLMSynthesis` with empty refs; invalid source index; invalid `composition_id`; declared confidence without sanitized explanation; field subject not coherent with envelope subject unless explicitly `Multi`; manual trust override attempts; serialized provenance still >1MB after deepest-first child elision.

Warnings are explicit and serializable: existing ADR-0105 variants plus `SourceTimestampUnknown`, `SourceTimestampImplausible`, `SubjectFitQualified`, and `DepthElided`. Soft size over 100KB emits a warning; >1MB collapses deepest composition subtrees and emits `ProvenanceWarning::DepthElided` per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:331-333`).

JSON Schema uses existing optional `schemars` (`src-tauri/Cargo.toml:81-90`). Provenance types derive `Serialize`, `Deserialize`, `PartialEq`, and `cfg_attr(feature = "mcp", derive(JsonSchema))`; the schema test runs with `--features mcp` and proves the generated schema can be embedded in MCP tool descriptions.

Forward compatibility: do not use `#[serde(deny_unknown_fields)]`. New additive fields such as DOS-296 `thread_ids` use `#[serde(default)]` and keep `provenance_schema_version = 1`, matching ADR-0124 (`.docs/decisions/0124-longitudinal-topic-threading.md:37-49`).

Composition merge order for W4-A is deterministic: compute child provenance first, sort/validate children by declared `composition_id` order from W3-A registry metadata, keep `sources[]` direct-only, keep transitive sources under `children[]`, then build root trust by appending own direct contributions followed by child effective summaries in declared order. W4-A can read a sparse tree and reproduce weakest-trust propagation without flattening sources.

## 4. Security

The new attack surface is provenance becoming trusted explanation UI and trust input. The builder fails closed on missing or incoherent subject attribution so sourced content cannot appear on the wrong account merely because it has good source attribution. `SanitizedExplanation` is constructor-gated; no raw prompt text, source snippets, secrets, customer names, access tokens, or Glean opaque payloads go into explanations or error messages. `SourceIdentifier::OpaqueGleanSource` carries opaque refs only, matching ADR-0107 (`.docs/decisions/0107-source-taxonomy-alignment.md:95-109`).

Trust is data trust, not authorization. ADR-0102 still requires confirmation, policy, or safe schema range before untrusted Transform output authorizes mutations (`.docs/decisions/0102-abilities-as-runtime-contract.md:313-323`). The provenance builder only computes the marker consumed by registry/services. Manual construction with mismatched trust is rejected by avoiding public constructors for computed fields and by finalize recomputation.

Validation errors must be structural and non-content-bearing: `MissingFieldAttribution { field_path }` is acceptable; `Missing attribution for "Acme renewal risk..."` is not. Schema generation must not expose ability names an actor cannot enumerate; W4-C bridge work still owns actor-filtered MCP discovery.

## 5. Performance

Finalize is O(serialized output leaves + direct sources + children). The heaviest normal path is `serde_json::to_value` plus JSON-pointer flattening; expected provenance remains ADR-0105's ~500B-50KB range (`.docs/decisions/0105-provenance-as-first-class-output.md:327-333`). Composition does not flatten transitive sources, so deep trees avoid quadratic source copying. Size checks serialize provenance once near finalize; soft >100KB warns, hard >1MB performs deepest-first child elision before returning.

No DB queries, locks, network, provider calls, or cache writes are introduced in W3-B. Subject-fit checks operate on already-declared refs. JSON Schema generation is test/build-time with the `mcp` feature, not on hot ability invocation paths.

Budget tests should cover a realistic wide output and a synthetic deep A->B->C->... chain. The synthetic case verifies deepest-first elision preserves root direct sources, root field attribution, aggregate trust, and a count of skipped levels.

## 6. Coding standards

W3-B adds substrate types and pure builder logic only. It performs no service mutation and therefore honors services-only mutation boundaries. Ability code using the builder must use `ctx.services.clock.now()` for `produced_at`; no `Utc::now()` or `thread_rng()` is introduced in `src-tauri/src/abilities/`, matching `ServiceContext` guidance (`src-tauri/src/services/context.rs:64-69`, `:109-121`).

Intelligence Loop check: no new schema write path, signal type, health-scoring rule, briefing surface, or feedback UI is introduced. Provenance is the substrate that later claims/feedback/trust work consumes. Fixtures must use synthetic IDs and generic entity names only. Clippy budget is zero new warnings; schema tests must run under the existing `mcp` feature rather than adding a new dependency.

Public constructors should encode invariants: `Confidence::declared` requires a `SanitizedExplanation`, `Confidence::implicit` is only used by direct projection helpers, and `SourceAttribution::new` derives scoring class instead of accepting it from author code. Any test-only bypasses live under `#[cfg(test)]`.

## 7. Integration with parallel wave-mates

W3-A/DOS-210 owns the abilities module root, registry metadata, category validation, and declared `composes` list. W3-B consumes that metadata shape for `CompositionId` validation but does not implement the macro or registry. If W3-B lands before W3-A, the PR stacks on W3-A's module-root branch; otherwise W3-B only adds `pub mod provenance;` to the W3-A-created abilities root as an agreed integration line.

W3-C/DOS-7 stores provenance JSON on claims and consumes `SubjectRef`, `source_asof`, and warnings; its current plan already expects W3-B to feed those fields (`.docs/plans/wave-W3/DOS-7-plan.md:87`). W3-C owns the `intelligence_claims` table and write path; W3-B only supplies serializable payloads.

W3-E/DOS-294 owns `FeedbackAction` and `claim_feedback`; W3-B can represent feedback as trust contributions by ID, but does not create feedback rows. W3-F/DOS-296 adds `thread_ids: Vec<ThreadId>` with `#[serde(default)]` and no schema-version bump; W3-B must not add `deny_unknown_fields` anywhere that would reject that additive field. W3-G/DOS-299 owns `source_asof` population helpers and semantic bounds; W3-B defines the field and warning carrier. W4-A/DOS-5 consumes the deterministic trust tree; W3-B does not implement numeric trust scoring.

## 8. Failure modes + rollback

Missing attribution, invalid subject fit, invalid composition refs, or bad declared confidence are hard finalize errors and return no `AbilityOutput<T>`. Unknown source timestamps are not hard errors; they produce `SourceTimestampUnknown` warnings per ADR-0105 (`.docs/decisions/0105-provenance-as-first-class-output.md:401-420`). Implausible timestamps are carried as warnings for DOS-299 freshness handling; W3-B does not quarantine or migrate data.

Oversized provenance degrades by eliding deepest children; if still over 1MB, finalize fails so a raw oversize tree never leaves the ability. If schema generation fails under `--features mcp`, the implementation fails CI because MCP tool descriptions would be incomplete.

Rollback is mechanical because W3-B has no migration and no runtime write path: revert the provenance module and its tests, then W3-A/W5 callers stop compiling until they drop imports. The W1-B universal write fence is honored by construction: this module does not write DB rows, files, signals, or external APIs. If a later ability writes after using provenance, that write remains behind ServiceContext/W1-B gates.

## 9. Test evidence to be produced

Unit tests in `src-tauri/src/abilities/provenance/`: `provenance_finalize_rejects_missing_field_attribution`, `provenance_finalize_rejects_ambiguous_subject_fit`, `provenance_direct_projection_autopopulates_field_attribution`, `provenance_llm_synthesis_requires_source_refs`, `provenance_declared_confidence_requires_explanation`, `provenance_read_structured_sources_trusted`, `provenance_transform_with_prompt_fingerprint_untrusted`, `provenance_publish_maintenance_inherit_weakest_child`, `provenance_contains_stored_synthesis_from_source_marker`, `composition_a_b_c_preserves_child_grandchild_tree`, `composition_refs_use_composition_id_not_position`, `source_asof_roundtrip_preserves_known_unknown_warning`, `json_roundtrip_preserves_equality`, `schemars_schema_for_provenance_is_valid_mcp_shape`, `serde_accepts_future_thread_ids_without_schema_bump`, and `provenance_size_budget_warns_then_depth_elides`.

Gate artifact for this PR: `cargo test provenance`, `cargo test --features mcp provenance`, and `cargo clippy --all-targets --features mcp -- -D warnings` from `src-tauri/`, plus the wave gate's broader W3 Suite S/P/E evidence. Suite S contribution: subject-fit reject tests for cross-entity bleed. Suite P contribution: budget/truncation test with deterministic size assertions. Suite E contribution: A->B->C composition + JSON roundtrip + schema generation fixture.

## 10. Open questions

1. ADR-0105's checked-in file does not contain the concrete `SubjectAttribution` Rust shape even though the 2026-04-24 split notes and Linear comment make it required. Architect-reviewer should confirm the field names above before coding.

2. The Linear feedback addendum asks provenance to answer which user feedback events changed trust/freshness/subject binding. This plan represents those through `TrustContribution` / subject-fit contributions by feedback ID while W3-E owns `claim_feedback` rows. Confirm no top-level `feedback_events[]` field is required in provenance schema v1.

3. Module-root ownership: W3-B's owned write set is `src-tauri/src/abilities/provenance/`, but Rust compilation needs an abilities module root. Confirm W3-A creates and owns the root before W3-B implementation, or approve a one-line shared module export.
