# ADR-0107: Source Taxonomy Alignment

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0098](0098-data-governance-source-aware-lifecycle.md), [ADR-0100](0100-glean-first-intelligence-architecture.md)  
**Supersedes:** The narrow `DataSource` enums implicit in [ADR-0098](0098-data-governance-source-aware-lifecycle.md) and [ADR-0100](0100-glean-first-intelligence-architecture.md); this ADR defines the authoritative taxonomy.  
**Required by:** [ADR-0105](0105-provenance-as-first-class-output.md) §4, [ADR-0108](0108-provenance-rendering-and-privacy.md)

## Context

Three ADRs touch source taxonomy:

- [ADR-0098](0098-data-governance-source-aware-lifecycle.md) defines a `data_source` tag and purge-on-revocation semantics. Its implicit enum is `user, google, glean, clay, ai, co_attendance`.
- [ADR-0100](0100-glean-first-intelligence-architecture.md) introduces `local_enrichment` and describes Glean as a multi-source feeder from Salesforce, Zendesk, Gong, Slack, P2, and org directories.
- [ADR-0105](0105-provenance-as-first-class-output.md) §4 references `DataSource` in provenance attribution, including Salesforce and others not in ADR-0098's enum.

The field naming is also inconsistent: [ADR-0080](0080-signal-intelligence-architecture.md) uses `source`; [ADR-0098](0098-data-governance-source-aware-lifecycle.md) and [ADR-0105](0105-provenance-as-first-class-output.md) use `data_source`. Lifecycle behavior per source (purge vs. flag-for-re-enrichment) is also not consistent: [ADR-0098](0098-data-governance-source-aware-lifecycle.md) says AI-generated intelligence is flagged for re-enrichment rather than purged, but downstream consumers (provenance, maintenance audit) had been assumed to purge.

This ADR defines the authoritative `DataSource` enum, the `SourceIdentifier` extension pattern, the canonical field name, and the per-source lifecycle rules that every provenance-consuming surface honors.

## Decision

### 1. Authoritative `DataSource` Enum

```rust
pub enum DataSource {
    // Direct first-party sources
    User,                    // User-typed, settings, user-written context
    Google,                  // Google Workspace (Gmail, Calendar, Drive)
    
    // Glean as aggregator — individual downstream sources accessed via Glean
    Glean { downstream: GleanDownstream },
    
    // Other direct integrations
    Clay,
    Ai,                      // Local LLM synthesis (not via Glean)
    CoAttendance,            // Co-attendance inference from calendars
    LocalEnrichment,         // Deterministic computation on local data
    
    // Catch-all for future integrations
    Other(SourceName),       // Stable registered name; ADR amendment required to add
}

pub enum GleanDownstream {
    Salesforce,
    Zendesk,
    Gong,
    Slack,
    P2,
    Wordpress,
    OrgDirectory,
    Documents,               // Generic Glean document results
    Unknown,                 // Glean-reported but downstream not disclosed
}
```

**Glean is modeled as an aggregator, not as a single source.** When an ability attributes `DataSource::Glean { downstream: Salesforce }`, it captures both the retrieval path (Glean) and the originating source (Salesforce). This preserves ADR-0100's Glean-first posture without erasing downstream lineage needed for trust and lifecycle decisions.

**Adding a new source requires an ADR amendment.** The `Other(SourceName)` variant exists for forward compatibility but is not the authoring path. New integrations add explicit enum variants via an ADR, keeping the taxonomy discoverable and typed.

### 2. Canonical Field Naming: `data_source`

Every place that stores or references a source of truth uses the field name `data_source`. This supersedes [ADR-0080](0080-signal-intelligence-architecture.md)'s `source` field on `SignalEvent`.

**Migration.** The `source` field on `SignalEvent` is renamed to `data_source` via a schema migration. Existing rows are re-typed in place (the values already map one-to-one with the new enum). The migration is additive-then-breaking: Phase 1 adds `data_source` and backfills from `source`; Phase 2 drops `source`. Both phases land in v1.4.0.

