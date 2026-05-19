# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry

**Status:** Accepted (substrate primitives for v1.4.0 spine; downstream enforcement in v1.4.1+)
**Date:** 2026-04-24
**Amended:** 2026-05-19 — added §5 `RecommendationClaim` variant for v1.4.6 Salience & Recommendations. Registers `ClaimType::Recommendation` in `CLAIM_TYPE_REGISTRY` (canonical persisted name `"recommendation"`) with `State` temporal scope, `Internal` sensitivity, `Medium` freshness decay, `Replace` commit policy, agent-only authorship, attaches to `Account` / `Project` / `Person` subjects. Salience factors, user-feedback state (`Accepted` / `Dismissed` / `NotUseful` / `TooNoisy` / `Converted`), conversion state, and supersession + dismissal lifecycle are declared at the substrate level; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`. Per the v1.4.6 W0 hardening cycle in [`.docs/plans/v1.4.6-waves.md`](../plans/v1.4.6-waves.md).
**Target:** v1.4.0 (schema + enums + registry); v1.4.1 (DOS-10 freshness + DOS-214 render policy); v1.4.2/3 (per-surface enforcement); v1.4.6 (Recommendation variant)
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

| Dimension | v1.4.0 spine | v1.4.1 | v1.4.2 | v1.4.3 | v1.4.6 |
|---|---|---|---|---|---|
| Temporal scope | column + enum + serde | DOS-10 freshness consults it; supersession semantics | — | — | `Recommendation` uses `State` |
| Sensitivity | column + enum + serde | DOS-214 render layer enforces | entity surfaces enforce ceiling | briefing surfaces enforce ceiling | `Recommendation` defaults `Internal` |
| Claim type registry | mechanism + initial set + CI lint | extensions as new abilities ship | extensions | extensions | `Recommendation` variant added (W0) |

### 5. RecommendationClaim variant (v1.4.6 amendment)

**Decision (2026-05-19, W0 of v1.4.6 Salience & Recommendations).** Recommendations are first-class claims, not a parallel substrate. They register in `CLAIM_TYPE_REGISTRY` and ride the same provenance, trust, supersession, and sensitivity primitives every other claim type already inherits from this ADR. The W0 amendment fixes the substrate-level shape; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`.

**Why a claim-type, not a separate table.** A recommendation says *"this action is recommended for this subject as of this moment, with this evidence and this expected impact"* — that is a claim. Trust band, freshness decay, contradiction handling, and the existing claim lifecycle (assert → corroborate → contradict → supersede → retract) already apply. Making it a claim means every existing intelligence-loop surface (entity context, briefing prep, surface render) consumes recommendations through the same render path as every other claim, with the same provenance receipt and the same trust signaling. A parallel `recommendation_claims` table would have re-invented all of that.

**Registry entry.** `ClaimType::Recommendation` is added with canonical persisted name `"recommendation"`. Metadata:

| Field | Value | Rationale |
|---|---|---|
| `default_temporal_scope` | `State` | A recommendation persists as the *current* recommendation for its subject until accepted, dismissed, or superseded. Not a `PointInTime` event; not a `Trend`. |
| `default_sensitivity` | `Internal` | Recommendations are derived intelligence. They reference upstream evidence whose own sensitivity may be higher; render-time masking still consults the upstream evidence's sensitivity (per §2). |
| `freshness_decay_class` | `Medium` | Weeks-scale half-life. The recommendation's evidence ages out on its own freshness clock; the recommendation itself decays alongside. W1-B salience re-scoring rebuilds candidacy continuously on signal arrival, so the freshness class governs how long a *quiet* recommendation remains valid before W4-B deviation detection re-evaluates. |
| `commit_policy_class` | `Replace` | A newer same-subject recommendation supersedes the older one. Recommendations are uniquely identified by `(subject, recommended_action_kind)`; corroboration semantics (`Reinforce`) don't apply because two distinct agent runs producing the same recommendation should *replace* (newer evidence wins), not stack confidence. Contradiction (`Fork`) is handled at the salience layer (W1-B): mutually-contradictory recommendations on the same subject lower each other's salience score and surface as a triage decision, not as parallel claim rows. |
| `allowed_actor_classes` | `[Agent]` | Recommendations are agent-generated. User feedback (Accepted / Dismissed / NotUseful / TooNoisy / Converted) flows through `services::claims::record_claim_feedback` and emits `ClaimFeedbackRecorded` signals — it does *not* write a separate user-authored Recommendation claim. |
| `canonical_subject_types` | `[Account, Project, Person]` | Recommendations attach to entities. Meeting-scoped recommendations attach to the meeting's containing entity (per the v1.4.6 W3 cross-surface render contract); they don't get their own meeting subject. Email subjects are excluded — the email is evidence, not the recommendation target. |

