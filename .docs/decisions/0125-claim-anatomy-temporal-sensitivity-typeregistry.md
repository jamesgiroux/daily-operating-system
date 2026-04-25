# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry

**Status:** Accepted (substrate primitives for v1.4.0 spine; downstream enforcement in v1.4.1+)
**Date:** 2026-04-24
**Target:** v1.4.0 (schema + enums + registry); v1.4.1 (DOS-10 freshness + DOS-214 render policy); v1.4.2/3 (per-surface enforcement)
**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0114](0114-scoring-unification.md)
**Pattern parallel:** [ADR-0115](0115-signal-granularity-audit.md) Signal Policy Registry

## Context

The 2026-04-24 Claim Anatomy Review (`.docs/plans/claim-anatomy-review-2026-04-24.md`) walked every dimension a claim should express in the v1.4.x substrate. Three dimensions were under-scrutinized at the substrate level and warrant a substrate primitive in v1.4.0 spine:

1. **Temporal scope** (Review §11) — distinguishes PointInTime / State / Trend claims; affects freshness, supersession, and contradiction semantics.
2. **Sensitivity** (Review §12) — claim-level sensitivity tier; structural safeguard against accidental surface leakage.
3. **Claim type taxonomy** (Review §15) — currently free-form strings; should be a closed-but-extensible registry following the ADR-0115 Signal Policy Registry pattern.

This ADR is the substrate allowance: schema fields + types + registry mechanism. Downstream enforcement (freshness scoring, render gates) is named per-dimension and lands in v1.4.1+.

## Decision

### 1. Temporal scope

```rust
pub enum TemporalScope {
    /// Claim is about an event that happened at a moment in time.
    /// Example: "Bob said X in 4/23 meeting." Doesn't go stale — the event occurred.
    /// Freshness: never decays.
    /// Supersession: cannot be superseded (different events are different facts).
    /// Contradiction: usually doesn't contradict another PointInTime; may contradict a State.
    PointInTime { occurred_at: DateTime<Utc> },

    /// Claim is about a persistent condition until contradicted.
    /// Example: "Bob is the champion." Has a half-life.
    /// Freshness: decays per DOS-10 table.
    /// Supersession: a newer State at same field_path supersedes.
    /// Contradiction: State + State with different values is a contradiction.
    State,

    /// Claim is about evolution over time.
    /// Example: "Engagement has been declining over the last 30 days."
    /// Freshness: based on recency of latest underlying data point.
    /// Supersession: recomputed; new trend on fresher data supersedes.
    /// Contradiction: another Trend with opposite direction at same field_path.
    Trend { window_start: DateTime<Utc>, window_end: DateTime<Utc> },
}
```

Schema allowance:

```sql
ALTER TABLE intelligence_claims ADD COLUMN temporal_scope TEXT NOT NULL DEFAULT 'state';
```

v1.4.0 spine ships the column + enum + serde impls. Default `State` for behavioral parity with current implicit assumption. v1.4.1 DOS-10 freshness factor consults `temporal_scope` to apply the right decay rule.

### 2. Sensitivity

```rust
pub enum ClaimSensitivity {
    /// Fine on customer-facing surfaces.
    /// Example: public press release content; product feature mentions.
    Public,

    /// Fine on user's own surfaces, briefings, MCP. Never customer-facing or shared/published.
    /// Example: CRM notes, internal Slack discussions.
    /// **Default for claims sourced from internal systems.**
    Internal,

    /// Fine on the user's own surfaces only. Not on MCP, not on Publish, not on briefings shared with others.
    /// Example: personal notes about a stakeholder's challenges.
    Confidential,

    /// Only the user themselves; not even trusted human analysts in multi-user (future).
    /// Example: legal-confidential, HR-sensitive content.
    UserOnly,
}
```

Schema allowance:

```sql
ALTER TABLE intelligence_claims ADD COLUMN sensitivity TEXT NOT NULL DEFAULT 'internal';
```

Default `Internal` is the conservative choice — most claims sourced from internal systems should default Internal. `Public` requires explicit author choice or source-class inheritance. `Confidential` / `UserOnly` require explicit author choice.

v1.4.0 spine ships the column + enum. Render-time gates land per-surface in v1.4.1+ (see §4).

### 3. Claim type registry

Today `claim_type` is a free-form string. Every Transform ability defines its own implicitly. Dedup (per claim_type), freshness decay (per claim_type per DOS-10), commit policy gates, rendering policy — all key on this string. Drift is invisible.

This ADR introduces `ClaimTypeRegistry` — a compile-time exhaustive const slice mapping every legal `claim_type` to its metadata:

