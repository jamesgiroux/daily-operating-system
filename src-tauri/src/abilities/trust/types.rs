use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::SubjectRef;

use super::config::TrustConfig;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TrustScore(pub f64);

impl TrustScore {
    pub const MIN: f64 = 0.0;
    pub const MAX: f64 = 1.0;

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
    pub value: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ConfidenceCaveat {
    FewSources,
    StaleSource { source: String, age_days: f64 },
    UnresolvedContradiction,
    InsufficientSignalDensity,
    UnknownTimestamp,
    CrossEntityReferences { hit_count: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustContext {
    #[schemars(with = "String")]
    pub now: DateTime<Utc>,
    pub config: TrustConfig,
    pub factor_inputs: TrustFactorInputs,
    pub cross_entity: CrossEntityCoherenceInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustFactorInputs {
    pub source_reliability: f64,
    pub freshness: FreshnessContext,
    pub corroboration_strength: f64,
    pub contradiction_count: u32,
    pub user_feedback: UserFeedbackSignal,
    pub subject_fit_confidence: f64,
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
