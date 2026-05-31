---
ticket: DOS-316
title: "v1.4.6 W4-B — Deviation detection + EntityDeviation salience factor"
parent_plan: ../v1.4.6-waves.md
fork_sha: df7706a63647116ff355d162773e84c51723d877
target_branch: codex/v1.4.6-w4b-deviation-detection
status: L0 cycle 3 authoritative — un-reset (cycle 2 reset reversed per user direction 2026-05-28); salience integration restored as substrate evolution; awaiting cycle 4 review
authors: James Giroux, Claude
related:
  - ADR-0102 (Abilities as Runtime Contract)
  - ADR-0104 (Execution Mode and Mode-Aware Services — ServiceContext substrate)
  - ADR-0109 (Temporal Primitives in the Entity Graph — TrajectoryBundle / EngagementCurve substrate)
  - ADR-0114 (Scoring Unification — pure factor extractors, caller pre-computes inputs)
  - ADR-0115 (Signal Granularity Audit — Policy Registry + entity-granularity signals)
  - ADR-0123 (Typed Claim Feedback Semantics)
  - ADR-0125 (Claim Anatomy / Sensitivity / TypeRegistry — ClaimTypeMetadata shape)
  - ADR-0126 (Memory Substrate Invariants — invariants 1, 5, 6, 9, 10)
  - DOS-329 / DOS-330 (W1-A / W1-B — RecommendationClaim + 10-factor salience scoring engine)
  - DOS-332 (W4-A — feedback substrate; merged at PR #410, eb0fb8d3)
  - DOS-317 (W4-C — engagement telemetry; parallel sibling per ADR-0126 inv 9 separation)
  - DOS-338 (W5-A — eval harness; downstream consumer)
  - DOS-811 (follow-on: ADR-0125 registry expected_presence — entity-lifecycle modeling)
  - DOS-812 (follow-on: ADR-0126 inv 5 consolidation precedence check consumer)
  - DOS-814 (CANCELED 2026-05-28 — folded back into this ticket per user direction)
---

# v1.4.6 W4-B — Deviation detection + EntityDeviation salience factor (DOS-316)

## §0. Cycle 3 reframing (2026-05-28)

Per user direction 2026-05-28: **salience scoring is part of the abilities runtime substrate.** Its output (`salience_factors` table, surfacing_decisions, `score_salience()` ability) is read by any abilities-runtime consumer — briefings, account-detail enrichment, MCP tools, W3-A's `dailyos/suggested-next-steps` block, and W5-A eval. Adding a factor to salience scoring is substrate evolution; it improves what every consumer reads. It is NOT a "wire to UI" concern.

The cycle 2 scope reset (which stripped salience integration into DOS-814 as a "consumer follow-on") was a framing error. Cycle 3 un-resets: salience integration is restored to W4-B as routine substrate-evolution work. DOS-814 is canceled. The cycle 1 codex findings flagged the integration's real cost (11+ file touches across schema/mirrors/cache/policy_registry) — those are not scope creep, they are the substrate-evolution discipline. Cycle 3 enumerates all of them honestly.

Cycle history below §15 records cycles 0–2 for traceability; their text is superseded by this cycle 3 body.

## §1. Scope statement and UI Surface Deferral compliance

Per the **UI Surface Deferral Amendment (2026-05-27)** in `.docs/plans/v1.4.6-waves.md:13-31`: no Gutenberg `dailyos/whats-unusual` block, no Tauri React `<DeviationSection>` component, no daily-briefing UI rendering. The DOS-316 Linear ticket's frontend pieces are pre-amendment scope and out of W4-B.

W4-B **does ship**:

1. **Deviation detection substrate** — per-`(entity, claim_type, field_path)` rolling baselines + 4 detection rules (Stale, OutOfRange, CadenceBreak, UnexpectedPresence; Missing deferred to DOS-811 — entity-lifecycle modeling).
2. **`SalienceFactorKind::EntityDeviation` factor wired into `score_salience()`** — the substrate consumer benefit. Any abilities-runtime call site that invokes salience scoring gets deviation-aware ranking.
3. **Signal pre-declares** for `EntityBaselineRecomputed` + `EntityDeviationDetected` with explicit JSON `value`-field schema.
4. **`is_deviation_flagged()` predicate** — substrate-level read API. DOS-812 builds the consolidation-pass consumer for ADR-0126 inv 5 enforcement.
5. **CI invariant gate** — deviation baselines table never imported by `services::trust*` or `abilities-runtime::abilities::trust::**`.
6. **All routine substrate-evolution cost** for adding a new salience factor (DB CHECK constraint, runtime + TS mirror, golden test, cache invalidation, render mappings).

## §2. Substrate-consultation reuse audit (per waves.md:1016)

Side-by-side comparison of v1.4.1 temporal substrate against W4-B's deviation-factor inputs:

| Primitive | Location (verified file:line) | Coverage |
|-----------|------------------------------|----------|
| `entity_engagement_curve` table | `src-tauri/src/migrations/152_dos_215_temporal_entity_type_keys.sql:7-18` | Per-entity-per-week engagement counts (meetings/emails/ratio). Engagement axis, not per-claim-type. |
| `TrajectoryBundle` struct | `abilities-runtime/src/abilities/temporal/mod.rs:111-117` | Reads `engagement_curve` + `role_progression`. Read-only bundle. |
| `EngagementWindow` struct | `temporal/mod.rs:73-78` | Per-week value type with bounded ratio. |
| `TrajectoryReadHandle` trait | `temporal/mod.rs:194-202` | Async read; `DEEP_LIMIT_WEEKS = 52` cap. |
| `refresh_engagement_curve` ability | `temporal/mod.rs:234-253` | Maintenance ability; writes weekly rows. |

**Gap analysis.** `entity_engagement_curve` carries engagement counts per `(entity_type, entity_id, week_start)`. It does NOT carry per-`(entity, claim_type, field_path)` rolling statistics needed for the EntityDeviation factor. Different aggregation axis, different cardinality, different consumers. New table `recommendation_deviation_baselines` is non-overlapping; reuse is impossible.

W4-B does NOT introduce a new temporal abstraction — it consumes the existing `intelligence_claims` time-series (filtered by claim_type + field_path) and writes a rolling-stats derived-state cache. The temporal substrate covers entity-level cadence; the deviation substrate covers claim-level numeric/cadence/presence anomaly.

## §3. Files owned (exclusive)

Substrate work:

1. **`src-tauri/src/services/recommendations/deviation.rs`** — currently 7-line docblock (`src-tauri/src/services/recommendations/deviation.rs:1-7`); W4-B fills with full implementation per §4.
2. **`src-tauri/src/migrations/274_recommendation_deviation_baselines.sql`** (new) — `recommendation_deviation_baselines` table per §6.
3. **`src-tauri/src/migrations/275_salience_factors_entity_deviation.sql`** (new) — CHECK-constraint migration extending `salience_factors_weights.factor_kind` + `salience_factors.factor_kind` to include `entity_deviation` (codex cycle 1 F5).
4. **`src-tauri/src/migrations.rs`** — register v274 + v275 migrations.

Salience integration (the substrate evolution that delivers the benefit):

5. **`src-tauri/src/services/recommendations/contracts.rs`** — add `SalienceFactorKind::EntityDeviation` variant (extend enum at `contracts.rs:236-247`) + `FactorRationale::EntityDeviation { kind: DeviationRationale }` variant (extend enum at `contracts.rs:255-291`).
6. **`src-tauri/abilities-runtime/src/abilities/recommendations/contracts.rs`** — runtime mirror of `SalienceFactorKind` + `FactorRationale` at the existing mirror location (`abilities-runtime/src/abilities/recommendations/contracts.rs:254` per codex F6).
7. **`src-tauri/src/services/recommendations/salience.rs`** — extend `DEFAULT_WEIGHTS` const with `EntityDeviation` at 1/11 share (rebalance per §7); extend `extract_factors()` (`salience.rs:437`) to call `score_entity_deviation()`; bump `SCORE_SALIENCE_SCHEMA_VERSION` (`salience.rs:24`) from current value to next (`v1 → v2`) so stored-salience rows force recompute (codex F7).
8. **`src-tauri/src/services/recommendations/render.rs`** — extend `factor_band()` (`render.rs:642`), `factor_label()` (`render.rs:743`), `salience_factor_kind_storage()` (`render.rs:813`) with the new variant.
9. **`src-tauri/src/services/context.rs`** — extend mapping at `context.rs:546` (per codex F6 enumeration).
10. **`src/services/recommendations/contracts.ts`** — TS mirror at `contracts.ts:139` (`SalienceFactorKind` union) + `FactorRationale` discriminated union extension.
11. **`src/services/recommendations/__tests__/contracts.golden.test.ts`** — update golden test at `:271` (currently asserts 10 factor kinds; W4-B asserts 11).

Signal infrastructure:

12. **`src-tauri/src/signals/policy_registry.rs`** — 5-site touch (codex cycle 3 W4-B B3):
    - Add `EntityBaselineRecomputed { entity_type, entity_id }` and `EntityDeviationDetected { entity_type, entity_id }` variants to `SignalType` enum (line 6+).
    - Add `from_name` arms (line ~145).
    - Add `canonical_name` arms (line ~274).
    - Append both names to `known_signal_type_names()` slice (line ~426).
    - Add `policy_for` arms (line ~671).

Tests + CI gate:

13. **`src-tauri/tests/recommendations_w4b_deviation.rs`** (new) — integration tests per §11.
14. **`src-tauri/tests/recommendations_w4b_invariants.rs`** (new) — CI invariant gate (deviation baselines not imported by trust). Lock-the-enumeration discipline per `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`.

**Don't touch.** Trust substrate (CI invariant test enforces). W4-A feedback.rs (merged W4-A). W4-C engagement.rs. W2-A surfacing.rs write paths (deviation feeds salience scoring; surfacing decisions are downstream).

## §4. Public API — `services/recommendations/deviation.rs`

All functions sync, follow ADR-0114 pure-factor extractor pattern. `TrajectoryBundle` (async to fetch) is pre-fetched at the salience pipeline call site, not inside the extractor.

```rust
/// Score the EntityDeviation salience factor for a single claim.
/// Sync. Called by extract_factors during score_salience.
pub fn score_entity_deviation(
    conn: &rusqlite::Connection,
    claim: &ClaimRow,                           // ClaimRow per salience.rs:99-102, matching extract_factors pattern
    trajectory: Option<&TrajectoryBundle>,      // pre-fetched by caller; Optional because not all candidates need temporal context
    now: DateTime<Utc>,
) -> Result<DeviationScore, DeviationError>;

/// UPSERT a baseline row from the current claim time-series for (entity, claim_type, field_path).
/// Idempotent. Called by maintenance pass or signal-triggered recompute.
pub fn recompute_baseline_for_entity(
    conn: &rusqlite::Connection,
    entity_type: &str,
    entity_id: &str,
    claim_type: ClaimType,
    field_path: &str,                           // '' for claim types without per-field meaning
    window_weeks: u16,                          // capped at DEEP_LIMIT_WEEKS = 52
    computed_at: DateTime<Utc>,
) -> Result<BaselineRecord, DeviationError>;

/// Predicate: is this claim currently flagged as a deviation? (ADR-0126 inv 5 consumer hook.)
/// Reads latest EntityDeviationDetected signal for the claim's subject entity + claim_type + field_path.
/// DOS-812 builds the consolidation-pass consumer; no consumer in v1.4.6 yet.
pub fn is_deviation_flagged(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
    now: DateTime<Utc>,
) -> Result<bool, DeviationError>;

/// Purge baseline rows for an entity (for entity-deletion cascade).
/// Not part of normal operation; ships for ADR-0098 source-revocation cascade compatibility.
pub fn purge_baselines_for_entity(
    ctx: &ServiceContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<u32, DeviationError>;
```

Types:

- `DeviationScore` (new): `{ magnitude: f64, rationale: DeviationRationale }`.
- `DeviationRationale` (new): typed enum with one variant per rule (see §5).
- `BaselineRecord` (new): UPSERT row shape.
- `DeviationError` (new): `enum { Db(rusqlite::Error), MissingRegistry(ClaimType), TemporalRead(String), InvalidWindow }`.

## §5. Deviation rules (5 implemented, 1 deferred)

Each rule reads a registry-side configuration (DOS-811 ships `deviation_rules: &'static [DeviationRuleKind]` per claim type; W4-B ships rules without registry data so detection is dormant for all claim types until DOS-811 lands per-type rule populations).

| Rule | DeviationRationale variant | Input source | W4-B status |
|------|---------------------------|--------------|-------------|
| **Stale** | `Stale { source_asof_age_days: u32 }` | Claim `source_asof` vs `freshness_threshold_days_for(class)` const map (§7.1) | ✓ shipped |
| **OutOfRange** | `OutOfRange { observed: f64, expected_band: (f64, f64) }` | `ObjectValue::Literal { literal_kind, value }` parsed numeric (Number/Money/Percentage only) vs baseline `mean ± k·stddev` | ✓ shipped (StructuredClaim-only scope) |
| **CadenceBreak** | `CadenceBreak { expected_cadence_days: u32, observed_cadence_days: u32 }` | Baseline `expected_cadence_d` vs claim time-series gap | ✓ shipped |
| **UnexpectedPresence** | `UnexpectedPresence` | Claim type not registered in DOS-811 expected_presence map | ✓ shipped as no-op (always false until DOS-811) |
| **Missing** | `Missing` | DOS-811 expected_presence map says claim type expected but absent | ⚠ deferred to DOS-811 (entity-lifecycle modeling) |

**Numeric extraction contract** (codex cycle 3 W4-B B6 — content-predicate, not producer-predicate):

```rust
fn extract_numeric_value(claim: &ClaimRow) -> Option<f64> {
    // Reads StructuredClaim ObjectValue::Literal { literal_kind, value: String } per
    // abilities-runtime/src/structured_claim.rs:32-43.
    // Returns Some only for LiteralKind ∈ { Number, Money, Percentage }.
    // Returns None for: FreeText, Resolved, Text, Date, Enum literals, malformed parse.
    // Unit/currency normalization: assumed homogeneous within (entity, claim_type, field_path).
    // Cross-unit comparison NOT silently coerced; flagged at score time as ambiguous.
}
```

Test: seed claims with `LiteralKind::{Number, Money, Percentage, Text, Date, Enum}`; assert extraction succeeds on first three, None on others; OutOfRange rule does NOT fire on None.

## §6. `recommendation_deviation_baselines` table (v274)

```sql
-- src-tauri/src/migrations/274_recommendation_deviation_baselines.sql
CREATE TABLE IF NOT EXISTS recommendation_deviation_baselines (
    entity_type        TEXT NOT NULL,
    entity_id          TEXT NOT NULL,
    claim_type         TEXT NOT NULL,
    field_path         TEXT NOT NULL DEFAULT '',   -- '' for claim types without per-field meaning
    window_start       TEXT NOT NULL,              -- ISO 8601 week_start of the rolling window
    window_weeks       INTEGER NOT NULL,           -- window size; default 12
    sample_count       INTEGER NOT NULL,
    mean_value         REAL,                       -- nullable: populated only when claim values are numeric
    stddev_value       REAL,                       -- same
    expected_cadence_d INTEGER,                    -- copied from registry typical_cadence_days at compute time
    last_observed_at   TEXT,                       -- most recent claim of this type for this entity+field
    computed_at        TEXT NOT NULL,
    confidence         REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (entity_type, entity_id, claim_type, field_path)
);

CREATE INDEX IF NOT EXISTS idx_recommendation_deviation_baselines_computed_at
    ON recommendation_deviation_baselines (computed_at);
```

**Verified PK design** (codex cycle 3 W4-B B2): `intelligence_claims` carry `field_path` (per `src-tauri/src/services/account_fact_claims.rs:1180` — one claim type with many field-level meanings). PK includes `field_path` so two fields under the same claim type maintain separate baselines.

**Mutation pattern.** UPSERT-overwrite on each `recompute_baseline_for_entity` call. Derived-state cache; ADR-0126 inv 1 does NOT apply (`intelligence_claims` immutability is for claim assertions only). Test: idempotent recompute → single row, `computed_at` updated.

**Migration slot.** v274 verified next-free at `src-tauri/src/migrations.rs:1102` (v273 is `migrate_v273_recommendation_w2_shape_repair`; v274–v288 reserved for v1.4.6 per the v269-v288 amendment block at `.docs/plans/v1.4.6-waves.md:13`). W4-C takes v276 (its salience-factors-userfit extension migration); W4-B takes v274 + v275.

## §7. Salience integration (substrate evolution)

The substrate-evolution work that ships the deliverable benefit. Without this, the deviation baselines are unread by any caller of `score_salience()`.

### §7.1. `FreshnessDecayClass` → threshold-days const map (codex cycle 3 W4-B B1 — FIXED hallucination)

**Verified enum variants** at `src-tauri/abilities-runtime/src/abilities/claims.rs:65-76`:

```rust
pub enum FreshnessDecayClass {
    Static,        // Does not decay
    Slow,          // Months-scale half-life
    Medium,        // Weeks-scale half-life
    Fast,          // Days-scale half-life
    EventBound,    // Tied to source event or explicit expiry
}
```

W4-B's Stale rule needs a threshold-days mapping. Ship the const map in `deviation.rs`:

```rust
fn freshness_threshold_days_for(class: FreshnessDecayClass) -> Option<u32> {
    match class {
        FreshnessDecayClass::Static => None,         // never stale
        FreshnessDecayClass::Slow => Some(180),      // ~6 months
        FreshnessDecayClass::Medium => Some(45),     // ~6 weeks
        FreshnessDecayClass::Fast => Some(7),        // 1 week
        FreshnessDecayClass::EventBound => None,     // freshness tied to source event, not days; Stale rule does not apply
    }
}
```

Static + EventBound return None → Stale rule does NOT fire (no day-threshold semantic applies). Test: claim with `Static` class never flagged Stale regardless of age; claim with `EventBound` class same.

If a future ticket promotes these thresholds to method `FreshnessDecayClass::threshold_days(&self)`, the const map becomes a thin wrapper. W4-B keeps it local for now.

### §7.2. `DEFAULT_WEIGHTS` rebalance

`SalienceWeights::validate` (`salience.rs:134-152`) enforces unit-sum to ± WEIGHT_EPSILON. Adding an 11th factor at 1/11 share (0.0909) requires proportional reduction across the existing 10.

```rust
// salience.rs::DEFAULT_WEIGHTS — was 10 entries at 0.10 each, now 11 entries at 1/11 = 0.0909...
const DEFAULT_WEIGHTS: &[(SalienceFactorKind, f64)] = &[
    (SalienceFactorKind::Importance,        1.0 / 11.0),
    (SalienceFactorKind::Novelty,           1.0 / 11.0),
    (SalienceFactorKind::Urgency,           1.0 / 11.0),
    (SalienceFactorKind::Timing,            1.0 / 11.0),
    (SalienceFactorKind::UserFit,           1.0 / 11.0),
    (SalienceFactorKind::Freshness,         1.0 / 11.0),
    (SalienceFactorKind::Trust,             1.0 / 11.0),
    (SalienceFactorKind::Corroboration,     1.0 / 11.0),
    (SalienceFactorKind::Contradiction,     1.0 / 11.0),
    (SalienceFactorKind::OpenLoopRelevance, 1.0 / 11.0),
    (SalienceFactorKind::EntityDeviation,   1.0 / 11.0),  // new
];
```

Sum = 11 × (1/11) = 1.0 exactly. Test: `SalienceWeights::default().validate()` passes.

W5-A eval harness tunes these later. W4-B ships equal-weight default consistent with cycle 0's "10-factor equal-weight starting point" framing.

### §7.3. `SCORE_SALIENCE_SCHEMA_VERSION` bump (codex cycle 1 F7)

`score_salience()` (`salience.rs:211`) returns latest stored salience before recomputing (per `salience.rs:221` cached-return path). Existing rows persist v1 schema (10 factors); without a version bump they bypass the new EntityDeviation factor indefinitely.

Bump `SCORE_SALIENCE_SCHEMA_VERSION` (`salience.rs:24`) from v1 → v2. The cached-return predicate at `:221` already checks schema version equality; v1 rows force recompute on next read. Documented in commit message + W4-B PR body.

Migration: no explicit purge needed — `salience_factors` rows with schema_version = 1 simply force a single recompute on next access. Test: seed v1 salience row, call `score_salience`, assert recompute produces v2 row with 11 factors.

### §7.4. CHECK constraint migration (v275)

Verified at `src-tauri/src/migrations/270_salience_factors.sql:2,26`: both `salience_factors_weights.factor_kind` and `salience_factors.factor_kind` carry CHECK constraints listing the 10 string variants.

```sql
-- src-tauri/src/migrations/275_salience_factors_entity_deviation.sql
-- Extend factor_kind CHECK constraints to include 'entity_deviation' per W4-B.
-- SQLite does not support ALTER TABLE DROP CONSTRAINT; rebuild via copy-recreate pattern.

-- Rename original tables out of the way
ALTER TABLE salience_factors_weights RENAME TO salience_factors_weights_pre_v275;
ALTER TABLE salience_factors        RENAME TO salience_factors_pre_v275;

-- Create new tables with extended CHECK constraints
CREATE TABLE salience_factors_weights (
    factor_kind TEXT NOT NULL PRIMARY KEY
        CHECK (factor_kind IN (
            'importance', 'novelty', 'urgency', 'timing', 'user_fit',
            'freshness', 'trust', 'corroboration', 'contradiction', 'open_loop_relevance',
            'entity_deviation'  -- new in v275
        )),
    weight REAL NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE salience_factors (
    -- ... existing columns ...
    factor_kind TEXT NOT NULL
        CHECK (factor_kind IN (
            'importance', 'novelty', 'urgency', 'timing', 'user_fit',
            'freshness', 'trust', 'corroboration', 'contradiction', 'open_loop_relevance',
            'entity_deviation'
        )),
    -- ... rest ...
);

-- Copy old rows back
INSERT INTO salience_factors_weights SELECT * FROM salience_factors_weights_pre_v275;
INSERT INTO salience_factors        SELECT * FROM salience_factors_pre_v275;

-- Drop staging
DROP TABLE salience_factors_weights_pre_v275;
DROP TABLE salience_factors_pre_v275;
```

Migration test: pre-existing rows survive; INSERT with `factor_kind = 'entity_deviation'` succeeds; INSERT with arbitrary string fails CHECK.

### §7.5. Render mappings (codex F6)

`render.rs::factor_band()` (`render.rs:642`) maps `SalienceFactorKind` → `PrimaryFactorBand` for surface privacy redaction. Add:

```rust
app::SalienceFactorKind::EntityDeviation => runtime::PrimaryFactorBand::NewInformation,
```

`factor_label()` (`render.rs:743`) — add `"entity deviation"`. `salience_factor_kind_storage()` (`render.rs:813`) — add `"entity_deviation"` matching the CHECK constraint string. `services/context.rs:546` mapping — add the new variant (per the existing exhaustive-match pattern in that file).

### §7.6. Runtime + TS mirrors + golden test

`abilities-runtime/src/abilities/recommendations/contracts.rs:254` mirror — add `SalienceFactorKind::EntityDeviation` and `FactorRationale::EntityDeviation` to match the app `contracts.rs`.

`src/services/recommendations/contracts.ts:139` TS mirror — extend the discriminated union for SalienceFactorKind + FactorRationale.

`src/services/recommendations/__tests__/contracts.golden.test.ts:271` — update assertion from 10 factor kinds to 11; add EntityDeviation rationale shape.

`pnpm tsc --noEmit` + `pnpm test` are NOT N/A — W4-B PR must pass both per CLAUDE.md gate. The wave plan's "Don't touch salience scoring" line at §1014 is reconciled by the wave plan amendment in §14 below (cycle 3 wave plan amendment necessary).

## §8. Signal pre-declares — 5-site policy_registry touch + JSON value schema

Verified at `src-tauri/src/signals/policy_registry.rs`: SignalType additions require five parallel-arm edits (codex cycle 3 W4-B B3).

**SignalType variants:**

```rust
// signals/policy_registry.rs line ~60+ (between existing variants — alphabetic-ish placement)
EntityBaselineRecomputed { entity_type: String, entity_id: String },
EntityDeviationDetected   { entity_type: String, entity_id: String },
```

**`from_name` arms (line ~145):**

```rust
"entity_baseline_recomputed" => /* parse entity_type/entity_id from context */ SignalType::EntityBaselineRecomputed { ... },
"entity_deviation_detected"  => SignalType::EntityDeviationDetected   { ... },
```

**`canonical_name` arms (line ~274):**

```rust
SignalType::EntityBaselineRecomputed { .. } => "entity_baseline_recomputed",
SignalType::EntityDeviationDetected   { .. } => "entity_deviation_detected",
```

**`known_signal_type_names()` slice (line ~426):**

```rust
&[
    // ... existing names ...
    "entity_baseline_recomputed",
    "entity_deviation_detected",
]
```

**`policy_for` arms (line ~671):**

```rust
SignalType::EntityBaselineRecomputed { .. } => SignalPolicy { granularity: SignalGranularity::Entity, coalesce: true, payload_privacy: PayloadPrivacy::NonPiiMetadata },
SignalType::EntityDeviationDetected   { .. } => SignalPolicy { granularity: SignalGranularity::Entity, coalesce: true, payload_privacy: PayloadPrivacy::NonPiiMetadata },
```

**JSON `value`-column schema** (codex cycle 1 F1 — signal_events table has no column-level home for `claim_type`/`severity`):

The `signal_events.value` column (TEXT NULL per `src-tauri/src/migrations/018_signal_bus.sql`) carries the structured payload. Emitter and reader share a typed Rust struct serialized via `serde_json` — NOT documented-only:

```rust
// In deviation.rs (or a shared helper module if cycle 4 prefers):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBaselineRecomputedPayload {
    pub claim_type: String,            // ClaimType serialized as snake_case
    pub field_path: String,
    pub window_weeks: u16,
    pub sample_count: u32,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDeviationDetectedPayload {
    pub claim_type: String,
    pub field_path: String,
    pub severity: DeviationSeverity,   // Low/Medium/High/Critical
    pub rule: DeviationRuleKind,
    pub evidence_ref: String,          // claim_id or baseline_id
    pub magnitude: f64,
}
```

Emission path: serialize to JSON, write to `signal_events.value`. Read path (`is_deviation_flagged`): query rows by `entity_type + entity_id + signal_type = "entity_deviation_detected"`, parse `value` as `EntityDeviationDetectedPayload`, filter by `claim_type + field_path` matching the queried claim.

**Test (round-trip)**: emit a deviation signal with a known payload; query `signal_events`; deserialize; assert all fields match. Unparseable rows (malformed JSON) treated as "not flagged" with a `warn!` log.

## §9. ADR-0126 invariant 5 — `is_deviation_flagged()` predicate-only

ADR-0126 inv 5 (`.docs/decisions/0126-memory-substrate-invariants.md:52-60`) binds the schema-assimilation/compression pass to NOT lower the weight of deviation-flagged claims. The enforcement consumer (a consolidation pass) does NOT exist in v1.4.6 (`consolidate_entity` is absent from `src-tauri/src/services/`, verified via grep). DOS-315 from v1.4.1 was the original framing but never shipped (Backlog status).

**W4-B obligation.** Expose `is_deviation_flagged(conn, claim_id, now) -> Result<bool, DeviationError>`. Implementation:

1. `claim_store::load_by_id(claim_id)` to resolve `(entity_type, entity_id, claim_type, field_path)`.
2. Query `signal_events` for the most recent `EntityDeviationDetected` row matching `(entity_type, entity_id, signal_type = "entity_deviation_detected")`.
3. Deserialize `value` as `EntityDeviationDetectedPayload`; filter by `claim_type + field_path` match.
4. Return `true` if the matching signal was emitted within the configured lookback (default 30 days), else `false`.

**Smoke test.** Seed deviation → assert `is_deviation_flagged(claim_id)` returns true. Seed non-deviation → false. Round-trip test (codex cycle 3 W4-B B4) verifies emitter and predicate agree on entity resolution.

**DOS-812** ships the consolidation-pass consumer per cycle 1 §A4. W4-B is unblocked of DOS-812 — `is_deviation_flagged()` ships as substrate, dead code until DOS-812 wires the consumer. Cycle 2 §B6 #2 flag stands: design-decision for DOS-812 is "revive DOS-315" vs "slim v1.4.6 consolidation pass."

## §10. CI invariant gate

Per memory `feedback_enumerate_channels_before_patching` + W4-C parallel discipline:

```rust
// src-tauri/tests/recommendations_w4b_invariants.rs
#[test]
fn deviation_baselines_not_imported_by_trust() {
    let scoped_files = collect_scoped_files();
    for path in scoped_files {
        let body = std::fs::read_to_string(&path).expect("file readable");

        // Direct module imports
        assert!(!body.contains("use crate::services::recommendations::deviation"),
            "{} imports deviation module — deviation is a salience factor input, not a trust input", path.display());

        // Glob and multi-import
        assert!(!body.contains("use crate::services::recommendations::*"),
            "{} glob-imports recommendations — deviation transitively accessible", path.display());
        // Multi-import regex check on { ... } body excludes "deviation"
        for capture in MULTI_IMPORT_RE.captures_iter(&body) {
            assert!(!capture.get(1).unwrap().as_str().contains("deviation"),
                "{} multi-imports deviation", path.display());
        }

        // Direct table reference
        assert!(!body.contains("recommendation_deviation_baselines"),
            "{} references deviation baselines table", path.display());
    }
}

fn collect_scoped_files() -> Vec<PathBuf> {
    // Locked enumeration, sha256 captured in CI artifact for drift detection.
    // Includes:
    //   - src-tauri/src/services/trust_recompute.rs
    //   - src-tauri/src/services/trust_extraction.rs
    //   - All abilities-runtime/src/abilities/trust/**.rs (24 files; explicit list at L1)
    //   - src-tauri/src/services/context.rs (trust-adjacent: imports both trust + recommendations)
    vec![/* explicit enumeration locked at L1 implementation */]
}
```

Pattern matches `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`.

## §11. Tests required (L1 deliverables, not L4 deferrals)

Per memory `feedback_ci_gate_inputs_are_L1_deliverables_not_verification_steps`, all of these ship in the W4-B PR:

1. **Baseline compute correctness** — seed 12 weekly claims with known mean/stddev; assert `recompute_baseline_for_entity` produces matching statistics.
2. **Per-`field_path` isolation** (codex cycle 3 B2) — seed two field_path values under same claim_type for same entity; assert two distinct baseline rows, no contamination.
3. **OutOfRange numeric extraction** (codex cycle 3 B6) — `LiteralKind::{Number, Money, Percentage, Text, Date, Enum}` claims; assert numeric extraction succeeds only on first three; OutOfRange rule does not fire on others.
4. **Stale rule + FreshnessDecayClass mapping** (codex cycle 3 B1 fix) — claim with each of 5 classes (Static, Slow, Medium, Fast, EventBound); assert Stale fires only on Slow/Medium/Fast past threshold; Static + EventBound never trigger Stale.
5. **CadenceBreak rule** — seed cadence pattern with known interval; introduce gap > expected_cadence_days; assert rule fires.
6. **UnexpectedPresence no-op until DOS-811** — assert rule returns false for all claim types (no expected_presence registry data yet).
7. **Empty/insufficient history** (cycle 1 §A7) — 0 and 2 observations (below 3-observation threshold); assert `score_entity_deviation` returns `DeviationScore { magnitude: 0.0 }` without panic.
8. **Window input validation** (cycle 1 §A7) — `recompute_baseline_for_entity(..., window_weeks: 0, ...)` returns `DeviationError::InvalidWindow`; same for `window_weeks: 1, 2`.
9. **Baseline UPSERT idempotence** — call recompute twice with same inputs; single row, `computed_at` updated.
10. **DEFAULT_WEIGHTS unit-sum regression** — `DEFAULT_WEIGHTS.iter().map(|(_, w)| w).sum::<f64>() - 1.0 < WEIGHT_EPSILON`.
11. **SCORE_SALIENCE_SCHEMA_VERSION cache bust** — seed v1 row; call `score_salience`; assert recompute, assert resulting row schema_version = v2 with 11 factors.
12. **CHECK constraint migration** — pre-existing rows survive v275; INSERT with `factor_kind = 'entity_deviation'` succeeds; INSERT with arbitrary string fails CHECK.
13. **Signal round-trip** (codex cycle 3 B5) — emit `EntityDeviationDetected` with known payload; deserialize from `signal_events.value`; assert all fields match. Unparseable row → `is_deviation_flagged` returns false + warn log.
14. **is_deviation_flagged predicate** (cycle 1 §A4) — seeded deviation → true; no deviation → false; deviation older than 30-day lookback → false.
15. **CI invariant gate** (§10) — gate test passes (no enumerated trust file imports deviation).
16. **Salience factor integration smoke** — seed deviation; call `score_salience`; assert the EntityDeviation factor value differs from baseline (no-deviation case).
17. **TS golden test** — `pnpm test` passes; golden snapshot asserts 11 factor kinds + new FactorRationale variant.
18. **Type-check** — `pnpm tsc --noEmit` passes (TS mirror compiles).

All pass under: `cargo clippy --lib -- -D warnings && cargo test --lib && cargo test --tests && pnpm tsc --noEmit && pnpm test`.

## §12. Done-when

- [ ] `recommendation_deviation_baselines` migrated (v274); `salience_factors` CHECK extended (v275). `cargo test` runs both cleanly.
- [ ] `deviation.rs` implements `score_entity_deviation` + `recompute_baseline_for_entity` + `is_deviation_flagged` + `purge_baselines_for_entity` per §4.
- [ ] 4 deviation rules implemented (Stale, OutOfRange, CadenceBreak, UnexpectedPresence). Missing deferred to DOS-811.
- [ ] `SalienceFactorKind::EntityDeviation` + `FactorRationale::EntityDeviation` added across **all 3 mirrors**: app contracts.rs, runtime contracts.rs, TS contracts.ts.
- [ ] `DEFAULT_WEIGHTS` rebalanced (1/11 each); unit-sum regression test green.
- [ ] `extract_factors()` extended with `Option<&TrajectoryBundle>` parameter; calls `score_entity_deviation()`. Caller (`score_salience()`) pre-fetches the bundle.
- [ ] `SCORE_SALIENCE_SCHEMA_VERSION` bumped v1 → v2; cache-bust test verified.
- [ ] `render.rs` mappings + `services/context.rs` mapping + TS golden test updated.
- [ ] `signals/policy_registry.rs` — 5-site touch complete (enum + from_name + canonical_name + known_signal_type_names + policy_for).
- [ ] JSON value-field schemas `EntityBaselineRecomputedPayload` + `EntityDeviationDetectedPayload` ship as typed structs; round-trip test passes.
- [ ] CI invariant gate ships in same PR; enumeration locked.
- [ ] All §11 tests pass.
- [ ] `cargo clippy --lib -- -D warnings && cargo test && pnpm tsc --noEmit && pnpm test` green.
- [ ] Wave plan amendment §14 recorded.
- [ ] L2 unanimous APPROVE; L2-status declared in commit message.

## §13. Wave plan amendment (W4-B authorized to touch salience scoring)

Wave plan line §1014 says "Don't touch salience scoring (W1-B locked)." This was authored under the cycle 0/1/2 framing that the deviation factor was a salience consumer. Cycle 3 reframe: the deviation factor IS substrate evolution; the wave plan amendment must reconcile.

**Proposed wave plan amendment** (cycle 1 §A11 flag #1 resolution): add new bullet at §1014:

> *Cycle 3 amendment (2026-05-28):* W4-B IS authorized to add `SalienceFactorKind::EntityDeviation` and the associated CHECK migration, DEFAULT_WEIGHTS rebalance, SCORE_SALIENCE_SCHEMA_VERSION bump, and 3-mirror enum extension. The salience integration is the substrate-evolution deliverable; the previous "Don't touch salience scoring" rule covered scoring weight tuning + algorithm rewrites, not factor additions. W4-B touches salience scoring as substrate extension.

W4-B PR includes the wave-plan-amendment commit alongside the substrate work.

## §14. Verified-against-codebase appendix (every primitive cited file:line)

| Claim | File:line | Verified |
|-------|-----------|----------|
| `deviation.rs` placeholder | `src-tauri/src/services/recommendations/deviation.rs:1-7` | ✓ |
| `salience.rs::extract_factors` sync, takes `&ClaimRow` | `src-tauri/src/services/recommendations/salience.rs:437` | ✓ |
| `score_salience()` sync entry | `salience.rs:211` | ✓ |
| `aggregate_salience()` weighted mean | `salience.rs:154` | ✓ |
| `SalienceWeights::validate` unit-sum enforce | `salience.rs:134-152` | ✓ |
| `SCORE_SALIENCE_SCHEMA_VERSION` cache version | `salience.rs:24` | ✓ |
| stored-salience cached-return path | `salience.rs:221` | ✓ |
| `SalienceFactorKind` 10 variants | `src-tauri/src/services/recommendations/contracts.rs:236-247` | ✓ |
| `FactorRationale` 10 variants | `contracts.rs:255-291` | ✓ |
| Runtime `SalienceFactorKind` mirror | `abilities-runtime/src/abilities/recommendations/contracts.rs:254` | ✓ |
| TS `SalienceFactorKind` mirror | `src/services/recommendations/contracts.ts:139` | ✓ |
| TS golden test asserts 10 factors | `src/services/recommendations/__tests__/contracts.golden.test.ts:271` | ✓ |
| `render.rs::factor_band` mapping | `render.rs:642` | ✓ |
| `render.rs::factor_label` mapping | `render.rs:743` | ✓ |
| `render.rs::salience_factor_kind_storage` | `render.rs:813` | ✓ |
| `context.rs` salience kind mapping | `src-tauri/src/services/context.rs:546` | ✓ |
| `salience_factors_weights` CHECK constraint | `src-tauri/src/migrations/270_salience_factors.sql:2` | ✓ |
| `salience_factors` CHECK constraint | `migrations/270_salience_factors.sql:26` | ✓ |
| `signal_events.value` TEXT NULL | `migrations/018_signal_bus.sql` | ✓ |
| `policy_registry.rs::SignalType` enum | `src-tauri/src/signals/policy_registry.rs:6+` | ✓ |
| `policy_registry.rs::from_name` parser | `policy_registry.rs:~145` | ✓ |
| `policy_registry.rs::canonical_name` | `policy_registry.rs:~274` | ✓ |
| `policy_registry.rs::known_signal_type_names` | `policy_registry.rs:~426` | ✓ |
| `policy_registry.rs::policy_for` | `policy_registry.rs:~671` | ✓ |
| `FreshnessDecayClass` 5 variants (Static, Slow, Medium, Fast, EventBound) | `abilities-runtime/src/abilities/claims.rs:65-76` | ✓ |
| `ClaimTypeMetadata` (no expected_presence today) | `claims.rs:224-242` | ✓ |
| `entity_engagement_curve` table | `migrations/152_dos_215_temporal_entity_type_keys.sql:7-18` | ✓ |
| `TrajectoryBundle` struct | `abilities-runtime/src/abilities/temporal/mod.rs:111-117` | ✓ |
| `TrajectoryReadHandle::read_trajectory_bundle` async | `temporal/mod.rs:194-202` | ✓ |
| `StructuredClaim::ObjectValue::Literal` shape | `abilities-runtime/src/structured_claim.rs:32-43` | ✓ |
| `LiteralKind` variants (Number, Text, Date, Money, Percentage, Enum) | `structured_claim.rs:46-54` | ✓ |
| `intelligence_claims.field_path` column | `migrations/129_dos_7_claims_schema.sql:15` + `account_fact_claims.rs:1180` | ✓ |
| ADR-0126 invariant 5 text | `.docs/decisions/0126-memory-substrate-invariants.md:52-60` | ✓ |
| ADR-0126 invariant 1 (immutability core) | `:20-26` | ✓ |
| ADR-0114 pure-factor extractor pattern | `.docs/decisions/0114-scoring-unification.md` | ✓ |
| ADR-0115 signal granularity registry | `.docs/decisions/0115-signal-granularity-audit.md` | ✓ |
| Migration slot v274 next-free | `src-tauri/src/migrations.rs:1101-1102` (v273 = w2_shape_repair, v274 absent) | ✓ |
| `consolidate_entity` does NOT exist | `grep -rn "consolidate_entity" src-tauri/` returns no matches | ✓ |
| Wave plan §1014 "Don't touch salience scoring" | `.docs/plans/v1.4.6-waves.md:1014` | ✓ — amendment proposed §13 |

## §15. L0 review routing (cycle 4)

Per wave plan §1054:

- **Mandatory**: `/codex challenge` adversarial review on the cycle 3 packet.
- **Planning reviewer (routed)**: `ce-feasibility-reviewer` (W4-B proposes net-new baseline storage + factor addition).
- **K-in (mandatory)**: `ce-learnings-researcher` re-run for class-pattern check.
- **Wave-plan-amendment review**: §13 wave plan amendment posted to the parent wave plan PR / Linear thread for explicit acknowledgment.

Cycle 4 goal: unanimous APPROVE on cycle 3 body. Cycle 3 history-section flags below.

---

## Cycle history (superseded by §0–§14 above)

- **Cycle 0 (2026-05-28)**: Initial packet authored. Substrate audit + 12 sections. Reviewer panel returned: ce-feasibility NEEDS_REVISION (10 findings including fabricated DOS-315/`consolidate_entity` reference); ce-learnings BLOCKED (migration slot v273 already taken); codex F1-F9 surfaced 7 HIGH + 2 MEDIUM new findings.
- **Cycle 1 (2026-05-28)**: Amendments folded ce-feasibility + K-in findings (§A1-§A11). Codex BLOCKED findings (signal payload schema, baseline identity coarseness, numeric extraction underspec, hallucinated `typical_freshness_days` API, salience CHECK migration, TS mirrors, SCORE_SALIENCE_SCHEMA_VERSION, dormant substrate, wave plan §1014 violation) were not yet folded.
- **Cycle 2 (2026-05-28)**: Scope reset stripped salience integration to DOS-814. Substrate-only sections (§B0-§B6). Cycle 3 reviewers surfaced 3 BLOCKING (hallucinated `FreshnessDecayClass::{Volatile,Standard,Stable}` variants — actual = `Static, Slow, Medium, Fast, EventBound`; migration slot needed re-verification; 5-site `policy_registry.rs` touch missed) + 4 NEEDS_REVISION + K-in body-not-reconciled finding.
- **Cycle 3 (2026-05-28)**: Per user direction, scope reset reversed — salience integration is substrate evolution, not consumer wiring. DOS-814 canceled, folded into this packet. Hallucinated APIs corrected with verified file:line citations (§14 appendix). Packet rewritten clean; cycle 0/1/2 amendments superseded by §0–§14.
