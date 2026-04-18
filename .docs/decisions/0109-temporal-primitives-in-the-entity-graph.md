# ADR-0109: Temporal Primitives in the Entity Graph

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0082](0082-entity-generic-prep-pipeline.md), [ADR-0097](0097-account-health-scoring-architecture.md), [ADR-0088](0088-people-relationship-network-intelligence.md)  
**Consumed by:** [ADR-0102](0102-abilities-as-runtime-contract.md) (Read abilities), [ADR-0105](0105-provenance-as-first-class-output.md) (provenance references trajectory snapshots)

## Context

DailyOS's differentiation is depth before every interaction. Depth requires reasoning about **change over time**, not just current state. Today, features that need trajectory information — a contact's role progressing from Director to VP, an account's health dropping 15% over 8 weeks, engagement on a given account tapering off in the last month — compute it on demand by scanning signals or reconstructing state from history. This is expensive, inconsistent across features, and produces subtly different answers depending on which feature's computation you look at.

A few specific consequences of this gap:

- `prepare_meeting` wants to show "what changed since last meeting" but reconstructs per-invocation from signal scans.
- `detect_risk_shift` recomputes trajectory each time it runs instead of reading a maintained summary.
- The weekly narrative lacks a consistent "what changed this week" view because each feature computes change differently.
- `get_entity_context` returns current state only; callers that need trajectory compute it themselves.

Making temporal primitives first-class — stored, maintained, indexed — lets every ability that cares about change consume a canonical view, and makes depth compound: one new signal source deepens every trajectory primitive it touches, and every ability that reads those primitives gets deeper automatically.

## Decision

Introduce a small, stable set of temporal primitives maintained alongside the entity graph. Abilities consume them through `get_entity_context` and related Read abilities; maintenance abilities populate and refresh them.

### 1. Core Temporal Primitive Types

```rust
pub enum TrajectoryKind {
    HealthCurve,              // Time series of health score dimensions per entity (ADR-0097)
    EngagementCurve,           // Time series of meeting + email cadence per entity
    RoleProgression,           // Ordered history of roles per person (current + past)
    RiskTrajectory,            // Time series of risk indicators + severity
    RelationshipStrength,      // Per-pair-of-people strength over time (ADR-0088)
    Custom(&'static str),      // Registered extensions require ADR amendment
}

pub struct TrajectorySnapshot<T> {
    pub kind: TrajectoryKind,
    pub entity_id: EntityId,              // Or (EntityId, EntityId) for relationship-pair trajectories
    pub series: Vec<DataPoint<T>>,        // Ordered by at, newest-first
    pub computed_at: DateTime<Utc>,       // When this snapshot was computed
    pub confidence: f32,                   // How much signal density supports the trajectory
}

pub struct DataPoint<T> {
    pub at: DateTime<Utc>,                // When this data point applies
    pub value: T,
    pub source_refs: Vec<SourceRef>,      // Points into provenance sources per ADR-0105
}
```

Each `TrajectoryKind` has a specific `T`:

- `HealthCurve` → `T = HealthScoreSnapshot` (six dimensions per [ADR-0097](0097-account-health-scoring-architecture.md), point-in-time)
- `EngagementCurve` → `T = EngagementWindow` (meetings_count, emails_count, bidirectional_ratio over a week)
- `RoleProgression` → `T = RoleEntry` (title, org, seniority level)
- `RiskTrajectory` → `T = RiskIndicator` (kind, severity, trigger description)
- `RelationshipStrength` → `T = RelationshipStrength` (interaction_count, sentiment_trend, mutual_response_rate)

### 2. Storage

Each primitive has a dedicated table:

```
entity_health_curve        (entity_id, at, dimension, score, source_refs_json)
entity_engagement_curve    (entity_id, week_start, meetings_count, emails_count, bidirectional_ratio, source_refs_json)
person_role_progression    (entity_id, started_at, ended_at, title, org, seniority, source_refs_json)
entity_risk_trajectory     (entity_id, at, kind, severity, trigger, source_refs_json)
relationship_strength      (user_entity_id, counterparty_id, window_start, strength_score, components_json)
```

Indexed on `(entity_id, at DESC)` for the primary query pattern "give me the last N data points for entity X." Storage overhead is bounded — typical trajectories carry 52 weekly points per entity per year, pruned to 2-year retention by default.

### 3. Computation

Trajectory primitives are computed by dedicated maintenance abilities:

- `refresh_health_curve(entity_id)` — runs after any health-score-affecting signal for the entity; appends a point if score changed materially, updates the latest point otherwise.
- `refresh_engagement_curve(entity_id)` — runs nightly per entity; aggregates the past week's meeting and email activity.
- `detect_role_change(person_id)` — runs when a person's title changes per any source; appends a `RoleEntry` with `ended_at` of the previous entry set.
- `recompute_risk_trajectory(entity_id)` — runs after any risk-indicator signal.
- `refresh_relationship_strength(pair)` — runs weekly.

These are Maintenance abilities subject to [ADR-0103](0103-maintenance-ability-safety-constraints.md): they have budgets, idempotency tests, dry-run support, and audit records.

### 4. Consumption by Read Abilities

