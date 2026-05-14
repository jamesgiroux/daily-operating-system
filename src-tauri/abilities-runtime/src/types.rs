use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::sensitivity::RenderableClaimText;

fn default_claim_version() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    Active,
    Dormant,
    Tombstoned,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurfacingState {
    Active,
    Dormant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemporalScope {
    State,
    PointInTime,
    Trend,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimSensitivity {
    Public,
    Internal,
    Confidential,
    UserOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IntelligenceClaim {
    pub id: String,
    #[serde(default = "default_claim_version")]
    pub claim_version: u64,
    pub subject_ref: String,
    pub claim_type: String,
    pub field_path: Option<String>,
    pub topic_key: Option<String>,
    pub text: String,
    pub dedup_key: String,
    pub item_hash: Option<String>,
    pub actor: String,
    pub data_source: String,
    pub source_ref: Option<String>,
    pub source_asof: Option<String>,
    pub observed_at: String,
    pub created_at: String,
    pub provenance_json: String,
    pub metadata_json: Option<String>,
    pub claim_state: ClaimState,
    pub surfacing_state: SurfacingState,
    pub demotion_reason: Option<String>,
    pub reactivated_at: Option<String>,
    pub retraction_reason: Option<String>,
    pub expires_at: Option<String>,
    pub superseded_by: Option<String>,
    pub trust_score: Option<f64>,
    pub trust_computed_at: Option<String>,
    pub trust_version: Option<i64>,
    pub thread_id: Option<String>,
    pub temporal_scope: TemporalScope,
    pub sensitivity: ClaimSensitivity,
    pub verification_state: crate::sensitivity::ClaimVerificationState,
    pub verification_reason: Option<String>,
    pub needs_user_decision_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum EntityContextText {
    Plain(String),
    Claim(RenderableClaimText),
}

impl EntityContextText {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(text) => text,
            Self::Claim(text) => &text.text,
        }
    }
}

impl From<String> for EntityContextText {
    fn from(value: String) -> Self {
        Self::Plain(value)
    }
}

impl From<&str> for EntityContextText {
    fn from(value: &str) -> Self {
        Self::Plain(value.to_string())
    }
}

impl From<RenderableClaimText> for EntityContextText {
    fn from(value: RenderableClaimText) -> Self {
        Self::Claim(value)
    }
}

impl PartialEq<&str> for EntityContextText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<EntityContextText> for &str {
    fn eq(&self, other: &EntityContextText) -> bool {
        *self == other.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityContextEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub title: EntityContextText,
    pub content: EntityContextText,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimSubjectRef {
    Account { id: String },
    Meeting { id: String },
    Person { id: String },
    Project { id: String },
    Email { id: String },
    Multi(Vec<ClaimSubjectRef>),
    Global,
}

pub fn subject_ref_from_json(value: &serde_json::Value) -> Result<ClaimSubjectRef, String> {
    let kind_raw = value
        .get("kind")
        .or_else(|| value.get("type"))
        .or_else(|| value.get("entity_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing kind/type".to_string())?;
    let kind = kind_raw.to_ascii_lowercase();

    match kind.as_str() {
        "account" | "accounts" => Ok(ClaimSubjectRef::Account {
            id: subject_id(value)?,
        }),
        "meeting" | "meetings" => Ok(ClaimSubjectRef::Meeting {
            id: subject_id(value)?,
        }),
        "person" | "people" => Ok(ClaimSubjectRef::Person {
            id: subject_id(value)?,
        }),
        "project" | "projects" => Ok(ClaimSubjectRef::Project {
            id: subject_id(value)?,
        }),
        "email" | "emails" => Ok(ClaimSubjectRef::Email {
            id: subject_id(value)?,
        }),
        "multi" => {
            let refs = value
                .get("subjects")
                .or_else(|| value.get("refs"))
                .and_then(|v| v.as_array())
                .ok_or_else(|| "multi subject_ref missing subjects".to_string())?
                .iter()
                .map(subject_ref_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ClaimSubjectRef::Multi(refs))
        }
        "global" => Ok(ClaimSubjectRef::Global),
        other => Err(format!("unsupported subject kind/type '{other}'")),
    }
}

pub fn prompt_input_sensitivity_allowed(sensitivity: &ClaimSensitivity) -> bool {
    matches!(
        sensitivity,
        ClaimSensitivity::Public | ClaimSensitivity::Internal
    )
}

pub fn prompt_input_sensitivity_name_allowed(sensitivity: &str) -> bool {
    matches!(
        sensitivity.trim().to_ascii_lowercase().as_str(),
        "public" | "internal"
    )
}

pub fn claim_allowed_for_prompt_input(claim: &IntelligenceClaim) -> bool {
    prompt_input_sensitivity_allowed(&claim.sensitivity)
}

fn subject_id(value: &serde_json::Value) -> Result<String, String> {
    value
        .get("id")
        .or_else(|| value.get("entity_id"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "missing id/entity_id".to_string())
}
