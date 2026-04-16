//! Entity Intelligence I/O and types (I130 / ADR-0057).
//!
//! Three-file entity pattern: dashboard.json (mechanical) + intelligence.json
//! (synthesized) + dashboard.md (artifact). This module owns the intelligence
//! layer — types, file I/O, and migration from the legacy CompanyOverview.
//!
//! Intelligence is entity-generic: the same `IntelligenceJson` schema applies
//! to accounts, projects, and people. The enrichment prompt is parameterized
//! by entity_type (handled in Phase 2).

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::accounts::CompanyOverview;
use crate::db::{ActionDb, DbAccount};
use crate::util::atomic_write_str;

// =============================================================================
// I576: Source Attribution Types
// =============================================================================

/// I576: Source attribution for individual intelligence items.
/// Every risk, win, stakeholder insight, etc. can carry provenance metadata
/// indicating where the intelligence came from and how confident we are.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSource {
    /// Source identifier: "user_correction", "transcript", "local_file",
    /// "glean_crm", "glean_zendesk", "glean_gong", "glean_chat", "email", "pty_synthesis"
    pub source: String,
    /// Confidence weight from ADR-0100 tiers × signal_weights Bayesian history.
    /// Range: 0.0-1.0. user_correction=1.0, glean_crm=0.9, transcript=0.8, pty_synthesis=0.5
    pub confidence: f64,
    /// When this item was sourced (ISO 8601 timestamp)
    pub sourced_at: String,
    /// Human-readable reference: "meeting 2026-03-10", "Salesforce", "you edited this"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// DOS-13: A recommended action produced by intelligence enrichment.
/// Richer than the `Vec<String>` health recommended_actions — includes rationale,
/// priority (0-4 integer), and optional suggested due date.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedAction {
    /// Concise action title (verb phrase).
    pub title: String,
    /// Why this action matters — references specific signals, people, or meetings.
    pub rationale: String,
    /// Priority 0 (none) to 4 (low). 1 = urgent, 2 = high, 3 = medium.
    #[serde(default = "default_recommended_priority")]
    pub priority: i32,
    /// Optional suggested due date (YYYY-MM-DD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_due: Option<String>,
}

fn default_recommended_priority() -> i32 {
    3
}

/// I576: Tombstone for user-dismissed intelligence items.
/// Prevents enrichment from re-creating items the user explicitly removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissedItem {
    /// The field path (e.g., "risks", "recentWins")
    pub field: String,
    /// Text content of the dismissed item (for fuzzy matching)
    pub content: String,
    /// When dismissed
    pub dismissed_at: String,
}

// =============================================================================
// Intelligence JSON Schema
// =============================================================================

/// A record of a user edit to an intelligence field.
///
/// Stored in intelligence.json to protect user corrections from being
/// overwritten by subsequent AI enrichment cycles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEdit {
    /// JSON path to the edited field (e.g. "executiveAssessment", "stakeholderInsights[0].name").
    pub field_path: String,
    /// ISO 8601 timestamp of when the edit was made.
    pub edited_at: String,
}

/// Consistency verification result status for intelligence output (I527).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsistencyStatus {
    Ok,
    Corrected,
    Flagged,
}

/// Severity classification for a consistency finding (I527).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsistencySeverity {
    High,
    Medium,
    Low,
}

/// A deterministic consistency finding with evidence (I527).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyFinding {
    /// Stable code identifier (e.g. ABSENCE_CONTRADICTION).
    pub code: String,
    pub severity: ConsistencySeverity,
    /// JSON-path-like field location in IntelligenceJson.
    pub field_path: String,
    /// Original contradictory claim snippet.
    pub claim_text: String,
    /// Deterministic evidence used for contradiction detection.
    pub evidence_text: String,
    /// Whether deterministic repair fixed this finding.
    #[serde(default)]
    pub auto_fixed: bool,
}

// =============================================================================
// Portfolio Intelligence (I384 — parent account hierarchy)
// =============================================================================

/// A child account flagged as a hotspot in the parent's portfolio assessment.
///
/// Hotspots are children with active risk or opportunity signals that warrant
/// executive attention at the portfolio level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHotspot {
    pub child_id: String,
    pub child_name: String,
    pub reason: String,
}

/// Portfolio-level intelligence for parent accounts (I384).
///
/// Synthesized from children's intelligence data. Only present on accounts
/// that have child accounts — leaf-node accounts never get this field.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioIntelligence {
    /// Executive summary of the portfolio's overall health.
    pub health_summary: Option<String>,
    /// Children with active risk or opportunity signals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hotspots: Vec<PortfolioHotspot>,
    /// Signal types appearing across 2+ children (e.g., "budget_risk", "expansion").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_bu_patterns: Vec<String>,
    /// Executive synthesis of the portfolio narrative.
    pub portfolio_narrative: Option<String>,
}

// =============================================================================
// Network Intelligence (I391 — person relationship graph)
// =============================================================================

/// A key relationship in a person's network graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkKeyRelationship {
    pub person_id: String,
    pub name: String,
    pub relationship_type: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_summary: Option<String>,
}

/// Network intelligence for person entities (I391, ADR-0088).
///
/// Only present on persons with relationship edges — sparse but always
/// defined for persons during enrichment. File-only (no DB cache column).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkIntelligence {
    #[serde(default = "default_network_health")]
    pub health: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_relationships: Vec<NetworkKeyRelationship>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opportunities: Vec<String>,
    #[serde(default)]
    pub influence_radius: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_summary: Option<String>,
}

pub(crate) fn default_network_health() -> String {
    "unknown".to_string()
}

// =============================================================================
// I396: Intelligence Report Types
// =============================================================================

/// ADR-0097: Structured account health representation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountHealth {
    /// 0-100
    #[serde(default)]
    pub score: f64,
    /// green | yellow | red
    #[serde(default = "default_health_band")]
    pub band: String,
    #[serde(default)]
    pub source: HealthSource,
    /// 0.0-1.0
    #[serde(default)]
    pub confidence: f64,
    /// DOS-84: true when >= 3 of 6 health dimensions have data (weight > 0).
    /// When false, frontend should show "Insufficient Data" instead of the score.
    #[serde(default)]
    pub sufficient_data: bool,
    #[serde(default)]
    pub trend: HealthTrend,
    #[serde(default)]
    pub dimensions: RelationshipDimensions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence: Option<HealthDivergence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<String>,
}

/// ADR-0097: Health trend direction with rationale, timeframe, and confidence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HealthTrend {
    #[serde(default = "default_health_trend_direction")]
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default)]
    pub confidence: f64,
}

/// ADR-0097: Six-dimension relationship health breakdown.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDimensions {
    #[serde(default)]
    pub meeting_cadence: DimensionScore,
    #[serde(default)]
    pub email_engagement: DimensionScore,
    #[serde(default)]
    pub stakeholder_coverage: DimensionScore,
    #[serde(default)]
    pub champion_health: DimensionScore,
    #[serde(default)]
    pub financial_proximity: DimensionScore,
    #[serde(default)]
    pub signal_momentum: DimensionScore,
}

/// ADR-0097: Per-dimension weighted score with supporting evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DimensionScore {
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub weight: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default = "default_dimension_trend")]
    pub trend: String,
}

/// Provenance for account health values.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum HealthSource {
    Org,
    #[default]
    Computed,
    #[serde(rename = "userSet", alias = "user_set")]
    UserSet,
}

/// ADR-0097: Divergence signal between baseline and relationship dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDivergence {
    pub severity: String,
    pub description: String,
    pub leading_indicator: bool,
}

/// I500 pluggability surface for external org-health data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrgHealthData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_band: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_likelihood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icp_fit: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub gathered_at: String,
}

/// I509 local transcript sentiment signal shape (owned by I503 types).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSentiment {
    #[serde(default)]
    pub overall: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_looking: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub competitor_mentions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub champion_present: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub champion_engaged: Option<String>,
    /// I554: Ownership language — customer|vendor|mixed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ownership_language: Option<String>,
    /// I554: Past-tense product references (churn predictor)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub past_tense_references: Option<bool>,
    /// I554: Data export interest (churn predictor)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_export_interest: Option<bool>,
    /// I554: Internal advocacy visible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_advocacy_visible: Option<bool>,
    /// I554: Roadmap interest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roadmap_interest: Option<bool>,
}

fn default_health_band() -> String {
    "yellow".to_string()
}

fn default_health_trend_direction() -> String {
    "stable".to_string()
}

fn default_timeframe() -> String {
    "30d".to_string()
}

fn default_dimension_trend() -> String {
    "stable".to_string()
}

/// A success metric / KPI tracked for an entity (I396).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessMetric {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// An open commitment — a promise made to/from the account (I396).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCommitment {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

/// Relationship depth assessment (I396).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDepth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub champion_strength: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executive_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholder_coverage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_gaps: Option<Vec<String>>,
}

// =============================================================================
// I508a: Intelligence Dimension Sub-Structs
// =============================================================================

// -- Dimension 1: Strategic Assessment --

/// A competitive insight detected from meetings, documents, or external sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitiveInsight {
    /// Competitor name or product.
    pub competitor: String,
    /// "displacement" | "evaluation" | "mentioned" | "incumbent"
    pub threat_level: Option<String>,
    /// What was said or observed about this competitor.
    pub context: Option<String>,
    /// Source: "meeting" | "email" | "glean" | "user"
    pub source: Option<String>,
    /// When this was detected (ISO date).
    pub detected_at: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

/// A strategic priority tracked for this account or project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategicPriority {
    /// The priority or initiative.
    pub priority: String,
    /// "active" | "at_risk" | "completed" | "paused"
    pub status: Option<String>,
    /// Who owns this priority.
    pub owner: Option<String>,
    /// Source: "meeting" | "qbr" | "glean" | "user"
    pub source: Option<String>,
    /// Expected timeline or deadline.
    pub timeline: Option<String>,
}

// -- Dimension 2: Relationship Health --

/// Coverage assessment of stakeholder roles for an account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoverageAssessment {
    /// Ratio of preset stakeholder_roles that have at least one person assigned (0.0-1.0).
    pub role_fill_rate: Option<f64>,
    /// Roles from preset that have no assigned person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<String>,
    /// Roles that are filled with assigned people.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covered: Vec<String>,
    /// Overall coverage level: "strong" | "adequate" | "thin" | "critical"
    pub level: Option<String>,
}

/// An organizational change detected at the account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgChange {
    /// What changed: "departure" | "hire" | "promotion" | "reorg" | "role_change"
    pub change_type: String,
    /// Person affected (name or person_id if known).
    pub person: String,
    /// Previous state (e.g., previous role, previous department).
    pub from: Option<String>,
    /// New state.
    pub to: Option<String>,
    /// When detected (ISO date).
    pub detected_at: Option<String>,
    /// Source: "glean" | "meeting" | "email" | "user"
    pub source: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

/// An internal team member assigned to this account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamMember {
    /// Person ID if known, otherwise None.
    pub person_id: Option<String>,
    /// Display name.
    pub name: String,
    /// Internal role on this account: "RM" | "AE" | "TAM" | "Division Lead" | etc.
    pub role: String,
    /// Source: "glean" | "user" | "crm"
    pub source: Option<String>,
}

// -- Dimension 3: Engagement Cadence --

/// Meeting cadence assessment for an account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CadenceAssessment {
    /// Meetings per month (30d rolling average).
    pub meetings_per_month: Option<f64>,
    /// Trend: "increasing" | "stable" | "declining" | "erratic"
    pub trend: Option<String>,
    /// Days since last meeting.
    pub days_since_last: Option<u32>,
    /// Assessment: "healthy" | "adequate" | "sparse" | "cold"
    pub assessment: Option<String>,
    /// Evidence strings for transparency.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

/// Email responsiveness assessment for an account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResponsivenessAssessment {
    /// Trend in reply cadence: "improving" | "stable" | "slowing" | "gone_quiet"
    pub trend: Option<String>,
    /// Volume trend: "increasing" | "stable" | "decreasing"
    pub volume_trend: Option<String>,
    /// Assessment: "responsive" | "normal" | "slow" | "unresponsive"
    pub assessment: Option<String>,
    /// Evidence strings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

// -- Dimension 4: Value & Outcomes --

/// An active blocker impeding progress on this account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blocker {
    /// What is blocked.
    pub description: String,
    /// Who owns resolving it (person name or team).
    pub owner: Option<String>,
    /// How long it's been blocked (ISO date or duration).
    pub since: Option<String>,
    /// Impact: "critical" | "high" | "moderate" | "low"
    pub impact: Option<String>,
    /// Source: "meeting" | "email" | "glean" | "user"
    pub source: Option<String>,
}

