# I508 — Intelligence Schema Redesign for Multi-Source Enrichment

**Priority:** P0
**Area:** Backend / Intelligence + Frontend
**Version:** v1.0.0 (Phase 2)
**Depends on:** I503 (health schema types — `AccountHealth`, `TranscriptSentiment`, `OrgHealthData` must exist before I508 adds `health: Option<AccountHealth>` to `IntelligenceJson`)
**Absorbs:** I488 (semantic gap queries → becomes a mechanism within the new schema)
**Research:** `.docs/research/2026-02-28-intelligence-schema-research.md`

## Execution split (locked for v1.0.0)

I508 is the umbrella issue. Delivery is split into three concrete tracks:

- **I508a** — Schema/type composition only (`io.rs` + `src/types/index.ts`)
- **I508b** — Enrichment prompt/schema update (depends on I508a)
- **I508c** — Dimension-aware semantic gap query evolution (parallel with I508b after I508a)

This split is mandatory for Phase 2 execution sequencing and should be used for assignment, tracking, and merge gating.

## Problem

`IntelligenceJson` was designed for a world where the only inputs were Google Calendar, Gmail, and local workspace files. The schema asks the LLM to fill fields that make sense for meeting-and-email intelligence:

- `executive_assessment` (prose synthesis of meetings/emails)
- `risks` (from meeting discussions and email escalations)
- `recent_wins` (from meeting outcomes)
- `current_state` (working/not working/unknowns from meetings)
- `stakeholder_insights` (from meeting attendance and email threads)
- `value_delivered` (from meeting notes)
- `success_metrics` (rarely filled — no natural source)
- `open_commitments` (rarely filled — no natural source)
- `relationship_depth` (champion/exec/coverage — subjective LLM assessment)

This schema doesn't know what Glean makes available. Glean surfaces CRM data, support tickets, internal docs, Slack threads, competitive intelligence, product adoption data, org charts, and strategic planning documents. None of these have a home in the current schema. The enrichment prompt asks questions calibrated for meeting transcripts; it doesn't ask the questions Glean can answer.

The result: connecting Glean gives the LLM more context, but the output structure doesn't expand to capture what that context reveals. Glean data goes in as unstructured text and comes out crammed into fields designed for meeting intelligence. A Zendesk escalation trend becomes a bullet in `risks`. A competitor displacement becomes a sentence in `executive_assessment`. Product adoption data has nowhere to go.

### Why this must come first

Every downstream issue assumes a schema that can hold the intelligence it produces:
- **I499 (health engine)** computes 6 dimensions and needs `AccountHealth` in the schema — but "health" itself should be informed by support data, product adoption, and competitive context that the current schema can't capture
- **I500 (Glean org-score)** needs to store structured org health data — currently has no field
- **I505 (Glean stakeholder intelligence)** discovers 72+ contacts with roles — but `stakeholder_insights` is a flat list of names with prose assessments, not structured coverage data
- **I504 (AI relationship inference)** needs relationship data in the prompt schema — but the current schema has no `inferred_relationships` field
- **I507 (source attribution)** needs `source_attribution` alongside intelligence fields — schema must support provenance

Building these features against the current schema means each one hacks its field into an existing structure. Redesigning first means they all land cleanly.

### Local mode benefits equally

The richer schema defines what good account intelligence looks like — regardless of how it's filled. In local mode (no Glean), the system:
- Fills what it can from meetings, emails, and transcripts
- Leaves richer fields empty with low confidence
- The gap becomes visible to the user: "No competitive intelligence available — connect Glean or add context manually"
- The system can prompt the user to fill gaps it can't fill from available sources

This is the "work harder to surface insights or query the user directly" model. The schema doesn't dumb down for limited sources; it shows what's known and what's missing.

## Design

### 1. Intelligence dimensions (research-grounded)

Industry analysis of Gong, Hook, Gainsight, Vitally, and ChurnZero reveals convergence on 4-7 intelligence dimensions with consistent separation patterns. See `.docs/research/2026-02-28-intelligence-schema-research.md` for full analysis. DailyOS's 6 dimensions are grounded in these industry parallels while playing to our unique strengths.