**Why unify.** Two names for the same concept create drift. Every query, filter, index, and type consumer must know which field to use. One name, one enum, everywhere.

### 3. `SourceIdentifier` Extensibility

`SourceIdentifier` from [ADR-0105](0105-provenance-as-first-class-output.md) §4 gains variants to match the expanded `DataSource` enum:

```rust
pub enum SourceIdentifier {
    // First-party
    Signal { signal_id: SignalId },
    Entity { entity_id: EntityId, field: Option<String> },
    EmailThread { thread_id: ThreadId, message_id: Option<MessageId> },
    Meeting { meeting_id: MeetingId },
    Document { document_id: DocumentId, chunk_id: Option<ChunkId> },
    UserEntry { entry_id: ContextEntryId },
    
    // Glean aggregator
    GleanAssessment {
        assessment_id: AssessmentId,
        dimension: Option<String>,
        cited_sources: Vec<GleanCitedSource>,
    },
    
    // Direct provider invocations
    ProviderCompletion { completion_id: String, provider: ProviderKind },
    
    // Glean-cited sources DailyOS cannot locally see
    OpaqueGleanSource {
        downstream: GleanDownstream,
        opaque_ref: String,             // Glean-internal reference
        cited_as_of: DateTime<Utc>,
    },
}

pub struct GleanCitedSource {
    pub downstream: GleanDownstream,
    pub citation: String,               // Glean-provided citation text or URL
    pub confidence: Option<f32>,
}
```

