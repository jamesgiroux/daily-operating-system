//! `RecommendationClaim` implementation surface — write helpers,
//! supersession logic, and the typed bridge from the salience
//! engine's scored candidate to a committable claim payload for
//! `services::claims::commit_claim`.

use serde_json::json;

use abilities_runtime::abilities::provenance::subject::SubjectRef;

use super::contracts::{
    ConversionState, RecommendationDraft, RecommendationMetadataEnvelope,
    RecommendationMetadataPayload, RecommendedAction, RECOMMENDATION_METADATA_SCHEMA_VERSION,
};
use crate::abilities::claims::ClaimType;
use crate::services::claims::ClaimProposal;

const RECOMMENDATION_ACTOR: &str = "agent";
const RECOMMENDATION_DATA_SOURCE: &str = "recommendation";
const CUSTOM_ACTION_KIND_MAX_BYTES: usize = 64;

pub fn metadata_envelope(draft: &RecommendationDraft) -> RecommendationMetadataEnvelope {
    RecommendationMetadataEnvelope {
        recommendation: RecommendationMetadataPayload {
            schema_version: RECOMMENDATION_METADATA_SCHEMA_VERSION,
            recommended_action: draft.recommended_action.clone(),
            evidence: draft.evidence.clone(),
            salience: draft.salience.clone(),
            feedback_state: super::contracts::FeedbackState::Pending,
            conversion_state: ConversionState::NotConverted,
        },
    }
}

pub fn action_key(action: &RecommendedAction) -> Result<String, RecommendationProposalError> {
    match action {
        RecommendedAction::ScheduleMeeting { .. } => Ok("scheduleMeeting".to_string()),
        RecommendedAction::SendMessage { .. } => Ok("sendMessage".to_string()),
        RecommendedAction::ReviewClaim { .. } => Ok("reviewClaim".to_string()),
        RecommendedAction::UpdateRecord { .. } => Ok("updateRecord".to_string()),
        RecommendedAction::InvestigateChange { .. } => Ok("investigateChange".to_string()),
        RecommendedAction::Custom { action_kind, .. } => {
            validate_custom_action_kind(action_kind)?;
            Ok(format!("custom.{action_kind}"))
        }
    }
}

pub fn claim_proposal_from_draft(
    draft: RecommendationDraft,
) -> Result<ClaimProposal, RecommendationProposalError> {
    let action_key = action_key(&draft.recommended_action)?;
    let subject_ref = subject_ref_for_commit_claim(&draft.subject)?;
    let metadata_json = serde_json::to_string(&metadata_envelope(&draft))?;

    Ok(ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref,
        claim_type: ClaimType::Recommendation.as_str().to_string(),
        field_path: Some(format!("recommendation.{action_key}")),
        topic_key: Some(action_key),
        text: draft.text,
        actor: RECOMMENDATION_ACTOR.to_string(),
        data_source: RECOMMENDATION_DATA_SOURCE.to_string(),
        source_ref: draft.source_ref,
        source_asof: draft.source_asof.map(|dt| dt.to_rfc3339()),
        observed_at: draft.observed_at.to_rfc3339(),
        provenance_json: draft.provenance_json,
        metadata_json: Some(metadata_json),
        thread_id: None,
        temporal_scope: None,
        sensitivity: None,
        supersedes: None,
        tombstone: None,
    })
}

fn subject_ref_for_commit_claim(
    subject: &SubjectRef,
) -> Result<String, RecommendationProposalError> {
    let value = match subject {
        SubjectRef::Account(id) => subject_json("account", id)?,
        SubjectRef::Project(id) => subject_json("project", id)?,
        SubjectRef::Person(id) => subject_json("person", id)?,
        SubjectRef::Action(_) => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "action" });
        }
        SubjectRef::Meeting(_) => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "meeting" });
        }
        SubjectRef::User(_) => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "user" });
        }
        SubjectRef::Global => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "global" });
        }
        SubjectRef::Multi(_) => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "multi" });
        }
        SubjectRef::Unknown => {
            return Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "unknown" });
        }
    };

    Ok(value.to_string())
}

fn subject_json(
    kind: &'static str,
    id: &str,
) -> Result<serde_json::Value, RecommendationProposalError> {
    if id.trim().is_empty() {
        return Err(RecommendationProposalError::EmptySubjectId { kind });
    }

    Ok(json!({ "kind": kind, "id": id }))
}

