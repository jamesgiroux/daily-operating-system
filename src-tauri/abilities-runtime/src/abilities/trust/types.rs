use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::SubjectRef;

use super::config::{TrustConfig, FACTOR_MAX, FACTOR_MIN};
use super::freshness_decay::RenewalContext;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TrustScore(pub f64);

impl TrustScore {
    pub const MIN: f64 = FACTOR_MIN;
    pub const MAX: f64 = FACTOR_MAX;

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustBand {
    LikelyCurrent,
    UseWithCaution,
    NeedsVerification,
    Unscored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustComputation {
    pub score: TrustScore,
    pub band: TrustBand,
    pub evidence: ConfidenceEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConfidenceEvidence {
    pub score: f64,
    pub band_label: String,
    pub factor_breakdown: Vec<FactorEvidence>,
    pub caveats: Vec<ConfidenceCaveat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FactorEvidence {
    pub name: String,
    pub weight: f64,
    pub raw_value: f64,
    pub value: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ConfidenceCaveat {
    FewSources,
    StaleSource {
        source: String,
        age_days: f64,
    },
    UnresolvedContradiction,
    InsufficientSignalDensity,
    UnknownTimestamp,
    CrossEntityReferences {
        hit_count: usize,
    },
    /// A pre-aggregation gate fired and capped the score below
    /// use_with_caution_min. Equal-weight geometric mean would otherwise dilute
    /// hard-policy factors dilute under geometric composition, so blockers
    /// run as gates instead of weighted contributions.
    /// so blockers run as gates instead of weighted contributions.
    TrustGateTriggered {
        gate: TrustGateKind,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustGateKind {
    SensitivityViolation,
    SourceWithdrawn,
    AuthoritativeContradiction,
    /// One or more trust-input reads failed (DB error, schema skew, or
    /// malformed row). The recompute can't trust any factor that depends on
    /// the unreadable state, so we lean to NeedsVerification rather than
    /// scoring with a possibly-stale partial picture. Producers set
    /// `TrustFactorInputs.read_state_indeterminate` explicitly when they
    /// could not enumerate the truth.
    IndeterminateReadState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustContext {
    #[schemars(with = "String")]
    pub now: DateTime<Utc>,
    #[serde(default)]
    pub renewal_context: Option<RenewalContext>,
    pub config: TrustConfig,
    pub factor_inputs: TrustFactorInputs,
    pub cross_entity: CrossEntityCoherenceInput,
    pub target_surface: Option<SurfaceClass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClass {
    Public,
    Internal,
    Confidential,
    UserOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustFactorInputs {
    pub source_reliability: f64,
    pub source_reliability_corroborators: Vec<CorroboratorWeight>,
    pub freshness: FreshnessContext,
    pub corroboration_strength: f64,
    pub contradiction_count: u32,
    pub user_feedback: UserFeedbackSignal,
    pub subject_fit_confidence: f64,
    /// How well the claim's internal sub-statements agree with each other.
    /// Range: normalized factor bounds.
    pub internal_consistency: f64,
    pub source_lifecycle: SourceLifecycleState,
    #[serde(default)]
    pub linear_issue_state: LinearIssueStateContext,
    /// True when at least one upstream read (corroborations, contradictions,
    /// feedback, source weights) failed. Triggers the IndeterminateReadState
    /// gate so the recompute fails closed instead of scoring on a partial
    /// picture. Defaults to false; producers explicitly opt in when they
    /// detected a read error.
    #[serde(default)]
    pub read_state_indeterminate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LinearIssueStateContext {
    #[serde(default)]
    pub signal: LinearIssueStateSignal,
    #[serde(default = "default_subject_matches")]
    pub subject_matches: bool,
}

impl Default for LinearIssueStateContext {
    fn default() -> Self {
        Self {
            signal: LinearIssueStateSignal::None,
            subject_matches: true,
        }
    }
}

fn default_subject_matches() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinearIssueStateSignal {
    #[default]
    None,
    UncategorizedIssue,
    StateChangedToInProgress,
    StateChangedToBlocked,
    StateChangedToDone,
    AssigneeChanged,
    PriorityChangedToUrgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceReliabilityInput {
    pub corroborators: Vec<CorroboratorWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CorroboratorWeight {
    pub evidence_weight: f64,
    pub confirms: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceLifecycleState {
    Active,
    Withdrawn,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserFeedbackSignal {
    None,
    Confirmed,
    Corrected,
    Retracted,
    WrongSubject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FreshnessContext {
    pub timestamp_known: bool,
    pub age_days: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetFootprint {
    pub subject: SubjectRef,
    pub names: Vec<String>,
    pub domains: Vec<String>,
    pub related_subjects: Vec<SubjectRef>,
    pub allowed_aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EntityFootprint {
    pub subject: SubjectRef,
    pub names: Vec<String>,
    pub domains: Vec<String>,
    pub infrastructure_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CrossEntityCoherenceInput {
    pub claim_text: String,
    pub target_footprint: TargetFootprint,
    pub portfolio_footprints: Vec<EntityFootprint>,
    pub cross_entity_context_expected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CrossEntityHit {
    pub token: String,
    pub kind: CrossEntityHitKind,
    pub source_subject: Option<SubjectRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrossEntityHitKind {
    Domain,
    InfrastructureId,
    CompanyName,
}
