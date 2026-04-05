# Intelligence Schema Research: Industry Models + DailyOS Architecture

**Date:** 2026-02-28
**Status:** Complete — feeds into I508 (Intelligence Schema Redesign)
**Context:** I508 proposes redesigning `IntelligenceJson` for multi-source enrichment. Before finalizing the intelligence dimensions, we studied how Gong, Hook, Gainsight, Vitally, and ChurnZero model account intelligence — then mapped DailyOS's current model and unique advantages against them.

---

## Methodology

1. **Industry research:** Four parallel research sprints covering Gong, Hook, Gainsight/Vitally/ChurnZero intelligence models
2. **Internal audit:** Full inventory of DailyOS's `IntelligenceJson` fields, enrichment prompt, signal taxonomy, and extraction gaps
3. **Cross-reference:** Mapped findings against existing research (health scoring research, Hook gap analysis, Glean integration analysis)
4. **Synthesis:** Identified universal patterns, DailyOS differentiators, and grounded dimensions for I508

---

## Part 1: Industry Intelligence Models

### 1.1 Gong — Revenue Intelligence

Gong's core architectural bet: **the call is the primary record of account activity.** Every recorded conversation produces structured intelligence that feeds upward into deal and account synthesis.

**Per-call extraction (300+ signals):**
- Talk ratio (rep vs. customer speaking time)
- Question density and types
- Longest monologue duration
- Next steps mentioned (boolean + extracted text)
- Topics discussed (NLP extraction against configured "trackers")
- Sentiment shifts during the call
- Competitor mentions
- Pricing discussions
- Action items with owners
- Forward-looking vs. backward-looking language ratio

**Account-level synthesis:**
- **Revenue Graph:** Unified data foundation mapping interactions → contacts → deals → accounts. No concept of "account intelligence" separate from the interactions that comprise it.
- **Deal Board:** Warning categories per deal — single-threaded (one contact only), going dark (no activity in N days), no decision-maker engaged, competitor mentioned, slipped timeline. These are computed from interaction patterns, not manually assessed.
- **AI Briefer:** Cross-conversation synthesis that produces executive summaries of account activity over configurable time windows. References specific calls with timestamps.
- **Account engagement map:** Grid showing which contacts have been engaged, how recently, and by whom on the internal team.

**What Gong does NOT do:**
- No native health score (imports from CRM if available)
- No relationship graph beyond engagement frequency
- No signal bus or temporal decay — engagement is current-state, not trended
- Email is an activity signal (sent/received/opened), not content-analyzed
- No proactive briefing — user pulls reports from the dashboard

**Key insight for DailyOS:** Gong's strength is per-interaction extraction depth. Talk ratios, question density, monologue detection, competitor tracking — these are derived from the same transcripts DailyOS already processes but doesn't extract at this granularity. This is an extraction gap, not an architectural gap.

### 1.2 Hook — Predictive Customer Success

Hook's architectural bet: **machine learning trained on YOUR org's renewal outcomes predicts better than rules or rubrics.**

**Core model: Engagement Level**
- 6-point scale: Influencer > Power > Active > Standard > Inactive > Zombie
- ML-scored, not rule-based — trained on the org's historical churn/renewal data
- Product usage signals are ~70-80% of the score (login frequency, feature adoption, usage breadth)
- Each level has behavioral definitions specific to the org
- 180-day prediction horizon for churn

**Echo agent (conversation analysis layer):**
- Powered by Claude (Anthropic partnership)
- Analyzes conversation transcripts for risk signals, sentiment shifts, competitive mentions
- Produces structured outputs: risk flags, action recommendations, stakeholder sentiment
- Separate from the Engagement Level score — conversation intelligence is a layer ON TOP of usage scoring

**Stakeholder model (pragmatic, not deep):**
- 5 user types mapped to engagement levels
- Focus is on last meeting date and engagement recency, not relationship graph
- No champion strength assessment, no coverage gap analysis
- "Who have we talked to recently?" not "who matters and how well do we know them?"

**Revenue dimensions:**
- Renewal Likelihood (scored separately from engagement)
- Upsell Level + Upsell Opportunity (expansion potential)
- Manual Risk overrides (CSM can flag accounts the ML misses)

