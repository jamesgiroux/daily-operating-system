# ADR-0097: Account Health Scoring Architecture

**Date:** 2026-02-28
**Status:** Proposed
**Target:** v1.1.0
**Extends:** ADR-0086 (Intelligence as Shared Service), ADR-0095 (Dual-Mode Context Architecture)
**Supersedes:** I484 (health score always-on — scope was too narrow)
**Research:** `.docs/research/2026-02-28-hook-gap-analysis.md`, `.docs/research/2026-02-28-health-scoring-research.md`

## Context

### The Problem

DailyOS has two disconnected health systems:

1. **`accounts.health`** — a user-set RAG status (green/yellow/red) stored on the accounts table. This is what the user sees as a colored dot on the accounts list and a badge on account detail. It's manual, subjective, and often the best leading indicator because the TAM knows things the system doesn't.

2. **`entity_intelligence.health_score`** — an LLM-assessed numeric score (0-100) produced during enrichment. It has no defined rubric, no algorithmic foundation, and a sparsity gate that returns null for ~80% of accounts ("only populate when the account has 3+ signals and 2+ meetings"). This score is **never displayed to the user** — it exists only as input to the Account Health Review report's narrative.

These two systems are never merged. The LLM reads the user's RAG status as context but doesn't update it. The user never sees the numeric score. The numeric score has no defined meaning — the LLM invents its interpretation on each enrichment.

### Industry Context

Analysis of six CS platforms (Gainsight, Hook, Vitally, Totango, ChurnZero, Planhat) reveals universal health scoring dimensions:

| Dimension | Platforms Using | DailyOS Coverage |
|-----------|----------------|-----------------|
| Product usage (frequency, depth, breadth) | 6/6 | None natively. Partial via Glean |
| Support health (tickets, SLA, severity) | 6/6 | Text via Glean only |
| Engagement cadence (meetings, emails) | 6/6 | **Strong** — first-party calendar + email |
| Communication sentiment (email/call tone) | 4/6 (growing) | Email: yes. Transcripts: **no** |
| CSM/human assessment (manual RAG) | 5/6 | **Strong** — user-set RAG + NPS |
| NPS/survey data | 6/6 | Static number, manually entered |
| Financial/renewal risk | 4/6 | ARR + contract dates exist |
| Relationship depth | 3/6 | **DailyOS's strongest signal** — exceeds all platforms |
| Lifecycle-aware weighting | 5/6 | Has the data, never uses it |

**Hook's key finding:** Product usage telemetry alone accounts for 70-80% of churn prediction accuracy. DailyOS will never have direct product telemetry — but the user's org already computes health scores from product usage, support SLAs, and commercial data via their CS/RevOps stack, surfaced through Glean.

### The Org's Existing Health Model

Validated via Glean queries: the user's org (VIP) maintains a multi-factor health model that produces:

- **3-band RAG score** (health_score_3_green, health_score_2_yellow, health_score_1_red) backed by granular numeric scores
- **Inputs:** support signals (ticket volume, severity, SLA performance, sentiment), commercial/renewal context (ACV, renewal date, renewal likelihood), product adoption/engagement (customer stage, package level, add-ons, platform usage flags)
- **Distribution:** synced to Salesforce account records, mirrored into Zendesk, indexed by Glean

This model covers the dimensions DailyOS lacks natively (product usage, support SLAs, ICP fit). DailyOS trying to independently replicate this from meeting notes and emails is solving the wrong problem.

### What DailyOS Uniquely Provides

DailyOS has **relationship intelligence** that no other tool — including the org's CS stack — produces:

- Meeting cadence with trend detection (30d vs 90d, frequency changes)
- Per-email sentiment analysis with entity linkage
- Stakeholder coverage assessment (champion strength, executive access, coverage gaps)
- Cross-entity signal propagation with Bayesian learning and time-decay
- Personal context (user's priorities and value proposition shaping interpretation)
- Meeting-level intelligence (pre-briefs, transcript analysis)

The relationship signals are **leading indicators** — a champion disengaging shows up in meeting cadence weeks before it shows up in product usage or support ticket volume.

## Decision

### One Score, Two Layers

Health scoring uses a **single headline score** with **two evidence layers** that explain it:

**Layer 1 — Baseline Score:** The account's mechanical health from the best available source.
- When Glean provides an org health score: use it. It's backed by product telemetry, support SLAs, and commercial data that DailyOS can't replicate.
- When no org score is available (Glean not connected, or org doesn't have a health model): DailyOS computes its own baseline from algorithmic signals with an explicit lower-confidence band.

**Layer 2 — Relationship Context:** DailyOS's unique contribution. Not a second number — structured evidence that explains, confirms, or challenges the baseline. Six algorithmic dimensions:

| Dimension | Computation | What It Measures |
|-----------|------------|-----------------|
| Meeting Cadence | 30d count vs 90d average → trend ratio | Are we meeting enough? Is cadence increasing or declining? |
| Email Engagement | Response rate + sentiment ratio + cadence anomalies | Is the customer responsive? Is tone shifting? |
| Stakeholder Coverage | % of preset roles filled × engagement recency per stakeholder | Do we know the right people? Are they engaged? |
| Champion Health | Champion meeting attendance + sentiment + email engagement | Is our champion still active and positive? |
| Financial Proximity | Days to renewal with exponential urgency + ARR weight | How urgent is the financial timeline? |
| Signal Momentum | Positive vs negative signal ratio, 30d window, time-decayed | Is the trend improving or declining? |

Each dimension produces a 0-100 sub-score. These are **not** averaged into a competing number — they are structured evidence displayed beneath the headline score.

**Divergence Detection:** When relationship context materially contradicts the baseline score, surface it as an exception:
- "Health is Green, but meeting cadence has dropped 60% in 30 days" → leading indicator of decline
- "Health is Yellow, but champion engagement is strong and email sentiment is positive" → relationship is a buffer

### The Trust Hierarchy

| Scenario | Interpretation | Presentation |
|----------|---------------|-------------|
| Org Green + Relationship Strong | Everything aligns. High confidence. | Green band, no alert |
| Org Green + Relationship Declining | **Leading indicator.** Relationship degrading before telemetry catches up. | Green band + amber divergence alert with evidence |
| Org Yellow/Red + Relationship Strong | Product/support issues but relationship holding. TAM is effective. | Yellow/Red band + positive relationship note |
| Org Red + Relationship Declining | Everything is bad. Urgent. | Red band + critical alert. Highest portfolio priority |
| No org score available | DailyOS computes baseline from relationship signals only | Score with "Low confidence — limited signal coverage" qualifier |

The principle: **DailyOS never overrides the org score.** It adds context, flags divergence, and provides the relationship layer the org's tools can't see.

### Sparse Data Handling

Combining two industry patterns:

1. **Planhat-style neutral baseline:** Accounts with no data start at 50 (neutral), not 0 or undefined. Signals move the score up or down from neutral.

2. **Vitally-style null redistribution:** When a relationship dimension has no data (e.g., no email signals), its weight redistributes proportionally across dimensions that DO have data. An account with only meeting data scores on meeting cadence alone, with meeting cadence carrying full weight.

3. **Confidence band:** Every score carries a confidence qualifier based on signal coverage:
   - 5-6 dimensions populated: High confidence
   - 3-4 dimensions populated: Moderate confidence
   - 1-2 dimensions populated: Low confidence — "Limited signal history"
   - 0 dimensions: Neutral (50) — "No relationship data yet"

### Lifecycle-Aware Weighting

Different lifecycle stages weight dimensions differently:

| Lifecycle Stage | Weight Adjustment |
|----------------|-------------------|
| Onboarding | Stakeholder Coverage ↑, Meeting Cadence ↑, Financial Proximity ↓ |
| Adoption | Signal Momentum ↑, Champion Health ↑ |
| Renewal | Financial Proximity ↑↑, Champion Health ↑, Email Engagement ↑ |
| At-Risk | All warning signals ↑, Signal Momentum ↑↑ |
| Mature/Nurture | Meeting Cadence ↓ (lower cadence is normal), Stakeholder Coverage ↑ |

Weight adjustments are multiplicative modifiers (e.g., 1.5x for ↑, 0.7x for ↓) applied to the base weights, then re-normalized to sum to 1.0.

### The LLM's Role

The LLM does **not** pick the health score number. It explains the number.

**Algorithmic (deterministic, repeatable, auditable):**
- Compute each relationship dimension score from structured data
- Parse org health score from Glean results
- Detect divergence between baseline and relationship signals
- Compute confidence band from signal coverage

**LLM-assessed (needs judgment, context-dependent):**
- Narrative synthesis: "JHI is healthy because..." — the WHY behind the numbers
- Divergence interpretation: "Org says green but meeting cadence dropped — this likely means..."
- Risk pattern recognition: Connecting signals the algorithm can't — "champion's tone shifted in last 2 transcripts"
- Actionable recommendation: "Schedule executive touchpoint before renewal conversation"

The enrichment prompt receives the pre-computed dimension scores and produces a `healthNarrative` that synthesizes them into prose. The prompt has a defined rubric — not "assess the health" but "given these computed scores, explain what they mean for this account."

### Transcript Sentiment Extraction

Currently, the transcript processing prompt extracts SUMMARY, DISCUSSION, ANALYSIS, ACTIONS, WINS, RISKS, DECISIONS — **no structured sentiment.**

Email enrichment already extracts structured sentiment (`positive | neutral | negative | mixed`). Meeting sentiment is arguably more valuable — a tense meeting is a stronger signal than a terse email.

Add to the transcript prompt:

```
SENTIMENT:
- overall: positive|neutral|negative|mixed
- customer: positive|neutral|negative|mixed
- engagement: high|moderate|low|disengaged
- forward_looking: yes|no
- competitor_mentions: [list or "none"]
- champion_present: yes|no|unknown
- champion_engaged: yes|no|n/a
END_SENTIMENT
```

These structured signals feed directly into the relationship health dimensions:
- `customer` sentiment → Email Engagement + Signal Momentum dimensions
- `engagement` level → Meeting Cadence quality (not just quantity)
- `forward_looking` → positive momentum signal ("future speak" per ChurnZero)
- `competitor_mentions` → risk signal, feeds into divergence detection
- `champion_present` / `champion_engaged` → Champion Health dimension

### Glean Org-Score Parsing

Currently, Glean health data enters as free text in search results. The system should extract structured fields:

```rust
pub struct OrgHealthData {
    pub health_band: Option<String>,        // "green" | "yellow" | "red"
    pub health_score: Option<f64>,          // e.g., 75.0 (if available)
    pub renewal_likelihood: Option<String>, // "green" | "yellow" | "red"
    pub growth_tier: Option<String>,        // "Tier 1 – Expansion"
    pub customer_stage: Option<String>,     // "Adoption" | "Mature" | etc.
    pub support_tier: Option<String>,       // "Enhanced" | "Premier" | "Standard"
    pub icp_fit: Option<String>,            // "Good Fit" | "Excellent Fit"
    pub source: String,                     // "glean_salesforce" | "glean_zendesk"
    pub gathered_at: String,                // ISO timestamp
}
```

Extraction approach: pattern matching on known field formats from Glean search results. The fields `health_score_3_green`, `renewal_likelihood`, `growth_tier`, `customer_stage` appear as tags and org-level fields in Zendesk/Salesforce data that Glean surfaces. A parser extracts these into the structured type.

When `OrgHealthData` is available, it becomes the baseline score. When unavailable, DailyOS falls back to its own computed baseline.

### intelligence.json Evolution

New and modified fields on `IntelligenceJson`:

```rust
// Replace existing health fields
pub health: Option<AccountHealth>,  // was: health_score + health_trend (separate)

pub struct AccountHealth {
    // Baseline
    pub score: f64,                          // 0-100, the headline number
    pub band: String,                        // "green" | "yellow" | "red"
    pub source: HealthSource,                // Org | Computed | UserSet
    pub confidence: f64,                     // 0.0-1.0

    // Trend
    pub trend: HealthTrend,

    // Relationship dimensions (evidence layer)
    pub dimensions: RelationshipDimensions,

    // Divergence
    pub divergence: Option<HealthDivergence>,

    // Narrative (LLM-synthesized)
    pub narrative: String,                   // "JHI is healthy because..."
    pub recommended_actions: Vec<String>,
}

pub struct HealthTrend {
    pub direction: String,       // "improving" | "stable" | "declining" | "volatile"
    pub rationale: String,       // cited evidence
    pub timeframe: String,       // "30d" | "90d"
    pub confidence: f64,         // 0.0-1.0
}

pub struct RelationshipDimensions {
    pub meeting_cadence: DimensionScore,
    pub email_engagement: DimensionScore,
    pub stakeholder_coverage: DimensionScore,
    pub champion_health: DimensionScore,
    pub financial_proximity: DimensionScore,
    pub signal_momentum: DimensionScore,
}

pub struct DimensionScore {
    pub score: f64,              // 0-100
    pub weight: f64,             // effective weight after lifecycle adjustment + null redistribution
    pub evidence: Vec<String>,   // cited signals driving this score
    pub trend: String,           // "improving" | "stable" | "declining"
}

pub enum HealthSource {
    Org,        // From Glean/org CS stack
    Computed,   // DailyOS algorithmic baseline
    UserSet,    // User's manual RAG override treated as baseline
}

pub struct HealthDivergence {
    pub severity: String,        // "minor" | "notable" | "critical"
    pub description: String,     // "Org health is Green but meeting cadence dropped 60% in 30 days"
    pub leading_indicator: bool, // true when relationship signals predict future baseline change
}
```

### Surface Contract

Where health appears across the app:

| Surface | What Renders | Data Source |
|---------|-------------|-------------|
| **Accounts list** | Health dot (colored by band) + trend arrow | `AccountHealth.band` + `trend.direction` |
| **Account detail hero** | Health band ("72 — Stable ↗") + confidence qualifier | `AccountHealth.score` + `band` + `trend` + `confidence` |
| **Account detail State of Play** | 6 relationship dimensions with sub-scores and evidence | `RelationshipDimensions` |
| **Account detail divergence** | Amber/red alert banner when divergence detected | `HealthDivergence` |
| **Meeting briefing hero** | Account health band + any divergence alert | `AccountHealth.band` + `divergence` |
| **Meeting briefing risks** | Health-derived risks (declining cadence, champion disengagement) | From `dimensions` where score < threshold |
| **Daily briefing attention** | "N accounts with declining relationship signals" | Aggregated from all accounts' `trend.direction == "declining"` |
| **Week page** | Health context per meeting's account | `AccountHealth.band` per meeting entity |
| **Portfolio page** (I492) | Health heatmap, exception list, divergence alerts | All accounts' `AccountHealth` |
| **Reports** | All reports consume structured health dimensions | `AccountHealth` struct |

### CSM Assessment Integration

The user's manual RAG status (`accounts.health`) is treated as follows:

- It is ONE of the relationship dimensions, weighted at ~15% (matching industry consensus of 10-20%)
- When the user explicitly sets RAG to red, it does NOT override the computed score — but it creates a guaranteed divergence alert if the computed score is green ("User assessment: Red. System assessment: Green. Investigate.")
- The user's RAG remains editable and visible as a separate badge on the account detail page — it is their professional judgment, preserved alongside the system's assessment
- Both the user's RAG and the computed score feed into the LLM's narrative synthesis

### Future Direction: Meeting Analytics

Gong provides deeper conversation analytics (talk ratios, trackers, deal health scores) that are not currently surfaced via Glean. DailyOS could independently compute similar meeting-level analytics from transcripts it already processes:

- **Talk time ratios** — who dominated the conversation (extractable from speaker-labeled transcripts)
- **Question density** — how many questions the customer asked (engagement signal)
- **Forward-looking language** — "next quarter," "roadmap," "planning" (the "future speak" signal)
- **Escalation language** — "frustrated," "concerned," "alternative" (risk signal)
- **Decision density** — how many decisions were made per meeting (meeting effectiveness)

These would feed into the Meeting Cadence and Champion Health dimensions — measuring not just "did we meet" but "was the meeting productive and positive." This is a future enhancement beyond v1.1.0 scope but architecturally enabled by the transcript sentiment extraction added in this ADR.

DailyOS's advantage over Gong: Gong can tell you talk ratios and tracker hits. DailyOS can tell you WHY those metrics matter for this specific account given its health state, renewal timeline, stakeholder coverage, and your professional priorities. The interpretation layer is DailyOS's contribution.

## Consequences

### Positive
- Every account gets a health score — no more 80% null rate
- Health scores are backed by defined, auditable dimensions — not LLM guesswork
- The org's existing health model is consumed and enhanced, not duplicated
- Relationship signals (DailyOS's unique advantage) are structured and visible
- Divergence detection provides the leading-indicator value that justifies DailyOS alongside Glean
- Transcript sentiment fills a major gap in meeting-derived signals
- Sparse accounts get neutral scores with confidence bands — not null

### Negative
- Glean org-score parsing is pattern-matching on search result text — fragile if the org changes field formats
- Six algorithmic dimensions require tuning — initial weights are hypotheses, not validated
- The HealthTrend computation depends on having multiple enrichment cycles to compare — first enrichment produces "unknown" trend
- Transcript sentiment adds ~100 tokens to an already large transcript prompt

### Risks
- Over-engineering: the algorithmic dimension scores could become a maintenance burden if weights need constant tuning. Mitigation: start with simple computations, add complexity only when validated against real data
- User trust: if the computed score frequently disagrees with the user's intuition, they'll ignore it. Mitigation: confidence bands + cited evidence + the user's own RAG as a weighted input
- Glean dependency: if Glean's search result format changes, org-score parsing breaks silently. Mitigation: structured extraction with fallback to "no org score available"

## Implementation

Supersedes I484. Implemented across 5 new issues:

| Issue | Scope |
|-------|-------|
| I499 | Health scoring engine — 6 algorithmic dimensions, lifecycle weighting, sparse data handling, confidence bands |
| I500 | Glean org-score parsing — extract structured health data from Glean results, store as baseline |
| I501 | Transcript sentiment extraction — add structured sentiment/engagement/champion fields to transcript prompt |
| I502 | Health surfaces — render health band, dimensions, divergence across all 7+ app surfaces |
| I503 | intelligence.json health schema — `AccountHealth` struct, `HealthTrend`, `RelationshipDimensions`, migration |

Dependency order: I503 (schema) → I499 (engine) + I500 (Glean parsing) + I501 (transcript sentiment) → I502 (surfaces).
