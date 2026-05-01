use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::field::SourceRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectRef {
    Account(String),
    Project(String),
    Person(String),
    Meeting(String),
    User(String),
    Global,
    Multi(Vec<SubjectRef>),
    Unknown,
}

impl SubjectRef {
    pub fn matches_or_contains(&self, other: &SubjectRef) -> bool {
        self == other
            || matches!(self, SubjectRef::Multi(subjects) if subjects.iter().any(|s| s.matches_or_contains(other)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectBindingKind {
    DirectInput,
    Inherited,
    Inferred,
    SourceMatched,
    UserConfirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectFitStatus {
    Confident,
    Ambiguous,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubjectFitAssessment {
    pub status: SubjectFitStatus,
    pub confidence: f32,
    pub method: String,
}

impl SubjectFitAssessment {
    pub fn confident(
        method: impl Into<String>,
        confidence: f32,
    ) -> Result<Self, SubjectAttributionError> {
        Self::new(SubjectFitStatus::Confident, confidence, method)
    }

    pub fn ambiguous(
        method: impl Into<String>,
        confidence: f32,
    ) -> Result<Self, SubjectAttributionError> {
        Self::new(SubjectFitStatus::Ambiguous, confidence, method)
    }

    pub fn blocked(
        method: impl Into<String>,
        confidence: f32,
    ) -> Result<Self, SubjectAttributionError> {
        Self::new(SubjectFitStatus::Blocked, confidence, method)
    }

    fn new(
        status: SubjectFitStatus,
        confidence: f32,
        method: impl Into<String>,
    ) -> Result<Self, SubjectAttributionError> {
        if !(0.0..=1.0).contains(&confidence) || !confidence.is_finite() {
            return Err(SubjectAttributionError::InvalidFitConfidence);
        }

        let method = method.into();
        if method.trim().is_empty() {
            return Err(SubjectAttributionError::EmptyFitMethod);
        }

        Ok(Self {
            status,
            confidence,
            method,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompetingSubject {
    pub subject: SubjectRef,
    pub confidence: f32,
    pub reason: String,
}

impl CompetingSubject {
    pub fn new(
        subject: SubjectRef,
        confidence: f32,
        reason: impl Into<String>,
    ) -> Result<Self, SubjectAttributionError> {
        if !(0.0..=1.0).contains(&confidence) || !confidence.is_finite() {
            return Err(SubjectAttributionError::InvalidFitConfidence);
        }

        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(SubjectAttributionError::EmptyCompetingSubjectReason);
        }

        Ok(Self {
            subject,
            confidence,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubjectAttribution {
    pub subject: SubjectRef,
    pub binding: SubjectBindingKind,
    pub supporting_source_refs: Vec<SourceRef>,
    pub competing_subjects: Vec<CompetingSubject>,
    pub fit: SubjectFitAssessment,
}

impl SubjectAttribution {
    pub fn new(
        subject: SubjectRef,
        binding: SubjectBindingKind,
        supporting_source_refs: Vec<SourceRef>,
        competing_subjects: Vec<CompetingSubject>,
        fit: SubjectFitAssessment,
    ) -> Result<Self, SubjectAttributionError> {
        if matches!(subject, SubjectRef::Unknown) && matches!(fit.status, SubjectFitStatus::Confident) {
            return Err(SubjectAttributionError::UnknownSubjectCannotBeConfident);
        }

        Ok(Self {
            subject,
            binding,
            supporting_source_refs,
            competing_subjects,
            fit,
        })
    }

    pub fn direct_confident(subject: SubjectRef) -> Self {
        Self {
            subject,
            binding: SubjectBindingKind::DirectInput,
            supporting_source_refs: Vec::new(),
            competing_subjects: Vec::new(),
            fit: SubjectFitAssessment {
                status: SubjectFitStatus::Confident,
                confidence: 1.0,
                method: "direct_input".into(),
            },
        }
    }

    pub fn is_confident(&self) -> bool {
        matches!(self.fit.status, SubjectFitStatus::Confident)
    }

    pub fn is_coherent_with(&self, envelope_subject: &SubjectAttribution) -> bool {
        envelope_subject.subject.matches_or_contains(&self.subject)
            || self.subject.matches_or_contains(&envelope_subject.subject)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SubjectAttributionError {
    #[error("subject fit confidence must be finite and within [0.0, 1.0]")]
    InvalidFitConfidence,
    #[error("subject fit method cannot be empty")]
    EmptyFitMethod,
    #[error("competing subject reason cannot be empty")]
    EmptyCompetingSubjectReason,
    #[error("unknown subject cannot be marked confident")]
    UnknownSubjectCannotBeConfident,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_finalize_rejects_ambiguous_subject_fit_subject_side() {
        let subject = SubjectAttribution::new(
            SubjectRef::Account("acct-1".into()),
            SubjectBindingKind::Inferred,
            Vec::new(),
            Vec::new(),
            SubjectFitAssessment::ambiguous("fixture", 0.4).unwrap(),
        )
        .unwrap();

        assert!(!subject.is_confident());
    }

    #[test]
    fn multi_subject_matches_member_subjects() {
        let envelope = SubjectAttribution::direct_confident(SubjectRef::Multi(vec![
            SubjectRef::Account("acct-1".into()),
            SubjectRef::Meeting("meeting-x".into()),
        ]));
        let field = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));

        assert!(field.is_coherent_with(&envelope));
    }
}
