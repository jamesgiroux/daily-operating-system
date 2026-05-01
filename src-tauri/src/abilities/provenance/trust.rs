use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::envelope::ComposedProvenance;
use super::source::{DataSource, EntityId, SourceAttribution, SourceIndex};
use super::CompositionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TrustAssessment {
    pub effective: EffectiveTrust,
    pub contributions: Vec<TrustContribution>,
    pub contains_stored_synthesis: bool,
}

impl TrustAssessment {
    pub fn compute(
        sources: &[SourceAttribution],
        children: &[ComposedProvenance],
        has_prompt_fingerprint: bool,
    ) -> Self {
        let mut contributions = Vec::new();
        let mut contains_stored_synthesis = false;

        for (index, source) in sources.iter().enumerate() {
            let source_index = SourceIndex(index);
            let reason = if let Some((entity_id, field)) = source.stored_synthesis_entity_field() {
                contains_stored_synthesis = true;
                contributions.push(TrustContribution {
                    source: TrustContributionSource::StoredSynthesisField { entity_id, field },
                    reason: TrustReason::StoredSynthesis,
                });
                continue;
            } else if source.synthesis_marker.is_some() {
                contains_stored_synthesis = true;
                TrustReason::StoredSynthesis
            } else if matches!(source.data_source, DataSource::Ai) {
                TrustReason::DirectlyFromLLMSynthesis
            } else if source.data_source.is_structured_trusted_source() {
                TrustReason::DirectlyFromStructuredSource
            } else {
                TrustReason::UnboundedFreeText
            };

            contributions.push(TrustContribution {
                source: TrustContributionSource::DirectSource { source_index },
                reason,
            });
        }

        for child in children {
            if child.provenance.trust.contains_stored_synthesis {
                contains_stored_synthesis = true;
            }
            contributions.push(TrustContribution {
                source: TrustContributionSource::ComposedChild {
                    composition_id: child.composition_id.clone(),
                },
                reason: if child.provenance.trust.effective == EffectiveTrust::Untrusted {
                    TrustReason::ComposedUntrustedChild
                } else {
                    TrustReason::DirectlyFromStructuredSource
                },
            });
        }

        let untrusted = has_prompt_fingerprint
            || contributions.iter().any(|contribution| {
                matches!(
                    contribution.reason,
                    TrustReason::DirectlyFromLLMSynthesis
                        | TrustReason::ComposedUntrustedChild
                        | TrustReason::StoredSynthesis
                        | TrustReason::UnboundedFreeText
                )
            });

        Self {
            effective: if untrusted {
                EffectiveTrust::Untrusted
            } else {
                EffectiveTrust::Trusted
            },
            contributions,
            contains_stored_synthesis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveTrust {
    Trusted,
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TrustContribution {
    pub source: TrustContributionSource,
    pub reason: TrustReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustContributionSource {
    DirectSource { source_index: SourceIndex },
    ComposedChild { composition_id: CompositionId },
    StoredSynthesisField { entity_id: EntityId, field: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustReason {
    DirectlyFromStructuredSource,
    DirectlyFromLLMSynthesis,
    ComposedUntrustedChild,
    StoredSynthesis,
    UnboundedFreeText,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::provenance::{
        AbilityExecutionMode, AbilityVersion, Actor, FieldAttribution, FieldPath, InputsSnapshot,
        InvocationId, Provenance, SchemaVersion, SourceIdentifier, SubjectAttribution, SubjectRef,
    };
    use chrono::TimeZone;

    fn source(data_source: DataSource) -> SourceAttribution {
        let observed_at = chrono::Utc
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap();
        SourceAttribution::new(data_source, Vec::new(), observed_at, Some(observed_at), 1.0, None)
            .unwrap()
    }

    #[test]
    fn provenance_read_structured_sources_trusted() {
        let trust = TrustAssessment::compute(&[source(DataSource::Google)], &[], false);

        assert_eq!(trust.effective, EffectiveTrust::Trusted);
    }

    #[test]
    fn provenance_transform_with_prompt_fingerprint_untrusted() {
        let trust = TrustAssessment::compute(&[source(DataSource::Google)], &[], true);

        assert_eq!(trust.effective, EffectiveTrust::Untrusted);
    }

    #[test]
    fn provenance_contains_stored_synthesis_from_source_marker() {
        let observed_at = chrono::Utc
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap();
        let marker = crate::abilities::provenance::SynthesisMarker {
            producer_ability: "summarize_account".into(),
            producer_invocation_id: InvocationId::new("invocation-child"),
            produced_at: observed_at,
        };
        let source = SourceAttribution::new(
            DataSource::Google,
            vec![SourceIdentifier::Entity {
                entity_id: EntityId::new("acct-1"),
                field: Some("summary".into()),
            }],
            observed_at,
            Some(observed_at),
            1.0,
            Some(marker),
        )
        .unwrap();

        let trust = TrustAssessment::compute(&[source], &[], false);

        assert_eq!(trust.effective, EffectiveTrust::Untrusted);
        assert!(trust.contains_stored_synthesis);
    }

    #[test]
    fn provenance_publish_maintenance_inherit_weakest_child() {
        let observed_at = chrono::Utc
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap();
        let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
        let child = Provenance {
            provenance_schema_version: crate::abilities::provenance::PROVENANCE_SCHEMA_VERSION,
            ability_name: "transform_child".into(),
            ability_version: AbilityVersion::new(1, 0),
            ability_schema_version: SchemaVersion(1),
            invocation_id: InvocationId::new("child-invocation"),
            produced_at: observed_at,
            inputs_snapshot: InputsSnapshot::default(),
            actor: Actor::System {
                component: "fixture".into(),
            },
            mode: AbilityExecutionMode::Evaluate,
            trust: TrustAssessment {
                effective: EffectiveTrust::Untrusted,
                contributions: Vec::new(),
                contains_stored_synthesis: false,
            },
            sources: Vec::new(),
            thread_ids: Vec::new(),
            prompt_fingerprint: None,
            children: Vec::new(),
            field_attributions: std::collections::BTreeMap::from([(
                FieldPath::new("/name").unwrap(),
                FieldAttribution::constant(subject.clone()),
            )]),
            subject,
            warnings: Vec::new(),
        };
        let children = vec![ComposedProvenance::new(CompositionId::new("child_a"), child)];

        let trust = TrustAssessment::compute(&[], &children, false);

        assert_eq!(trust.effective, EffectiveTrust::Untrusted);
    }
}