**What Hook does NOT do:**
- No meeting-level intelligence (no pre-briefs, no per-call analysis)
- No email content analysis (email is a signal source, not analyzed for commitments or sentiment)
- No relationship depth beyond recency
- No signal sophistication (no Bayesian fusion, no temporal decay, no feedback loops on individual signals)
- No cross-entity hierarchy intelligence (no parent/child portfolio views)

**Key insight for DailyOS:** Hook's Engagement Level proves that product usage is the dominant churn predictor for most SaaS companies. DailyOS will never have native product usage data — but via Glean, it can access whatever product usage data the org indexes. The schema must have a home for this. Meanwhile, Hook's relationship model is notably weak — DailyOS already exceeds it.

### 1.3 Gainsight — CS Operations Platform

Gainsight is the market leader (~$250M ARR, 2000+ customers) with the deepest CS feature set.

**Health scoring: DEAR Framework**
Four evaluative dimensions applied across data sources:
- **D**eployment — is the product being used as intended?
- **E**ngagement — are stakeholders actively engaging with us?
- **A**doption — is usage growing and deepening?
- **R**OI — is the customer achieving their goals?

**C360 (Customer 360 view): 17 sections**
The canonical customer record. Key sections relevant to intelligence schema design:
1. Health score (composite, decomposed into dimensions visible separately)
2. Attributes (firmographics, ICP fit, segment)
3. Relationships (contacts with roles, engagement status)
4. Success Plans (objectives → milestones → tasks)
5. Timeline (activity stream across all touchpoints)
6. Renewals & Expansion (pipeline, forecast)
7. Support (tickets, SLA, CSAT)
8. Product Usage (telemetry, feature adoption)
9. NPS/Surveys (sentiment tracking)
10. CTAs (calls-to-action generated from rules)

**Decomposed scoring is mandatory:**
Gainsight independently arrived at the same conclusion as every other platform: a single composite score is necessary for portfolio sorting, but the decomposed dimensions MUST be visible separately. A score of 72 means nothing without knowing which dimensions are contributing.

**Lifecycle-aware scoring:**
Score configurations differ by lifecycle stage. An onboarding account is weighted toward deployment and engagement. A mature account is weighted toward adoption and ROI. A renewal-stage account is weighted toward renewal signals and executive alignment.

### 1.4 Vitally — Modern CS Platform

**4-dimension framework:**
1. Implementation (onboarding completion, time-to-value)
2. Engagement (meeting cadence, email responsiveness, stakeholder breadth)
3. Usage (product telemetry)
4. Outcomes (success plan progress, value delivered)

**Sparse data handling:** null-exclusion with proportional weight redistribution. If product usage data isn't available, the score reweights across remaining dimensions rather than penalizing the account. Already adopted in DailyOS's ADR-0097.

**Key differentiator:** Vitally treats engagement and usage as SEPARATE dimensions. An account can have high engagement (frequent meetings, responsive emails) but low usage (not logging in). This divergence is a signal itself — it means the relationship is healthy but the product isn't sticky.

### 1.5 ChurnZero — Dual-Track Scoring

**Two separate scores:**
1. **ChurnScore** — ML-predicted churn risk based on usage, engagement, support data
2. **Relationship Score** — meeting cadence, email engagement, stakeholder coverage, sentiment

These are explicitly tracked as separate dimensions because they can diverge: a customer might have great product usage but a deteriorating relationship (or vice versa). ChurnZero's insight: relationship health and product health are correlated but NOT the same thing.

### 1.6 Industry Consensus

Across all five platforms, clear patterns emerge:

**Universal dimensions (4-6 across all platforms):**

| Dimension | Gainsight | Hook | Vitally | ChurnZero | Gong |
|-----------|-----------|------|---------|-----------|------|
| Product Usage/Adoption | DEAR: D+A | Primary signal (~70-80%) | Usage | ChurnScore input | N/A |
| Relationship/Engagement | DEAR: E | Engagement Level | Engagement | Relationship Score | Engagement map |
| Sentiment/Voice of Customer | Timeline + NPS | Echo agent | Outcomes (partial) | ChurnScore input | Per-call sentiment |
| Support Health | C360 section | N/A | N/A | ChurnScore input | N/A |
| Financial/Renewal | C360 section | Revenue dims | N/A | N/A | Deal Board |
| CSM Assessment | CTA-driven | Manual risk override | N/A | N/A | N/A |

