use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::envelope::CompositionId;
use super::source::SourceIndex;
use super::subject::SubjectAttribution;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct FieldPath(String);

impl FieldPath {
    pub fn root() -> Self {
        Self(String::new())
    }

    pub fn new(pointer: impl Into<String>) -> Result<Self, FieldAttributionError> {
        let pointer = pointer.into();
        if pointer.is_empty() || pointer.starts_with('/') {
            Ok(Self(pointer))
        } else {
            Err(FieldAttributionError::InvalidFieldPath)
        }
    }

    pub fn from_json_pointer(pointer: impl Into<String>) -> Result<Self, FieldAttributionError> {
        Self::new(pointer)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn covers(&self, leaf: &FieldPath) -> bool {
        if self.is_root() || self == leaf {
            return true;
        }

        leaf.as_str()
            .strip_prefix(self.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceRef {
    Source {
        source_index: SourceIndex,
    },
    Child {
        composition_id: CompositionId,
        field_path: FieldPath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DerivationKind {
    Direct,
    Composed { composition_id: CompositionId },
    Computed { algorithm: String },
    LlmSynthesis,
    Constant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Confidence {
    pub value: f32,
    pub kind: ConfidenceKind,
}

impl Confidence {
    pub fn declared(
        value: f32,
        _explanation: &SanitizedExplanation,
    ) -> Result<Self, FieldAttributionError> {
        Self::new(value, ConfidenceKind::Declared)
    }

    pub fn provider_reported(value: f32) -> Result<Self, FieldAttributionError> {
        Self::new(value, ConfidenceKind::ProviderReported)
    }

    pub fn composed_min(value: f32) -> Result<Self, FieldAttributionError> {
        Self::new(value, ConfidenceKind::ComposedMin)
    }

    pub fn computed(value: f32) -> Result<Self, FieldAttributionError> {
        Self::new(value, ConfidenceKind::Computed)
    }

    pub(crate) fn implicit() -> Self {
        Self {
            value: 1.0,
            kind: ConfidenceKind::Implicit,
        }
    }

    pub fn new(value: f32, kind: ConfidenceKind) -> Result<Self, FieldAttributionError> {
        if !(0.0..=1.0).contains(&value) || !value.is_finite() {
            return Err(FieldAttributionError::InvalidConfidence);
        }

        Ok(Self { value, kind })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceKind {
    Declared,
    ProviderReported,
    ComposedMin,
    Computed,
    Implicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SanitizedExplanation(String);

impl SanitizedExplanation {
    pub fn new(value: impl Into<String>) -> Result<Self, FieldAttributionError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(FieldAttributionError::EmptyExplanation);
        }

        let lowered = trimmed.to_ascii_lowercase();
        let blocked = ["ignore previous", "system:", "developer:", "assistant:"];
        if blocked.iter().any(|token| lowered.contains(token)) {
            return Err(FieldAttributionError::UnsafeExplanation);
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FieldAttribution {
    pub subject: SubjectAttribution,
    pub derivation: DerivationKind,
    pub source_refs: Vec<SourceRef>,
    pub confidence: Confidence,
    pub explanation: Option<SanitizedExplanation>,
}

impl FieldAttribution {
    pub fn new(
        subject: SubjectAttribution,
        derivation: DerivationKind,
        source_refs: Vec<SourceRef>,
        confidence: Confidence,
        explanation: Option<SanitizedExplanation>,
    ) -> Result<Self, FieldAttributionError> {
        Self::validate_shape(&derivation, &source_refs, &confidence, explanation.as_ref())?;
        Ok(Self {
            subject,
            derivation,
            source_refs,
            confidence,
            explanation,
        })
    }

    pub fn direct(subject: SubjectAttribution, source_index: SourceIndex) -> Self {
        Self {
            subject,
            derivation: DerivationKind::Direct,
            source_refs: vec![SourceRef::Source { source_index }],
            confidence: Confidence::implicit(),
            explanation: None,
        }
    }

    pub fn composed(
        subject: SubjectAttribution,
        composition_id: CompositionId,
        field_path: FieldPath,
        confidence: Confidence,
    ) -> Result<Self, FieldAttributionError> {
        Self::new(
            subject,
            DerivationKind::Composed {
                composition_id: composition_id.clone(),
            },
            vec![SourceRef::Child {
                composition_id,
                field_path,
            }],
            confidence,
            None,
        )
    }

    pub fn computed(
        subject: SubjectAttribution,
        algorithm: impl Into<String>,
        source_refs: Vec<SourceRef>,
        confidence: Confidence,
    ) -> Result<Self, FieldAttributionError> {
        Self::new(
            subject,
            DerivationKind::Computed {
                algorithm: algorithm.into(),
            },
            source_refs,
            confidence,
            None,
        )
    }

    pub fn llm_synthesis(
        subject: SubjectAttribution,
        source_refs: Vec<SourceRef>,
        confidence: Confidence,
        explanation: Option<SanitizedExplanation>,
    ) -> Result<Self, FieldAttributionError> {
        Self::new(
            subject,
            DerivationKind::LlmSynthesis,
            source_refs,
            confidence,
            explanation,
        )
    }

    pub fn constant(subject: SubjectAttribution) -> Self {
        Self {
            subject,
            derivation: DerivationKind::Constant,
            source_refs: Vec::new(),
            confidence: Confidence::implicit(),
            explanation: None,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), FieldAttributionError> {
        Self::validate_shape(
            &self.derivation,
            &self.source_refs,
            &self.confidence,
            self.explanation.as_ref(),
        )
    }

    fn validate_shape(
        derivation: &DerivationKind,
        source_refs: &[SourceRef],
        confidence: &Confidence,
        explanation: Option<&SanitizedExplanation>,
    ) -> Result<(), FieldAttributionError> {
        if matches!(confidence.kind, ConfidenceKind::Declared) && explanation.is_none() {
            return Err(FieldAttributionError::DeclaredConfidenceMissingExplanation);
        }

        match derivation {
            DerivationKind::Constant => Ok(()),
            DerivationKind::LlmSynthesis if source_refs.is_empty() => {
                Err(FieldAttributionError::LlmSynthesisMissingSourceRefs)
            }
            _ if source_refs.is_empty() => Err(FieldAttributionError::MissingSourceRefs),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FieldAttributionError {
    #[error("field path must be a JSON pointer or the empty root pointer")]
    InvalidFieldPath,
    #[error("confidence must be finite and within [0.0, 1.0]")]
    InvalidConfidence,
    #[error("declared confidence requires a sanitized explanation")]
    DeclaredConfidenceMissingExplanation,
    #[error("LLM synthesis attribution requires at least one source reference")]
    LlmSynthesisMissingSourceRefs,
    #[error("non-constant field attribution requires at least one source reference")]
    MissingSourceRefs,
    #[error("sanitized explanation cannot be empty")]
    EmptyExplanation,
    #[error("sanitized explanation contains unsafe instruction-like text")]
    UnsafeExplanation,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::provenance::{
        SubjectAttribution, SubjectBindingKind, SubjectFitAssessment, SubjectRef,
    };

    fn subject() -> SubjectAttribution {
        SubjectAttribution::new(
            SubjectRef::Account("acct-1".into()),
            SubjectBindingKind::DirectInput,
            Vec::new(),
            Vec::new(),
            SubjectFitAssessment::confident("fixture", 1.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn provenance_declared_confidence_requires_explanation() {
        let attr = FieldAttribution::new(
            subject(),
            DerivationKind::Direct,
            vec![SourceRef::Source {
                source_index: SourceIndex(0),
            }],
            Confidence {
                value: 0.8,
                kind: ConfidenceKind::Declared,
            },
            None,
        );

        assert_eq!(
            attr.unwrap_err(),
            FieldAttributionError::DeclaredConfidenceMissingExplanation
        );
    }

    #[test]
    fn provenance_llm_synthesis_requires_source_refs() {
        let explanation = SanitizedExplanation::new("Provider reported confidence").unwrap();
        let confidence = Confidence::declared(0.8, &explanation).unwrap();
        let attr = FieldAttribution::llm_synthesis(subject(), Vec::new(), confidence, Some(explanation));

        assert_eq!(
            attr.unwrap_err(),
            FieldAttributionError::LlmSynthesisMissingSourceRefs
        );
    }

    #[test]
    fn composition_refs_use_composition_id_not_position() {
        let child_path = FieldPath::new("/name").unwrap();
        let attr = FieldAttribution::composed(
            subject(),
            CompositionId::new("account_lookup"),
            child_path.clone(),
            Confidence::composed_min(0.9).unwrap(),
        )
        .unwrap();

        assert_eq!(
            attr.source_refs,
            vec![SourceRef::Child {
                composition_id: CompositionId::new("account_lookup"),
                field_path: child_path,
            }]
        );
    }
}