**Glean-sourced opaque data attribution.** When Glean returns an assessment derived from content DailyOS has no local visibility into (e.g., a Zendesk ticket the user's Glean account can read but DailyOS cannot), the ability attributes `DataSource::Glean { downstream: Zendesk }` with `SourceIdentifier::OpaqueGleanSource { downstream: Zendesk, opaque_ref, ... }`. Lifecycle of opaque sources is delegated to the user's Glean policy — DailyOS cannot purge data it does not hold, but it can mask or invalidate derived artifacts when the Glean connection is revoked.

### 4. Scoring vs. Context vs. Reference Classification

Every `DataSource` variant has a scoring class that determines how it contributes to numeric scoring:

| DataSource | ScoringClass | Reasoning |
|------------|--------------|-----------|
| `User` | Scoring | User-authored data is authoritative |
| `Google` (Calendar, Gmail metadata) | Scoring | Structured first-party |
| `Glean { downstream: Salesforce }` | Scoring | CRM is the system of record |
| `Glean { downstream: Zendesk }` | Scoring | Support volume is scoring-relevant |
| `Glean { downstream: Gong }` | Scoring | Call data is scoring-relevant |
| `Glean { downstream: Slack }` | Context | Per [ADR-0100](0100-glean-first-intelligence-architecture.md) — context only |
| `Glean { downstream: P2 }` | Context | Per [ADR-0100](0100-glean-first-intelligence-architecture.md) — context only |
| `Glean { downstream: OrgDirectory }` | Scoring | Org structure is factual |
| `Glean { downstream: Documents }` | Reference | Documents cite, don't score directly |
| `Clay` | Scoring | Enrichment data |
| `Ai` | Reference | LLM synthesis is reference material, not direct scoring |
| `CoAttendance` | Scoring | Derived but algorithmic |
| `LocalEnrichment` | Scoring | Deterministic computation |

The `scoring_class` field on `SourceAttribution` ([ADR-0105](0105-provenance-as-first-class-output.md) §4) is derived from this table, not author-set. Consumers that aggregate sources for scoring filter by class to avoid conflating Slack chatter with Salesforce ARR.

### 5. Per-Source Lifecycle Behavior

[ADR-0098](0098-data-governance-source-aware-lifecycle.md)'s lifecycle rules are specified per source class:

```rust
pub enum LifecycleBehavior {
    /// Destructive purge on source revocation — remove the source rows entirely.
    Purge,
    
    /// Mark the data as derived-from-revoked-source; surface as masked, do not delete.
    Mask,
    
    /// AI/LLM-synthesized content: flag for re-enrichment on next sync, retain until replaced.
    /// Preserves continuity when a transient connection drop is not a true revocation.
    FlagForReEnrichment,
    
    /// User-authored; revocation is deletion of the user entry itself, handled by user.
    UserControlled,
}
```

**Default lifecycle by source:**

| DataSource | LifecycleBehavior |
|------------|-------------------|
| `User` | UserControlled |
| `Google` | Purge (on OAuth revocation) |
| `Glean { downstream }` | Mask (DailyOS cannot purge what it did not store; masks downstream-derived state) |
| `Clay` | Purge |
| `Ai` | FlagForReEnrichment (per ADR-0098) |
| `CoAttendance` | Purge |
| `LocalEnrichment` | Purge |

**Resolution of the ADR-0098/ADR-0105 conflict.** ADR-0098 is right: AI-synthesized output is flagged, not purged, because the underlying source revocation removes the input data but the synthesis itself may remain useful as an annotation awaiting re-generation. [ADR-0105](0105-provenance-as-first-class-output.md) §8 is amended: "source revocation triggers a masking pass" applies only to sources whose `LifecycleBehavior` is `Mask`; for `Purge`, the records are deleted; for `FlagForReEnrichment`, the records remain with a flag. [ADR-0108](0108-provenance-rendering-and-privacy.md) specifies the masking and flag-rendering shapes surfaces see.

### 6. Synthesis Markers on Stored AI Output

Every stored field that is itself LLM-synthesized carries a `SynthesisMarker` (per [ADR-0105](0105-provenance-as-first-class-output.md) §4) so the trust model's `contains_stored_synthesis` check works. This ADR specifies the marker storage:

- Every entity table with fields that may carry LLM output (e.g., `entity_assessment.summary`, `meeting_prep.brief`) has a sibling column or JSON field `synthesized_fields` listing which field names are synthesis outputs and by which ability.
- Migration-time backfill: existing stored AI output is tagged retroactively based on the ability that produced it (best-effort; some historical data may not be attributable and is marked `synthesized_fields: ["*unknown*"]` to err on the side of `Untrusted`).
- Reading a synthesized field populates `SourceAttribution.synthesis_marker` so the trust builder sees it.

Per-ability migration plan is in [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md).

## Consequences

### Positive

1. **One authoritative `DataSource` enum.** No drift between ADRs; every consumer uses the same values.
2. **Glean as aggregator preserved.** Downstream sources remain attributable; lineage not erased.
3. **Opaque Glean data handled.** `OpaqueGleanSource` allows attribution to data DailyOS cannot locally see.
4. **Scoring class eliminates source conflation.** Slack chatter and Salesforce ARR are distinguishable at attribution time.
5. **Lifecycle behavior per class.** ADR-0098's intent is now explicit: AI is flagged, Google is purged, Glean is masked.
6. **Field naming unified.** `data_source` everywhere; `source` migration scheduled in v1.4.0.
7. **Stored AI output is identifiable.** Synthesis markers close the trust-laundering hole from [ADR-0105](0105-provenance-as-first-class-output.md) §3.

### Negative

1. **Expanded enum is more surface area.** Every consumer that switches on `DataSource` must handle more variants.
2. **Schema migration for `source` → `data_source`.** Touches every signal-reading query.
3. **Synthesis marker backfill imperfect.** Historical data may be tagged conservatively as `*unknown*`, slightly inflating `Untrusted` trust assessments until data churns.
4. **`Other(SourceName)` escape hatch.** Tempting shortcut to skip ADR amendments for new integrations.

### Risks

1. **Glean downstream misattribution.** An ability attributes content as `Glean { downstream: Salesforce }` when it was actually from Zendesk. Mitigation: Glean API provides downstream metadata; abilities use it directly, not inferred.
2. **Opaque source purge expectations.** A user expects "revoke Glean" to purge all Glean-derived data; DailyOS can only mask because it cannot reach into Glean's stores. Mitigation: Settings UX explains masking behavior; Glean-side revocation is the user's responsibility via Glean's own controls.
3. **Scoring class drift.** A future integration's scoring class is debated. Mitigation: default new integrations to `Reference` (most conservative) and require ADR amendment to promote to `Scoring` or `Context`.
4. **Lifecycle classification disputes.** A reviewer disagrees on `Purge` vs. `Mask` for a new source. Mitigation: the default is `Mask` (preserves data; conservative for users); promoting to `Purge` requires justification.

## References

- [ADR-0098: Data Governance — Source-Aware Lifecycle](0098-data-governance-source-aware-lifecycle.md) — This ADR extends with formal enum, per-source lifecycle behavior, and resolves ADR-0098/ADR-0105 conflict on AI output (flagged, not purged).
- [ADR-0100: Glean-First Intelligence Architecture](0100-glean-first-intelligence-architecture.md) — This ADR extends with `GleanDownstream` enum preserving downstream attribution; honors context-only rule for Slack/P2.
- [ADR-0080: Signal Intelligence Architecture](0080-signal-intelligence-architecture.md) — `source` field renamed to `data_source` via schema migration.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §4 `SourceAttribution` uses this ADR's `DataSource` and `SourceIdentifier`.
- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Renders masking behavior and flag-for-re-enrichment status.
- [ADR-0112: Migration Strategy — Parallel Run and Cutover](0112-migration-strategy-parallel-run-and-cutover.md) — Specifies the `source` → `data_source` migration and the synthesis-marker backfill.

---

## Amendment — 2026-04-24 PM — `LegacyUnattributed` `DataSource` variant

### Why

Codex round 2 finding 13 surfaced that the v1.4.0 spine plan (DOS-299) needed to preserve item-level confidence semantics for legacy items without `ItemSource` (the current `HasSource::effective_confidence` defaults missing source to `0.5` per `intelligence/io.rs:1185-1189`). My earlier writeup proposed registering this in the `CLAIM_TYPE_REGISTRY` (per ADR-0125). That was wrong — it's a `DataSource` taxonomy concern, not a claim type. ADR-0107 owns `DataSource`; this amendment adds the variant.

### What

Add to the `DataSource` enum:

```rust
pub enum DataSource {
    // ...existing variants (User, Google, Glean { downstream }, Clay, Ai, CoAttendance, LocalEnrichment, Other(SourceName))...
    /// Backfill-only marker for claims migrated from legacy items that pre-date `ItemSource`
    /// attribution. The underlying source is unknown beyond "DailyOS produced this before
    /// source taxonomy existed." Trust treatment matches `HasSource::effective_confidence`
    /// default (0.5) so backfill preserves existing scoring behavior exactly.
    LegacyUnattributed,
}
```

### Properties

- **`ScoringClass`:** `Reference` (most conservative — these claims cannot drive scoring decisions on their own).
- **`LifecycleBehavior`:** `FlagForReEnrichment` (the underlying data is gone; the claim survives until re-enrichment supersedes it with a typed source).
- **Default `confidence`:** `0.5` (matches the legacy `effective_confidence` default).
- **Backfill-only:** new code MUST NOT write `LegacyUnattributed` claims. Production migration is the only legitimate writer. CI lint enforces no `DataSource::LegacyUnattributed` outside `services/migrations/` paths.

### Lifecycle

- v1.4.0 spine: backfill writes `LegacyUnattributed` claims for legacy items without `ItemSource`. Trust band parity preserved per DOS-299 acceptance criterion.
- v1.4.1+: enrichment paths supersede `LegacyUnattributed` claims with typed-source claims as they re-enrich entities. Number of `LegacyUnattributed` claims decreases monotonically.
- v1.5.x: when `LegacyUnattributed` claim count drops below 5% of total active claims, schedule retirement migration (drop the variant, force any remaining to be re-enriched or flagged).

### Consumer issues affected

- DOS-299 — backfill writes `LegacyUnattributed` for legacy items; preserves trust band parity.
- DOS-300 — `CLAIM_TYPE_REGISTRY` does NOT include `legacy_unattributed`; that was a category error in the earlier writeup.
- DOS-7 — backfill migration depends on this variant existing.