// -- Dimension 5: Commercial Context --

/// Contract and commercial context for an account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractContext {
    /// annual | multi_year | month_to_month
    pub contract_type: Option<String>,
    /// true if contract auto-renews unless cancelled.
    pub auto_renew: Option<bool>,
    /// ISO date — when the relationship began.
    pub contract_start: Option<String>,
    /// ISO date — from accounts.contract_end or Glean/CRM.
    pub renewal_date: Option<String>,
    /// Current ARR from vitals or Glean/CRM.
    pub current_arr: Option<f64>,
    /// For multi-year: years remaining on current term.
    pub multi_year_remaining: Option<i32>,
    /// Outcome of previous renewal: expanded | flat | contracted | contentious | first_term
    pub previous_renewal_outcome: Option<String>,
    /// Known procurement requirements (PO process, legal review timeline, budget approval chain).
    pub procurement_notes: Option<String>,
    /// Customer's fiscal year start month (1-12).
    pub customer_fiscal_year_start: Option<i32>,
}

/// An expansion signal or upsell opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionSignal {
    /// What the expansion opportunity is.
    pub opportunity: String,
    /// Estimated ARR impact if known.
    pub arr_impact: Option<f64>,
    /// Source: meeting discussion, Glean doc, user-entered.
    pub source: Option<String>,
    /// exploring | evaluating | committed | blocked
    pub stage: Option<String>,
    /// I554: Signal strength classification — strong | moderate | early
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

/// Renewal outlook assessment for an account.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenewalOutlook {
    /// high | moderate | low — AI-assessed confidence in successful renewal.
    pub confidence: Option<String>,
    /// Specific risk factors for THIS renewal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risk_factors: Vec<String>,
    /// Is there upsell/expansion potential tied to the renewal conversation?
    pub expansion_potential: Option<String>,
    /// When to start the renewal conversation.
    pub recommended_start: Option<String>,
    /// What strengthens our position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub negotiation_leverage: Vec<String>,
    /// What weakens our position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub negotiation_risk: Vec<String>,
}

// -- Dimension 6: External Health Signals --

/// Support ticket health data from external systems.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SupportHealth {
    /// Open ticket count.
    pub open_tickets: Option<u32>,
    /// Tickets with severity P1/P2.
    pub critical_tickets: Option<u32>,
    /// Average resolution time (hours or days).
    pub avg_resolution_time: Option<String>,
    /// Trend: "improving" | "stable" | "degrading"
    pub trend: Option<String>,
    /// CSAT score if available (0-100).
    pub csat: Option<f64>,
    /// Source: "glean_zendesk" | "glean_intercom" | etc.
    pub source: Option<String>,
}

/// Product adoption signals from external systems.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionSignals {
    /// Active users / licensed users ratio (0.0-1.0).
    pub adoption_rate: Option<f64>,
    /// Trend: "growing" | "stable" | "declining"
    pub trend: Option<String>,
    /// Key features adopted or not adopted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_adoption: Vec<String>,
    /// Last login or usage date (ISO).
    pub last_active: Option<String>,
    /// Source: "glean" | "product_data"
    pub source: Option<String>,
}

/// Customer satisfaction data (NPS/CSAT).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SatisfactionData {
    /// NPS score (-100 to 100).
    pub nps: Option<i32>,
    /// CSAT score (0-100).
    pub csat: Option<f64>,
    /// Survey date (ISO).
    pub survey_date: Option<String>,
    /// Verbatim feedback if available.
    pub verbatim: Option<String>,
    /// Source: "glean" | "survey_tool"
    pub source: Option<String>,
}

/// I651: Product classification from Salesforce via Glean.
/// Contains current product subscriptions and tier information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProductClassification {
    /// Array of active product subscriptions (e.g., CMS, Analytics)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<ProductInfo>,
}

/// I651: Individual product information from Salesforce.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProductInfo {
    /// Product type: "cms", "analytics", etc.
    #[serde(rename = "type")]
    pub type_: Option<String>,
    /// Product tier: "enhanced", "standard", "basic", "premier", "signature", "unknown", etc.
    pub tier: Option<String>,
    /// Annual recurring revenue for this product
    pub arr: Option<f64>,
    /// Billing terms: "annual", "monthly", "multi_year"
    #[serde(rename = "billingTerms")]
    pub billing_terms: Option<String>,
}

/// A Gong call summary produced by Glean enrichment (I535).
///
/// Contains key metadata from a recorded call — title, date, participants,
/// topic summary, and overall sentiment. Used as supplementary context
/// during transcript processing to connect prior call history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GongCallSummary {
    pub title: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    pub key_topics: String,
    /// "positive" | "neutral" | "negative"
    pub sentiment: String,
}

/// Top-level intelligence file (intelligence.json).
///
/// Entity-generic — same schema for accounts, projects, and people per ADR-0057.
/// Factual data (ARR, health, lifecycle) stays in dashboard.json. Intelligence
/// is synthesized assessment that the AI produces from all available signals.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub entity_type: String,
    #[serde(default)]
    pub enriched_at: String,
    #[serde(default)]
    pub source_file_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_manifest: Vec<SourceManifestEntry>,

    /// Prose assessment: account situation / project status / relationship brief.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executive_assessment: Option<String>,

    /// I576: Concise editorial pull quote for visual storytelling.
    /// One impactful sentence — the single thing a reader should remember.
    /// Distinct from executiveAssessment which is a multi-paragraph narrative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_quote: Option<String>,

    /// Account risks / project blockers / relationship risks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risks: Vec<IntelRisk>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_wins: Vec<IntelWin>,

    /// Working / not working / unknowns assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<CurrentState>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder_insights: Vec<StakeholderInsight>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_delivered: Vec<ValueItem>,

    /// Prep items for the next meeting with this entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_readiness: Option<MeetingReadiness>,

    /// Company/project context from web search or overview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_context: Option<CompanyContext>,

    /// Portfolio intelligence for parent accounts (I384).
    /// Only present on accounts with child accounts — leaf nodes never get this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portfolio: Option<PortfolioIntelligence>,

    /// Network intelligence for person entities (I391, ADR-0088).
    /// Only present on persons with relationship edges — sparse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkIntelligence>,

    /// User edits — field paths that the user has manually corrected.
    /// Enrichment cycles preserve these fields instead of overwriting them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_edits: Vec<UserEdit>,

    /// ADR-0097: Structured account health with dimension-level evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<AccountHealth>,

    /// I500: Optional external/org health baseline payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_health: Option<OrgHealthData>,

    /// I396: Success metrics / KPIs tracked for this entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_metrics: Option<Vec<SuccessMetric>>,

    /// I396: Open commitments (promises made to/from the account).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_commitments: Option<Vec<OpenCommitment>>,

    /// I396: Relationship depth assessment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_depth: Option<RelationshipDepth>,

    /// I527: Consistency pass status (ok/corrected/flagged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency_status: Option<ConsistencyStatus>,

    /// I527: Contradiction findings produced by deterministic checks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consistency_findings: Vec<ConsistencyFinding>,

    /// I527: Timestamp of the most recent consistency pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency_checked_at: Option<String>,

    // =========================================================================
    // I508a: Intelligence Dimension Fields
    // =========================================================================

    // Dimension 1: Strategic Assessment
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub competitive_context: Vec<CompetitiveInsight>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategic_priorities: Vec<StrategicPriority>,

    // Dimension 2: Relationship Health
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_assessment: Option<CoverageAssessment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organizational_changes: Vec<OrgChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_team: Vec<InternalTeamMember>,

    // Dimension 3: Engagement Cadence
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_cadence: Option<CadenceAssessment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_responsiveness: Option<ResponsivenessAssessment>,

    // Dimension 4: Value & Outcomes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,

    // Dimension 5: Commercial Context
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_context: Option<ContractContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_signals: Vec<ExpansionSignal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal_outlook: Option<RenewalOutlook>,
    /// I651: Product classification from Salesforce (Glean-only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_classification: Option<ProductClassification>,

    // Dimension 6: External Health Signals
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_health: Option<SupportHealth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_adoption: Option<AdoptionSignals>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nps_csat: Option<SatisfactionData>,

    // Cross-cutting: source attribution (I507)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<std::collections::HashMap<String, Vec<String>>>,

    // I535: Gong call summaries from Glean enrichment (Glean-only field)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gong_call_summaries: Vec<GongCallSummary>,

    // I554: Success plan signals synthesized from aggregate context
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_plan_signals: Option<crate::types::SuccessPlanSignals>,

    /// Phase 2a: Domain list for account domain matching (entity resolution).
    /// Extracted from Glean enrichment, email classification, or meeting attendees.
    /// Used to populate account_domains table for domain-based entity linking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,

    /// I576: Tombstones for dismissed items — prevents re-creation on enrichment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dismissed_items: Vec<DismissedItem>,

    /// DOS-13: AI-recommended actions from intelligence enrichment.
    /// Rich structured recommendations with title, rationale, priority, and optional due date.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<RecommendedAction>,
}

/// I508a: Serialization wrapper for all dimension fields stored in `dimensions_json`.
/// One blob column instead of 15 individual columns — these fields are always
/// loaded/saved together and rarely queried individually.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DimensionsBlob {
    /// I576: Concise editorial pull quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_quote: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub competitive_context: Vec<CompetitiveInsight>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategic_priorities: Vec<StrategicPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_assessment: Option<CoverageAssessment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organizational_changes: Vec<OrgChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_team: Vec<InternalTeamMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_cadence: Option<CadenceAssessment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_responsiveness: Option<ResponsivenessAssessment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_context: Option<ContractContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_signals: Vec<ExpansionSignal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal_outlook: Option<RenewalOutlook>,
    /// I651: Product classification from Salesforce (Glean-only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_classification: Option<ProductClassification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_health: Option<SupportHealth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_adoption: Option<AdoptionSignals>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nps_csat: Option<SatisfactionData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<std::collections::HashMap<String, Vec<String>>>,
    /// DOS-13: AI-recommended actions from intelligence enrichment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_actions: Vec<RecommendedAction>,
}

impl IntelligenceJson {
    /// Pack I508a dimension fields into a single JSON blob for DB storage.
    pub(crate) fn dimensions_blob(&self) -> DimensionsBlob {
        DimensionsBlob {
            pull_quote: self.pull_quote.clone(),
            competitive_context: self.competitive_context.clone(),
            strategic_priorities: self.strategic_priorities.clone(),
            coverage_assessment: self.coverage_assessment.clone(),
            organizational_changes: self.organizational_changes.clone(),
            internal_team: self.internal_team.clone(),
            meeting_cadence: self.meeting_cadence.clone(),
            email_responsiveness: self.email_responsiveness.clone(),
            blockers: self.blockers.clone(),
            contract_context: self.contract_context.clone(),
            expansion_signals: self.expansion_signals.clone(),
            renewal_outlook: self.renewal_outlook.clone(),
            product_classification: self.product_classification.clone(),
            support_health: self.support_health.clone(),
            product_adoption: self.product_adoption.clone(),
            nps_csat: self.nps_csat.clone(),
            source_attribution: self.source_attribution.clone(),
            recommended_actions: self.recommended_actions.clone(),
        }
    }

    /// Unpack I508a dimension fields from DB blob into self.
    pub(crate) fn apply_dimensions_blob(&mut self, blob: &DimensionsBlob) {
        self.pull_quote = blob.pull_quote.clone();
        self.competitive_context = blob.competitive_context.clone();
        self.strategic_priorities = blob.strategic_priorities.clone();
        self.coverage_assessment = blob.coverage_assessment.clone();
        self.organizational_changes = blob.organizational_changes.clone();
        self.internal_team = blob.internal_team.clone();
        self.meeting_cadence = blob.meeting_cadence.clone();
        self.email_responsiveness = blob.email_responsiveness.clone();
        self.blockers = blob.blockers.clone();
        self.contract_context = blob.contract_context.clone();
        self.expansion_signals = blob.expansion_signals.clone();
        self.renewal_outlook = blob.renewal_outlook.clone();
        self.product_classification = blob.product_classification.clone();
        self.support_health = blob.support_health.clone();
        self.product_adoption = blob.product_adoption.clone();
        self.nps_csat = blob.nps_csat.clone();
        self.source_attribution = blob.source_attribution.clone();
        self.recommended_actions = blob.recommended_actions.clone();
    }
}

fn default_version() -> u32 {
    1
}

// =============================================================================
// I576: HasSource trait for source-attributed items
// =============================================================================

/// I576: Trait for intelligence items that carry source attribution.
pub trait HasSource {
    fn item_source(&self) -> Option<&ItemSource>;