fn validate_custom_action_kind(action_kind: &str) -> Result<(), RecommendationProposalError> {
    if action_kind.is_empty() {
        return Err(RecommendationProposalError::EmptyCustomActionKind);
    }

    if action_kind.len() > CUSTOM_ACTION_KIND_MAX_BYTES {
        return Err(RecommendationProposalError::CustomActionKindTooLong {
            max: CUSTOM_ACTION_KIND_MAX_BYTES,
            actual: action_kind.len(),
        });
    }

    let invalid = action_kind.chars().any(|ch| {
        !ch.is_ascii()
            || ch.is_ascii_whitespace()
            || ch.is_ascii_control()
            || matches!(ch, '/' | '\\')
    });
    if invalid {
        return Err(RecommendationProposalError::InvalidCustomActionKind {
            action_kind: action_kind.to_string(),
        });
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RecommendationProposalError {
    #[error("custom recommendation action kind cannot be empty")]
    EmptyCustomActionKind,
    #[error("custom recommendation action kind is {actual} bytes; maximum is {max}")]
    CustomActionKindTooLong { max: usize, actual: usize },
    #[error("custom recommendation action kind contains unsupported characters: {action_kind}")]
    InvalidCustomActionKind { action_kind: String },
    #[error("recommendation subject kind {kind} is not supported by ClaimType::Recommendation")]
    UnsupportedSubjectKind { kind: &'static str },
    #[error("recommendation subject kind {kind} has an empty id")]
    EmptySubjectId { kind: &'static str },
    #[error("failed to serialize recommendation metadata envelope: {0}")]
    SerializeMetadata(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Utc};
    use serde_json::Value;

    use crate::services::recommendations::contracts::{
        EvidenceRef, FactorRationale, SalienceFactor, SalienceFactorKind, SalienceScore,
    };

    fn fixture_time() -> DateTime<Utc> {
        "2026-05-26T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture time parses")
    }

    fn draft_with_action(recommended_action: RecommendedAction) -> RecommendationDraft {
        RecommendationDraft {
            subject: SubjectRef::Account("acct-example".to_string()),
            recommended_action,
            evidence: vec![EvidenceRef {
                source: "claim:claim-source-1".to_string(),
                chunk: None,
            }],
            provenance_json: serde_json::json!({ "abilityName": "recommend_for_entity" })
                .to_string(),
            source_ref: Some("run:recommendation-1".to_string()),
            source_asof: Some(fixture_time()),
            observed_at: fixture_time(),
            text: "Schedule follow-up with the account.".to_string(),
            salience: SalienceScore {
                total: 0.5,
                factors: vec![SalienceFactor {
                    kind: SalienceFactorKind::Importance,
                    value: Some(0.5),
                    weight: 0.14,
                    rationale: FactorRationale::Importance {
                        trust_band:
                            abilities_runtime::abilities::trust::types::TrustBand::LikelyCurrent,
                        source_authority: 0.8,
                    },
                }],
            },
        }
    }

    #[test]
    fn draft_to_claim_proposal_uses_claim_substrate_shape() {
        let draft = draft_with_action(RecommendedAction::ScheduleMeeting {
            entity_id: "acct-example".to_string(),
            when_window: "next_week".to_string(),
            rationale: "recent support trend needs follow-up".to_string(),
        });

        let proposal = claim_proposal_from_draft(draft).expect("draft converts");

        assert_eq!(proposal.id, None);
        assert_eq!(proposal.expected_claim_version, None);
        assert_eq!(proposal.claim_type, "recommendation");
        assert_eq!(
            proposal.field_path.as_deref(),
            Some("recommendation.scheduleMeeting")
        );
        assert_eq!(proposal.topic_key.as_deref(), Some("scheduleMeeting"));
        assert_eq!(proposal.actor, "agent");
        assert_eq!(proposal.data_source, "recommendation");
        assert_eq!(proposal.source_ref.as_deref(), Some("run:recommendation-1"));
        assert_eq!(
            proposal.source_asof.as_deref(),
            Some("2026-05-26T12:00:00+00:00")
        );
        assert_eq!(proposal.observed_at, "2026-05-26T12:00:00+00:00");
        assert_eq!(proposal.temporal_scope, None);
        assert_eq!(proposal.sensitivity, None);
        assert_eq!(proposal.supersedes, None);
        assert!(proposal.tombstone.is_none());
        assert_eq!(
            serde_json::from_str::<Value>(&proposal.subject_ref).expect("subject json"),
            serde_json::json!({ "kind": "account", "id": "acct-example" })
        );

        let metadata = serde_json::from_str::<Value>(proposal.metadata_json.as_deref().unwrap())
            .expect("metadata json");
        assert_eq!(
            metadata["recommendation"]["recommendedAction"]["kind"],
            "scheduleMeeting"
        );
        assert_eq!(metadata["recommendation"]["feedbackState"], "pending");
        assert_eq!(
            metadata["recommendation"]["conversionState"],
            serde_json::json!({ "kind": "notConverted" })
        );
    }

    #[test]
    fn custom_action_key_accepts_safe_opaque_token() {
        let action = RecommendedAction::Custom {
            action_kind: "open_loop_42".to_string(),
            payload: serde_json::json!({ "opaque": true }),
        };

        assert_eq!(action_key(&action).unwrap(), "custom.open_loop_42");
    }

    #[test]
    fn custom_action_key_rejects_empty_or_unsafe_tokens() {
        for action_kind in ["", "has space", "bad/slash", "bad\\slash", "unicodé"] {
            let action = RecommendedAction::Custom {
                action_kind: action_kind.to_string(),
                payload: serde_json::json!({}),
            };

            assert!(
                action_key(&action).is_err(),
                "{action_kind:?} should reject"
            );
        }
    }

    #[test]
    fn draft_to_claim_proposal_rejects_subjects_outside_recommendation_registry() {
        let mut draft = draft_with_action(RecommendedAction::InvestigateChange {
            entity_id: "meeting-1".to_string(),
            change_summary: "new meeting evidence".to_string(),
        });
        draft.subject = SubjectRef::Meeting("meeting-1".to_string());

        assert!(matches!(
            claim_proposal_from_draft(draft),
            Err(RecommendationProposalError::UnsupportedSubjectKind { kind: "meeting" })
        ));
    }
}