```rust
pub struct ClaimTypeMetadata {
    /// Canonical string written into the `intelligence_claims.claim_type` column.
    pub name: &'static str,

    /// Default temporal scope for this claim type. Authors may override per-claim
    /// when the specific assertion warrants different semantics.
    pub default_temporal_scope: TemporalScope,

    /// Default sensitivity. Same author-override semantics.
    pub default_sensitivity: ClaimSensitivity,

    /// Freshness decay class — referenced by DOS-10's per-source / per-type half-life table.
    pub freshness_decay_class: FreshnessDecayClass,

    /// Commit policy class per ADR-0113 §3 — most types are Standard;
    /// specific types may require gated commit (e.g. `tombstone` is always immediate).
    pub commit_policy_class: CommitPolicyClass,

    /// Which actor classes may write this claim type.
    pub allowed_actor_classes: &'static [ClaimActorClass],

    /// Which `SubjectRef` variants this claim type can attach to.
    /// Example: `stakeholder_role` claims attach to `Person` only;
    /// `account_health_band` attaches to `Account` only.
    pub canonical_subject_types: &'static [SubjectType],
}

pub const CLAIM_TYPE_REGISTRY: &[ClaimTypeMetadata] = &[
    ClaimTypeMetadata {
        name: "stakeholder_role",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::CrmField,
        commit_policy_class: CommitPolicyClass::Standard,
        allowed_actor_classes: &[ClaimActorClass::User, ClaimActorClass::Human, ClaimActorClass::Agent, ClaimActorClass::External],
        canonical_subject_types: &[SubjectType::Person],
    },
    ClaimTypeMetadata {
        name: "renewal_date",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::SalesforceFieldUpdate,
        commit_policy_class: CommitPolicyClass::Standard,
        allowed_actor_classes: &[ClaimActorClass::User, ClaimActorClass::Agent, ClaimActorClass::External],
        canonical_subject_types: &[SubjectType::Account],
    },
    // ... initial set covering DOS-218 (get_entity_context) + DOS-219 (prepare_meeting) outputs ~10-15 entries
];
```

Enforcement (compile-time):

- A claim row written with a `claim_type` not in the registry **fails CI lint**.
- New claim types require an ADR amendment (or registry-extension PR with documented rationale linking back to this ADR).
- The pattern parallels [ADR-0115](0115-signal-granularity-audit.md) Signal Policy Registry — same exhaustiveness check shape.

v1.4.0 spine ships the registry mechanism + initial set covering the two pilot abilities (DOS-218 + DOS-219). New claim types are added incrementally as new abilities ship in v1.4.1+.

### 4. Per-dimension downstream landing

| Dimension | v1.4.0 spine | v1.4.1 | v1.4.2 | v1.4.3 |
|---|---|---|---|---|
| Temporal scope | column + enum + serde | DOS-10 freshness consults it; supersession semantics | — | — |
| Sensitivity | column + enum + serde | DOS-214 render layer enforces | entity surfaces enforce ceiling | briefing surfaces enforce ceiling |
| Claim type registry | mechanism + initial set + CI lint | extensions as new abilities ship | extensions | extensions |

## Non-goals for spine

- DOS-10 freshness formula consultation of `temporal_scope` (v1.4.1).
- Per-surface sensitivity ceilings (v1.4.1+).
- Render-time enforcement (v1.4.1+).
- Counter-claim denormalized boolean (v1.4.1; deferred per anatomy review §13).
- Causal lineage between claims (v1.5.x; deferred per anatomy review §14).
- Locale on FieldAttribution (v1.4.1; deferred per anatomy review §17).
- Decision relevance / actionability tier (v1.5.x; deferred per anatomy review §18).
- Reversibility (lives at action / publish level; declared out of scope per anatomy review §16).

## Consequences

### Positive

- Substrate has explicit fields for the three dimensions; downstream enforcement is additive and non-breaking.
- **Temporal scope** makes "this event happened" structurally distinct from "this state holds" — eliminates the wrong-decay-on-PointInTime bug before it ships.
- **Sensitivity** makes surface-leak prevention substrate-level instead of surface-policy convention. Surfaces that respect the sensitivity ceiling cannot leak Internal content into Public surfaces by construction.
- **Claim type registry** forces deliberation on new claim types and prevents silent taxonomy drift. Adding a new claim type is now an explicit, reviewed event — not a string typed into a Transform ability.

### Negative / risks

- Three new schema fields. Acceptable; all default to safe values (`State`, `Internal`).
- Registry maintenance burden — every new claim type requires a registration PR. Mitigated by mirroring ADR-0115 Signal Policy Registry pattern (proven on signals; same author cadence).
- Authors may forget to set non-default `temporal_scope` and `sensitivity`. Mitigated by `ClaimTypeMetadata.default_*` values being looked up at write time when claim row leaves either field as default — the metadata supplies the right value for the type.

### Neutral

- v1.4.0 spine behavior is unchanged: defaults preserve current implicit assumptions until v1.4.1+ enforcement lands.
- The registry pattern matches an existing precedent (ADR-0115); no new architectural concept.

## References

- [ADR-0113: Human and Agent Analysis as First-Class Claim Sources](0113-human-and-agent-analysis-as-first-class-claim-sources.md) — claim row schema; this ADR adds three fields.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `Provenance.warnings` interacts with sensitivity at render-time masking.
- [ADR-0114: Scoring Unification](0114-scoring-unification.md) — `freshness_weight` factor will consult `temporal_scope` in v1.4.1.
- [ADR-0115: Signal Granularity, Policy Registry, and Durable Invalidation](0115-signal-granularity-audit.md) — pattern parallel for the claim type registry's compile-time exhaustiveness.
- [ADR-0124: Longitudinal Topic Threading](0124-longitudinal-topic-threading.md) — same substrate-allowance pattern.
- `.docs/plans/claim-anatomy-review-2026-04-24.md` — sourcing review (§11, §12, §15 for the three dimensions covered here).