    /// Effective confidence for health scoring.
    /// Returns the source confidence if present, or a default baseline.
    fn effective_confidence(&self) -> f64 {
        self.item_source().map(|s| s.confidence).unwrap_or(0.5) // default: pty_synthesis baseline
    }
}

impl HasSource for IntelRisk {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for IntelWin {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for StakeholderInsight {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for ValueItem {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for CompetitiveInsight {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for OrgChange {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for OpenCommitment {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

impl HasSource for ExpansionSignal {
    fn item_source(&self) -> Option<&ItemSource> {
        self.item_source.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceManifestEntry {
    pub filename: String,
    pub modified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Whether this file was selected for the prompt context (vs skipped by budget).
    #[serde(default = "default_selected", skip_serializing_if = "is_true")]
    pub selected: bool,
    /// Reason the file was skipped (only set when selected=false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

fn default_selected() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelRisk {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "default_urgency")]
    pub urgency: String,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

pub(crate) fn default_urgency() -> String {
    "watch".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelWin {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderInsight {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Deterministic link to a Person entity (I420: reconciliation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    /// Suggested Person link (0.6–0.85 confidence) awaiting user confirmation (I420).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_person_id: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
    /// I576: Structured source attribution with confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_source: Option<ItemSource>,
    /// I576: True if multiple sources disagree on this item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingReadiness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prep_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headquarters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

// =============================================================================
// File I/O
// =============================================================================

const INTEL_FILENAME: &str = "intelligence.json";

/// Maximum bytes of file content to include in the intelligence prompt context.
/// Keeps prompt size manageable (~10KB) while preserving the most relevant signals.
/// Read intelligence.json from an entity directory.
///
/// I513: Deprecated for Tauri app call sites — DB is the sole source of truth.
/// Remaining callers: MCP sidecar (mcp/main.rs), internal io.rs (apply_stakeholders_update).
/// Do NOT add new callers — use `db.get_entity_intelligence()` instead.
pub fn read_intelligence_json(dir: &Path) -> Result<IntelligenceJson, String> {
    let path = dir.join(INTEL_FILENAME);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

    // Legacy compatibility: map healthScore/healthTrend payloads into ADR-0097 health.
    if value.get("health").is_none() {
        let legacy_score = value.get("healthScore").and_then(|v| v.as_f64());
        let legacy_trend = value.get("healthTrend").cloned();
        if let Some(score) = legacy_score {
            let direction = legacy_trend
                .as_ref()
                .and_then(|v| v.get("direction"))
                .and_then(|v| v.as_str())
                .unwrap_or("stable");
            let rationale = legacy_trend
                .as_ref()
                .and_then(|v| v.get("rationale"))
                .and_then(|v| v.as_str());
            let band = if score >= 70.0 {
                "green"
            } else if score >= 40.0 {
                "yellow"
            } else {
                "red"
            };
            let health = serde_json::json!({
                "score": score,
                "band": band,
                "source": "computed",
                "confidence": 0.3,
                "trend": {
                    "direction": direction,
                    "rationale": rationale,
                    "timeframe": "30d",
                    "confidence": 0.3
                },
                "dimensions": {
                    "meetingCadence": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"},
                    "emailEngagement": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"},
                    "stakeholderCoverage": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"},
                    "championHealth": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"},
                    "financialProximity": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"},
                    "signalMomentum": {"score": 0.0, "weight": 0.0, "evidence": [], "trend": "stable"}
                },
                "recommendedActions": []
            });
            if let Some(obj) = value.as_object_mut() {
                obj.insert("health".to_string(), health);
            }
        }
    }

    serde_json::from_value(value).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write intelligence.json atomically to an entity directory.
pub fn write_intelligence_json(dir: &Path, intel: &IntelligenceJson) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    let path = dir.join(INTEL_FILENAME);
    let content =
        serde_json::to_string_pretty(intel).map_err(|e| format!("Serialize error: {}", e))?;
    atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

/// Check if intelligence.json exists in an entity directory.
pub fn intelligence_exists(dir: &Path) -> bool {
    dir.join(INTEL_FILENAME).exists()
}

// =============================================================================
// Field Update (User Edits)
// =============================================================================

/// Navigate a serde_json::Value by a dotted/indexed path and set the value.
///
/// Supports paths like:
/// - `"executiveAssessment"` → root field
/// - `"stakeholderInsights[0].name"` → array index + field
/// - `"currentState.working[0]"` → nested field + array index
/// - `"risks[2].text"` → array index + field
fn set_json_path(
    root: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let segments = parse_path_segments(path)?;
    let mut current = root;

    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        match seg {
            PathSegment::Field(name) => {
                if is_last {
                    current[name.as_str()] = value;
                    return Ok(());
                }
                current = current
                    .get_mut(name.as_str())
                    .ok_or_else(|| format!("Field '{}' not found at segment '{}'", path, name))?;
            }
            PathSegment::Index(name, idx) => {
                let arr = current
                    .get_mut(name.as_str())
                    .ok_or_else(|| format!("Field '{}' not found", name))?;
                let arr = arr
                    .as_array_mut()
                    .ok_or_else(|| format!("Field '{}' is not an array", name))?;
                if *idx >= arr.len() {
                    return Err(format!(
                        "Index {} out of bounds for '{}' (len {})",
                        idx,
                        name,
                        arr.len()
                    ));
                }
                if is_last {
                    arr[*idx] = value;
                    return Ok(());
                }
                current = &mut arr[*idx];
            }
        }
    }
    Err(format!("Empty path: '{}'", path))
}

enum PathSegment {
    Field(String),
    Index(String, usize),
}

/// Parse "stakeholderInsights[0].name" into [Index("stakeholderInsights", 0), Field("name")]
fn parse_path_segments(path: &str) -> Result<Vec<PathSegment>, String> {
    let mut segments = Vec::new();
    for part in path.split('.') {
        if let Some(bracket_pos) = part.find('[') {
            let name = &part[..bracket_pos];
            let rest = &part[bracket_pos + 1..];
            let idx_str = rest.trim_end_matches(']');
            let idx: usize = idx_str
                .parse()
                .map_err(|_| format!("Invalid index in path segment: '{}'", part))?;
            segments.push(PathSegment::Index(name.to_string(), idx));
        } else {
            segments.push(PathSegment::Field(part.to_string()));
        }
    }
    Ok(segments)
}

/// Apply a field update to an intelligence.json on disk.
///
/// Reads the file, applies the update via JSON path navigation,
/// records a UserEdit entry, validates by re-parsing, and writes back.
pub fn apply_intelligence_field_update(
    dir: &Path,
    field_path: &str,
    value: &str,
) -> Result<IntelligenceJson, String> {
    let intel_path = dir.join(INTEL_FILENAME);
    let content = std::fs::read_to_string(&intel_path)
        .map_err(|e| format!("Failed to read {}: {}", intel_path.display(), e))?;

    let mut json_val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", intel_path.display(), e))?;

    // Apply the update
    let new_value: serde_json::Value = serde_json::from_str(value)
        .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
    set_json_path(&mut json_val, field_path, new_value)?;

    // I576: Tag edited items with user_correction source attribution.
    // If the edited path points to an item in a Vec (e.g., "risks[0]"),
    // set the itemSource on that item.
    if let Ok(segments) = parse_path_segments(field_path) {
        if let Some(PathSegment::Index(arr_name, idx)) = segments.last() {
            if let Some(arr) = json_val
                .get_mut(arr_name.as_str())
                .and_then(|v| v.as_array_mut())
            {
                if let Some(item) = arr.get_mut(*idx) {
                    if item.is_object() {
                        item["itemSource"] = serde_json::json!({
                            "source": "user_correction",
                            "confidence": 1.0,
                            "sourcedAt": Utc::now().to_rfc3339(),
                            "reference": "user edit"
                        });
                    }
                }
            }
        }
    }

    // Record user edit (dedup: replace existing edit for same path)
    let edits = json_val.get_mut("userEdits").and_then(|v| v.as_array_mut());
    let edit_entry = serde_json::json!({
        "fieldPath": field_path,
        "editedAt": Utc::now().to_rfc3339(),
    });
    if let Some(arr) = edits {
        arr.retain(|e| e.get("fieldPath").and_then(|v| v.as_str()) != Some(field_path));
        arr.push(edit_entry);
    } else {
        json_val["userEdits"] = serde_json::json!([edit_entry]);
    }

    // Validate by re-parsing into typed struct
    let intel: IntelligenceJson =
        serde_json::from_value(json_val).map_err(|e| format!("Updated JSON is invalid: {}", e))?;

    // Write back
    write_intelligence_json(dir, &intel)?;

    Ok(intel)
}

/// Apply a field update to an in-memory IntelligenceJson (DB-sourced).
///
/// Same logic as `apply_intelligence_field_update` but operates on a struct
/// instead of reading from disk. Used when the DB is the source of truth
/// (post-I513, Glean enrichment writes directly to DB).
pub fn apply_intelligence_field_update_in_memory(
    existing: IntelligenceJson,
    field_path: &str,
    value: &str,
) -> Result<IntelligenceJson, String> {
    let mut json_val = serde_json::to_value(&existing)
        .map_err(|e| format!("Failed to serialize existing intelligence: {}", e))?;

    // Apply the update
    let new_value: serde_json::Value = serde_json::from_str(value)
        .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
    set_json_path(&mut json_val, field_path, new_value)?;

    // I576: Tag edited items with user_correction source attribution.
    if let Ok(segments) = parse_path_segments(field_path) {
        if let Some(PathSegment::Index(arr_name, idx)) = segments.last() {
            if let Some(arr) = json_val
                .get_mut(arr_name.as_str())
                .and_then(|v| v.as_array_mut())
            {
                if let Some(item) = arr.get_mut(*idx) {
                    if item.is_object() {
                        item["itemSource"] = serde_json::json!({
                            "source": "user_correction",
                            "confidence": 1.0,
                            "sourcedAt": Utc::now().to_rfc3339(),
                            "reference": "user edit"
                        });
                    }
                }
            }
        }
    }

    // Record user edit (dedup: replace existing edit for same path)
    let edits = json_val.get_mut("userEdits").and_then(|v| v.as_array_mut());
    let edit_entry = serde_json::json!({
        "fieldPath": field_path,
        "editedAt": Utc::now().to_rfc3339(),
    });
    if let Some(arr) = edits {
        arr.retain(|e| e.get("fieldPath").and_then(|v| v.as_str()) != Some(field_path));
        arr.push(edit_entry);
    } else {
        json_val["userEdits"] = serde_json::json!([edit_entry]);
    }

    // Validate by re-parsing into typed struct
    let intel: IntelligenceJson =
        serde_json::from_value(json_val).map_err(|e| format!("Updated JSON is invalid: {}", e))?;

    Ok(intel)
}

/// Replace the stakeholderInsights array and record as user-edited.
pub fn apply_stakeholders_update(
    dir: &Path,
    stakeholders: Vec<StakeholderInsight>,
) -> Result<IntelligenceJson, String> {
    let mut intel = read_intelligence_json(dir)?;
    intel.stakeholder_insights = stakeholders;

    // Record user edit
    let now = Utc::now().to_rfc3339();
    intel
        .user_edits
        .retain(|e| e.field_path != "stakeholderInsights");
    intel.user_edits.push(UserEdit {
        field_path: "stakeholderInsights".to_string(),
        edited_at: now,
    });

    write_intelligence_json(dir, &intel)?;
    Ok(intel)
}

/// Replace the stakeholderInsights array in-memory (DB-first path).
///
/// Same logic as `apply_stakeholders_update` but operates on an existing
/// `IntelligenceJson` instead of reading from disk, allowing the caller
/// to prefer DB-sourced intelligence over disk.
pub fn apply_stakeholders_update_in_memory(
    existing: IntelligenceJson,
    stakeholders: Vec<StakeholderInsight>,
) -> Result<IntelligenceJson, String> {
    let mut intel = existing;
    intel.stakeholder_insights = stakeholders;

    // Record as user-edited
    let edit_entry = UserEdit {
        field_path: "stakeholderInsights".to_string(),
        edited_at: Utc::now().to_rfc3339(),
    };
    intel
        .user_edits
        .retain(|e| e.field_path != "stakeholderInsights");
    intel.user_edits.push(edit_entry);
    Ok(intel)
}

/// Resolve entity directory from workspace, entity_type, and DB records.
pub fn resolve_entity_dir(
    workspace: &Path,
    entity_type: &str,
    entity_name: &str,
    account: Option<&DbAccount>,
) -> Result<std::path::PathBuf, String> {
    match entity_type {
        "account" => {
            if let Some(acct) = account {
                Ok(crate::accounts::resolve_account_dir(workspace, acct))
            } else {
                Ok(crate::accounts::account_dir(workspace, entity_name))
            }
        }
        "project" => Ok(crate::projects::project_dir(workspace, entity_name)),
        "person" => Ok(crate::people::person_dir(workspace, entity_name)),
        _ => Err(format!("Unsupported entity type: {}", entity_type)),
    }
}

/// Preserve user-edited fields from an existing intelligence after AI enrichment.
///
/// For each field in `user_edits`, copies the value from `existing` into `new_intel`,
/// then carries forward the `user_edits` list.
pub fn preserve_user_edits(new_intel: &mut IntelligenceJson, existing: &IntelligenceJson) {
    if existing.user_edits.is_empty() {
        return;
    }

    // Serialize both to serde_json::Value for field-level operations
    let existing_val: serde_json::Value = match serde_json::to_value(existing) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut new_val: serde_json::Value = match serde_json::to_value(&*new_intel) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut updated_edits = Vec::new();

    for edit in &existing.user_edits {
        // I633: Array-indexed paths like "stakeholderInsights[0].role" break when
        // enrichment reorders the array. Match by identity ("name") instead of index.
        if let Some(resolved) =
            resolve_array_path_by_identity(&existing_val, &new_val, &edit.field_path)
        {
            // Read the user-edited value from existing at the original path
            if let Some(val) = get_json_path(&existing_val, &edit.field_path) {
                if set_json_path(&mut new_val, &resolved, val.clone()).is_ok() {
                    // Update the stored path to the new index
                    updated_edits.push(crate::intelligence::io::UserEdit {
                        field_path: resolved,
                        edited_at: edit.edited_at.clone(),
                    });
                    continue;
                }
            }
        }

        // Fallback: direct path restoration (non-array or identity match failed)
        if let Some(val) = get_json_path(&existing_val, &edit.field_path) {
            let _ = set_json_path(&mut new_val, &edit.field_path, val.clone());
        }
        updated_edits.push(edit.clone());
    }

    // Re-parse and carry forward user_edits (with updated paths)
    if let Ok(mut restored) = serde_json::from_value::<IntelligenceJson>(new_val) {
        restored.user_edits = updated_edits;
        *new_intel = restored;
    }
}

/// For array-indexed paths, resolve the correct index in the new array by matching
/// the element's identity field ("name" for stakeholders, "title" for risks/wins).
///
/// Example: "stakeholderInsights[0].role" where existing[0].name = "Alice"
/// → finds Alice in new array at index 2 → returns "stakeholderInsights[2].role"
fn resolve_array_path_by_identity(
    existing: &serde_json::Value,
    new: &serde_json::Value,
    path: &str,
) -> Option<String> {
    let segments = parse_path_segments(path).ok()?;

    // Find the first Index segment
    let (seg_idx, arr_name, old_index) = segments.iter().enumerate().find_map(|(i, seg)| {
        if let PathSegment::Index(name, idx) = seg {
            Some((i, name.clone(), *idx))
        } else {
            None
        }
    })?;

    // Identity key lookup: "name" for stakeholderInsights, "title" for risks/wins
    let identity_key = match arr_name.as_str() {
        "stakeholderInsights" => "name",
        "risks" => "title",
        "recentWins" => "title",
        _ => return None, // Unknown array type — fall back to index
    };

    // Get the identity value from the old array element
    let old_arr = existing.get(&arr_name)?.as_array()?;
    let old_element = old_arr.get(old_index)?;
    let identity_val = old_element.get(identity_key)?;

    // Find the matching element in the new array
    let new_arr = new.get(&arr_name)?.as_array()?;
    let new_index = new_arr
        .iter()
        .position(|elem| elem.get(identity_key) == Some(identity_val))?;

    if new_index == old_index {
        return None; // Same index — no remapping needed, use direct path
    }

    // Rebuild the path with the new index
    let mut new_segments = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        if i == seg_idx {
            new_segments.push(format!("{}[{}]", arr_name, new_index));
        } else {
            match seg {
                PathSegment::Field(name) => new_segments.push(name.clone()),
                PathSegment::Index(name, idx) => new_segments.push(format!("{}[{}]", name, idx)),
            }
        }
    }
    Some(new_segments.join("."))
}

/// Read a value at a JSON path (for preserve_user_edits).
fn get_json_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let segments = parse_path_segments(path).ok()?;
    let mut current = root;

    for seg in &segments {
        match seg {
            PathSegment::Field(name) => {
                current = current.get(name.as_str())?;
            }
            PathSegment::Index(name, idx) => {
                let arr = current.get(name.as_str())?.as_array()?;
                current = arr.get(*idx)?;
            }
        }
    }
    Some(current)
}

// =============================================================================
// Migration: CompanyOverview → intelligence.json
// =============================================================================

/// Migrate legacy CompanyOverview from dashboard.json into intelligence.json.
///
/// Non-destructive: creates intelligence.json if it doesn't exist and
/// dashboard.json has a company_overview. Leaves dashboard.json untouched.
/// Returns the created IntelligenceJson, or None if no migration needed.
pub fn migrate_company_overview_to_intelligence(
    workspace: &Path,
    account: &DbAccount,
    overview: &CompanyOverview,
) -> Option<IntelligenceJson> {
    let dir = crate::accounts::resolve_account_dir(workspace, account);

    // Don't overwrite existing intelligence
    if intelligence_exists(&dir) {
        return None;
    }

    // Only migrate if there's actual content
    if overview.description.is_none()
        && overview.industry.is_none()
        && overview.size.is_none()
        && overview.headquarters.is_none()
    {
        return None;
    }

    let intel = IntelligenceJson {
        version: 1,
        entity_id: account.id.clone(),
        entity_type: "account".to_string(),
        enriched_at: overview
            .enriched_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        company_context: Some(CompanyContext {
            description: overview.description.clone(),
            industry: overview.industry.clone(),
            size: overview.size.clone(),
            headquarters: overview.headquarters.clone(),
            additional_context: None,
        }),
        ..Default::default()
    };

    match write_intelligence_json(&dir, &intel) {
        Ok(()) => {
            log::info!(
                "Migrated CompanyOverview → intelligence.json for '{}'",
                account.name
            );
            Some(intel)
        }
        Err(e) => {
            log::warn!(
                "Failed to migrate intelligence for '{}': {}",
                account.name,
                e
            );
            None
        }
    }
}

// =============================================================================
// DB Cache Operations
// =============================================================================

fn synthesize_health_from_legacy(
    score: Option<f64>,
    trend_json: Option<&str>,
) -> Option<AccountHealth> {
    let score = score?;
    let trend = trend_json
        .and_then(|j| serde_json::from_str::<HealthTrend>(j).ok())
        .unwrap_or_default();
    let band = if score >= 70.0 {
        "green"
    } else if score >= 40.0 {
        "yellow"
    } else {
        "red"
    };
    Some(AccountHealth {
        score,
        band: band.to_string(),
        source: HealthSource::Computed,
        confidence: 0.3,
        sufficient_data: false, // DOS-84: DB-restored scores lack dimension context
        trend,
        dimensions: RelationshipDimensions::default(),
        divergence: None,
        narrative: None,
        recommended_actions: Vec::new(),
    })
}

impl ActionDb {
    /// Insert or update the entity_assessment cache row.
    /// Health/coherence data goes to entity_quality.
    pub fn upsert_entity_intelligence(
        &self,
        intel: &IntelligenceJson,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn_ref();

        // 1. entity_assessment — all assessment/prose columns
        let dimensions_json = serde_json::to_string(&intel.dimensions_blob()).ok();
        conn.execute(
            "INSERT INTO entity_assessment (
                entity_id, entity_type, enriched_at, source_file_count,
                executive_assessment, risks_json, recent_wins_json,
                current_state_json, stakeholder_insights_json,
                next_meeting_readiness_json, company_context_json,
                value_delivered, success_metrics, open_commitments,
                relationship_depth, health_json, org_health_json, consistency_status,
                consistency_findings_json, consistency_checked_at,
                portfolio_json, network_json, user_edits_json, source_manifest_json,
                dimensions_json, success_plan_signals_json, pull_quote
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
            ON CONFLICT(entity_id) DO UPDATE SET
                entity_type = excluded.entity_type,
                enriched_at = excluded.enriched_at,
                source_file_count = excluded.source_file_count,
                executive_assessment = excluded.executive_assessment,
                risks_json = excluded.risks_json,
                recent_wins_json = excluded.recent_wins_json,
                current_state_json = excluded.current_state_json,
                stakeholder_insights_json = excluded.stakeholder_insights_json,
                next_meeting_readiness_json = excluded.next_meeting_readiness_json,
                company_context_json = excluded.company_context_json,
                value_delivered = excluded.value_delivered,
                success_metrics = excluded.success_metrics,
                open_commitments = excluded.open_commitments,
                relationship_depth = excluded.relationship_depth,
                health_json = excluded.health_json,
                org_health_json = excluded.org_health_json,
                consistency_status = excluded.consistency_status,
                consistency_findings_json = excluded.consistency_findings_json,
                consistency_checked_at = excluded.consistency_checked_at,
                portfolio_json = excluded.portfolio_json,
                network_json = excluded.network_json,
                user_edits_json = excluded.user_edits_json,
                source_manifest_json = excluded.source_manifest_json,
                dimensions_json = excluded.dimensions_json,
                success_plan_signals_json = excluded.success_plan_signals_json,
                pull_quote = excluded.pull_quote",
            rusqlite::params![
                intel.entity_id,
                intel.entity_type,
                intel.enriched_at,
                intel.source_file_count,
                intel.executive_assessment,
                serde_json::to_string(&intel.risks).ok(),
                serde_json::to_string(&intel.recent_wins).ok(),
                serde_json::to_string(&intel.current_state).ok(),
                serde_json::to_string(&intel.stakeholder_insights).ok(),
                serde_json::to_string(&intel.next_meeting_readiness).ok(),
                serde_json::to_string(&intel.company_context).ok(),
                serde_json::to_string(&intel.value_delivered).ok(),
                serde_json::to_string(&intel.success_metrics).ok(),
                serde_json::to_string(&intel.open_commitments).ok(),
                serde_json::to_string(&intel.relationship_depth).ok(),
                intel.health.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                intel
                    .org_health
                    .as_ref()
                    .and_then(|v| serde_json::to_string(v).ok()),
                serde_json::to_string(&intel.consistency_status).ok(),
                serde_json::to_string(&intel.consistency_findings).ok(),
                intel.consistency_checked_at,
                serde_json::to_string(&intel.portfolio).ok(),
                serde_json::to_string(&intel.network).ok(),
                serde_json::to_string(&intel.user_edits).ok(),
                serde_json::to_string(&intel.source_manifest).ok(),
                dimensions_json,
                intel.success_plan_signals.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                intel.pull_quote,
            ],
        )?;

        // 1b. Clear stale feedback — feedback is keyed by field path (e.g.
        // "riskFactors[0]"), which is positional. Re-enrichment produces new content
        // at the same positions, so old votes don't apply to new analysis.
        // Preserve field conflict dismissals (account_field_conflict:*) — those are
        // user decisions about data source conflicts, not positional analysis votes.
        conn.execute(
            "DELETE FROM intelligence_feedback WHERE entity_id = ?1 AND entity_type = ?2 \
             AND field NOT LIKE 'account_field_conflict:%'",
            rusqlite::params![intel.entity_id, intel.entity_type],
        )?;

        // 2. entity_quality — keep scalar health mirrors for transitional compatibility.
        if let Some(health) = intel.health.as_ref() {
            conn.execute(
                "INSERT INTO entity_quality (entity_id, entity_type, health_score, health_trend)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(entity_id) DO UPDATE SET
                     health_score = excluded.health_score,
                     health_trend = excluded.health_trend",
                rusqlite::params![
                    intel.entity_id,
                    intel.entity_type,
                    health.score,
                    serde_json::to_string(&health.trend).ok(),
                ],
            )?;
        }

        Ok(())
    }

    /// Get cached entity intelligence (from entity_assessment + entity_quality).
    pub fn get_entity_intelligence(
        &self,
        entity_id: &str,
    ) -> Result<Option<IntelligenceJson>, rusqlite::Error> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT ea.entity_id, ea.entity_type, ea.enriched_at, ea.source_file_count,
                    ea.executive_assessment, ea.risks_json, ea.recent_wins_json,
                    ea.current_state_json, ea.stakeholder_insights_json,
                    ea.next_meeting_readiness_json, ea.company_context_json,
                    ea.health_json, ea.org_health_json, eq.health_score, eq.health_trend,
                    ea.value_delivered, ea.success_metrics, ea.open_commitments,
                    ea.relationship_depth, ea.consistency_status, ea.consistency_findings_json,
                    ea.consistency_checked_at, ea.portfolio_json, ea.network_json,
                    ea.user_edits_json, ea.source_manifest_json, ea.dimensions_json,
                    ea.pull_quote
             FROM entity_assessment ea
             LEFT JOIN entity_quality eq ON eq.entity_id = ea.entity_id
             WHERE ea.entity_id = ?1",
        )?;

        let result = stmt.query_row(rusqlite::params![entity_id], |row| {
            let risks_json: Option<String> = row.get(5)?;
            let wins_json: Option<String> = row.get(6)?;
            let state_json: Option<String> = row.get(7)?;
            let stakeholder_json: Option<String> = row.get(8)?;
            let readiness_json: Option<String> = row.get(9)?;
            let company_json: Option<String> = row.get(10)?;
            let health_json: Option<String> = row.get(11)?;
            let org_health_json: Option<String> = row.get(12)?;
            let health_score: Option<f64> = row.get(13)?;
            let health_trend_json: Option<String> = row.get(14)?;
            let value_delivered_json: Option<String> = row.get(15)?;
            let success_metrics_json: Option<String> = row.get(16)?;
            let open_commitments_json: Option<String> = row.get(17)?;
            let relationship_depth_json: Option<String> = row.get(18)?;
            let consistency_status_json: Option<String> = row.get(19)?;
            let consistency_findings_json: Option<String> = row.get(20)?;
            let portfolio_json: Option<String> = row.get(22)?;
            let network_json: Option<String> = row.get(23)?;
            let user_edits_json: Option<String> = row.get(24)?;
            let source_manifest_json: Option<String> = row.get(25)?;
            let dimensions_json_raw: Option<String> = row.get(26)?;
            let pull_quote: Option<String> = row.get(27)?;

            let health = health_json
                .as_deref()
                .and_then(|j| serde_json::from_str::<AccountHealth>(j).ok())
                .or_else(|| {
                    synthesize_health_from_legacy(health_score, health_trend_json.as_deref())
                });

            let mut intel = IntelligenceJson {
                version: 1,
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                enriched_at: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                source_file_count: row.get::<_, Option<usize>>(3)?.unwrap_or(0),
                source_manifest: source_manifest_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                executive_assessment: row.get(4)?,
                risks: risks_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                recent_wins: wins_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                current_state: state_json.and_then(|j| serde_json::from_str(&j).ok()),
                stakeholder_insights: stakeholder_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                value_delivered: value_delivered_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                next_meeting_readiness: readiness_json.and_then(|j| serde_json::from_str(&j).ok()),
                company_context: company_json.and_then(|j| serde_json::from_str(&j).ok()),
                portfolio: portfolio_json.and_then(|j| serde_json::from_str(&j).ok()),
                network: network_json.and_then(|j| serde_json::from_str(&j).ok()),
                user_edits: user_edits_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                health,
                org_health: org_health_json.and_then(|j| serde_json::from_str(&j).ok()),
                success_metrics: success_metrics_json.and_then(|j| serde_json::from_str(&j).ok()),
                open_commitments: open_commitments_json.and_then(|j| serde_json::from_str(&j).ok()),
                relationship_depth: relationship_depth_json
                    .and_then(|j| serde_json::from_str(&j).ok()),
                consistency_status: consistency_status_json
                    .and_then(|j| serde_json::from_str(&j).ok()),
                consistency_findings: consistency_findings_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                consistency_checked_at: row.get(21)?,
                pull_quote,
                ..Default::default()
            };
            // Unpack I508a dimensions blob if present
            if let Some(ref dj) = dimensions_json_raw {
                if let Ok(blob) = serde_json::from_str::<DimensionsBlob>(dj) {
                    intel.apply_dimensions_blob(&blob);
                }
            }
            Ok(intel)
        });

        match result {
            Ok(intel) => Ok(Some(intel)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete cached entity intelligence.
    pub fn delete_entity_intelligence(&self, entity_id: &str) -> Result<(), rusqlite::Error> {
        self.conn_ref().execute(
            "DELETE FROM entity_assessment WHERE entity_id = ?1",
            rusqlite::params![entity_id],
        )?;
        Ok(())
    }
}
// =============================================================================
// Markdown Generation (I134 — three-file dashboard.md)
// =============================================================================

/// Format intelligence sections as markdown for inclusion in dashboard.md.
///
/// Used by both `write_account_markdown()` and `write_project_markdown()` to
/// inject synthesized intelligence into the generated artifact. Returns empty
/// string if there's nothing meaningful to render.
pub fn format_intelligence_markdown(intel: &IntelligenceJson) -> String {
    let mut md = String::new();

    // Executive Assessment — the most important section
    if let Some(ref assessment) = intel.executive_assessment {
        if !assessment.is_empty() {
            md.push_str("## Executive Assessment\n\n");
            md.push_str(assessment);
            md.push_str("\n\n");
            if !intel.enriched_at.is_empty() {
                md.push_str(&format!(
                    "_Last enriched: {}_\n\n",
                    intel
                        .enriched_at
                        .split('T')
                        .next()
                        .unwrap_or(&intel.enriched_at)
                ));
            }
        }
    }

    // Risks
    if !intel.risks.is_empty() {
        md.push_str("## Risks\n\n");
        for r in &intel.risks {
            md.push_str(&format!("- **{}** {}", r.urgency, r.text));
            if let Some(ref source) = r.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Recent Wins
    if !intel.recent_wins.is_empty() {
        md.push_str("## Recent Wins\n\n");
        for w in &intel.recent_wins {
            md.push_str(&format!("- {}", w.text));
            if let Some(ref impact) = w.impact {
                md.push_str(&format!(" \u{2014} {}", impact));
            }
            if let Some(ref source) = w.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Current State
    if let Some(ref state) = intel.current_state {
        let has_content = !state.working.is_empty()
            || !state.not_working.is_empty()
            || !state.unknowns.is_empty();
        if has_content {
            md.push_str("## Current State\n\n");
            if !state.working.is_empty() {
                md.push_str("### What's Working\n\n");
                for item in &state.working {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
            if !state.not_working.is_empty() {
                md.push_str("### What's Not Working\n\n");
                for item in &state.not_working {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
            if !state.unknowns.is_empty() {
                md.push_str("### Unknowns\n\n");
                for item in &state.unknowns {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
        }
    }

    // Next Meeting Readiness
    if let Some(ref readiness) = intel.next_meeting_readiness {
        if !readiness.prep_items.is_empty() {
            md.push_str("## Next Meeting Readiness\n\n");
            if let Some(ref title) = readiness.meeting_title {
                md.push_str(&format!("**{}**", title));
                if let Some(ref date) = readiness.meeting_date {
                    md.push_str(&format!(" on {}", date));
                }
                md.push_str("\n\n");
            }
            for item in &readiness.prep_items {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
    }

    // Stakeholder Insights
    if !intel.stakeholder_insights.is_empty() {
        md.push_str("## Stakeholder Insights\n\n");
        for s in &intel.stakeholder_insights {
            md.push_str(&format!("### {}", s.name));
            if let Some(ref role) = s.role {
                md.push_str(&format!(" \u{2014} {}", role));
            }
            md.push('\n');
            if let Some(ref assessment) = s.assessment {
                md.push_str(assessment);
            }
            if let Some(ref engagement) = s.engagement {
                md.push_str(&format!(" Engagement: {}.", engagement));
            }
            if let Some(ref source) = s.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push_str("\n\n");
        }
    }

    // Value Delivered
    if !intel.value_delivered.is_empty() {
        md.push_str("## Value Delivered\n\n");
        for v in &intel.value_delivered {
            md.push_str("- ");
            if let Some(ref date) = v.date {
                md.push_str(&format!("**{}** ", date));
            }
            md.push_str(&v.statement);
            if let Some(ref impact) = v.impact {
                md.push_str(&format!(" \u{2014} {}", impact));
            }
            if let Some(ref source) = v.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Company / Project Context (from web search or overview)
    if let Some(ref ctx) = intel.company_context {
        let has_content = ctx.description.is_some()
            || ctx.industry.is_some()
            || ctx.size.is_some()
            || ctx.headquarters.is_some()
            || ctx.additional_context.is_some();
        if has_content {
            md.push_str("## Company Context\n\n");
            if let Some(ref desc) = ctx.description {
                md.push_str(desc);
                md.push_str("\n\n");
            }
            if let Some(ref industry) = ctx.industry {
                md.push_str(&format!("**Industry:** {}  \n", industry));
            }
            if let Some(ref size) = ctx.size {
                md.push_str(&format!("**Size:** {}  \n", size));
            }
            if let Some(ref hq) = ctx.headquarters {
                md.push_str(&format!("**Headquarters:** {}  \n", hq));
            }
            if let Some(ref additional) = ctx.additional_context {
                md.push_str(&format!("\n{}\n", additional));
            }
            md.push('\n');
        }
    }

    md
}

// =============================================================================
// Content Indexing (shared logic for accounts + projects)
// =============================================================================

/// Files to skip during content indexing (managed by the app).
pub(crate) const CONTENT_SKIP_FILES: &[&str] = &[
    "dashboard.json",
    "dashboard.md",
    "intelligence.json",
    ".DS_Store",
];

/// Recursively collect content files from an entity directory.
///
/// Skips hidden files/dirs, underscore-prefixed dirs, managed files,
/// and child entity boundaries (subdirs containing dashboard.json).
/// Used by both account and project content indexing.
pub(crate) fn collect_content_files(
    dir: &std::path::Path,
    _entity_root: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip hidden and underscore-prefixed entries at every level
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }

        if path.is_dir() {
            // Stop at child entity boundaries — subdirs with their own dashboard.json
            // are separate entities and indexed independently
            if path.join("dashboard.json").exists() {
                continue;
            }
            collect_content_files(&path, _entity_root, out);
        } else {
            if CONTENT_SKIP_FILES.contains(&name.as_str()) {
                continue;
            }
            out.push(path);
        }
    }
}

// =============================================================================
// Content Classification + Mechanical Summary (I139)
// =============================================================================

/// Classify content type from filename and format. Returns `(content_type, priority)`.
///
/// Pure mechanical — no AI cost, deterministic. First pattern match wins.
/// Priority scale: 5 (general) to 10 (dashboard).
pub(crate) fn classify_content(filename: &str, format: &str) -> (&'static str, i32) {
    let lower = filename.to_lowercase();

    if lower.contains("dashboard") {
        return ("dashboard", 10);
    }
    if lower.contains("transcript")
        || lower.contains("recording")
        || lower.contains("call-notes")
        || lower.contains("call_notes")
    {
        return ("transcript", 9);
    }
    if lower.contains("stakeholder")
        || lower.contains("org-chart")
        || lower.contains("relationship")
    {
        return ("stakeholder-map", 9);
    }
    if lower.contains("success-plan")
        || lower.contains("success_plan")
        || lower.contains("strategy")
    {
        return ("success-plan", 8);
    }
    if lower.contains("qbr")
        || (lower.contains("quarterly") && lower.contains("review"))
        || lower.contains("business-review")
    {
        return ("qbr", 8);
    }
    if lower.contains("contract")
        || lower.contains("agreement")
        || lower.contains("sow")
        || lower.contains("msa")
    {
        return ("contract", 7);
    }
    if lower.contains("notes") || lower.contains("memo") || lower.contains("minutes") {
        return ("notes", 7);
    }
    if format == "Pptx" {
        return ("presentation", 6);
    }
    if format == "Xlsx" {
        return ("spreadsheet", 6);
    }

    ("general", 5)
}

/// Extract a semantic content date from a filename as an RFC3339 string.
///
/// Many workspace files follow the pattern `YYYY-MM-DD-description.ext`. The embedded date
/// is the *content* date (when the meeting/event happened), which is more useful for filtering
/// than the filesystem mtime (which reflects when the file was copied/synced).
/// Returns `YYYY-MM-DDT00:00:00+00:00` if a date prefix is found, else `modified_at`.
pub(crate) fn content_date_rfc3339(filename: &str, modified_at: &str) -> String {
    if filename.len() >= 10 {
        let prefix = &filename[..10];
        if prefix.as_bytes()[4] == b'-'
            && prefix.as_bytes()[7] == b'-'
            && prefix[..4].chars().all(|c| c.is_ascii_digit())
            && prefix[5..7].chars().all(|c| c.is_ascii_digit())
            && prefix[8..10].chars().all(|c| c.is_ascii_digit())
        {
            return format!("{}T00:00:00+00:00", prefix);
        }
    }
    modified_at.to_string()
}

/// Apply a recency boost: files from the last 30 days get +1 priority (capped at 10).
///
/// Uses the filename-embedded date when available (more reliable than filesystem mtime
/// for files that have been copied/synced).
pub(crate) fn apply_recency_boost(base_priority: i32, filename: &str, modified_at: &str) -> i32 {
    let cutoff_30d = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    let effective_date = content_date_rfc3339(filename, modified_at);
    if effective_date >= cutoff_30d {
        (base_priority + 1).min(10)
    } else {
        base_priority
    }
}

/// Generate a mechanical summary from extracted text.
///
/// Extracts markdown headings as table of contents + first non-heading paragraph
/// as context. Target: ~`max_chars` chars per file. Zero AI cost.
pub(crate) fn mechanical_summary(text: &str, max_chars: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut headings: Vec<&str> = Vec::new();
    let mut first_paragraph: Option<&str> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            // Strip the leading '#' characters and whitespace for cleaner output
            let heading_text = trimmed.trim_start_matches('#').trim();
            if !heading_text.is_empty() {
                headings.push(heading_text);
            }
        } else if first_paragraph.is_none() {
            // First non-empty, non-heading line is the context paragraph
            first_paragraph = Some(trimmed);
        }
    }

    let mut result = String::new();

    if let Some(para) = first_paragraph {
        result.push_str(para);
    }

    if !headings.is_empty() {
        if !result.is_empty() {
            result.push_str("\n\nSections: ");
        } else {
            result.push_str("Sections: ");
        }
        result.push_str(&headings.join(", "));
    }

    if result.is_empty() {
        // Fallback: take first max_chars of raw text
        let truncated = &text[..text.len().min(max_chars)];
        return truncated.to_string();
    }

    if result.len() > max_chars {
        // Truncate to max_chars at a word boundary if possible
        let truncated = &result[..max_chars];
        if let Some(last_space) = truncated.rfind(' ') {
            return result[..last_space].to_string();
        }
        return truncated.to_string();
    }

    result
}

/// Extract text from a file and produce a mechanical summary.
/// Returns `(extracted_at, summary)`. Both are `None` if extraction fails.
pub(crate) fn extract_and_summarize(path: &std::path::Path) -> (Option<String>, Option<String>) {
    match crate::processor::extract::extract_text(path) {
        Ok(text) if !text.is_empty() => {
            let summary = mechanical_summary(&text, 500);
            let extracted_at = Utc::now().to_rfc3339();
            (
                Some(extracted_at),
                if summary.is_empty() {
                    None
                } else {
                    Some(summary)
                },
            )
        }
        _ => (None, None),
    }
}

/// Sync the content index for any entity. Compares filesystem against DB,
/// adds new files, updates changed files, removes deleted files.
///
/// Entity-generic: works for accounts, projects, and future entity types.
/// Returns `(added, updated, removed)` counts.
pub(crate) fn sync_content_index_for_entity(
    entity_dir: &std::path::Path,
    entity_id: &str,
    entity_type: &str,
    workspace: &std::path::Path,
    db: &ActionDb,
) -> Result<(usize, usize, usize), String> {
    use std::collections::HashMap;

    if !entity_dir.exists() {
        return Ok((0, 0, 0));
    }

    let now = Utc::now().to_rfc3339();
    let mut added = 0usize;
    let mut updated = 0usize;
    let mut removed = 0usize;

    // Build a HashMap of existing DB records for this entity (O(1) lookup)
    let existing = db
        .get_entity_files(entity_id)
        .map_err(|e| format!("DB error: {}", e))?;
    let mut db_map: HashMap<String, crate::db::DbContentFile> =
        existing.into_iter().map(|f| (f.id.clone(), f)).collect();

    // Scan the filesystem recursively
    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
    collect_content_files(entity_dir, entity_dir, &mut file_paths);

    for path in &file_paths {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Detect format via existing extract module
        let format = crate::processor::extract::detect_format(path);
        let format_label = format!("{:?}", format);

        // Get file metadata
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let file_size = metadata.len() as i64;
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|| now.clone());

        // Compute relative path from workspace root
        let relative_path = path
            .strip_prefix(workspace)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| filename.clone());

        // Use path relative to entity dir for stable, collision-free IDs
        let rel_from_entity = path
            .strip_prefix(entity_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| filename.clone());

        let id = crate::util::slugify(&format!("{}/{}", entity_id, rel_from_entity));

        // Classify content type + priority from filename and format
        let (content_type, base_priority) = classify_content(&filename, &format_label);
        let priority = apply_recency_boost(base_priority, &filename, &modified_at);

        // Check if record exists in DB
        if let Some(existing_record) = db_map.remove(&id) {
            // File exists in DB — check if it changed (compare modified_at)
            if existing_record.modified_at != modified_at || existing_record.file_size != file_size
            {
                // File changed — extract summary for new content
                let (extracted_at_val, summary_val) = extract_and_summarize(path);
                let record = crate::db::DbContentFile {
                    id,
                    entity_id: entity_id.to_string(),
                    entity_type: entity_type.to_string(),
                    filename,
                    relative_path,
                    absolute_path: path.to_string_lossy().to_string(),
                    format: format_label,
                    file_size,
                    modified_at,
                    indexed_at: now.clone(),
                    extracted_at: extracted_at_val,
                    summary: summary_val,
                    embeddings_generated_at: None,
                    content_type: content_type.to_string(),
                    priority,
                };
                let _ = db.upsert_content_file(&record);
                updated += 1;
            } else if existing_record.summary.is_none() {
                // Unchanged but never summarized — backfill summary
                let (extracted_at_val, summary_val) = extract_and_summarize(path);
                if summary_val.is_some() {
                    let _ = db.update_content_extraction(
                        &existing_record.id,
                        &extracted_at_val.unwrap_or_else(|| now.clone()),
                        summary_val.as_deref(),
                        Some(content_type),
                        Some(priority),
                    );
                }
            }
            // Unchanged with existing summary — skip
        } else {
            // New file — extract summary + insert
            let (extracted_at_val, summary_val) = extract_and_summarize(path);
            let record = crate::db::DbContentFile {
                id,
                entity_id: entity_id.to_string(),
                entity_type: entity_type.to_string(),
                filename,
                relative_path,
                absolute_path: path.to_string_lossy().to_string(),
                format: format_label,
                file_size,
                modified_at,
                indexed_at: now.clone(),
                extracted_at: extracted_at_val,
                summary: summary_val,
                embeddings_generated_at: None,
                content_type: content_type.to_string(),
                priority,
            };
            let _ = db.upsert_content_file(&record);
            added += 1;
        }
    }

    // Any records left in db_map no longer have matching files — remove them
    for id in db_map.keys() {
        let _ = db.delete_content_file(id);
        removed += 1;
    }

    Ok((added, updated, removed))
}

// =============================================================================
// Keyword extraction from enrichment response (I305)
// =============================================================================

/// Extract keywords from an AI intelligence response.
/// Parses the JSON to find the `keywords` array and returns it as a JSON string.
pub fn extract_keywords_from_response(response: &str) -> Option<String> {
    // Try to find JSON block in the response
    let json_str = if let Some(start) = response.find('{') {
        let depth_track = response[start..]
            .chars()
            .fold((0i32, 0usize), |(depth, end), ch| {
                let new_depth = match ch {
                    '{' => depth + 1,
                    '}' => depth - 1,
                    _ => depth,
                };
                if new_depth == 0 && depth > 0 {
                    (0, end + 1)
                } else {
                    (new_depth, end + ch.len_utf8())
                }
            });
        &response[start..start + depth_track.1]
    } else {
        return None;
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let keywords = parsed.get("keywords")?.as_array()?;

    let kw_strings: Vec<String> = keywords
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty() && s.len() < 100) // Sanity: skip empty or absurdly long entries
        .take(20) // Cap at 20 keywords
        .collect();

    if kw_strings.is_empty() {
        return None;
    }

    serde_json::to_string(&kw_strings).ok()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn sample_intel() -> IntelligenceJson {
        IntelligenceJson {
            version: 1,
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-02-01T10:00:00Z".to_string(),
            source_file_count: 3,
            source_manifest: vec![SourceManifestEntry {
                filename: "qbr-notes.md".to_string(),
                modified_at: "2026-01-30T10:00:00Z".to_string(),
                format: Some("markdown".to_string()),
                content_type: Some("qbr".to_string()),
                selected: true,
                skip_reason: None,
            }],
            executive_assessment: Some(
                "Acme is in a strong position with steady renewal trajectory.".to_string(),
            ),
            risks: vec![IntelRisk {
                text: "Champion leaving in Q2".to_string(),
                source: Some("qbr-notes.md".to_string()),
                urgency: "critical".to_string(),
                item_source: None,
                discrepancy: None,
            }],
            recent_wins: vec![IntelWin {
                text: "Expanded to 3 new teams".to_string(),
                source: Some("capture".to_string()),
                impact: Some("20% seat growth".to_string()),
                item_source: None,
                discrepancy: None,
            }],
            current_state: Some(CurrentState {
                working: vec!["Onboarding flow".to_string()],
                not_working: vec!["Reporting integration".to_string()],
                unknowns: vec!["Budget for next year".to_string()],
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Alice VP".to_string(),
                role: Some("VP Engineering".to_string()),
                assessment: Some("Strong advocate, drives adoption.".to_string()),
                engagement: Some("high".to_string()),
                source: Some("meetings".to_string()),
                person_id: None,
                suggested_person_id: None,
                item_source: None,
                discrepancy: None,
            }],
            value_delivered: vec![ValueItem {
                date: Some("2026-01-15".to_string()),
                statement: "Reduced onboarding time by 40%".to_string(),
                source: Some("qbr-deck.pdf".to_string()),
                impact: Some("$50k savings".to_string()),
                item_source: None,
                discrepancy: None,
            }],
            next_meeting_readiness: Some(MeetingReadiness {
                meeting_title: Some("Weekly sync".to_string()),
                meeting_date: Some("2026-02-05".to_string()),
                prep_items: vec![
                    "Review reporting blockers".to_string(),
                    "Prepare champion transition plan".to_string(),
                ],
            }),
            company_context: Some(CompanyContext {
                description: Some("Enterprise SaaS platform.".to_string()),
                industry: Some("Technology".to_string()),
                size: Some("500-1000".to_string()),
                headquarters: Some("San Francisco, USA".to_string()),
                additional_context: None,
            }),
            portfolio: None,
            network: None,
            user_edits: Vec::new(),
            health: None,
            org_health: None,
            success_metrics: None,
            open_commitments: None,
            relationship_depth: None,
            consistency_status: None,
            consistency_findings: Vec::new(),
            consistency_checked_at: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_intelligence_json_roundtrip() {
        let intel = sample_intel();
        let json_str = serde_json::to_string_pretty(&intel).expect("serialize");
        let parsed: IntelligenceJson = serde_json::from_str(&json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "acme-corp");
        assert_eq!(parsed.entity_type, "account");
        assert_eq!(parsed.risks.len(), 1);
        assert_eq!(parsed.risks[0].urgency, "critical");
        assert_eq!(parsed.recent_wins.len(), 1);
        assert_eq!(parsed.stakeholder_insights.len(), 1);
        assert_eq!(parsed.value_delivered.len(), 1);
        assert!(parsed.next_meeting_readiness.is_some());
        assert!(parsed.company_context.is_some());
        assert_eq!(parsed.source_manifest.len(), 1);
    }

    #[test]
    fn test_intelligence_json_missing_fields() {
        // Minimal JSON — serde should fill defaults for all missing fields
        let json_str = r#"{"entityId": "beta", "entityType": "project"}"#;
        let parsed: IntelligenceJson = serde_json::from_str(json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "beta");
        assert_eq!(parsed.entity_type, "project");
        assert_eq!(parsed.version, 1);
        assert!(parsed.risks.is_empty());
        assert!(parsed.recent_wins.is_empty());
        assert!(parsed.executive_assessment.is_none());
        assert!(parsed.current_state.is_none());
        assert!(parsed.company_context.is_none());
        assert!(parsed.consistency_status.is_none());
        assert!(parsed.consistency_findings.is_empty());
        assert!(parsed.consistency_checked_at.is_none());
    }

    #[test]
    fn test_intelligence_json_consistency_roundtrip() {
        let mut intel = sample_intel();
        intel.consistency_status = Some(ConsistencyStatus::Corrected);
        intel.consistency_findings = vec![ConsistencyFinding {
            code: "ABSENCE_CONTRADICTION".to_string(),
            severity: ConsistencySeverity::High,
            field_path: "executiveAssessment".to_string(),
            claim_text: "Matt has never appeared in a recorded meeting.".to_string(),
            evidence_text: "Matt appears in 2 recorded meetings.".to_string(),
            auto_fixed: true,
        }];
        intel.consistency_checked_at = Some("2026-03-03T18:00:00Z".to_string());

        let json = serde_json::to_string(&intel).expect("serialize");
        let parsed: IntelligenceJson = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            parsed.consistency_status,
            Some(ConsistencyStatus::Corrected)
        );
        assert_eq!(parsed.consistency_findings.len(), 1);
        assert_eq!(parsed.consistency_findings[0].code, "ABSENCE_CONTRADICTION");
        assert!(parsed.consistency_findings[0].auto_fixed);
        assert_eq!(
            parsed.consistency_checked_at.as_deref(),
            Some("2026-03-03T18:00:00Z")
        );
    }

    #[test]
    fn test_write_read_intelligence_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let intel = sample_intel();

        write_intelligence_json(dir.path(), &intel).expect("write");
        assert!(intelligence_exists(dir.path()));

        let read_back = read_intelligence_json(dir.path()).expect("read");
        assert_eq!(read_back.entity_id, "acme-corp");
        assert_eq!(read_back.risks.len(), 1);
        assert_eq!(read_back.source_file_count, 3);
    }

    #[test]
    fn test_migrate_company_overview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();

        // Create account directory
        let acct_dir = workspace.join("Accounts/Acme Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Acme Corp".to_string()),
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        };

        let overview = CompanyOverview {
            description: Some("Cloud platform company.".to_string()),
            industry: Some("SaaS".to_string()),
            size: Some("200-500".to_string()),
            headquarters: Some("NYC".to_string()),
            enriched_at: Some("2026-01-15T10:00:00Z".to_string()),
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_some());

        let intel = result.unwrap();
        assert_eq!(intel.entity_id, "acme-corp");
        assert_eq!(intel.entity_type, "account");
        assert!(intel.company_context.is_some());
        let ctx = intel.company_context.unwrap();
        assert_eq!(ctx.description.as_deref(), Some("Cloud platform company."));
        assert_eq!(ctx.industry.as_deref(), Some("SaaS"));

        // File should exist now
        assert!(intelligence_exists(&acct_dir));

        // Second migration should return None (file already exists)
        let second = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(second.is_none());
    }

    #[test]
    fn test_migrate_empty_overview_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let acct_dir = workspace.join("Accounts/Empty Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "empty-corp".to_string(),
            name: "Empty Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Empty Corp".to_string()),
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        };

        let overview = CompanyOverview {
            description: None,
            industry: None,
            size: None,
            headquarters: None,
            enriched_at: None,
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_none());
    }

    #[test]
    fn test_db_upsert_get_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");

        let fetched = db
            .get_entity_intelligence("acme-corp")
            .expect("get")
            .expect("should exist");

        assert_eq!(fetched.entity_id, "acme-corp");
        assert_eq!(fetched.entity_type, "account");
        assert_eq!(fetched.executive_assessment, intel.executive_assessment);
        assert_eq!(fetched.risks.len(), 1);
        assert_eq!(fetched.risks[0].urgency, "critical");
        assert_eq!(fetched.recent_wins.len(), 1);
        assert_eq!(fetched.stakeholder_insights.len(), 1);
        assert!(fetched.company_context.is_some());
    }

    #[test]
    fn test_db_intelligence_missing_returns_none() {
        let db = test_db();
        let result = db
            .get_entity_intelligence("nonexistent")
            .expect("should not error");
        assert!(result.is_none());
    }

    #[test]
    fn test_db_delete_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_some());

        db.delete_entity_intelligence("acme-corp").expect("delete");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_none());
    }

    #[test]
    fn test_db_upsert_overwrites() {
        let db = test_db();
        let mut intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("first upsert");

        // Update the assessment
        intel.executive_assessment = Some("Updated assessment.".to_string());
        intel.risks.push(IntelRisk {
            text: "New risk".to_string(),
            source: None,
            urgency: "watch".to_string(),
            item_source: None,
            discrepancy: None,
        });

        db.upsert_entity_intelligence(&intel)
            .expect("second upsert");

        let fetched = db.get_entity_intelligence("acme-corp").unwrap().unwrap();
        assert_eq!(
            fetched.executive_assessment.as_deref(),
            Some("Updated assessment.")
        );
        assert_eq!(fetched.risks.len(), 2);
    }

    #[test]
    fn test_db_dimensions_roundtrip() {
        let db = test_db();
        let mut intel = sample_intel();

        // Populate I508a dimension fields across all 6 dimensions
        intel.competitive_context = vec![CompetitiveInsight {
            competitor: "Rival Corp".to_string(),
            threat_level: Some("high".to_string()),
            context: Some("Competing for same segment".to_string()),
            source: Some("qbr-notes.md".to_string()),
            detected_at: Some("2026-02-01".to_string()),
            item_source: None,
            discrepancy: None,
        }];
        intel.strategic_priorities = vec![StrategicPriority {
            priority: "Expand enterprise tier".to_string(),
            status: Some("in_progress".to_string()),
            owner: Some("VP Sales".to_string()),
            source: Some("strategy-deck.pdf".to_string()),
            timeline: Some("Q2 2026".to_string()),
        }];
        intel.coverage_assessment = Some(CoverageAssessment {
            role_fill_rate: Some(0.75),
            gaps: vec!["No executive sponsor".to_string()],
            covered: vec!["Technical champion".to_string()],
            level: Some("partial".to_string()),
        });
        intel.organizational_changes = vec![OrgChange {
            change_type: "departure".to_string(),
            person: "Jane CTO".to_string(),
            from: Some("CTO".to_string()),
            to: None,
            detected_at: Some("2026-01-20".to_string()),
            source: Some("email".to_string()),
            item_source: None,
            discrepancy: None,
        }];
        intel.internal_team = vec![InternalTeamMember {
            person_id: Some("p-alice".to_string()),
            name: "Alice".to_string(),
            role: "CSM".to_string(),
            source: Some("crm".to_string()),
        }];
        intel.meeting_cadence = Some(CadenceAssessment {
            meetings_per_month: Some(4.0),
            trend: Some("stable".to_string()),
            days_since_last: Some(3),
            assessment: Some("Healthy cadence".to_string()),
            evidence: vec!["Weekly sync on Tuesdays".to_string()],
        });
        intel.email_responsiveness = Some(ResponsivenessAssessment {
            trend: Some("improving".to_string()),
            volume_trend: Some("increasing".to_string()),
            assessment: Some("Responsive within 24h".to_string()),
            evidence: vec!["Reply time < 4h last month".to_string()],
        });
        intel.blockers = vec![Blocker {
            description: "SSO integration stalled".to_string(),
            owner: Some("Engineering".to_string()),
            since: Some("2026-01-10".to_string()),
            impact: Some("Blocking 200-seat rollout".to_string()),
            source: Some("meeting-notes.md".to_string()),
        }];
        intel.contract_context = Some(ContractContext {
            contract_type: Some("annual".to_string()),
            auto_renew: Some(true),
            contract_start: Some("2025-03-01".to_string()),
            renewal_date: Some("2026-03-01".to_string()),
            current_arr: Some(120_000.0),
            multi_year_remaining: None,
            previous_renewal_outcome: Some("expanded".to_string()),
            procurement_notes: None,
            customer_fiscal_year_start: Some(1),
        });
        intel.expansion_signals = vec![ExpansionSignal {
            opportunity: "APAC team onboarding".to_string(),
            arr_impact: Some(30_000.0),
            source: Some("champion-email".to_string()),
            stage: Some("discovery".to_string()),
            strength: Some("early".to_string()),
            item_source: None,
            discrepancy: None,
        }];
        intel.renewal_outlook = Some(RenewalOutlook {
            confidence: Some("high".to_string()),
            risk_factors: vec!["Budget freeze possible".to_string()],
            expansion_potential: Some("$30k APAC".to_string()),
            recommended_start: Some("2026-01-15".to_string()),
            negotiation_leverage: vec!["Multi-year discount".to_string()],
            negotiation_risk: vec!["Competitor POC".to_string()],
        });
        intel.support_health = Some(SupportHealth {
            open_tickets: Some(3),
            critical_tickets: Some(0),
            avg_resolution_time: Some("2.5 days".to_string()),
            trend: Some("improving".to_string()),
            csat: Some(4.5),
            source: Some("zendesk".to_string()),
        });
        intel.product_adoption = Some(AdoptionSignals {
            adoption_rate: Some(0.72),
            trend: Some("growing".to_string()),
            feature_adoption: vec!["dashboard".to_string(), "reports".to_string()],
            last_active: Some("2026-02-28".to_string()),
            source: Some("product-analytics".to_string()),
        });
        intel.nps_csat = Some(SatisfactionData {
            nps: Some(45),
            csat: Some(4.2),
            survey_date: Some("2026-01-15".to_string()),
            verbatim: Some("Great product, needs better reporting".to_string()),
            source: Some("survey-tool".to_string()),
        });
        let mut source_attr = std::collections::HashMap::new();
        source_attr.insert(
            "competitive_context".to_string(),
            vec!["qbr-notes.md".to_string()],
        );
        intel.source_attribution = Some(source_attr);

        // Write to DB
        db.upsert_entity_intelligence(&intel)
            .expect("upsert with dimensions");

        // Read back
        let fetched = db
            .get_entity_intelligence("acme-corp")
            .expect("get")
            .expect("should exist");

        // Verify all 15 I508a fields survived the roundtrip
        assert_eq!(fetched.competitive_context.len(), 1);
        assert_eq!(fetched.competitive_context[0].competitor, "Rival Corp");
        assert_eq!(fetched.strategic_priorities.len(), 1);
        assert_eq!(
            fetched.strategic_priorities[0].priority,
            "Expand enterprise tier"
        );
        assert!(fetched.coverage_assessment.is_some());
        assert_eq!(
            fetched.coverage_assessment.as_ref().unwrap().role_fill_rate,
            Some(0.75)
        );
        assert_eq!(fetched.organizational_changes.len(), 1);
        assert_eq!(fetched.internal_team.len(), 1);
        assert_eq!(fetched.internal_team[0].name, "Alice");
        assert!(fetched.meeting_cadence.is_some());
        assert_eq!(
            fetched.meeting_cadence.as_ref().unwrap().meetings_per_month,
            Some(4.0)
        );
        assert!(fetched.email_responsiveness.is_some());
        assert_eq!(fetched.blockers.len(), 1);
        assert_eq!(fetched.blockers[0].description, "SSO integration stalled");
        assert!(fetched.contract_context.is_some());
        assert_eq!(
            fetched.contract_context.as_ref().unwrap().current_arr,
            Some(120_000.0)
        );
        assert_eq!(fetched.expansion_signals.len(), 1);
        assert!(fetched.renewal_outlook.is_some());
        assert_eq!(
            fetched.renewal_outlook.as_ref().unwrap().confidence,
            Some("high".to_string())
        );
        assert!(fetched.support_health.is_some());
        assert_eq!(
            fetched.support_health.as_ref().unwrap().open_tickets,
            Some(3)
        );
        assert!(fetched.product_adoption.is_some());
        assert_eq!(
            fetched.product_adoption.as_ref().unwrap().adoption_rate,
            Some(0.72)
        );
        assert!(fetched.nps_csat.is_some());
        assert_eq!(fetched.nps_csat.as_ref().unwrap().nps, Some(45));
        assert!(fetched.source_attribution.is_some());
        assert!(fetched
            .source_attribution
            .as_ref()
            .unwrap()
            .contains_key("competitive_context"));
    }

    // =========================================================================
    // I134: format_intelligence_markdown
    // =========================================================================

    #[test]
    fn test_format_intelligence_markdown_full() {
        let intel = IntelligenceJson {
            version: 1,
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-02-09T10:00:00Z".to_string(),
            source_file_count: 3,
            source_manifest: vec![],
            executive_assessment: Some("Acme is in strong position for renewal.".to_string()),
            risks: vec![IntelRisk {
                text: "Budget uncertainty for Q3".to_string(),
                source: Some("QBR notes".to_string()),
                urgency: "critical".to_string(),
                item_source: None,
                discrepancy: None,
            }],
            recent_wins: vec![IntelWin {
                text: "Expanded to 3 teams".to_string(),
                source: Some("capture".to_string()),
                impact: Some("20% seat growth".to_string()),
                item_source: None,
                discrepancy: None,
            }],
            current_state: Some(CurrentState {
                working: vec!["Onboarding flow".to_string()],
                not_working: vec!["Reporting delayed".to_string()],
                unknowns: vec!["FY budget".to_string()],
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Alice Chen".to_string(),
                role: Some("VP Engineering".to_string()),
                assessment: Some("Strong advocate.".to_string()),
                engagement: Some("high".to_string()),
                source: None,
                person_id: None,
                suggested_person_id: None,
                item_source: None,
                discrepancy: None,
            }],
            value_delivered: vec![ValueItem {
                date: Some("2026-01-15".to_string()),
                statement: "Reduced onboarding time by 40%".to_string(),
                source: Some("QBR".to_string()),
                impact: Some("$50k savings".to_string()),
                item_source: None,
                discrepancy: None,
            }],
            next_meeting_readiness: Some(MeetingReadiness {
                meeting_title: Some("Acme QBR".to_string()),
                meeting_date: Some("2026-02-15".to_string()),
                prep_items: vec![
                    "Review blockers".to_string(),
                    "Bring ROI metrics".to_string(),
                ],
            }),
            company_context: Some(CompanyContext {
                description: Some("Enterprise SaaS platform".to_string()),
                industry: Some("Technology".to_string()),
                size: Some("500-1000".to_string()),
                headquarters: Some("San Francisco".to_string()),
                additional_context: None,
            }),
            portfolio: None,
            network: None,
            user_edits: Vec::new(),
            health: None,
            org_health: None,
            success_metrics: None,
            open_commitments: None,
            relationship_depth: None,
            consistency_status: None,
            consistency_findings: Vec::new(),
            consistency_checked_at: None,
            ..Default::default()
        };

        let md = format_intelligence_markdown(&intel);

        // All sections present
        assert!(md.contains("## Executive Assessment"));
        assert!(md.contains("Acme is in strong position"));
        assert!(md.contains("_Last enriched: 2026-02-09_"));

        assert!(md.contains("## Risks"));
        assert!(md.contains("**critical** Budget uncertainty"));
        assert!(md.contains("_(source: QBR notes)_"));

        assert!(md.contains("## Recent Wins"));
        assert!(md.contains("Expanded to 3 teams"));

        assert!(md.contains("## Current State"));
        assert!(md.contains("### What's Working"));
        assert!(md.contains("### What's Not Working"));
        assert!(md.contains("### Unknowns"));

        assert!(md.contains("## Next Meeting Readiness"));
        assert!(md.contains("**Acme QBR** on 2026-02-15"));
        assert!(md.contains("Review blockers"));

        assert!(md.contains("## Stakeholder Insights"));
        assert!(md.contains("### Alice Chen"));

        assert!(md.contains("## Value Delivered"));
        assert!(md.contains("**2026-01-15** Reduced onboarding"));

        assert!(md.contains("## Company Context"));
        assert!(md.contains("Enterprise SaaS platform"));
        assert!(md.contains("**Industry:** Technology"));
    }

    #[test]
    fn test_format_intelligence_markdown_empty() {
        let intel = IntelligenceJson::default();
        let md = format_intelligence_markdown(&intel);
        assert!(
            md.is_empty(),
            "Empty intelligence should produce empty markdown"
        );
    }

    #[test]
    fn test_format_intelligence_markdown_partial() {
        let intel = IntelligenceJson {
            executive_assessment: Some("Situation looks good.".to_string()),
            enriched_at: "2026-02-09T10:00:00Z".to_string(),
            ..Default::default()
        };
        let md = format_intelligence_markdown(&intel);
        assert!(md.contains("## Executive Assessment"));
        assert!(md.contains("Situation looks good."));
        // No other sections
        assert!(!md.contains("## Risks"));
        assert!(!md.contains("## Recent Wins"));
        assert!(!md.contains("## Current State"));
    }

    // =========================================================================
    // I139: Content classification + mechanical summary tests
    // =========================================================================

    #[test]
    fn test_classify_content_dashboard() {
        let (ct, p) = classify_content("Acme-dashboard.md", "Markdown");
        assert_eq!(ct, "dashboard");
        assert_eq!(p, 10);
    }

    #[test]
    fn test_classify_content_transcript() {
        let (ct, p) = classify_content("call-transcript-2025-01-28.md", "Markdown");
        assert_eq!(ct, "transcript");
        assert_eq!(p, 9);

        let (ct2, _) = classify_content("Weekly-Recording-Notes.md", "Markdown");
        assert_eq!(ct2, "transcript");

        let (ct3, _) = classify_content("customer-call_notes-q4.md", "Markdown");
        assert_eq!(ct3, "transcript");
    }

    #[test]
    fn test_classify_content_stakeholder() {
        let (ct, p) = classify_content("stakeholder-map.md", "Markdown");
        assert_eq!(ct, "stakeholder-map");
        assert_eq!(p, 9);

        let (ct2, _) = classify_content("org-chart-acme.xlsx", "Xlsx");
        assert_eq!(ct2, "stakeholder-map");
    }

    #[test]
    fn test_classify_content_success_plan() {
        let (ct, p) = classify_content("success-plan-2026.md", "Markdown");
        assert_eq!(ct, "success-plan");
        assert_eq!(p, 8);

        let (ct2, _) = classify_content("account_strategy.md", "Markdown");
        assert_eq!(ct2, "success-plan");
    }

    #[test]
    fn test_classify_content_qbr() {
        let (ct, p) = classify_content("Q4-QBR.pptx", "Pptx");
        assert_eq!(ct, "qbr");
        assert_eq!(p, 8);

        let (ct2, _) = classify_content("quarterly-business-review-2025.md", "Markdown");
        assert_eq!(ct2, "qbr");
    }

    #[test]
    fn test_classify_content_contract() {
        let (ct, p) = classify_content("master-agreement-v2.pdf", "Pdf");
        assert_eq!(ct, "contract");
        assert_eq!(p, 7);

        let (ct2, _) = classify_content("sow-phase2.docx", "Docx");
        assert_eq!(ct2, "contract");
    }

    #[test]
    fn test_classify_content_notes() {
        let (ct, p) = classify_content("meeting-notes-jan.md", "Markdown");
        assert_eq!(ct, "notes");
        assert_eq!(p, 7);
    }

    #[test]
    fn test_classify_content_format_fallback_pptx() {
        let (ct, p) = classify_content("slide-deck.pptx", "Pptx");
        assert_eq!(ct, "presentation");
        assert_eq!(p, 6);
    }

    #[test]
    fn test_classify_content_format_fallback_xlsx() {
        let (ct, p) = classify_content("data.xlsx", "Xlsx");
        assert_eq!(ct, "spreadsheet");
        assert_eq!(p, 6);
    }

    #[test]
    fn test_classify_content_default() {
        let (ct, p) = classify_content("random-file.md", "Markdown");
        assert_eq!(ct, "general");
        assert_eq!(p, 5);
    }

    #[test]
    fn test_classify_content_case_insensitive() {
        let (ct, _) = classify_content("ACME-DASHBOARD.MD", "Markdown");
        assert_eq!(ct, "dashboard");

        let (ct2, _) = classify_content("Call-Transcript-Feb.md", "Markdown");
        assert_eq!(ct2, "transcript");
    }

    #[test]
    fn test_recency_boost() {
        let recent = Utc::now().to_rfc3339();
        // No date prefix → falls back to modified_at
        assert_eq!(apply_recency_boost(5, "some-file.md", &recent), 6);
        assert_eq!(apply_recency_boost(10, "some-file.md", &recent), 10); // capped at 10

        let old = "2020-01-01T00:00:00+00:00";
        assert_eq!(apply_recency_boost(5, "some-file.md", old), 5); // no boost

        // Filename date takes precedence over modified_at
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let dated_filename = format!("{}-meeting-notes.md", today);
        // Even with old mtime, recent filename date gets the boost
        assert_eq!(apply_recency_boost(5, &dated_filename, old), 6);

        // Old filename date, even with recent mtime → no boost
        assert_eq!(
            apply_recency_boost(5, "2020-01-15-old-notes.md", &recent),
            5
        );
    }

    #[test]
    fn test_content_date_rfc3339() {
        // Filename with date prefix
        assert_eq!(
            content_date_rfc3339("2024-09-13-meeting.md", "2026-02-09T12:00:00+00:00"),
            "2024-09-13T00:00:00+00:00"
        );
        // No date prefix → falls back to modified_at
        assert_eq!(
            content_date_rfc3339("notes.md", "2026-02-09T12:00:00+00:00"),
            "2026-02-09T12:00:00+00:00"
        );
        // Short filename
        assert_eq!(
            content_date_rfc3339("a.md", "2026-02-09T12:00:00+00:00"),
            "2026-02-09T12:00:00+00:00"
        );
    }

    #[test]
    fn test_mechanical_summary_markdown() {
        let text = "# Account Overview\n\nAcme Corp is a leading SaaS provider.\n\n## Health\n\nCurrently green.\n\n## Risks\n\nBudget uncertainty.\n";
        let summary = mechanical_summary(text, 500);

        assert!(summary.contains("Acme Corp is a leading SaaS provider."));
        assert!(summary.contains("Sections:"));
        assert!(summary.contains("Account Overview"));
        assert!(summary.contains("Health"));
        assert!(summary.contains("Risks"));
    }

    #[test]
    fn test_mechanical_summary_plain_text() {
        let text = "This is a plain text document without any markdown headings. It has some content that should be captured as the first paragraph.";
        let summary = mechanical_summary(text, 500);

        assert!(summary.starts_with("This is a plain text"));
        assert!(!summary.contains("Sections:"));
    }

    #[test]
    fn test_mechanical_summary_empty() {
        let summary = mechanical_summary("", 500);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_mechanical_summary_truncation() {
        let text = "# Header\n\nA very long paragraph that goes on and on. ".repeat(20);
        let summary = mechanical_summary(&text, 100);
        assert!(summary.len() <= 100);
    }

    #[test]
    fn test_mechanical_summary_headings_only() {
        let text = "# Overview\n## Details\n## Timeline\n";
        let summary = mechanical_summary(text, 500);
        assert!(summary.starts_with("Sections:"));
        assert!(summary.contains("Overview"));
        assert!(summary.contains("Details"));
        assert!(summary.contains("Timeline"));
    }

    #[test]
    fn test_entity_files_sorted_by_priority() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Insert a low-priority file
        let low = crate::db::DbContentFile {
            id: "sort-test/general".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "random.md".to_string(),
            relative_path: "Accounts/Sort/random.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/random.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 100,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&low).unwrap();

        // Insert a high-priority file
        let high = crate::db::DbContentFile {
            id: "sort-test/dashboard".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "dashboard.md".to_string(),
            relative_path: "Accounts/Sort/dashboard.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/dashboard.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 200,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "dashboard".to_string(),
            priority: 10,
        };
        db.upsert_content_file(&high).unwrap();

        // Insert a mid-priority file
        let mid = crate::db::DbContentFile {
            id: "sort-test/notes".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "notes.md".to_string(),
            relative_path: "Accounts/Sort/notes.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/notes.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 150,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "notes".to_string(),
            priority: 7,
        };
        db.upsert_content_file(&mid).unwrap();

        let files = db.get_entity_files("sort-test").unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].content_type, "dashboard"); // priority 10
        assert_eq!(files[1].content_type, "notes"); // priority 7
        assert_eq!(files[2].content_type, "general"); // priority 5
    }

    #[test]
    fn test_resolve_array_path_identity_reorder() {
        // Existing: [Alice, Bob]  New: [Bob, Alice] — path should remap
        let existing = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Alice", "role": "champion"},
                {"name": "Bob", "role": "technical"}
            ]
        });
        let new = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Bob", "role": "from_ai"},
                {"name": "Alice", "role": "from_ai"}
            ]
        });