**Lifecycle additions beyond the base claim envelope.** Recommendations carry three substrate fields above what other claim types use, declared at the ADR level here and frozen as wire format in W1-A `contracts.rs`:

- **Salience factors.** Inspectable per-factor scoring (trust, novelty, freshness, urgency, similarity, fit). One row per factor at write time; the salience score is the deterministic combination, not an LLM judgment. ADR-0115 Signal Policy Registry exhaustiveness pattern applies: factor kinds are a closed enum. See [ADR-0114](0114-scoring-unification.md) for the underlying scoring composition.
- **User-feedback state.** Closed enum `{ Accepted | Dismissed | NotUseful | TooNoisy | Converted }`. Default `None` (no feedback yet). State transitions are append-only at the `claim_feedback` level (per existing v1.4.0 substrate); the denormalized current-state field on the recommendation claim row is rebuilt from the feedback log by the W4-A feedback loop on each `ClaimFeedbackRecorded` signal.
- **Conversion state.** Closed enum `{ NotConverted | InProgress | Converted | Abandoned }`. Default `NotConverted`. Updated when a downstream action (commitment, meeting outcome, account event) is causally linked back to the recommendation. v1.4.6 W4-A wires the link; v1.5.x causal-lineage substrate (deferred per §"Non-goals", anatomy review §14) makes the link first-class.

**Supersession + dismissal.** Standard claim lifecycle applies:
- *Supersession*: a newer recommendation on the same `(subject, recommended_action_kind)` writes a `supersedes` reference to the older claim's id and the older claim's `verification_state` transitions to `Superseded`.
- *Dismissal*: `record_claim_feedback` with action `Dismissed` flips the user-feedback state to `Dismissed` and emits `ClaimFeedbackRecorded`. The W2-A surfacing policy treats `Dismissed` as a hard suppression for the recommendation's lifetime (with a configurable cooldown); W4-A feedback weights downgrade the source factors that contributed to the dismissed recommendation, so future similar candidates score lower.
- *Retraction*: agents may retract their own recommendations (e.g. when upstream evidence is contradicted). Standard `ClaimRetracted` flow applies; W2-A removes from surface immediately on signal receipt.

**Surface render contract (v1.4.6 W3 consumers).** The `Recommendation` claim's render output goes through the same Shared Receipt DTO (`services::claim_receipt::contracts::ClaimReceipt`) every other claim uses. Per the v1.4.6 W0 close gate cross-version row, this ADR amendment is unblocked only after v1.4.4 W1-A ships the `ClaimReceipt` substrate.

**Out of scope at the ADR level.** Per `.docs/plans/v1.4.6-waves.md` line 213 owner table:
- `RecommendationClaim` DTO struct + serde encoding → W1-A (`services/recommendations/contracts.rs`)
- `recommendation_claims` table vs `intelligence_claims` extension → W1-A migration choice (slot range v260–v279)
- Salience scoring engine → W1-B (`services/recommendations/salience.rs`)
- Surfacing policy → W2-A (`services/recommendations/surfacing.rs`)
- Trigger policy → W2-B (`services/recommendations/triggers.rs`)
- Feedback wiring → W4-A (`services/recommendations/feedback.rs`)
- Cross-surface rendering → W3 (consumes `claim_receipt::contracts::ClaimReceipt`)

**ADR-0102 (abilities runtime) assessment.** v1.4.6's `recommend_for_entity` ability targets the existing `mcp_exposure: McpExposure::MetadataOnly` tri-state value (added in the 2026-05-10 amendment). No further ADR-0102 amendment needed for v1.4.6 W0. v1.4.7 may amend ADR-0102 to flip `mcp_exposure` to `Invocable` once the MCP `dailyos.read.portfolio_attention` consumer ships.

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
