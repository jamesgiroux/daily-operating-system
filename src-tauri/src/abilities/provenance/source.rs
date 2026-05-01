use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SourceIndex(pub usize);

impl SourceIndex {
    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SourceName(pub String);

impl SourceName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }
    };
}

id_newtype!(SignalId);
id_newtype!(MessageId);
id_newtype!(MeetingId);
id_newtype!(DocumentId);
id_newtype!(ChunkId);
id_newtype!(ContextEntryId);
id_newtype!(AssessmentId);
id_newtype!(ProviderRef);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    User,
    Google,
    Glean { downstream: GleanDownstream },
    Clay,
    Ai,
    CoAttendance,
    LocalEnrichment,
    Other(SourceName),
    LegacyUnattributed,
}

impl DataSource {
    pub fn scoring_class(&self) -> ScoringClass {
        match self {
            DataSource::User
            | DataSource::Google
            | DataSource::Glean {
                downstream:
                    GleanDownstream::Salesforce
                    | GleanDownstream::Zendesk
                    | GleanDownstream::Gong
                    | GleanDownstream::OrgDirectory,
            }
            | DataSource::Clay
            | DataSource::CoAttendance
            | DataSource::LocalEnrichment => ScoringClass::Scoring,
            DataSource::Glean {
                downstream: GleanDownstream::Slack | GleanDownstream::P2,
            } => ScoringClass::Context,
            DataSource::Glean { .. }
            | DataSource::Ai
            | DataSource::Other(_)
            | DataSource::LegacyUnattributed => ScoringClass::Reference,
        }
    }

    pub fn is_structured_trusted_source(&self) -> bool {
        matches!(
            self,
            DataSource::User
                | DataSource::Google
                | DataSource::Glean {
                    downstream: GleanDownstream::Salesforce
                        | GleanDownstream::Zendesk
                        | GleanDownstream::Gong
                        | GleanDownstream::OrgDirectory
                }
                | DataSource::Clay
                | DataSource::CoAttendance
                | DataSource::LocalEnrichment
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GleanDownstream {
    Salesforce,
    Zendesk,
    Gong,
    Slack,
    P2,
    Wordpress,
    OrgDirectory,
    Documents,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScoringClass {
    Scoring,
    Context,
    Reference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceIdentifier {
    Signal {
        signal_id: SignalId,
    },
    Entity {
        entity_id: EntityId,
        field: Option<String>,
    },
    EmailThread {
        thread_id: crate::abilities::provenance::ThreadId,
        message_id: Option<MessageId>,
    },
    Meeting {
        meeting_id: MeetingId,
    },
    Document {
        document_id: DocumentId,
        chunk_id: Option<ChunkId>,
    },
    UserEntry {
        entry_id: ContextEntryId,
    },
    GleanAssessment {
        assessment_id: AssessmentId,
        dimension: Option<String>,
        cited_sources: Vec<GleanCitedSource>,
    },
    ProviderCompletion {
        completion_id: String,
        provider: ProviderRef,
    },
    OpaqueGleanSource {
        downstream: GleanDownstream,
        opaque_ref: String,
        #[schemars(with = "String")]
        cited_as_of: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GleanCitedSource {
    pub downstream: GleanDownstream,
    pub citation: String,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SynthesisMarker {
    pub producer_ability: String,
    pub producer_invocation_id: crate::abilities::provenance::InvocationId,
    #[schemars(with = "String")]
    pub produced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceAttribution {
    pub data_source: DataSource,
    pub identifiers: Vec<SourceIdentifier>,
    #[schemars(with = "String")]
    pub observed_at: DateTime<Utc>,
    #[schemars(with = "Option<String>")]
    pub source_asof: Option<DateTime<Utc>>,
    pub evidence_weight: f32,
    pub scoring_class: ScoringClass,
    pub synthesis_marker: Option<SynthesisMarker>,
}

impl SourceAttribution {
    pub fn new(
        data_source: DataSource,
        identifiers: Vec<SourceIdentifier>,
        observed_at: DateTime<Utc>,
        source_asof: Option<DateTime<Utc>>,
        evidence_weight: f32,
        synthesis_marker: Option<SynthesisMarker>,
    ) -> Result<Self, SourceAttributionError> {
        if !(0.0..=1.0).contains(&evidence_weight) || !evidence_weight.is_finite() {
            return Err(SourceAttributionError::InvalidEvidenceWeight);
        }

        let scoring_class = data_source.scoring_class();
        Ok(Self {
            data_source,
            identifiers,
            observed_at,
            source_asof,
            evidence_weight,
            scoring_class,
            synthesis_marker,
        })
    }

    pub fn legacy_unattributed(
        observed_at: DateTime<Utc>,
    ) -> Result<Self, SourceAttributionError> {
        Self::new(
            DataSource::LegacyUnattributed,
            Vec::new(),
            observed_at,
            None,
            0.5,
            None,
        )
    }

    pub fn stored_synthesis_entity_field(&self) -> Option<(EntityId, String)> {
        self.synthesis_marker.as_ref()?;

        self.identifiers.iter().find_map(|identifier| match identifier {
            SourceIdentifier::Entity { entity_id, field } => {
                Some((entity_id.clone(), field.clone().unwrap_or_else(|| "unknown".into())))
            }
            _ => None,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SourceAttributionError {
    #[error("evidence weight must be finite and within [0.0, 1.0]")]
    InvalidEvidenceWeight,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn source_attribution_new_derives_scoring_class() {
        let observed_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        let source = SourceAttribution::new(
            DataSource::Glean {
                downstream: GleanDownstream::Slack,
            },
            Vec::new(),
            observed_at,
            Some(observed_at),
            0.4,
            None,
        )
        .unwrap();

        assert_eq!(source.scoring_class, ScoringClass::Context);
    }

    #[test]
    fn source_asof_roundtrip_preserves_known_unknown_warning_carrier() {
        let observed_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        let known = SourceAttribution::new(
            DataSource::Google,
            Vec::new(),
            observed_at,
            Some(observed_at),
            1.0,
            None,
        )
        .unwrap();
        let unknown = SourceAttribution::legacy_unattributed(observed_at).unwrap();

        let decoded_known: SourceAttribution =
            serde_json::from_value(serde_json::to_value(&known).unwrap()).unwrap();
        let decoded_unknown: SourceAttribution =
            serde_json::from_value(serde_json::to_value(&unknown).unwrap()).unwrap();

        assert_eq!(decoded_known.source_asof, Some(observed_at));
        assert_eq!(decoded_unknown.source_asof, None);
    }
}