        let result = resolve_array_path_by_identity(&existing, &new, "stakeholderInsights[0].role");
        // Alice was at [0] in existing, now at [1] in new
        assert_eq!(result, Some("stakeholderInsights[1].role".to_string()));
    }

    #[test]
    fn test_resolve_array_path_identity_same_index() {
        // Same order → returns None (no remap needed)
        let existing = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Alice", "role": "champion"},
                {"name": "Bob", "role": "technical"}
            ]
        });
        let new = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Alice", "role": "from_ai"},
                {"name": "Bob", "role": "from_ai"}
            ]
        });

        let result = resolve_array_path_by_identity(&existing, &new, "stakeholderInsights[0].role");
        // Same index → None (fallback to direct path)
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_array_path_identity_missing_element() {
        // Element removed in new array → returns None
        let existing = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Alice", "role": "champion"},
                {"name": "Bob", "role": "technical"}
            ]
        });
        let new = serde_json::json!({
            "stakeholderInsights": [
                {"name": "Charlie", "role": "from_ai"}
            ]
        });

        let result = resolve_array_path_by_identity(&existing, &new, "stakeholderInsights[0].role");
        // Alice not in new → None (fallback)
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_array_path_identity_non_array_path() {
        // Non-array path → returns None
        let existing = serde_json::json!({"executiveAssessment": "text"});
        let new = serde_json::json!({"executiveAssessment": "new text"});

        let result = resolve_array_path_by_identity(&existing, &new, "executiveAssessment");
        assert_eq!(result, None);
    }
}