#### Dimension 1: Strategic Assessment
**Industry parallel:** Gainsight C360 Overview, Gong Account Summary, Hook Risk Assessment
**What it answers:** "Where does this account stand and where is it heading?"

DailyOS's core strength is narrative synthesis — the executive assessment that no competitor produces at this depth. This dimension retains existing fields and adds competitive context and strategic priorities that industry competitors track.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `executive_assessment` | String (prose) | Existing | Meetings, emails | + Internal docs, CRM notes |
| `risks` | Vec\<IntelRisk\> | Existing | Meeting discussions, emails | + Support escalations, churn signals |
| `recent_wins` | Vec\<IntelWin\> | Existing | Meeting outcomes | + Case studies, customer comms |
| `current_state` | CurrentState | Existing | LLM assessment | + CRM status |
| `competitive_context` | Vec\<CompetitiveInsight\> | **New** | Meeting mentions | + Win/loss reports, competitive docs |
| `strategic_priorities` | Vec\<StrategicPriority\> | **New** | Meeting agendas, QBR docs | + Customer's internal strategy docs |

**Why risks stay here (not a separate dimension):** Industry treats risks as part of the strategic narrative, not a separate category. Gong surfaces risks in deal summaries. Gainsight surfaces risks in C360 overview. Separating risks into their own dimension was ungrounded — no competitor does this.

#### Dimension 2: Relationship Health
**Industry parallel:** ChurnZero Relationship Score, Gong Engagement Map
**What it answers:** "How healthy are our relationships with this account's people?"

DailyOS's deepest differentiator. No competitor has `RelationshipDepth` with champion strength, executive access, and coverage gaps. No competitor has a person relationship graph with confidence-weighted edges. ChurnZero validates this as a separate dimension from engagement cadence — relationship depth and engagement frequency are correlated but NOT the same thing.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `stakeholder_insights` | Vec\<StakeholderInsight\> | Existing | Meeting attendance, emails | + Org chart, role context |
| `relationship_depth` | RelationshipDepth | Existing | Meeting frequency, emails | + Org chart coverage |
| `coverage_assessment` | Option\<CoverageAssessment\> | **New** | Entity-people links | + Full org roster |
| `organizational_changes` | Vec\<OrgChange\> | **New** | — | Org chart diffs, hiring/departure |
| `internal_team` | Vec\<InternalTeamMember\> | **New** | — | CRM account team, RM/AE/TAM |

**DailyOS advantage vs. Gong:** Gong shows an engagement map (who talked to whom, when). DailyOS shows relationship DEPTH — how the relationship has evolved, who's the champion, how strong is their alignment. The engagement map is a fact; the relationship assessment is an interpretation.

#### Dimension 3: Engagement Cadence
**Industry parallel:** Vitally Engagement dimension, Gainsight DEAR "E"
**What it answers:** "Is this account getting the right level of attention?"

Separated from Relationship Health following Vitally's proven pattern. An account can have high engagement (frequent meetings, responsive emails) but deteriorating relationship depth (champion disengaged, new stakeholder not cultivated). This divergence is a leading indicator that neither dimension captures alone.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `meeting_cadence` | Option\<CadenceAssessment\> | **New** | Calendar data (frequency, gaps, consistency) | — |
| `email_responsiveness` | Option\<ResponsivenessAssessment\> | **New** | Email signals (reply latency, volume) | — |

**DailyOS advantage:** Unlike Gainsight where engagement is logged manually in CRM, DailyOS computes engagement directly from the operator's actual calendar and email. No data entry required — the truth is in the calendar.

#### Dimension 4: Value & Outcomes
**Industry parallel:** Gainsight DEAR "R" (ROI), Vitally Outcomes dimension
**What it answers:** "What have we delivered, and are we on track?"