**Structural patterns:**
1. **Decomposed scoring is mandatory** — every platform shows dimensions separately
2. **Lifecycle-stage-adjusted weighting** — what matters differs by customer maturity
3. **Manual override as ONE dimension (10-20% weight)** — not a trump card
4. **Relationship intelligence is emerging as a separate track** — ChurnZero most explicit about this
5. **Source attribution is implicit** — platforms know which dimension comes from which data source by construction (usage comes from telemetry, engagement from meetings/email, etc.)

---

## Part 2: DailyOS Current Intelligence Model

### 2.1 IntelligenceJson Field Inventory

Current fields on `IntelligenceJson` (from `intelligence/io.rs`):

| Field | Type | Typical Fill Rate | Primary Source |
|-------|------|-------------------|----------------|
| `executive_assessment` | String | High (~90%) | LLM synthesis of all inputs |
| `risks` | Vec\<IntelRisk\> | Moderate (~60%) | Meeting discussions, emails |
| `recent_wins` | Vec\<IntelWin\> | Moderate (~50%) | Meeting outcomes |
| `current_state` | CurrentState | Moderate (~50%) | LLM assessment |
| `stakeholder_insights` | Vec\<StakeholderInsight\> | High (~80%) | Meeting attendance, emails |
| `value_delivered` | Vec\<ValueItem\> | Low (~30%) | Meeting notes, QBR docs |
| `next_meeting_readiness` | MeetingReadiness | High (~85%) | Calendar + intelligence |
| `company_context` | CompanyContext | Moderate (~60%) | Web search, overview |
| `health_score` | f64 | Low (~20%) | LLM assessment (sparsity gated) |
| `health_trend` | HealthTrend | Low (~20%) | LLM assessment |
| `success_metrics` | Vec\<SuccessMetric\> | Very low (~10%) | No natural source |
| `open_commitments` | Vec\<OpenCommitment\> | Very low (~10%) | Action items (indirect) |
| `relationship_depth` | RelationshipDepth | Low (~25%) | LLM assessment |
| `portfolio` | PortfolioIntelligence | Parent accounts only | Child intelligence rollup |
| `network` | NetworkIntelligence | Person entities only | Relationship graph |
| `user_edits` | Vec\<UserEdit\> | Rare | User corrections |

**Observation:** The schema was designed for a world where inputs are calendar, email, and transcripts. Fields that can be filled from those sources have reasonable fill rates. Fields that require other sources (`success_metrics`, `open_commitments`, health) are consistently sparse.

### 2.2 Signal Taxonomy

DailyOS has a sophisticated 5-tier signal system (`signals/bus.rs`):

| Tier | Source | Weight | Description |
|------|--------|--------|-------------|
| 1 | User correction | 1.0 | Highest — user always right |
| 2 | Meeting outcome | 0.9 | Transcript-derived, high quality |
| 3 | Email signal | 0.7 | Entity-linked email intelligence |
| 4 | Calendar signal | 0.6 | Meeting attendance, cadence |
| 5 | Computed/AI | 0.5 | AI-inferred, lowest base weight |

**Signal types emitted:** entity_linked, entity_unlinked, entity_created, meeting_outcome, email_signal, person_linked, enrichment_complete, action_created, action_completed, action_reopened, user_correction, email_disposition, prep_invalidated, glean (defined but not yet emitting)

