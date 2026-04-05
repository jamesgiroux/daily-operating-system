# I503 — intelligence.json Health Schema Evolution

**Priority:** P1
**Area:** Backend / Intelligence Schema + DB + Frontend Types
**Version:** 1.1.0
**Depends on:** None (foundation issue — I508 depends on THIS, not the reverse)
**ADR:** 0097

## Problem

Health scoring data on `IntelligenceJson` is currently two flat, disconnected fields: `health_score: Option<f64>` (an LLM-guessed 0-100 number with no defined rubric) and `health_trend: Option<HealthTrend>` (direction + rationale). The existing `HealthTrend` struct has only `direction: String` and `rationale: Option<String>` — no timeframe, no confidence, no evidence layers.

ADR-0097 defines a structured `AccountHealth` type with two evidence layers (baseline score + relationship dimensions), divergence detection, confidence bands, and LLM narrative synthesis. The current schema cannot represent any of this. Every downstream issue (I499 engine, I500 Glean parsing, I501 transcript sentiment, I502 surfaces) depends on this schema existing first.

The DB migration `045_intelligence_report_fields.sql` added `health_score REAL` and `health_trend TEXT` columns to `entity_intelligence`. These need to be replaced with a single `health TEXT` JSON column holding the full `AccountHealth` struct.

## Design

### New Rust Types (`src-tauri/src/intelligence/io.rs`)

Replace the existing `HealthTrend` struct and the `health_score`/`health_trend` fields on `IntelligenceJson` with the following types, all defined in `io.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountHealth {
    pub score: f64,                          // 0-100
    pub band: String,                        // "green" | "yellow" | "red"
    pub source: HealthSource,
    pub confidence: f64,                     // 0.0-1.0
    pub trend: HealthTrend,                  // expanded
    pub dimensions: RelationshipDimensions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence: Option<HealthDivergence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthTrend {
    pub direction: String,       // "improving" | "stable" | "declining" | "volatile"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,       // "30d" | "90d"
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDimensions {
    pub meeting_cadence: DimensionScore,
    pub email_engagement: DimensionScore,
    pub stakeholder_coverage: DimensionScore,
    pub champion_health: DimensionScore,
    pub financial_proximity: DimensionScore,
    pub signal_momentum: DimensionScore,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DimensionScore {
    pub score: f64,              // 0-100
    pub weight: f64,             // effective weight after lifecycle adjustment + null redistribution
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default = "default_dimension_trend")]
    pub trend: String,           // "improving" | "stable" | "declining"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthSource {
    Org,
    Computed,
    UserSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDivergence {
    pub severity: String,        // "minor" | "notable" | "critical"
    pub description: String,
    pub leading_indicator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgHealthData {
    pub health_band: Option<String>,
    pub health_score: Option<f64>,
    pub renewal_likelihood: Option<String>,
    pub growth_tier: Option<String>,
    pub customer_stage: Option<String>,
    pub support_tier: Option<String>,
    pub icp_fit: Option<String>,
    pub source: String,
    pub gathered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSentiment {
    pub overall: String,           // "positive" | "neutral" | "negative" | "mixed"
    pub customer: Option<String>,
    pub engagement: Option<String>, // "high" | "moderate" | "low" | "disengaged"
    pub forward_looking: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub competitor_mentions: Vec<String>,
    pub champion_present: Option<String>, // "yes" | "no" | "unknown"
    pub champion_engaged: Option<String>, // "yes" | "no" | "n/a"
}
```

### IntelligenceJson Field Change

On `IntelligenceJson`, replace:
```rust
pub health_score: Option<f64>,
pub health_trend: Option<HealthTrend>,
```

With:
```rust
/// ADR-0097: Structured account health with relationship dimensions.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub health: Option<AccountHealth>,
```

The old `HealthTrend` struct is replaced by the expanded version with `timeframe` and `confidence` fields. Add `#[serde(default)]` on the new `HealthTrend` fields to maintain backward compatibility during deserialization of existing data.

### DB Migration

New migration `054_health_schema_evolution.sql`:

```sql
-- I503: Replace flat health fields with structured AccountHealth JSON
ALTER TABLE entity_intelligence ADD COLUMN health TEXT;

-- Migrate existing data: wrap health_score + health_trend into the new health JSON
UPDATE entity_intelligence
SET health = json_object(
    'score', health_score,
    'band', CASE
        WHEN health_score >= 70 THEN 'green'
        WHEN health_score >= 40 THEN 'yellow'
        ELSE 'red'
    END,
    'source', 'computed',
    'confidence', 0.3,
    'trend', COALESCE(health_trend, '{"direction":"stable"}'),
    'dimensions', '{}',
    'narrative', NULL,
    'recommendedActions', '[]'
)
WHERE health_score IS NOT NULL;

-- I500: Add org_health column for Glean org-score data
ALTER TABLE entity_intelligence ADD COLUMN org_health TEXT;
```

The old `health_score` and `health_trend` columns are left in place for backward compatibility but no longer read by the application. They can be dropped in a future migration after confirming no regressions.

**Confidence 0.3 rationale:** Migrated scores get confidence 0.3 (low) because: (1) the original `health_score` was LLM-guessed with no defined rubric — it's a number without a methodology; (2) there are no dimension scores to back it — `dimensions` is `{}`; (3) the band is mechanically derived from the score, not assessed. Once I499 computes real dimension scores, confidence will rise to 0.6-0.9 based on data coverage. The 0.3 ensures migrated scores render with a "low confidence" qualifier rather than appearing authoritative.

### org_health column (for I500 Glean org-score data)

This migration also adds the `org_health` column needed by I500:

```sql
ALTER TABLE entity_intelligence ADD COLUMN org_health TEXT;
```

