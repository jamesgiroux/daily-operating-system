# I499 — Health Scoring Engine

**Priority:** P1
**Area:** Backend / Intelligence
**Version:** v1.0.0 (Phase 2)
**Depends on:** I503 (health schema types)
**ADR:** 0097

## Problem

DailyOS has no algorithmic health scoring. The existing `health_score` on `IntelligenceJson` is an LLM-guessed number with no rubric — the model invents its interpretation on each enrichment cycle, producing inconsistent, unauditable results. The sparsity gate (`3+ signals and 2+ meetings`) means ~80% of accounts get `null`. There is no defined computation, no dimensional decomposition, and no lifecycle-aware weighting.

ADR-0097 defines six relationship dimensions that DailyOS can compute deterministically from first-party data (meetings, emails, stakeholders, signals, financial data). These dimensions produce repeatable, auditable sub-scores that the LLM then synthesizes into narrative — but the LLM no longer picks the number.

## Design

### New Module: `src-tauri/src/intelligence/health_scoring.rs`

A pure computation module with no PTY/AI calls. Takes structured data from the DB and produces a `RelationshipDimensions` struct plus a composite score.

### Core Function

```rust
pub fn compute_account_health(
    db: &ActionDb,
    account: &DbAccount,
    org_health: Option<&OrgHealthData>,
) -> AccountHealth
```

This is the top-level entry point. It:
1. Computes each of the 6 dimension scores
2. Applies lifecycle-aware weight adjustments
3. Applies Vitally-style null redistribution for missing dimensions
4. Computes the composite relationship score (weighted average of populated dimensions)
5. Selects the baseline: org score (if available) > computed score > Planhat neutral (50)
6. Detects divergence between baseline and relationship context
7. Computes confidence band from signal coverage
8. Returns a fully populated `AccountHealth` (minus `narrative` and `recommended_actions`, which the LLM fills)

### Dimension Computations

**1. Meeting Cadence** — `compute_meeting_cadence(db, account_id) -> DimensionScore`

Data source: `get_stakeholder_signals()` from `db/signals.rs` (already computes `meeting_frequency_30d`, `meeting_frequency_90d`, `last_meeting`, `temperature`, `trend`).

Scoring logic:
- Base: ratio of 30d count to 90d average (`count_30d / (count_90d / 3.0)`)
- Ratio 0.8-1.2 = stable (score 60-80). Below 0.5 = declining (score 20-40). Above 1.5 = increasing (score 70-90).
- Recency bonus: last meeting < 7d = +10, < 14d = +5
- No meetings in 30d = score 20 (cold but not zero — Planhat neutral principle)
- No meetings ever = null dimension (weight redistributes)
- Evidence: `["12 meetings in 90d (avg 4/month)", "Last meeting 3 days ago", "Cadence increasing"]`

**2. Email Engagement** — `compute_email_engagement(db, account_id) -> DimensionScore`

Data sources:
- `list_recent_email_signals_for_entity()` from `db/signals.rs` — email signals with sentiment, urgency
- `entity_email_cadence` table (via cadence.rs) — rolling averages, anomalies

Scoring logic:
- Sentiment distribution: count positive/neutral/negative/mixed signals in 30d window
- Response rate proxy: `email_signals` does NOT have a `user_is_last_sender` field. **Alternative:** use email signal `direction` or infer from `sender_email` vs user's email. If no direction data is available, this sub-score uses volume trend only (cadence increasing/decreasing) and skips response rate. **TODO:** If email response rate is critical, add `direction TEXT` column to `email_signals` in I499's migration — populated by the email enrichment pipeline from sender/recipient analysis.
- Cadence anomalies from `signals/cadence.rs`: `gone_quiet` anomaly = score penalty (-20), `activity_spike` = context-dependent
- Base score: 50 (neutral) + sentiment_modifier (-20 to +20) + cadence_modifier (-20 to +20) + recency_modifier (-10 to +10)
- No email signals = null dimension

**3. Stakeholder Coverage** — `compute_stakeholder_coverage(db, account_id) -> DimensionScore`

Data sources:
- `entity_people` junction table — people linked to this account
- `account_team` table — people with assigned roles (champion, executive sponsor, etc.)
- Active preset's `stakeholder_roles` — defines what roles should be filled
- `person_relationships` table — relationship type, last interaction

