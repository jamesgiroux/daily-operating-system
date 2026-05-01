use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use schemars::schema::{Schema, SchemaObject};
use schemars::{gen::SchemaGenerator, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::field::{FieldAttribution, FieldPath};
use super::source::{EntityId, SourceAttribution, SourceIndex};
use super::subject::SubjectAttribution;
use super::trust::{EffectiveTrust, TrustAssessment};
use crate::abilities::registry::AbilityCategory;

pub const PROVENANCE_SCHEMA_VERSION: u32 = 1;

fn provenance_schema_version_default() -> u32 {
    PROVENANCE_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AbilityVersion {
    pub major: u16,
    pub minor: u16,
}

impl AbilityVersion {
    pub fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SchemaVersion(pub u32);

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct InvocationId(pub String);

impl InvocationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone)]
pub struct CompositionId(CompositionIdValue);

#[derive(Debug, Clone)]
enum CompositionIdValue {
    Static(&'static str),
    Owned(String),
}

impl CompositionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(CompositionIdValue::Owned(value.into()))
    }

    pub const fn from_static(value: &'static str) -> Self {
        Self(CompositionIdValue::Static(value))
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            CompositionIdValue::Static(value) => value,
            CompositionIdValue::Owned(value) => value.as_str(),
        }
    }
}

impl PartialEq for CompositionId {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CompositionId {}

impl PartialOrd for CompositionId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompositionId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for CompositionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Serialize for CompositionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CompositionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::new)
    }
}

