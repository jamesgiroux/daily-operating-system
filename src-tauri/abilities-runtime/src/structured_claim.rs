use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::predicates::registry::PredicateRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStatus {
    PendingBackfill,
    LegacyUnmigrated,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    Affirm,
    Negate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EntityRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectValue {
    Resolved {
        entity_ref: EntityRef,
    },
    Literal {
        literal_kind: LiteralKind,
        value: String,
    },
    FreeText {
        canonical: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LiteralKind {
    Number,
    Text,
    Date,
    Money,
    Percentage,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TemporalQualifier {
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RegionCode {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScopeMarker {
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NumericQualifier {
    pub name: String,
    pub value: String,
}

pub type QualifierKey = String;
pub type QualifierValue = String;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QualifierSet {
    pub time: Option<TemporalQualifier>,
    pub region: Option<RegionCode>,
    pub scope: Option<ScopeMarker>,
    pub entity: Option<EntityRef>,
    #[serde(default)]
    pub numerics: Vec<NumericQualifier>,
    #[serde(default)]
    pub extras: BTreeMap<QualifierKey, QualifierValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Confirmed,
    Pending,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StructuredClaim {
    pub subject_ref: EntityRef,
    pub predicate: PredicateRef,
    pub polarity: Polarity,
    pub object: ObjectValue,
    pub qualifiers: QualifierSet,
    pub status: ClaimStatus,
    pub sentiment: Option<Sentiment>,
}