DailyOS's weakest dimension today — `value_delivered` and `success_metrics` are almost always empty because the sources are sparse in local mode. Glean changes this significantly by surfacing QBR decks, success plans, and ROI documentation.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `value_delivered` | Vec\<ValueItem\> | Existing | Meeting notes, QBR docs | + Case studies, ROI docs |
| `success_metrics` | Vec\<SuccessMetric\> | Existing | — (rarely filled) | KPI dashboards, success plans |
| `open_commitments` | Vec\<OpenCommitment\> | Existing | Action items | + Project trackers, JIRA |
| `blockers` | Vec\<Blocker\> | **New** | Action items, meeting discussions | + Support tickets, engineering blockers |

**Why blockers split from risks:** A risk is "budget might get cut." A blocker is "the integration is stalled waiting on their IT team." Different urgency, different action. Industry practice separates potential (risks) from actual (blockers).

#### Dimension 5: Commercial Context
**Industry parallel:** Gainsight Renewals & Expansion, Hook Revenue dimensions, Gong Deal Board
**What it answers:** "What's the business relationship and where is it heading?"

DailyOS has minimal commercial intelligence today — ARR and renewal date are user-entered. Glean can surface rich commercial data from CRM. This dimension is the foundation for I490 (Renewal Readiness) — without defined sub-structs, the report has nothing structured to work with.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `contract_context` | Option\<ContractContext\> | **New** | User-entered vitals (ARR, dates, term) | + CRM contract data, procurement docs |
| `expansion_signals` | Vec\<ExpansionSignal\> | **New** | Meeting discussions, QBR outcomes | + Pipeline data, upsell indicators |
| `renewal_outlook` | Option\<RenewalOutlook\> | **New** | Health + relationship + contract assessment | + CRM renewal stage, forecast data |