Scoring logic:
- Role fill rate: count of preset `stakeholder_roles` that have at least one person assigned / total preset roles
- Engagement recency: for each filled role, compute days since last meeting with that person (via `meetings_history` + `meeting_attendees`)
- Score: `(role_fill_rate * 60) + (avg_recency_score * 40)` where recency_score is 100 for <7d, 80 for <14d, 50 for <30d, 20 for <60d, 0 for >60d
- No stakeholders linked = null dimension

**4. Champion Health** — `compute_champion_health(db, account_id) -> DimensionScore`

Data sources:
- `account_team` with role containing "champion" — identify the champion person
- `meetings_history` + `meeting_attendees` — champion's meeting attendance
- `email_signals` with `person_id` matching champion — champion's email engagement
- Existing `RelationshipDepth.champion_strength` from prior intelligence (if available)

Scoring logic:
- Champion identified? If no champion role assigned, null dimension
- Meeting attendance: champion attended N of last 5 account meetings → attendance_rate * 40
- Email activity: champion has email signals in last 30d → +20. Sentiment positive → +10. Negative → -20.
- Recency: last meeting with champion <14d = +10, <30d = +5, >60d = -20
- No champion assigned = null dimension

**5. Financial Proximity** — `compute_financial_proximity(db, account) -> DimensionScore`

Data source: `DbAccount` fields: `arr`, `contract_end`, `lifecycle`

Scoring logic:
- Days to renewal: `contract_end` parsed, compute days from now
- Exponential urgency curve: score = `100 * e^(-days/90)` (peaks at renewal, decays with distance)
- ARR weight: higher ARR accounts get slightly amplified scores (log scale, max 1.2x multiplier for >$500K)
- No `contract_end` = null dimension. No `arr` = use urgency alone.
- Evidence: `["Renewal in 45 days", "ARR: $250,000", "Lifecycle: renewal"]`

**6. Signal Momentum** — `compute_signal_momentum(db, account_id) -> DimensionScore`

Data sources:
- `signal_events` table — all signals for this entity in 30d window
- `captures` table — wins and risks in 30d
- Time-decay from `signals/decay.rs`: `decayed_weight(1.0, age_days, 30.0)`

Scoring logic:
- Gather all signals and captures for the entity in a 30d window
- Classify each as positive (win, expansion, positive sentiment) or negative (risk, churn signal, negative sentiment, gone_quiet)
- Apply time-decay weighting: recent signals count more
- Momentum ratio: `weighted_positive / (weighted_positive + weighted_negative)`
- Score: `momentum_ratio * 100`, clamped to 0-100
- No signals or captures = neutral (50), not null — unlike other dimensions, signal_momentum always has a value

### Lifecycle-Aware Weighting

Base weights (sum to 1.0):
```
meeting_cadence:       0.20
email_engagement:      0.15
stakeholder_coverage:  0.20
champion_health:       0.15
financial_proximity:   0.15
signal_momentum:       0.15
```

Lifecycle multipliers (applied to base weights, then re-normalized):

| Lifecycle | meeting | email | stakeholder | champion | financial | signal |
|-----------|---------|-------|-------------|----------|-----------|--------|
| onboarding | 1.5 | 1.0 | 1.5 | 1.0 | 0.7 | 1.0 |
| adoption | 1.0 | 1.0 | 1.0 | 1.5 | 1.0 | 1.5 |
| renewal | 1.0 | 1.3 | 1.0 | 1.3 | 2.0 | 1.3 |
| at-risk | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 2.0 |
| mature | 0.7 | 1.0 | 1.3 | 1.0 | 1.0 | 1.0 |
| (default) | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 |

### Null Redistribution

When a dimension has no data (null), its weight redistributes proportionally across populated dimensions. Implementation:

```rust
fn redistribute_weights(dimensions: &RelationshipDimensions, base_weights: &[f64; 6]) -> [f64; 6] {
    let populated: Vec<(usize, f64)> = [
        dimensions.meeting_cadence.is_populated(),
        dimensions.email_engagement.is_populated(),
        // ... etc
    ].iter().enumerate()
     .filter(|(_, pop)| **pop)
     .map(|(i, _)| (i, base_weights[i]))
     .collect();

    let total_populated_weight: f64 = populated.iter().map(|(_, w)| w).sum();
    let mut final_weights = [0.0f64; 6];
    for (i, w) in populated {
        final_weights[i] = w / total_populated_weight;
    }
    final_weights
}
```

### Confidence Band