**Ownership:** I503 owns the column creation. I500 owns reading/writing it. This avoids a migration ordering dependency between I500 and I503 — I503 creates the column, I500 populates it.

### DB Read/Write Updates

In `io.rs`, the `upsert_entity_intelligence()` and `get_entity_intelligence()` functions currently read/write `health_score` and `health_trend` as separate columns. Update them to:
- Write: serialize `AccountHealth` to JSON, store in `health` column
- Read: deserialize `health` column JSON into `Option<AccountHealth>`
- Keep writing `health_score` and `health_trend` for backward compat during transition

### Frontend TypeScript Types

In `src/types/index.ts`, update `EntityIntelligence`:

Replace:
```typescript
healthScore?: number | null;
healthTrend?: { direction: string; rationale?: string } | null;
```

With:
```typescript
/** ADR-0097: Structured account health with relationship dimensions. */
health?: AccountHealth | null;
```

Add new types:
```typescript
export interface AccountHealth {
  score: number;
  band: "green" | "yellow" | "red";
  source: "org" | "computed" | "userSet";
  confidence: number;
  trend: HealthTrend;
  dimensions: RelationshipDimensions;
  divergence?: HealthDivergence | null;
  narrative?: string | null;
  recommendedActions?: string[];
}

export interface HealthTrend {
  direction: "improving" | "stable" | "declining" | "volatile";
  rationale?: string;
  timeframe?: string;
  confidence?: number;
}

export interface RelationshipDimensions {
  meetingCadence: DimensionScore;
  emailEngagement: DimensionScore;
  stakeholderCoverage: DimensionScore;
  championHealth: DimensionScore;
  financialProximity: DimensionScore;
  signalMomentum: DimensionScore;
}

export interface DimensionScore {
  score: number;
  weight: number;
  evidence?: string[];
  trend: "improving" | "stable" | "declining";
}

export interface HealthSource {
  type: "org" | "computed" | "userSet";
}

export interface HealthDivergence {
  severity: "minor" | "notable" | "critical";
  description: string;
  leadingIndicator: boolean;
}

export interface OrgHealthData {
  healthBand?: string;
  healthScore?: number;
  renewalLikelihood?: string;
  growthTier?: string;
  customerStage?: string;
  supportTier?: string;
  icpFit?: string;
  source: string;
  gatheredAt: string;
}

export interface TranscriptSentiment {
  overall: string;
  customer?: string;
  engagement?: string;
  forwardLooking?: boolean;
  competitorMentions?: string[];
  championPresent?: string;
  championEngaged?: string;
}
```

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/io.rs` | Replace `HealthTrend` struct, add `AccountHealth`, `RelationshipDimensions`, `DimensionScore`, `HealthSource`, `HealthDivergence`, `OrgHealthData`, `TranscriptSentiment` types. Replace `health_score`/`health_trend` fields on `IntelligenceJson` with `health: Option<AccountHealth>`. Update `upsert_entity_intelligence()` and `get_entity_intelligence()` to read/write the new `health` column. |
| `src-tauri/src/migrations/054_health_schema_evolution.sql` | New migration: add `health TEXT` and `org_health TEXT` columns to `entity_intelligence`, migrate existing health data. |
| `src-tauri/src/migrations.rs` | Register migration 054. |
| `src/types/index.ts` | Replace `healthScore`/`healthTrend` on `EntityIntelligence` with `health?: AccountHealth`. Add all new TypeScript interfaces. |
| `src-tauri/src/intelligence/prompts.rs` | Update the enrichment prompt to request `AccountHealth` JSON structure instead of flat `healthScore`/`healthTrend` fields. |
| `src-tauri/src/intel_queue.rs` | Update `write_enrichment_results()` to persist the new `health` field. |

## Acceptance Criteria

1. `AccountHealth`, `RelationshipDimensions`, `DimensionScore`, `HealthSource`, `HealthDivergence`, `OrgHealthData`, and `TranscriptSentiment` types compile and serialize/deserialize correctly with `serde_json`
2. Existing `entity_intelligence` rows with `health_score` values are migrated to the new `health` JSON column on schema upgrade
3. `IntelligenceJson` with the old `healthScore`/`healthTrend` JSON keys still deserializes without error (backward compatibility via `#[serde(alias)]` or `#[serde(default)]`)
4. `get_entity_intelligence()` returns the new `AccountHealth` struct when the `health` column is populated
5. `upsert_entity_intelligence()` writes the full `AccountHealth` JSON to the `health` column
6. Frontend `EntityIntelligence.health` type matches the Rust `AccountHealth` serialization (verified by running `pnpm dev` and checking an account detail page with health data)
7. `cargo test` passes — no regressions in intelligence IO tests
8. `cargo clippy --workspace --all-features --lib --bins -- -D warnings` passes

## Pluggable Input Sources (Dual-Mode)

The types defined here (`AccountHealth`, `RelationshipDimensions`, `OrgHealthData`, `TranscriptSentiment`) serve both local and remote computation modes. `OrgHealthData` is the primary pluggability surface — in local mode it may be None or sparsely populated from Glean search; in remote mode (v1.1.0+) a Glean Agent fills it completely. `RelationshipDimensions` are always computed locally. `TranscriptSentiment` can come from either local transcript processing or Glean's Gong integration. All types use `source: String` fields to track provenance regardless of fill path. See `.docs/research/2026-03-04-dual-mode-intelligence-architecture.md`.

## Out of Scope

- Computing dimension scores (I499)
- Parsing Glean org health data (I500)
- Extracting transcript sentiment (I501)
- Rendering health data in the UI (I502)
- Dropping the old `health_score`/`health_trend` columns (future cleanup)