`get_entity_context` returns an optional `trajectory: TrajectoryBundle` field:

```rust
pub struct TrajectoryBundle {
    pub health_curve: Option<TrajectorySnapshot<HealthScoreSnapshot>>,
    pub engagement_curve: Option<TrajectorySnapshot<EngagementWindow>>,
    pub role_progression: Option<TrajectorySnapshot<RoleEntry>>,         // For People entities
    pub risk_trajectory: Option<TrajectorySnapshot<RiskIndicator>>,
    pub relationship_strength: Vec<TrajectorySnapshot<RelationshipStrength>>, // Per counterparty
}
```

The `depth` input parameter on `get_entity_context` controls which trajectories are hydrated: `Shallow` returns `None`; `Standard` returns most recent snapshot per kind; `Deep` returns up to 52 weeks of detail.

Transform abilities that want trajectory use it via `get_entity_context`:

```rust
let ctx_bundle = ctx.invoke_typed(get_entity_context, GetEntityContextInput {
    entity_id: account_id,
    depth: ContextDepth::Deep,
}).await?;

if let Some(curve) = ctx_bundle.data.trajectory.health_curve {
    // Compare current score to 8 weeks ago
    let baseline = curve.series.iter().find(|p| p.at < eight_weeks_ago);
    // ...
}
```

### 5. Provenance Integration

Each `DataPoint<T>` carries `source_refs` pointing to the signals and entities that contributed to it. When an ability composes a trajectory and uses it in synthesis, its provenance includes the trajectory's source references transitively via `SourceRef::Child` per [ADR-0105](0105-provenance-as-first-class-output.md) §5.

### 6. Invalidation and Recompute

Trajectory tables are derived from signals and entities. When a source is revoked per [ADR-0107](0107-source-taxonomy-alignment.md), the trajectory data points derived from that source are invalidated: the row is marked with `source_invalidated_at` and `get_entity_context` filters it out. A subsequent refresh maintenance run recomputes the trajectory with remaining sources.

### 7. Prioritization for v1.4.0

Not all primitives land simultaneously. Phase order:

**Phase 1 (foundational, v1.4.0):**
- `EngagementCurve` — highest value, simplest computation
- `RoleProgression` — discrete events, easy to populate

**Phase 2 (v1.4.0):**
- `HealthCurve` — depends on [ADR-0097](0097-account-health-scoring-architecture.md) being accepted
- `RiskTrajectory` — depends on risk-indicator signal types stabilizing

**Phase 3 (post-v1.4.0):**
- `RelationshipStrength` — requires pair-level modeling from [ADR-0088](0088-people-relationship-network-intelligence.md)
- `Custom(...)` extensions

## Consequences

### Positive

1. **Depth compounds across abilities.** One new signal source integrated into engagement-curve maintenance deepens every ability that reads engagement via `get_entity_context`.
2. **Consistent "what changed" reasoning.** Every ability that needs trajectory reads from the same maintained tables, not ad-hoc per-feature computation.
3. **Explicit temporal intelligence.** The product's "continuity over time" promise has a concrete substrate, not just verbal claims.
4. **Trajectory is cheap to read.** Indexed tables; no per-invocation recomputation.
5. **Source-attributable.** Each data point carries source refs, integrating cleanly with provenance.
6. **Invalidation honors lifecycle.** Source revocation cleanly invalidates derived trajectory points.

### Negative

1. **New storage surface.** Five new tables, each indexed by entity and time.
2. **Maintenance cost.** Refresh abilities run regularly; computation of engagement curves for many entities adds latency to maintenance windows.
3. **Historical backfill.** Initial population requires scanning existing signal history; expensive for long-lived accounts.

### Risks

1. **Sparse trajectory data.** Entities with few signals produce unreliable trajectory curves. Mitigation: `confidence` field surfaces the sparsity; consumers can filter low-confidence curves from synthesis.
2. **Refresh cascade.** Signal emissions trigger refreshes for many entities simultaneously. Mitigation: refreshes are debounced and batched nightly per entity; per-signal refresh only for high-urgency signals.
3. **Schema evolution of `HealthScoreSnapshot`.** [ADR-0097](0097-account-health-scoring-architecture.md)'s dimensions may change. Mitigation: stored snapshots retain the dimensions as of their computation time; readers handle evolution via ADR-0097's own schema versioning.
4. **Phase 3 `Custom` extensions fragment the taxonomy.** Mitigation: `Custom(...)` requires ADR amendment per §1; no casual additions.

## References

- [ADR-0082: Entity-Generic Prep Pipeline](0082-entity-generic-prep-pipeline.md) — Entity abstraction on which trajectories hang.
- [ADR-0097: Account Health Scoring Architecture](0097-account-health-scoring-architecture.md) — `HealthScoreSnapshot` dimensions.
- [ADR-0088: People Relationship Network Intelligence](0088-people-relationship-network-intelligence.md) — `RelationshipStrength` primitive.
- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — `get_entity_context` consumes trajectory bundle.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `DataPoint.source_refs` integrate with provenance.
- [ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md) — Source revocation invalidates trajectory data points.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — Refresh abilities operate under maintenance safety.