```rust
fn compute_confidence(dimensions: &RelationshipDimensions) -> f64 {
    let populated_count = [
        dimensions.meeting_cadence, dimensions.email_engagement,
        dimensions.stakeholder_coverage, dimensions.champion_health,
        dimensions.financial_proximity, dimensions.signal_momentum,
    ].iter().filter(|d| d.score > 0.0 || !d.evidence.is_empty()).count();

    match populated_count {
        5..=6 => 0.9,  // High
        3..=4 => 0.6,  // Moderate
        1..=2 => 0.3,  // Low
        0 => 0.1,      // No data — neutral score
        _ => 0.1,
    }
}
```

### Divergence Detection

When `OrgHealthData` is available as baseline:

```rust
fn detect_divergence(
    org_band: &str,
    relationship_score: f64,
) -> Option<HealthDivergence> {
    let org_range = match org_band {
        "green" => 70.0..=100.0,
        "yellow" => 40.0..=69.9,
        "red" => 0.0..=39.9,
        _ => return None,
    };

    if org_band == "green" && relationship_score < 40.0 {
        Some(HealthDivergence {
            severity: "critical".into(),
            description: format!("Org health is Green but relationship score is {:.0} — meeting cadence or stakeholder engagement may be declining before telemetry reflects it", relationship_score),
            leading_indicator: true,
        })
    } else if org_band == "red" && relationship_score > 70.0 {
        Some(HealthDivergence {
            severity: "notable".into(),
            description: "Org health is Red but relationship signals are strong — TAM engagement may be buffering product/support issues".into(),
            leading_indicator: false,
        })
    }
    // ... additional cases for yellow + moderate divergence
}
```

### Strategic Bucket Classification

A derived function that maps health scores + dimensions into the four-bucket model used by I491 (Portfolio Health report) and I492 (Portfolio page):

```rust
pub enum AccountBucket {
    GrowthFocus,         // invest — expanding, high engagement, expansion signals
    AtRiskSaveable,      // rescue — declining but champion engaged, relationship recoverable
    AtRiskSaveUnlikely,  // escalate — declining with structural damage
    Autopilot,           // monitor — stable, healthy, low-touch
}

pub fn classify_account_bucket(health: &AccountHealth) -> (AccountBucket, String) {
    // Returns (bucket, one-sentence rationale)
    // Decision tree:
    // 1. Score >= 70 AND no declining dimensions → Autopilot
    // 2. Score >= 70 AND expansion_signals present → GrowthFocus
    // 3. Score 40-69 AND champion_health populated AND score > 50 → GrowthFocus
    // 4. Score < 70 AND champion_health > 50 AND meeting_cadence > 40 → AtRiskSaveable
    // 5. Score < 70 AND (champion null OR champion < 30 OR meeting_cadence < 30) → AtRiskSaveUnlikely
    // 6. Default: Autopilot for scores >= 60, AtRiskSaveable for < 60
}
```

The bucket classification is NOT a health score threshold — it synthesizes multiple dimensions. An account with score 65 (moderate) but a strong champion and active expansion conversations is GrowthFocus, not Autopilot. An account with score 55 but no champion, no exec sponsor, and no meetings in 30 days is AtRiskSaveUnlikely, not just "needs attention."

### Integration Point

In `intel_queue.rs`, `gather_enrichment_input()` calls `compute_account_health()` BEFORE building the prompt. The computed `AccountHealth` (without narrative) is:
1. Serialized as JSON context in the enrichment prompt, so the LLM receives pre-computed scores
2. Stored on the `EnrichmentInput` struct so `write_enrichment_results()` can merge the LLM's narrative into the algorithmic scores