**Sub-struct definitions:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractContext {
    /// annual | multi_year | month_to_month
    pub contract_type: Option<String>,
    /// true if contract auto-renews unless cancelled
    pub auto_renew: Option<bool>,
    /// ISO date — when the relationship began (tenure = renewal risk factor)
    pub contract_start: Option<String>,
    /// ISO date — from accounts.contract_end or Glean/CRM
    pub renewal_date: Option<String>,
    /// Current ARR from vitals or Glean/CRM
    pub current_arr: Option<f64>,
    /// For multi-year: years remaining on current term
    pub multi_year_remaining: Option<i32>,
    /// Outcome of previous renewal: expanded | flat | contracted | contentious | first_term
    pub previous_renewal_outcome: Option<String>,
    /// Known procurement requirements (PO process, legal review timeline, budget approval chain)
    pub procurement_notes: Option<String>,
    /// Customer's fiscal year start month (1-12) — budget cycle affects renewal timing
    pub customer_fiscal_year_start: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionSignal {
    /// What the expansion opportunity is
    pub opportunity: String,
    /// Estimated ARR impact if known
    pub arr_impact: Option<f64>,
    /// Source: meeting discussion, Glean doc, user-entered
    pub source: Option<String>,
    /// exploring | evaluating | committed | blocked
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenewalOutlook {
    /// high | moderate | low — AI-assessed confidence in successful renewal
    pub confidence: Option<String>,
    /// Specific risk factors for THIS renewal (not general account risks)
    pub risk_factors: Vec<String>,
    /// Is there upsell/expansion potential tied to the renewal conversation?
    pub expansion_potential: Option<String>,
    /// When to start the renewal conversation (based on contract type + procurement timeline)
    pub recommended_start: Option<String>,
    /// What strengthens our position: value delivered, switching costs, champion advocacy
    pub negotiation_leverage: Vec<String>,
    /// What weakens our position: competitive pressure, unresolved issues, sponsor departure
    pub negotiation_risk: Vec<String>,
}
```

**Data flow for ContractContext:** This struct merges data from three sources:
1. **User-entered vitals** — ARR, contract_end, contract_start (from accounts table via VitalsStrip)
2. **Glean/CRM** — contract type, auto-renew flag, procurement details, customer fiscal year
3. **AI-inferred** — previous renewal outcome (from account_events history), procurement notes (from meeting discussions)

The enrichment prompt receives the user-entered vitals as facts and the Glean context as additional evidence. The LLM synthesizes both into `ContractContext`. Fields the LLM has no evidence for remain None — visible to the user as "add this to unlock richer renewal intelligence."

**Note on data model:** `accounts.contract_end` and `account_events` renewal rows are currently unsynchronized. I508 doesn't fix this — it defines what intelligence looks like. The sync issue is a data model concern for the vitals/events layer. See I490 design notes for how the report handles both sources.

#### Dimension 6: External Health Signals
**Industry parallel:** Gainsight C360 Support/Usage sections, ChurnZero ChurnScore inputs, Gainsight DEAR "D"
**What it answers:** "What do signals outside our direct engagement tell us?"

Almost entirely Glean-dependent. In local mode, these fields will be empty — and that's correct behavior. The gap is visible to the user as "connect Glean to see support health, product adoption, and satisfaction data." This dimension exists because the industry consensus is that product usage is the #1 churn predictor (~70-80% of prediction weight in Hook's model). DailyOS will never collect this natively but must have a home for it when available via Glean.

| Field | Type | Status | Local Sources | Glean Sources |
|---|---|---|---|---|
| `support_health` | Option\<SupportHealth\> | **New** | — | Zendesk/Intercom ticket data |
| `product_adoption` | Option\<AdoptionSignals\> | **New** | — | Product usage data if indexed |
| `nps_csat` | Option\<SatisfactionData\> | **New** | — | Survey data if indexed |

**Design principle:** These fields exist even when empty. Empty is a feature — it shows the user what intelligence is available vs. missing, and which connections would fill the gaps.

### Sub-struct definitions (Dimensions 1-4, 6)

The following sub-struct types are referenced in the dimension tables above but not yet defined. These MUST be defined alongside `IntelligenceJson` in `io.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageAssessment {
    /// Ratio of preset stakeholder_roles that have at least one person assigned (0.0-1.0)
    pub role_fill_rate: Option<f64>,
    /// Roles from preset that have no assigned person
    pub gaps: Vec<String>,
    /// Roles that are filled with assigned people
    pub covered: Vec<String>,
    /// Overall coverage level: "strong" | "adequate" | "thin" | "critical"
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgChange {
    /// What changed: "departure" | "hire" | "promotion" | "reorg" | "role_change"
    pub change_type: String,
    /// Person affected (name or person_id if known)
    pub person: String,
    /// Previous state (e.g., previous role, previous department)
    pub from: Option<String>,
    /// New state
    pub to: Option<String>,
    /// When detected (ISO date)
    pub detected_at: Option<String>,
    /// Source: "glean" | "meeting" | "email" | "user"
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamMember {
    /// Person ID if known, otherwise name
    pub person_id: Option<String>,
    pub name: String,
    /// Internal role on this account: "RM" | "AE" | "TAM" | "Division Lead" | etc.
    pub role: String,
    /// Source: "glean" | "user" | "crm"
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CadenceAssessment {
    /// Meetings per month (30d rolling average)
    pub meetings_per_month: Option<f64>,
    /// Trend: "increasing" | "stable" | "declining" | "erratic"
    pub trend: Option<String>,
    /// Days since last meeting
    pub days_since_last: Option<u32>,
    /// Assessment: "healthy" | "adequate" | "sparse" | "cold"
    pub assessment: Option<String>,
    /// Evidence strings for transparency
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResponsivenessAssessment {
    /// Trend in reply cadence: "improving" | "stable" | "slowing" | "gone_quiet"
    pub trend: Option<String>,
    /// Volume trend: "increasing" | "stable" | "decreasing"
    pub volume_trend: Option<String>,
    /// Assessment: "responsive" | "normal" | "slow" | "unresponsive"
    pub assessment: Option<String>,
    /// Evidence strings
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blocker {
    /// What is blocked
    pub description: String,
    /// Who owns resolving it (person name or team)
    pub owner: Option<String>,
    /// How long it's been blocked
    pub since: Option<String>,
    /// Impact: "critical" | "high" | "moderate" | "low"
    pub impact: Option<String>,
    /// Source: "meeting" | "email" | "glean" | "user"
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SupportHealth {
    /// Open ticket count
    pub open_tickets: Option<u32>,
    /// Tickets with severity P1/P2
    pub critical_tickets: Option<u32>,
    /// Average resolution time (hours or days)
    pub avg_resolution_time: Option<String>,
    /// Trend: "improving" | "stable" | "degrading"
    pub trend: Option<String>,
    /// CSAT score if available (0-100)
    pub csat: Option<f64>,
    /// Source: "glean_zendesk" | "glean_intercom" | etc.
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionSignals {
    /// Active users / licensed users ratio (0.0-1.0)
    pub adoption_rate: Option<f64>,
    /// Trend: "growing" | "stable" | "declining"
    pub trend: Option<String>,
    /// Key features adopted or not adopted
    pub feature_adoption: Vec<String>,
    /// Last login or usage date (ISO)
    pub last_active: Option<String>,
    /// Source: "glean" | "product_data"
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SatisfactionData {
    /// NPS score (-100 to 100)
    pub nps: Option<i32>,
    /// CSAT score (0-100)
    pub csat: Option<f64>,
    /// Survey date (ISO)
    pub survey_date: Option<String>,
    /// Verbatim feedback if available
    pub verbatim: Option<String>,
    /// Source: "glean" | "survey_tool"
    pub source: Option<String>,
}
```

**TypeScript equivalents** must be added to `src/types/index.ts` for all 9 types above. They follow the same field structure with camelCase naming.

### LLM vs computed ownership for Engagement Cadence

**Critical clarification:** Dimension 3 (Engagement Cadence) has an ownership conflict with I499 (health scoring engine):

- **I499 computes** `meeting_cadence` and `email_engagement` as deterministic `DimensionScore` values from calendar/email data. These feed into health scoring.
- **I508 defines** `meeting_cadence: Option<CadenceAssessment>` and `email_responsiveness: Option<ResponsivenessAssessment>` as LLM-assessed fields on `IntelligenceJson`.

**Resolution:** These are complementary, not conflicting:
- `IntelligenceJson.meeting_cadence` (CadenceAssessment) = LLM's qualitative assessment, stored in intelligence.json. The LLM describes what the cadence means for the relationship.
- `AccountHealth.dimensions.meeting_cadence` (DimensionScore) = algorithmic score from I499, stored in the `health` field. Numbers, weights, evidence.

The LLM receives the computed DimensionScore as context and produces the CadenceAssessment narrative. The prompt should instruct: "Given the computed meeting cadence score of {score}/100, assess what this cadence means for this account relationship."

### Execution split (mandatory)

I508 is executed as sub-issues for deterministic sequencing and parallel ownership:
- **I508a** — Define all sub-struct types in `io.rs` and `types/index.ts`. Pure type work, no prompt changes.
- **I508b** — Update enrichment prompt with new JSON schema and evidence-based guidance. Depends on I508a.
- **I508c** — Evolve `semantic_gap_queries()` to dimension-aware gap detection. Depends on I508a and can run parallel with I508b.

#### Retained as-is (no dimension assignment)
- `next_meeting_readiness` — meeting-specific, not account intelligence
- `company_context` — initial enrichment, adequate as-is
- `portfolio` — parent accounts (I384), adequate as-is
- `network` — person entities (I391), adequate as-is
- `user_edits` — internal tracking, no change
- `health_score` / `health_trend` — evolves per I503/ADR-0097 into `health: Option<AccountHealth>`

### Why 6 dimensions, not 4 or 8

Industry converges on 4-7 dimensions. Our 6 map to specific parallels:

| DailyOS Dimension | Industry Parallel | Why Separate |
|---|---|---|
| Strategic Assessment | Gainsight Overview, Gong Summary | Core DailyOS strength — narrative synthesis |
| Relationship Health | ChurnZero Relationship Score | DailyOS's deepest differentiator |
| Engagement Cadence | Vitally Engagement | Proven separation from relationship (Vitally/ChurnZero) |
| Value & Outcomes | Gainsight ROI/Success Plans | Distinct from engagement — delivering value vs. being engaged |
| Commercial Context | Gainsight Renewals, Hook Revenue | Financial signals are a different concern |
| External Health Signals | Gainsight Support/Usage, ChurnZero ChurnScore | Requires external data — clearly delineated from local intel |

The key separation that industry validates: **Relationship Health and Engagement Cadence are correlated but NOT the same thing** (ChurnZero's core insight). High engagement with deteriorating relationship depth is a leading indicator that engagement cadence alone misses.

### 2. Schema evolution strategy

**NOT a breaking change.** The existing fields (`risks`, `recent_wins`, `stakeholder_insights`, etc.) stay in place. New dimensions are additive. The Rust struct uses `#[serde(default)]` on all new fields so existing `intelligence.json` files deserialize without error.

```rust
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceJson {
    // ... existing fields preserved ...

    // Dimension 1: Strategic Assessment (new fields only — existing fields stay where they are)
    #[serde(default)]
    pub competitive_context: Vec<CompetitiveInsight>,
    #[serde(default)]
    pub strategic_priorities: Vec<StrategicPriority>,

    // Dimension 2: Relationship Health (new fields only)
    #[serde(default)]
    pub coverage_assessment: Option<CoverageAssessment>,
    #[serde(default)]
    pub organizational_changes: Vec<OrgChange>,
    #[serde(default)]
    pub internal_team: Vec<InternalTeamMember>,

    // Dimension 3: Engagement Cadence
    #[serde(default)]
    pub meeting_cadence: Option<CadenceAssessment>,
    #[serde(default)]
    pub email_responsiveness: Option<ResponsivenessAssessment>,

    // Dimension 4: Value & Outcomes (new fields only)
    #[serde(default)]
    pub blockers: Vec<Blocker>,

    // Dimension 5: Commercial Context
    #[serde(default)]
    pub contract_context: Option<ContractContext>,
    #[serde(default)]
    pub expansion_signals: Vec<ExpansionSignal>,
    #[serde(default)]
    pub renewal_outlook: Option<RenewalOutlook>,

    // Dimension 6: External Health Signals
    #[serde(default)]
    pub support_health: Option<SupportHealth>,
    #[serde(default)]
    pub product_adoption: Option<AdoptionSignals>,
    #[serde(default)]
    pub nps_csat: Option<SatisfactionData>,

    // Cross-cutting
    #[serde(default)]
    pub source_attribution: Option<HashMap<String, Vec<String>>>,  // I507

    // Health evolves per I503/ADR-0097
    #[serde(default)]
    pub health: Option<AccountHealth>,
}
```

### 3. Enrichment prompt evolution

The enrichment prompt's JSON schema section expands to include new fields. The prompt framing changes from "analyze these meetings and emails" to "analyze all available context about this account":

**Current framing:** "Based on the meeting history, email signals, and file summaries, produce..."

**New framing:** "Based on all available context about this account — meetings, emails, documents, organizational data, and any other sources — produce a comprehensive intelligence assessment. Fill every field you have evidence for. For fields with no available evidence, omit them."

The prompt lists all fields with guidance on what constitutes evidence:
- `competitive_context`: "Only include if specific competitors are mentioned in the context. Include the source type (meeting, document, email)."
- `support_health`: "Only include if support ticket or customer satisfaction data appears in the context."
- `product_adoption`: "Only include if product usage or feature adoption data appears in the context."
- `meeting_cadence`: "Assess from calendar data — frequency, consistency, gaps. Compare to expected cadence for account tier."
- `email_responsiveness`: "Assess from email signal data — are response times lengthening? Is volume changing?"
- `blockers`: "Distinguish from risks. Blockers are active impediments with known owners. Risks are potential future problems."

This is key: the prompt doesn't assume Glean is connected. It asks for everything the LLM can extract from whatever context it receives. In local mode with only meetings and emails, the LLM simply omits fields it has no evidence for. In remote mode with Glean documents, the LLM fills richer fields because the context contains richer data.

### 4. Gap detection (absorbs I488)

With the richer schema, `semantic_gap_query()` evolves from a 3-field checker to a dimension-aware gap detector:

```rust
pub fn semantic_gap_queries(prior: Option<&IntelligenceJson>) -> Vec<GapQuery> {
    let mut gaps = Vec::new();

    if let Some(p) = prior {
        // Dimension 1: Strategic Assessment
        if p.risks.is_empty() {
            gaps.push(GapQuery {
                dimension: "strategic_risk",
                terms: "risks concerns escalation churn blockers",
                priority: 1,
            });
        }
        if p.competitive_context.is_empty() {
            gaps.push(GapQuery {
                dimension: "competitive",
                terms: "competitor alternative evaluation displacement",
                priority: 2,
            });
        }

        // Dimension 4: Value & Outcomes
        if p.success_metrics.as_ref().map_or(true, |m| m.is_empty()) {
            gaps.push(GapQuery {
                dimension: "success_metrics",
                terms: "KPI metrics success plan outcomes targets",
                priority: 2,
            });
        }
        if p.open_commitments.as_ref().map_or(true, |c| c.is_empty()) {
            gaps.push(GapQuery {
                dimension: "commitments",
                terms: "commitments deliverables action items timeline",
                priority: 3,
            });
        }

        // Dimension 5: Commercial Context
        if p.contract_context.is_none() {
            gaps.push(GapQuery {
                dimension: "contract",
                terms: "contract ARR renewal expansion license",
                priority: 2,
            });
        }

        // Dimension 6: External Health Signals
        if p.support_health.is_none() {
            gaps.push(GapQuery {
                dimension: "support",
                terms: "support tickets escalation CSAT NPS satisfaction",
                priority: 2,
            });
        }
        if p.product_adoption.is_none() {
            gaps.push(GapQuery {
                dimension: "product_adoption",
                terms: "product usage adoption feature activation login",
                priority: 2,
            });
        }
    } else {
        // First enrichment: broad search
        gaps.push(GapQuery {
            dimension: "initial",
            terms: "account overview assessment status relationship",
            priority: 1,
        });
    }

    gaps
}
```

When Glean is connected, these gap queries are sent as additional Glean searches (the mechanism I488 proposed). When Glean is not connected, they drive local file ranking (the existing use case). Same interface, different fill rates.

### 5. Frontend type evolution

`src/types/index.ts` gets matching TypeScript types for all new fields. The account detail page and report generators can reference richer intelligence. New fields render when populated, show "No data available" gracefully when empty — making gaps visible to the user as a feature, not a bug.

### 6. What this does NOT do

- Does NOT change the DB schema for `entity_intelligence` — intelligence.json is a JSON blob column, so new fields are automatically stored
- Does NOT change the Glean search mechanism — I508 defines what to ask for, not how to search
- Does NOT create new AI call sites — the single enrichment pipeline still runs one LLM call per entity
- Does NOT remove existing fields — backward compatible, additive only
- Does NOT implement per-interaction extraction depth (talk ratios, question density, per-speaker sentiment) — those are post-v1.1.0 extraction enhancements documented in the research

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/io.rs` | Add new struct types (`CompetitiveInsight`, `StrategicPriority`, `CoverageAssessment`, `OrgChange`, `InternalTeamMember`, `CadenceAssessment`, `ResponsivenessAssessment`, `Blocker`, `ContractContext`, `ExpansionSignal`, `RenewalOutlook`, `SupportHealth`, `AdoptionSignals`, `SatisfactionData`). Add fields to `IntelligenceJson` with `#[serde(default)]`. |
| `src-tauri/src/intelligence/prompts.rs` | Expand enrichment prompt JSON schema to include all new fields with evidence-based guidance. Update prompt framing from meeting-centric to source-agnostic. Evolve `semantic_gap_query()` into `semantic_gap_queries()` returning structured `Vec<GapQuery>`. Make public. |
| `src/types/index.ts` | Add matching TypeScript types for all new intelligence fields. |
| `src-tauri/src/context_provider/glean.rs` | Wire gap queries as additional Glean searches when connected (mechanism from I488). |

## Acceptance Criteria

1. New fields added to `IntelligenceJson` — all with `#[serde(default)]`, existing intelligence.json files deserialize without error
2. Enrichment prompt asks for all new fields with evidence-based guidance — LLM fills what it can, omits what it can't
3. **Local mode**: enrich account with only meetings/emails. `risks`, `executive_assessment`, `stakeholder_insights` populated (as today). New fields (`competitive_context`, `support_health`, etc.) are empty — this is correct behavior, not a bug
4. **Remote mode**: enrich account with Glean connected. At least 2 new dimension fields populated from Glean context that would have been empty in local mode
5. `semantic_gap_queries()` returns structured gap queries covering all 6 dimensions where gaps exist
6. Gap queries sent to Glean when connected (deduped against standard results)
7. Gap queries drive local file ranking when Glean not connected (existing behavior preserved)
8. Frontend types match — `pnpm tsc --noEmit` passes
9. No DB migration needed — intelligence.json is a JSON blob column
10. Dimensions align with industry parallels documented in research — not ad-hoc categories

## Relationship to Other Issues

- **Absorbs I488** — gap query mechanism is one piece of the broader schema redesign
- **Depends on I503** — `AccountHealth` type is defined in I503 and referenced as `health: Option<AccountHealth>` in `IntelligenceJson`. I503 must land first so the type exists.
- **Prerequisite for I500** — Glean org-score data needs a home (`support_health`, `contract_context`) that only exists after I508
- **Prerequisite for I505** — `coverage_assessment` and `organizational_changes` are where Glean stakeholder intelligence surfaces in intelligence.json
- **Prerequisite for I507** — `source_attribution` field is defined here
- **I499 (health engine)** can proceed in parallel if I503 lands its types within I508's structure
- **I504 (AI relationship inference)** can proceed in parallel — prompt schema for `inferredRelationships` is additive

## Pluggable Input Sources (Dual-Mode)

### Mapping to Glean Agent outputs

The team is building Glean Agents for Book Ranking & Review, Customer Expansion Research, and Negative Sentiment Escalation. The I508 schema maps directly to their structured outputs:

- **Book Ranking signals** → `expansion_signals` (ExpansionSignal), `risks` (IntelRisk), `support_health` (SupportHealth), `product_adoption` (AdoptionSignals). The ranking criteria (renewal timing, traffic trends, competitive CMS, stakeholder changes) are exactly what these dimension fields capture.
- **Expansion Research outputs** → `expansion_signals` (domain whitespace, cross-sell), `competitive_context` (CompetitiveInsight), `organizational_changes` (OrgChange for stakeholder shifts)
- **Sentiment Escalation triggers** → `support_health.trend` = "degrading", email `sentiment` signals, `signal_momentum` declining. These feed into health scoring (I499) which triggers surfacing (I532).

When Glean Agents produce structured responses, they map directly to these types — no schema changes needed. The `source` fields on sub-structs track provenance: `"glean_agent"` vs `"local_enrichment"` vs `"user"`.

The intelligence schema defined here serves both local and remote computation modes. In local mode, the enrichment pipeline fills fields from meetings, emails, and local files. In remote mode (v1.1.0+), Glean Agents provide structured responses that map directly to these same types — `CompetitiveInsight`, `SupportHealth`, `AdoptionSignals`, etc. The schema is source-agnostic by design: each field can be filled by any provider. The `source` fields on sub-structs (`OrgChange.source`, `Blocker.source`, etc.) track provenance regardless of fill path. No schema changes are needed when switching from local to remote intelligence sources. See `.docs/research/2026-03-04-dual-mode-intelligence-architecture.md`.

## Out of Scope

- DB schema changes beyond intelligence.json (entity_intelligence is already a JSON blob)
- Changing how Glean searches work (I508 defines what to ask for, not how to search)
- New AI call sites or prompt pipelines — single enrichment pipeline, one LLM call
- UI for displaying new dimensions on account detail — that's I493 (v1.1.1) and the report issues
- Removing any existing fields — purely additive
- Per-interaction extraction enhancements (talk ratios, question density, per-speaker sentiment) — post-v1.1.0, documented in research