**What's NOT signaled:**
- Meeting cadence changes (frequency increase/decrease over time)
- Stakeholder attendance pattern changes
- Email response latency trends
- No-show detection
- Meeting-to-meeting continuity (were last meeting's action items addressed?)

### 2.3 Enrichment Extraction Capabilities

**From transcripts (via Granola/Quill):**
- Summary, discussion points, action items, wins, risks, decisions
- Sentiment per meeting (overall, not per-speaker)
- Key quotes
- NOT extracted: talk ratios, question density, monologue detection, per-speaker sentiment, forward-looking language ratio

**From email (via email signal extraction):**
- Sentiment (positive/negative/neutral), urgency level
- Commitment language detection
- Entity linkage
- Thread-level patterns
- NOT extracted: reply latency trends, CC pattern changes, email volume trends

**From calendar:**
- Meeting frequency, attendee lists, entity linkage
- NOT extracted: cadence consistency, attendee changes over time, no-shows

### 2.4 Fields With No Natural Source

These fields exist in the schema but have no reliable source in local mode:

| Field | Why It's Empty | What Could Fill It |
|-------|---------------|--------------------|
| `success_metrics` | No KPI data source | Glean (dashboards, success plans) |
| `open_commitments` | Action items are stored separately, not synthesized back | Action system + Glean (project trackers) |
| `health_score` | Sparsity gated + no rubric | ADR-0097 (computed, not LLM-assessed) |
| `value_delivered` | Rarely discussed explicitly in meetings | Glean (case studies, ROI docs, QBR decks) |

---

## Part 3: DailyOS Unique Advantages

### 3.1 What DailyOS Does Better Than All Competitors

**1. Meeting-level intelligence depth**
No CS platform produces per-meeting intelligence briefings. Gong extracts from calls but doesn't synthesize preparation. Gainsight has timeline entries but not pre-meeting intelligence. DailyOS's meeting prep — synthesizing relationship history, recent signals, stakeholder context, and entity intelligence into a briefing BEFORE the meeting — is genuinely unique.

**2. Relationship depth as a first-class dimension**
ChurnZero separates relationship health from churn prediction. DailyOS goes further: `RelationshipDepth` tracks champion strength, executive access, stakeholder coverage, and coverage gaps as structured fields. The `person_relationships` table stores confidence-weighted edges between people. No other platform has this level of relationship structure.

**3. Signal sophistication**
Thompson Sampling (Bayesian fusion) with per-source reliability weights, temporal decay, and user-correction feedback loops. Gainsight uses rule-based triggers. Hook uses ML but it's a black box. Gong has no signal bus at all. DailyOS's signal architecture is the most transparent and self-improving.

**4. Personal context layer**
Role presets, value propositions, priorities, context entries — the intelligence is shaped by who YOU are, not a generic CS template. No other platform personalizes intelligence output to the individual operator's professional context.

**5. Entity hierarchy intelligence**
Parent/child account portfolios with cross-BU pattern detection, hotspot surfacing, and portfolio narrative synthesis. Gainsight has parent-child relationships but no portfolio-level intelligence synthesis.

**6. Multi-entity signal propagation**
A signal on one entity can trigger prep invalidation, cross-entity rules, and cascade effects. No other platform has this level of signal interconnection.

**7. Local-first privacy**
All data on the user's machine. No SaaS vendor risk. This isn't a feature for most CS teams, but for executives with sensitive portfolio data, it's a trust requirement.

### 3.2 Where DailyOS Matches Industry Standard

- Risk and opportunity identification (meeting/email-derived)
- Stakeholder engagement tracking
- Executive assessment / account narrative
- Report generation (EBR/QBR, Account Health Review, Risk Briefing)
- Renewal tracking (when date is set)

### 3.3 Where DailyOS Falls Short

**1. Per-interaction extraction depth (vs. Gong)**
DailyOS processes the same transcripts Gong does but extracts less. Missing: talk ratios, question density, monologue detection, per-speaker sentiment, forward-looking language, competitor tracker matching. These are extractable from transcripts DailyOS already has — the gap is in the extraction prompt, not the data source.

**2. Product usage data (vs. Hook, Gainsight)**
No native product telemetry. This is the industry's #1 churn predictor. Mitigation: Glean can surface product usage data if it's indexed (dashboards, internal reports). The schema needs a home for this data even if DailyOS doesn't collect it natively.

**3. Support health (vs. Gainsight, ChurnZero)**
No native support ticket data. Same mitigation as product usage: Glean surfaces it, schema needs a home.

**4. Lifecycle-stage-adjusted intelligence**
DailyOS has lifecycle stages on accounts but they don't change what intelligence is gathered or how it's weighted. Every account gets the same enrichment regardless of whether it's onboarding or renewing. Industry best practice: what matters changes with lifecycle stage.

**5. Temporal trend computation**
Signals exist but trends aren't computed. "Meeting cadence decreased 40% over 90 days" — the data is in the calendar but the computation doesn't exist. Same for email response latency trends, stakeholder engagement trends.

---

## Part 4: Grounded Intelligence Dimensions for I508

Based on the industry analysis and DailyOS's unique position, the schema redesign should organize around dimensions that:

1. **Have clear data sources** — either local (calendar, email, transcripts) or remote (Glean)
2. **Match industry consensus** — don't invent dimensions no one else uses
3. **Play to DailyOS's strengths** — relationship depth, meeting intelligence, signal sophistication
4. **Accommodate Glean data without requiring it** — local mode fills what it can, remote mode fills richer fields

### Proposed Dimensions (grounded)

#### Dimension 1: Strategic Assessment
**Industry parallel:** Gainsight C360 Overview, Gong Account Summary, Hook Risk Assessment
**What it answers:** "Where does this account stand and where is it heading?"

This is DailyOS's existing strength: `executive_assessment`, `risks`, `recent_wins`, `current_state`. The LLM reads all available context and produces a prose synthesis. This is the narrative layer that every competitor either doesn't have (Hook shows numbers, not narratives) or does poorly (Gainsight CTAs are mechanical).

**Existing fields (retain):** `executive_assessment`, `risks`, `recent_wins`, `current_state`
**New fields:**
- `competitive_context: Vec<CompetitiveInsight>` — competitor mentions from meetings, emails, or Glean docs (Gong has "competitor tracker" — we should too)
- `strategic_priorities: Vec<StrategicPriority>` — customer's stated priorities from meetings/QBRs (Gainsight Success Plans capture these)

**Local sources:** Meetings (discussions, agendas), emails (escalation language, priority shifts), transcripts
**Glean sources:** CRM notes, internal strategy docs, competitive intelligence reports, win/loss analyses

#### Dimension 2: Relationship Health
**Industry parallel:** ChurnZero Relationship Score, Gong Engagement Map, Vitally Engagement dimension
**What it answers:** "How healthy is our engagement with this account's people?"

This is DailyOS's deepest differentiator. No competitor has `RelationshipDepth` with champion strength, executive access, and coverage gaps. No competitor has a person relationship graph with confidence-weighted edges.

**Existing fields (retain):** `stakeholder_insights`, `relationship_depth`
**New fields:**
- `coverage_assessment: Option<CoverageAssessment>` — structured view of role coverage vs. gaps (I505)
- `organizational_changes: Vec<OrgChange>` — departures, reorgs, new hires detected via Glean
- `internal_team: Vec<InternalTeamMember>` — who on OUR side covers this account (RM, AE, TAM) — from Glean CRM data

**Local sources:** Meeting attendance patterns, email threads, person_relationships table
**Glean sources:** Org charts, hiring/departure signals, CRM account team assignments

**DailyOS advantage:** Gong shows an engagement map (who talked to whom, when). DailyOS can show relationship DEPTH (how the relationship has evolved, what the engagement pattern means, who's the champion and how strong is their alignment). The engagement map is a fact; the relationship assessment is an interpretation. DailyOS does the interpretation.

#### Dimension 3: Engagement Cadence
**Industry parallel:** Vitally Engagement dimension, Gainsight DEAR "E", ChurnZero engagement signals
**What it answers:** "Is this account getting the right level of attention?"

This is partially new for DailyOS. The raw data exists (calendar, email) but isn't computed into structured assessments today.

**New fields:**
- `meeting_cadence: Option<CadenceAssessment>` — frequency, consistency, trend (computed from calendar data)
- `email_responsiveness: Option<ResponsivenessAssessment>` — reply latency trends, volume patterns (computed from email signals)

**Local sources:** Calendar data (meeting frequency, gaps, attendee consistency), email data (response times, volume)
**Glean sources:** None needed — this is entirely local data

**DailyOS advantage:** Unlike Gainsight or ChurnZero where engagement is a score from CRM activity logging, DailyOS computes engagement directly from the operator's actual calendar and email. No data entry, no CRM logging — the truth is in the calendar.

#### Dimension 4: Value & Outcomes
**Industry parallel:** Gainsight DEAR "R" (ROI), Vitally Outcomes dimension, Success Plans
**What it answers:** "What have we delivered, and are we on track?"

This is DailyOS's weakest dimension today. `value_delivered` and `success_metrics` are almost always empty because the sources are sparse in local mode. Glean changes this significantly.

**Existing fields (retain):** `value_delivered`, `success_metrics`, `open_commitments`
**New fields:**
- `blockers: Vec<Blocker>` — active blockers with owner and timeline (split from risks — risks are potential, blockers are actual)

**Local sources:** Meeting notes (wins, deliverables), action items (commitments)
**Glean sources:** Case studies, ROI documentation, success plans, project trackers, QBR decks

**Key insight:** Separating `blockers` from `risks` follows industry practice. A risk is "budget might get cut." A blocker is "the integration is stalled waiting on their IT team." Different urgency, different action.

#### Dimension 5: Commercial Context
**Industry parallel:** Gainsight Renewals & Expansion, Hook Revenue dimensions, Gong Deal Board
**What it answers:** "What's the business relationship and where is it heading?"

DailyOS has minimal commercial intelligence today — ARR and renewal date are user-entered on the account. Glean can surface rich commercial data from CRM.

**New fields:**
- `contract_context: Option<ContractContext>` — ARR, renewal date, contract terms, pricing model (Glean CRM or user-entered)
- `expansion_signals: Vec<ExpansionSignal>` — upsell indicators from meetings, emails, or Glean pipeline data
- `renewal_outlook: Option<RenewalOutlook>` — confidence assessment combining health + relationship + commercial signals

**Local sources:** User-entered ARR/renewal date, meeting discussions about expansion
**Glean sources:** CRM contract data, pipeline/opportunity records, renewal stage

#### Dimension 6: External Health Signals
**Industry parallel:** Gainsight C360 Support section, ChurnZero ChurnScore inputs, Gainsight DEAR "D"
**What it answers:** "What do signals outside our direct engagement tell us?"

This dimension is almost entirely Glean-dependent. In local mode, these fields will be empty — and that's correct. The gap is visible to the user as "connect Glean to see support health, product adoption, and satisfaction data."

**New fields:**
- `support_health: Option<SupportHealth>` — ticket volume, SLA adherence, severity trends, CSAT (Glean: Zendesk/Intercom)
- `product_adoption: Option<AdoptionSignals>` — usage metrics, feature adoption, activation status (Glean: product analytics if indexed)
- `nps_csat: Option<SatisfactionData>` — survey scores, verbatim feedback (Glean: survey tools if indexed)

**Local sources:** None
**Glean sources:** Support systems, product analytics, survey platforms

**Design principle:** These fields exist in the schema even when empty. In local mode, the gap is visible — the system shows "No support data available" rather than pretending support doesn't matter. This is the "work harder to surface insights or query the user" model the user described.

### Why 6 Dimensions, Not 4 or 8

The industry converges on 4-7 dimensions. Our 6 are grounded in specific parallels:

| DailyOS Dimension | Closest Industry Parallel | Why Separate |
|-------------------|--------------------------|--------------|
| Strategic Assessment | Gainsight Overview, Gong Summary | DailyOS's core strength — narrative synthesis |
| Relationship Health | ChurnZero Relationship Score | DailyOS's deepest differentiator |
| Engagement Cadence | Vitally Engagement | Separating from relationship follows Vitally/ChurnZero's proven pattern |
| Value & Outcomes | Gainsight ROI/Success Plans | Distinct from engagement — delivering value vs. being engaged |
| Commercial Context | Gainsight Renewals, Hook Revenue | Financial signals are a different concern than operational health |
| External Health Signals | Gainsight Support/Usage, ChurnZero ChurnScore | Requires external data sources — clearly delineated from local intelligence |

The key separation that industry validates: **Relationship Health and Engagement Cadence are correlated but NOT the same thing** (ChurnZero's core insight). High engagement (frequent meetings) with deteriorating relationship depth (champion disengaged, new stakeholder not cultivated) is a leading indicator that engagement cadence alone misses.

---

## Part 5: Extraction Opportunities (Post-I508)

These are capabilities DailyOS could add by extracting more from data it already has — no new data sources required. Each represents a post-v1.1.0 enhancement.

### From Transcripts (data already processed)
- **Talk ratio:** Rep vs. customer speaking time — extract from transcript structure
- **Question density:** Count questions asked by each party
- **Per-speaker sentiment:** Not just overall meeting sentiment, but per-attendee
- **Forward-looking language ratio:** Proportion of conversation about future vs. past
- **Competitor mentions:** Track specific competitor names across conversations
- **Escalation language detection:** "concerned about," "executive review," "reconsider"
- **Decision-maker engagement:** Is the economic buyer speaking or just listening?

### From Email (data already processed)
- **Reply latency trends:** Are response times lengthening? (leading churn indicator)
- **CC pattern changes:** New people being CC'd (expanding scope) or dropped (disengaging)
- **Email volume trends:** Is communication frequency changing?
- **Tone shift detection:** Sentiment change over time, not just per-email

### From Calendar (data already processed)
- **Cadence consistency:** Regular cadence vs. irregular scheduling
- **Attendee changes:** Who started attending or stopped attending
- **No-show detection:** Scheduled meetings that didn't happen
- **Meeting-to-meeting continuity:** Were last meeting's action items addressed?

### Computed (from existing signals)
- **Signal momentum:** Rate of signal emission change over time (already partially designed in ADR-0097)
- **Engagement half-life:** Time until engagement decays to 50% of peak
- **Relationship velocity:** Rate of relationship depth change

---

## Part 6: I508 Revision Recommendations

Based on this research, I508's intelligence dimensions should be revised from the original 6 ad-hoc categories to the 6 research-grounded dimensions defined in Part 4. Key changes from the original I508 spec:

### What Changes
1. **Dimension names and organization** — the original 6 (Strategic Position, Risk & Opportunity, Engagement Health, Stakeholder Landscape, Value & Outcomes, Commercial Context) become the 6 grounded dimensions above. Most content is preserved but reorganized.
2. **Risk & Opportunity merged into Strategic Assessment** — industry treats risks as part of the strategic narrative, not a separate dimension. Opportunities become `expansion_signals` in Commercial Context.
3. **Stakeholder Landscape renamed to Relationship Health** — reflects the ChurnZero insight that relationship health is a measurable dimension, not just a stakeholder list
4. **External Health Signals added as explicit dimension** — makes the Glean dependency visible and honest. Empty in local mode is correct behavior.
5. **Engagement Cadence extracted from Engagement Health** — following Vitally's proven separation of "how engaged" from "how healthy the relationship is"

### What Stays the Same
- All existing fields preserved (backward compatible, additive only)
- `#[serde(default)]` on all new fields
- No DB migration needed
- Same enrichment pipeline (one LLM call per entity)
- Gap detection evolved from 3-field checker to dimension-aware
- Source-agnostic schema — local mode fills what it can, remote fills richer fields

### Design Principle for Empty Fields
The schema defines **what good intelligence looks like** regardless of data availability. In local mode, many fields will be empty — this is a feature, not a bug. The system shows the user: "You have deep relationship intelligence and engagement data. You're missing support health, product adoption, and commercial context. Connect Glean or add this context manually."

This is the "work harder to surface insights or query the user" model. The schema doesn't dumb down for limited sources. It makes gaps visible and actionable.

---

## Sources

### Industry Research
- Gong Revenue Intelligence Platform: Revenue Graph, AI Briefer, Deal Board, Engagement Map
- Hook Products: Echo agent, Engagement Level scoring, Activator automation
- Gainsight: C360, DEAR Framework, Scorecards, Success Plans, Staircase AI
- Vitally: 4-dimension health framework, null-exclusion scoring, lifecycle stage guide
- ChurnZero: Dual-track scoring (ChurnScore + Relationship Score), Health Score Dashboard

### Internal
- DailyOS `intelligence/io.rs`: IntelligenceJson struct, all field types and defaults
- DailyOS `intelligence/prompts.rs`: Enrichment prompt schema, `semantic_gap_query()`, transcript extraction
- DailyOS `signals/bus.rs`: Signal taxonomy, source weights, propagation rules
- DailyOS `signals/feedback.rs`: Bayesian learning, Thompson Sampling, Beta distributions
- `.docs/research/2026-02-28-health-scoring-research.md`: ADR-0097 foundations
- `.docs/research/2026-02-28-hook-gap-analysis.md`: Hook vs. DailyOS + Glean gap analysis
- `.docs/research/glean-integration-analysis.md`: Glean integration architecture
- `.docs/decisions/0097-account-health-scoring-architecture.md`: Health scoring ADR