The LLM prompt instructs: "Given these computed health dimensions, synthesize a `healthNarrative` explaining what these scores mean for this account. Do NOT change the numeric scores — explain them."

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/health_scoring.rs` | New file: `compute_account_health()`, six dimension computation functions, lifecycle weighting, null redistribution, confidence bands, divergence detection. |
| `src-tauri/src/intelligence/mod.rs` | Add `pub mod health_scoring;` |
| `src-tauri/src/intel_queue.rs` | Call `compute_account_health()` in `gather_enrichment_input()` for account entities. Pass computed health into prompt context and into `EnrichmentInput`. Merge LLM narrative into algorithmic scores in `write_enrichment_results()`. |
| `src-tauri/src/intelligence/prompts.rs` | Add computed health dimensions as structured context in the enrichment prompt. Instruct LLM to produce `healthNarrative` and `recommendedActions` without changing scores. |

## Acceptance Criteria

1. `compute_account_health()` returns an `AccountHealth` with all 6 dimension scores populated for an account that has meetings, email signals, stakeholders, and financial data
2. An account with only meeting data produces scores for `meeting_cadence` only; other dimensions redistribute weight. Composite score reflects meeting cadence alone.
3. An account with zero data produces score 50 (Planhat neutral), confidence 0.1, all dimensions at 0 with empty evidence
4. Lifecycle weighting changes scores: an account in "renewal" lifecycle with `contract_end` in 30 days scores higher on `financial_proximity` than the same account in "mature" lifecycle
5. Divergence is detected when org health band is "green" but computed relationship score is below 40
6. Confidence band reflects dimension coverage: 5-6 populated = 0.9, 3-4 = 0.6, 1-2 = 0.3
7. `cargo test` includes unit tests for each dimension computation, lifecycle weighting, null redistribution, and divergence detection
8. Integration: after enrichment, the account's intelligence.json contains `health.dimensions` with evidence strings, not just numbers
9. `classify_account_bucket()` assigns every account to exactly one of four buckets (GrowthFocus / AtRiskSaveable / AtRiskSaveUnlikely / Autopilot) with a one-sentence rationale
10. Bucket classification uses multiple dimensions — not just health score thresholds. An account at score 65 with strong champion + expansion signals = GrowthFocus, not Autopilot

## Lifecycle String Values

The `lifecycle` field on `DbAccount` is a free-text string. There is no `CHECK` constraint or enum in the DB schema. Known values used in the codebase:

- `"onboarding"` — new account, recently signed
- `"adoption"` — past onboarding, building usage
- `"renewal"` — approaching contract renewal
- `"at-risk"` — identified risk signals
- `"mature"` — long-standing stable account
- `""` or `None` — lifecycle not set (uses default weights)

The lifecycle weighting table above should use case-insensitive matching and treat unknown values as default (1.0 multiplier across all dimensions). **Do not fail or panic on unexpected lifecycle values.**

## Sparse Account Behavior (Normal Case)

For most users, 3 of 6 dimensions will be null (no champion assigned, no financial data, no email signals). This is the **normal case**, not an edge case. Null redistribution must handle it gracefully:

- 1-2 populated dimensions: confidence 0.3, composite score heavily weighted toward what's available
- Score 50 (Planhat neutral) when zero data — not 0. A score of 0 implies "terrible health," but zero data means "unknown."
- Evidence strings should explain: "Based on meeting cadence only — connect email, assign champion, and set renewal date for a more complete picture"

## Breakability

I499 can be broken into sub-issues:
- **I499a** — Core `compute_account_health()` function, lifecycle weighting, null redistribution, confidence bands. Pure computation, no DB integration.
- **I499b** — Individual dimension computation functions (meeting_cadence, email_engagement, stakeholder_coverage, champion_health, financial_proximity, signal_momentum). Each is independent.
- **I499c** — Divergence detection (requires I500 to provide `OrgHealthData`).
- **I499d** — Integration into `intel_queue.rs` (prompt context, merge with LLM narrative).

## Pluggable Input Sources (Dual-Mode)

`compute_account_health()` accepts `Option<OrgHealthData>` as its baseline input. In local mode, this is None or parsed from Glean search results (I500). In remote mode (v1.1.0+), a Glean Agent provides structured `OrgHealthData` directly — same type, richer data (support tickets, NPS, product usage that local mode cannot access). The 6 relationship dimensions are always computed locally from first-party data (meetings, emails, stakeholders). The pluggability is at the baseline layer, not the relationship layer. This matches ADR-0097's "One Score, Two Layers" architecture: the org layer is pluggable, the relationship layer stays local. See `.docs/research/2026-03-04-dual-mode-intelligence-architecture.md`.

## Out of Scope

- Parsing Glean org health data (I500 — this issue accepts `Option<OrgHealthData>` as input)
- Transcript sentiment feeding into dimensions (I509 — interaction dynamics extraction)
- Frontend rendering of dimension scores (I502)
- Tuning dimension weights based on real data — initial weights are hypotheses
- Historical health trend computation (requires multiple enrichment cycles to compare)
- Adding `direction` column to `email_signals` — evaluate during implementation; if email response rate is critical, add as part of I499