impl JsonSchema for CompositionId {
    fn schema_name() -> String {
        "CompositionId".into()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        String::json_schema(gen)
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct ThreadId(pub String);

impl ThreadId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct HashValue(pub String);

impl HashValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SourceClass(pub String);

impl SourceClass {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Actor {
    User,
    Agent { name: String, version: String },
    System { component: String },
    Human { role: String, id: String },
    External { source: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AbilityExecutionMode {
    Live,
    Simulate,
    Evaluate,
}

impl From<crate::services::context::ExecutionMode> for AbilityExecutionMode {
    fn from(value: crate::services::context::ExecutionMode) -> Self {
        match value {
            crate::services::context::ExecutionMode::Live => Self::Live,
            crate::services::context::ExecutionMode::Simulate => Self::Simulate,
            crate::services::context::ExecutionMode::Evaluate => Self::Evaluate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ModelName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PromptTemplateId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PromptVersion(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PromptFingerprint {
    pub provider: String,
    pub model: ModelName,
    pub prompt_template_id: PromptTemplateId,
    pub prompt_template_version: PromptVersion,
    pub canonical_prompt_hash: HashValue,
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub seed: Option<u64>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub provider_completion_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EntityWatermark {
    pub entity_version: u64,
    #[schemars(with = "String")]
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InputsSnapshot {
    #[schemars(with = "Option<String>")]
    pub newest_signal_at: Option<DateTime<Utc>>,
    pub entity_watermarks: BTreeMap<EntityId, EntityWatermark>,
    #[schemars(with = "BTreeMap<SourceClass, String>")]
    pub source_freshness: BTreeMap<SourceClass, DateTime<Utc>>,
    pub provider_config_hash: HashValue,
    pub glean_connected: bool,
}

impl Default for InputsSnapshot {
    fn default() -> Self {
        Self {
            newest_signal_at: None,
            entity_watermarks: BTreeMap::new(),
            source_freshness: BTreeMap::new(),
            provider_config_hash: HashValue::new("unknown"),
            glean_connected: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AbilityOutput<T> {
    pub(in crate::abilities) data: T,
    pub(in crate::abilities) provenance: Provenance,
    pub(in crate::abilities) ability_version: AbilityVersion,
    pub(in crate::abilities) diagnostics: Diagnostics,
}

impl<T> AbilityOutput<T> {
    pub(in crate::abilities) fn new(data: T, provenance: Provenance) -> Self {
        Self {
            data,
            ability_version: provenance.ability_version.clone(),
            diagnostics: Diagnostics::default(),
            provenance,
        }
    }

    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn into_data(self) -> T {
        self.data
    }

    pub fn into_parts(self) -> (T, Provenance) {
        (self.data, self.provenance)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostics {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provenance {
    #[serde(default = "provenance_schema_version_default")]
    pub provenance_schema_version: u32,
    pub ability_name: String,
    pub ability_version: AbilityVersion,
    pub ability_schema_version: SchemaVersion,
    pub invocation_id: InvocationId,
    #[schemars(with = "String")]
    pub produced_at: DateTime<Utc>,
    pub inputs_snapshot: InputsSnapshot,
    pub actor: Actor,
    pub mode: AbilityExecutionMode,
    pub trust: TrustAssessment,
    pub sources: Vec<SourceAttribution>,
    #[serde(default)]
    pub thread_ids: Vec<ThreadId>,
    pub prompt_fingerprint: Option<PromptFingerprint>,
    pub children: Vec<ComposedProvenance>,
    pub field_attributions: BTreeMap<FieldPath, FieldAttribution>,
    pub subject: SubjectAttribution,
    pub warnings: Vec<ProvenanceWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedProvenance {
    pub composition_id: CompositionId,
    pub provenance: Box<Provenance>,
}

impl ComposedProvenance {
    pub fn new(composition_id: CompositionId, provenance: Provenance) -> Self {
        Self {
            composition_id,
            provenance: Box::new(provenance),
        }
    }
}

impl Serialize for ComposedProvenance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.provenance.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ComposedProvenance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let provenance = Provenance::deserialize(deserializer)?;
        let composition_id = CompositionId::new(provenance.ability_name.clone());
        Ok(Self {
            composition_id,
            provenance: Box::new(provenance),
        })
    }
}

impl JsonSchema for ComposedProvenance {
    fn schema_name() -> String {
        "ProvenanceChild".into()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            reference: Some("#/definitions/Provenance".into()),
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceWarning {
    DepthElided {
        skipped_levels: u32,
        elided_children: u32,
        aggregate_source_count: u32,
        effective_trust: EffectiveTrust,
    },
    SourceStale {
        source_index: SourceIndex,
        age_seconds: i64,
    },
    SourceUnresolvable {
        source_index: SourceIndex,
        reason: String,
    },
    AttributionIncomplete {
        field: FieldPath,
    },
    Masked {
        reason: MaskReason,
    },
    SourceTimestampUnknown {
        source_index: SourceIndex,
        fallback: SourceTimestampFallback,
    },
    SourceTimestampImplausible {
        source_index: SourceIndex,
        reason: String,
    },
    SubjectFitQualified {
        field: Option<FieldPath>,
        status: String,
    },
    OptionalComposedReadFailed {
        composition_id: CompositionId,
        reason: String,
    },
    SoftSizeLimitExceeded {
        bytes: usize,
        soft_budget_bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceTimestampFallback {
    ObservedAt,
    ClaimCreatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaskReason {
    SourceRevoked,
    ActorNotAuthorized,
    Sensitive,
    Other(String),
}

#[allow(clippy::too_many_arguments)]
pub fn provenance_for_test(
    ability_name: impl Into<String>,
    produced_at: DateTime<Utc>,
    subject: SubjectAttribution,
    sources: Vec<SourceAttribution>,
    children: Vec<ComposedProvenance>,
    field_attributions: BTreeMap<FieldPath, FieldAttribution>,
    prompt_fingerprint: Option<PromptFingerprint>,
    warnings: Vec<ProvenanceWarning>,
) -> Provenance {
    let trust = TrustAssessment::compute(
        &sources,
        &children,
        &field_attributions,
        AbilityCategory::Read,
        prompt_fingerprint.is_some(),
    );
    Provenance {
        provenance_schema_version: PROVENANCE_SCHEMA_VERSION,
        ability_name: ability_name.into(),
        ability_version: AbilityVersion::new(1, 0),
        ability_schema_version: SchemaVersion(1),
        invocation_id: InvocationId::new("invocation-fixture"),
        produced_at,
        inputs_snapshot: InputsSnapshot::default(),
        actor: Actor::System {
            component: "fixture".into(),
        },
        mode: AbilityExecutionMode::Evaluate,
        trust,
        sources,
        thread_ids: Vec::new(),
        prompt_fingerprint,
        children,
        field_attributions,
        subject,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::provenance::{
        FieldAttribution, FieldPath, SubjectAttribution, SubjectRef,
    };
    use chrono::TimeZone;

    fn fixture_provenance() -> Provenance {
        let produced_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
        provenance_for_test(
            "fixture",
            produced_at,
            subject.clone(),
            Vec::new(),
            Vec::new(),
            BTreeMap::from([(
                FieldPath::new("/name").unwrap(),
                FieldAttribution::constant(subject),
            )]),
            None,
            Vec::new(),
        )
    }

    #[test]
    fn json_roundtrip_preserves_equality() {
        let provenance = fixture_provenance();
        let decoded: Provenance =
            serde_json::from_value(serde_json::to_value(&provenance).unwrap()).unwrap();

        assert_eq!(decoded, provenance);
    }

    #[test]
    fn schemars_schema_for_provenance_is_valid_shape() {
        let schema = schemars::schema_for!(Provenance);
        let value = serde_json::to_value(schema).unwrap();
        let rendered = value.to_string();

        assert!(rendered.contains("provenance_schema_version"));
        assert!(rendered.contains("thread_ids"));
    }

    #[test]
    fn composition_a_b_c_preserves_child_grandchild_tree() {
        let produced_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
        let c = provenance_for_test(
            "c",
            produced_at,
            subject.clone(),
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            None,
            Vec::new(),
        );
        let b = provenance_for_test(
            "b",
            produced_at,
            subject.clone(),
            Vec::new(),
            vec![ComposedProvenance::new(CompositionId::new("c"), c)],
            BTreeMap::new(),
            None,
            Vec::new(),
        );
        let a = provenance_for_test(
            "a",
            produced_at,
            subject,
            Vec::new(),
            vec![ComposedProvenance::new(CompositionId::new("b"), b)],
            BTreeMap::new(),
            None,
            Vec::new(),
        );

        assert_eq!(a.children[0].composition_id, CompositionId::new("b"));
        assert_eq!(a.children[0].provenance.children[0].composition_id, CompositionId::new("c"));
    }
}
