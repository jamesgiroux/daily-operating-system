# ADR-0105: Provenance as First-Class Output

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md)  
**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
**Required by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 (audit), [ADR-0102](0102-abilities-as-runtime-contract.md) §10 (trust boundary)  
**Companions:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (prompt fingerprint details), [ADR-0107](0107-source-taxonomy-alignment.md) (`DataSource` taxonomy), [ADR-0108](0108-provenance-rendering-and-privacy.md) (surface rendering and safety)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §9 Rule 5 declares that every ability output carries provenance via `AbilityOutput<T>`. §10 declares Transform outputs are untrusted for mutation authorization. §11.3 declares composition produces nested provenance trees. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 requires maintenance audit records to carry provenance. All of these contracts depend on one shape: the `Provenance` envelope. ADR-0102 references it but does not specify it.

This ADR defines the envelope shape, trust classification, and composition semantics. It deliberately does not define: prompt fingerprinting (see [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), the authoritative `DataSource` taxonomy (see [ADR-0107](0107-source-taxonomy-alignment.md)), or per-surface rendering rules (see [ADR-0108](0108-provenance-rendering-and-privacy.md)). Splitting these concerns keeps each ADR implementable atomically and testable in isolation.

## Decision

Every ability output carries a `Provenance` envelope via the `AbilityOutput<T>` wrapper from [ADR-0102](0102-abilities-as-runtime-contract.md) §6. The envelope captures identity, temporal context, trust classification, source attribution, composition tree, and field-level attribution.

### 1. The Provenance Envelope

```rust
pub struct Provenance {
    // Envelope schema version — for forward-compatibility on this ADR's structure itself
    pub provenance_schema_version: u32,
    
    // Identity
    pub ability_name: &'static str,
    pub ability_version: AbilityVersion,
    pub ability_schema_version: SchemaVersion,   // The ability's I/O schema version, per ADR-0102 §8
    pub invocation_id: InvocationId,
    
    // Temporal context
    pub produced_at: DateTime<Utc>,              // From ctx.services.clock.now() per ADR-0104 §6
    pub inputs_snapshot: InputsSnapshot,          // See §2
    
    // Execution context
    pub actor: Actor,
    pub mode: ExecutionMode,
    
    // Trust classification
    pub trust: TrustAssessment,                   // See §3
    
    // Source attribution — directly cited sources (transitive sources live under children[])
    pub sources: Vec<SourceAttribution>,
    
    // Prompt fingerprint — populated when IntelligenceProvider was invoked. Shape per ADR-0106.
    pub prompt_fingerprint: Option<PromptFingerprint>,
    
    // Composition tree — child provenance from composed abilities
    pub children: Vec<Provenance>,
    
    // Field-level attribution (required for LLM-synthesized fields; see §5)
    pub field_attributions: BTreeMap<FieldPath, FieldAttribution>,
    
    // Diagnostics
    pub warnings: Vec<ProvenanceWarning>,
}

pub enum ProvenanceWarning {
    DepthElided { skipped_levels: u32 },
    SourceStale { source_index: usize, age: Duration },
    SourceUnresolvable { source_index: usize, reason: String },
    AttributionIncomplete { field: FieldPath },
    Masked { reason: MaskReason },
}
```

`provenance_schema_version` is the schema version of this ADR's envelope structure. A consumer seeing a higher version must parse forward-compatibly — unknown fields are ignored; known fields retain their meaning. The initial version is `1`. Any breaking change requires a new version and a migration path.

`ability_schema_version` is the ability's I/O schema version per [ADR-0102](0102-abilities-as-runtime-contract.md) §8 — a different concept, retained here for completeness so a consumer reading provenance knows exactly which ability contract produced the output.

### 2. Inputs Snapshot

Reproducibility requires a precise description of what the ability saw when it ran. The `InputsSnapshot` captures this beyond just signals:

```rust
pub struct InputsSnapshot {
    pub newest_signal_at: Option<DateTime<Utc>>,    // Max observed_at across signal inputs
    pub entity_watermarks: BTreeMap<EntityId, EntityWatermark>,  // Version + last_updated per entity read
    pub source_freshness: BTreeMap<SourceClass, DateTime<Utc>>,  // Per-source last-sync timestamps
    pub provider_config_hash: Hash,                  // Hash of provider settings at invocation time
    pub glean_connected: bool,                       // Per ADR-0100
}

pub struct EntityWatermark {
    pub entity_version: u64,
    pub last_updated: DateTime<Utc>,
}
```

This does not claim to enable exact content-level replay of deleted or mutated sources. It supports three legitimate use cases:

1. **Staleness detection.** A consumer renders "as of 14:23 UTC, freshest Glean sync 3 hours ago."
2. **Diff reasoning.** A second invocation with different watermarks shows what changed.
3. **Eval reproducibility under fixtures.** In `Evaluate` mode, the fixture's state is deterministic; the snapshot plus the fixture reconstructs the run. [ADR-0110](0110-evaluation-harness-for-abilities.md) (forthcoming — renumbered from 0107) specifies how.

**Reproducibility is a scoped claim, not an unqualified one.** Full content-level replay of non-fixture runs is not promised and is out of scope for this ADR.

### 3. Trust Assessment

Trust is a structured record, not a collapsed nested enum. It encodes the weakest trust encountered, the mixing history, and whether the output is safe to authorize mutation on without a trust signal per [ADR-0102](0102-abilities-as-runtime-contract.md) §10.

```rust
pub struct TrustAssessment {
    /// The effective trust of this output for mutation-authorization purposes.
    pub effective: EffectiveTrust,
    
    /// Contributions to the trust computation. Lists every ancestor whose
    /// output mixed into this one, annotated with why its trust class applies.
    pub contributions: Vec<TrustContribution>,
    
    /// True if any ancestor (Read or otherwise) returned content that was
    /// originally LLM-synthesized and stored, even if this ability's direct
    /// category is Read. Closes the "stored AI output laundering" hole.
    pub contains_stored_synthesis: bool,
}

pub enum EffectiveTrust {
    /// Safe to use for mutation authorization without a trust signal.
    Trusted,
    
    /// Must not be used for mutation authorization without confirmation,
    /// policy, or schema-bounded value per ADR-0102 §10.
    Untrusted,
}

pub struct TrustContribution {
    pub source: TrustContributionSource,
    pub reason: TrustReason,
}

pub enum TrustContributionSource {
    DirectSource { index: usize },         // Points into Provenance.sources
    ComposedChild { index: usize },        // Points into Provenance.children
    StoredSynthesisField { entity_id: EntityId, field: String },
}

pub enum TrustReason {
    DirectlyFromStructuredSource,          // User-typed, Salesforce field, calendar event attribute
    DirectlyFromLLMSynthesis,              // This ability invoked IntelligenceProvider
    ComposedUntrustedChild,                // Child ability returned Untrusted
    StoredSynthesis,                       // Read ability exposed a stored LLM-synthesized value
    UnboundedFreeText,                     // External text that was not schema-bounded
}
```

**Rules for computing `effective`:**

1. If `prompt_fingerprint.is_some()`, this ability invoked the LLM → `Untrusted`.
2. If any `contributions[i]` has `reason == ComposedUntrustedChild` or `DirectlyFromLLMSynthesis` or `StoredSynthesis` or `UnboundedFreeText`, the output is `Untrusted`.
3. If every contribution is `DirectlyFromStructuredSource`, the output is `Trusted`.
4. `contains_stored_synthesis` is true when any source is a stored field known to have been LLM-synthesized (per [ADR-0107](0107-source-taxonomy-alignment.md), stored fields carry a `synthesized_by` marker so this can be detected at read time).

**This closes the laundering hole.** A Read ability that returns stored AI output (e.g., a cached `entity_assessment.summary` from Glean) is `Untrusted` because the stored field itself carries a synthesis marker. The `contains_stored_synthesis` flag and `TrustReason::StoredSynthesis` contribution make the reasoning explicit.

**`effective` is computed by the provenance builder, not the author.** The author cannot set it; the registry rejects manually-constructed `Provenance` with mismatched `effective` and `contributions`.

### 4. Source Attribution

Source attribution references the authoritative `DataSource` taxonomy in [ADR-0107](0107-source-taxonomy-alignment.md). This ADR specifies the shape; that ADR specifies the enum values and lifecycle behavior.

```rust
pub struct SourceAttribution {
    pub data_source: DataSource,                 // Defined by ADR-0107
    pub identifiers: Vec<SourceIdentifier>,      // Defined by ADR-0107 (extensible enum)
    pub observed_at: DateTime<Utc>,              // When DailyOS last ingested this source
    pub source_asof: Option<DateTime<Utc>>,      // When the source itself was authored (if known)
    pub evidence_weight: f32,                    // How strongly this source contributed, [0.0, 1.0]
    pub scoring_class: ScoringClass,             // See below
    pub synthesis_marker: Option<SynthesisMarker>, // Set if this source is itself stored LLM synthesis
}

pub enum ScoringClass {
    /// Source contributes to numeric scoring (health, risk, etc.). Example: Salesforce field.
    Scoring,
    
    /// Source is context-only, never feeds numeric scoring. Per ADR-0100, Slack and P2 are Context.
    Context,
    
    /// Source is reference material for LLM synthesis, not direct attribution.
    Reference,
}

pub struct SynthesisMarker {
    pub producer_ability: &'static str,
    pub producer_invocation_id: InvocationId,
    pub produced_at: DateTime<Utc>,
}
```

**`evidence_weight` is a contribution weight, not a reliability score.** It is author-supplied (by the ability's planner or synthesizer) and indicates how much this particular source mattered for this specific output. It is not a stored per-source reliability — those are defined by [ADR-0080](0080-signal-intelligence-architecture.md) and consumed separately. We deliberately do not duplicate ADR-0080's reliability into every provenance envelope.

**`scoring_class` is derived from the `DataSource` per [ADR-0107](0107-source-taxonomy-alignment.md).** It is informational, so consumers can filter context-only sources from scoring analysis.

**Glean-sourced attribution for invisible content.** When Glean returns an assessment computed from sources DailyOS cannot locally see (e.g., Zendesk tickets the user's Glean account can read but DailyOS cannot), the ability attributes `data_source: Glean` with a `SourceIdentifier::GleanAssessment { assessment_id, cited_sources: Vec<GleanCitedSource> }`. Lifecycle of these invisible sources is governed by the user's Glean policy, not DailyOS's local lifecycle. Details in [ADR-0107](0107-source-taxonomy-alignment.md).

### 5. Field-Level Attribution

Every field in the ability's output is attributable to one or more sources or children:

```rust
pub struct FieldPath(String);  // JSON-pointer-like, e.g. "/topics/0/title"

pub struct FieldAttribution {
    pub derivation: DerivationKind,
    pub source_refs: Vec<SourceRef>,
    pub confidence: Confidence,
    pub explanation: Option<SanitizedExplanation>,  // Sanitization rules in ADR-0108
}

pub enum DerivationKind {
    Direct,                                        // Copied from a single source identifier
    Composed { child_idx: usize },                 // From a composed child's output
    Computed { algorithm: &'static str },          // Algorithmic from structured sources
    LLMSynthesis,                                  // Produced by the LLM given the listed sources
    Constant,                                      // Fixed value, not derived
}

pub struct Confidence {
    pub value: f32,                                // [0.0, 1.0]
    pub kind: ConfidenceKind,                      // How the value was obtained
}

pub enum ConfidenceKind {
    Declared,              // Author-set (must justify in explanation)
    ProviderReported,      // Returned by IntelligenceProvider
    ComposedMin,           // Min of composed children's confidences
    Computed,              // From algorithm
    Implicit,              // Direct copy is always 1.0 (trivially true)
}
```

**Rules:**

1. Every field in the ability's domain output MUST have an entry in `field_attributions`. No "optional for Read" escape hatch — Read abilities' fields are typically `Direct` with `ConfidenceKind::Implicit`, which is trivial to populate but must exist.
2. `LLMSynthesis` derivation requires non-empty `source_refs`. An LLM-synthesized field with zero sources is a registration error.
3. `Declared` confidence requires a non-empty `explanation` explaining the basis. The builder rejects `Declared` without explanation.
4. `Confidence.value == 1.0` is not a bypass for attribution. Direct copies are 1.0 and still have an attribution entry (it's trivial, but it exists). LLM synthesis with declared 1.0 requires an explanation.
5. `FieldPath` uses JSON-Pointer syntax with a stability caveat: array-index paths (`/topics/0`) are tolerated for render-time display but are not stable across re-runs. Stable identifiers (`/topics/[id=abc123]/title`) SHOULD be used when the output structure contains stable IDs.

### 6. Composition and Merge Semantics

When ability A invokes ability B, A's provenance includes B's as a nested `child`:

```
Provenance(A) {
  sources:  [... A's direct sources ...],
  children: [Provenance(B), Provenance(C), ...],
  field_attributions: {
    "/some_field": FieldAttribution { derivation: Composed { child_idx: 0 }, ... }
  }
}
```

**Merge rules:**

1. `sources[]` lists the ability's *own* direct sources; transitive sources live under `children[]`. The tree stays sparse.
2. `trust.effective` merges bottom-up per the rules in §3.
3. `prompt_fingerprint` does not merge — each ability has its own or none.
4. `inputs_snapshot` is the ability's own view at invocation start. Children capture their own snapshots; no aggregation.
5. Deduplication across `children[i].sources[]` is render-time, not construction-time.
6. Cycle guard: the registry rejects ability composition cycles at registration time per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.1, so nested `Provenance` trees are guaranteed acyclic.

**`SourceRef::Child(child_idx, field_path)` uses a stable `composition_id`.** To avoid the brittleness of positional `child_idx`, the `children` vector is indexed by a stable `CompositionId` per declared `composes` entry in the ability's metadata (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1). The `child_idx` is the index into the `composes` list, not the runtime invocation order. A `SourceRef::Child(child_idx, field_path)` with a `child_idx` outside the declared `composes` list is a registration error.

### 7. Provenance Builder Pattern

Contributors produce provenance via a builder that enforces the rules in §3 and §5:

```rust
pub async fn prepare_meeting(
    ctx: &AbilityContext<'_>,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    let mut prov = ProvenanceBuilder::new(ctx, "prepare_meeting");
    
    // Structured source (Trusted)
    let meeting = ctx.services.meetings.get(input.meeting_id).await?;
    let meeting_src = prov.source(
        DataSource::Google,
        SourceIdentifier::Meeting { meeting_id: meeting.id },
        ScoringClass::Scoring,
    );
    
    // Composed Read ability
    let account_ctx = ctx.invoke_typed(get_entity_context, GetEntityContextInput { ... }).await?;
    let account_child_idx = prov.compose(&account_ctx.provenance);
    
    // LLM synthesis — builder enforces that this triggers Untrusted trust
    let completion = ctx.intelligence.complete(prompt, ModelTier::Synthesis).await?;
    prov.prompt_fingerprint(completion.fingerprint);  // Shape per ADR-0106
    
    // Explicit attributions
    prov.attribute("/topics", DerivationKind::LLMSynthesis, &[
        SourceRef::Source(meeting_src),
        SourceRef::Child(account_child_idx, "/entity_state".into()),
    ], Confidence::provider_reported(completion.confidence));
    
    Ok(prov.finalize(MeetingBrief { topics: completion.topics, ... }))
}
```

The builder:

- Auto-populates `field_attributions` for direct projections (`pass_through` helper).
- Computes `trust.effective` from sources and composition.
- Rejects manual overrides to computed fields (`trust.effective`, `contains_stored_synthesis`).
- Wraps the domain output in `AbilityOutput<T>` with the built provenance.
- Fails at build time if any output field lacks attribution.

### 8. Storage Integration

Provenance persists anywhere the ability output is stored. Specifically:

1. **Maintenance audit table.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 `MaintenanceAuditRecord` adds a `provenance: Provenance` field. Source revocation triggers a masking pass: a `Provenance` whose sources reference a revoked `DataSource` is replaced with a `ProvenanceMasked` marker per [ADR-0108](0108-provenance-rendering-and-privacy.md). **Interaction with [ADR-0094](0094-audit-log-and-enterprise-observability.md):** ADR-0094's append-only JSONL security audit remains unchanged. The `maintenance_audit` SQLite table is a separate operational log for runtime diagnostics. Provenance content in the operational log is subject to masking; the JSONL security log is unchanged.

2. **Outbox table.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §2 outbox entries add a `provenance: Provenance` field. Outbox provenance captures the ability's full output-time provenance, including the invocation that generated the pending external mutation.

3. **Planned mutations.** [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §3 `PlannedMutation` adds a `provenance_ref: InvocationId` field pointing into the ability's provenance envelope. Plans do not carry a full duplicated provenance; they reference the envelope by invocation ID and field path, preventing storage duplication.

4. **Source revocation behavior.** Revoking a source invalidates derived artifacts per [ADR-0107](0107-source-taxonomy-alignment.md)'s lifecycle rules, which may be "purge" (destructive) or "flag for re-enrichment" (preservation) depending on the source class. ADR-0105 defines how masking presents to consumers; ADR-0107 defines when purge vs. flag applies.

**Storage format.** Provenance is stored as JSON in a single column per containing record. Indexes on `invocation_id`, `ability_name`, `produced_at`, and `data_source` (via a normalized xref table) support the queries [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 requires. A full JSON-column approach is feasible at the six-user scale; if scale changes, a normalized `provenance` / `provenance_sources` split can be introduced without changing the envelope contract.

### 9. Size Budget and Truncation

A typical Read ability produces ~500B–5KB of provenance. Transform abilities with field attributions can reach ~50KB. Deeply composed Maintenance invocations with >8 composition levels or >50 sources can exceed 500KB, which is excessive for per-invocation storage.

**Soft budget.** Abilities SHOULD keep provenance under 100KB. The registry warns at build time when provenance exceeds the soft budget.

**Hard truncation.** When a provenance envelope exceeds 1MB at finalize time, composition subtrees are collapsed from deepest first into a `ProvenanceWarning::DepthElided` summary carrying aggregate source counts and trust classification. Truncation is a finalize-time concern; the full tree never leaves the ability in its raw form above the hard limit.

## Consequences

### Positive

1. **Explainability is structural.** Every output answers "where did this come from?" without additional instrumentation.
2. **Trust boundary is enforceable and closes the laundering hole.** [ADR-0102](0102-abilities-as-runtime-contract.md) §10 rejects mutations authorized by `Untrusted` output at the registry layer, and `contains_stored_synthesis` catches the case where stored AI output is surfaced by a Read ability.
3. **Scoped reproducibility.** The `InputsSnapshot` supports staleness detection, diff reasoning, and fixture-mode replay — the three use cases that actually matter — without overpromising exact content-level replay of live runs.
4. **Composition is explicit.** Nested provenance makes composition visible in output, not hidden in call stacks, with a stable `composition_id` avoiding positional brittleness.
5. **Source revocation can cascade correctly.** Provenance references `DataSource` via [ADR-0107](0107-source-taxonomy-alignment.md), and lifecycle behavior (purge vs. flag) is governed by that ADR. Masking shape is governed by [ADR-0108](0108-provenance-rendering-and-privacy.md).
6. **Storage integration is explicit.** Maintenance audit, outbox, and plans each know exactly how they carry or reference provenance. Plans use references to avoid duplication.

### Negative

1. **Every ability pays an authoring cost.** Provenance building is part of the ability's job. Contributors must think about source attribution when writing synthesis.
2. **Provenance can be large.** Deep composition with full field attribution produces substantial JSON. Soft budget warns; hard truncation bounds worst-case.
3. **Serialization overhead on every Tauri/MCP response.** Acceptable at scale but measurable.
4. **Field-level attribution is tedious.** Builder auto-populates direct projections; synthesized fields require explicit attribution.

### Risks

1. **Attribution drift.** The mapping from output fields to sources drifts as the ability evolves. Mitigation: [ADR-0110](0110-evaluation-harness-for-abilities.md) (forthcoming — renumbered) cross-checks sample attributions against LLM self-evaluation during eval runs.
2. **Trust miscomputation.** The builder's trust computation misses a contribution. Mitigation: `contributions[]` is auditable — a failing case is a diagnosable bug with an explicit contribution list, not a silent `Untrusted`/`Trusted` flip.
3. **Storage volume growth.** Provenance retained indefinitely alongside outputs accumulates. Mitigation: same retention as output; no separate provenance retention policy.
4. **`composition_id` drift.** An ability's declared `composes` list changes between versions, invalidating stored `SourceRef::Child` references. Mitigation: registry's version-gating in [ADR-0102](0102-abilities-as-runtime-contract.md) §8 applies — major version bumps for breaking changes.
5. **Soft-budget over-emission.** Abilities routinely exceed 100KB and ignore the warning. Mitigation: CI fails builds where >10% of abilities exceed the soft budget.
6. **Forward-compatibility regressions.** A consumer written against `provenance_schema_version = 1` breaks when version `2` adds a required variant. Mitigation: breaking changes require a new `provenance_schema_version` and a deprecation window for consumers.

## References

- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Defines `AbilityOutput<T>` wrapper; §9 Rule 5 requires provenance; §10 consumes `TrustAssessment`; §11.3 requires composition merge.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — §8 maintenance audit records add a `provenance: Provenance` field per §8 of this ADR.
- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — `mode` and clock injection for `produced_at`; `PlannedMutation` adds a `provenance_ref` field.
- [ADR-0094: Audit Log and Enterprise Observability](0094-audit-log-and-enterprise-observability.md) — Coexistence: ADR-0094's append-only JSONL security audit is unchanged; this ADR's provenance lives in operational `maintenance_audit` SQLite table with masking.
- [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md) — Trust classification and source lifecycle background.
- **[ADR-0106: Prompt Fingerprinting and Provider Interface Extension](0106-prompt-fingerprinting-and-provider-interface.md)** — Specifies `PromptFingerprint` shape, canonicalization, and amendments to [ADR-0091](0091-intelligence-provider-abstraction.md).
- **[ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md)** — Authoritative `DataSource` enum, `SourceIdentifier` extensibility, and lifecycle behavior per source class.
- **[ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md)** — Per-surface rendering rules, actor-filtered display, `ProvenanceMasked` shape, sanitization of `explanation` text, and P2 publication safety.
- **ADR-0109 (forthcoming): Temporal Primitives in the Entity Graph** — Defines trajectory types referenced from provenance when an ability consumes trajectory inputs.
- **ADR-0110 (forthcoming): Evaluation Harness for Abilities** — Uses `prompt_fingerprint` and `field_attributions` for regression detection.
- **ADR-0111 (forthcoming): Surface-Independent Ability Invocation** — References the rendering rules in [ADR-0108](0108-provenance-rendering-and-privacy.md).
- **ADR-0112 (forthcoming): Migration Strategy — Parallel Run and Cutover** — Covers migration of existing stored AI content to carry `synthesis_marker` so the trust model's `contains_stored_synthesis` check works retroactively.
