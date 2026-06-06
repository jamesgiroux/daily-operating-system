//! Feedback loop.
//!
//! Wires user feedback (accepted / dismissed / not-useful /
//! too-noisy / converted) on a `Recommendation` claim into the
//! existing ADR-0123 claim feedback substrate and recommendation
//! metadata state. Does not introduce a parallel feedback path.

use abilities_runtime::abilities::recommendations::contracts as runtime;
use abilities_runtime::abilities::registry::Actor;
use chrono::{DateTime, Utc};

use crate::abilities::claims::ClaimType;
use crate::abilities::feedback::FeedbackAction;
use crate::db::ActionDb;
use crate::services::claims::{
    load_claim_by_id, record_claim_feedback, update_recommendation_feedback_metadata,
    with_claim_transaction, ClaimError, ClaimFeedbackInput,
};
use crate::services::context::ServiceContext;
use crate::services::recommendations::contracts as app;

/// Record a user's decision on a recommendation claim.
///
/// Mapping is intentionally the locked v1.4.6 W4-A table: recommendation
/// decisions reuse ADR-0123 `claim_feedback` actions, while conversion IDs are
/// stored only in `metadata_json.recommendation.conversionState`.
pub fn record_recommendation_feedback(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: runtime::ClaimId,
    decision: runtime::RecommendationFeedbackDecision,
    context: runtime::RecommendationFeedbackContext,
    actor: &Actor,
) -> Result<runtime::SubmitRecommendationFeedbackResponse, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ClaimError::Mode(error.to_string()))?;

    let recorded_at = ctx.clock.now();
    let mapping = map_feedback_decision(&decision, &context)?;
    let decided_state = runtime::FeedbackState::Decided(decision.clone());
    let app_decided_state = app::FeedbackState::Decided(runtime_decision_to_app(&decision)?);
    let app_conversion_state = runtime_conversion_state_to_app(&mapping.conversion_state);
    let feedback_state_json = serde_json::to_string(&app_decided_state)?;
    let conversion_state_json = serde_json::to_string(&app_conversion_state)?;

    with_claim_transaction(db, |tx| {
        let claim = load_claim_by_id(tx.conn_ref(), &claim_id.0)?
            .ok_or_else(|| ClaimError::UnknownClaimId(claim_id.0.clone()))?;
        if claim.claim_type != ClaimType::Recommendation.as_str() {
            return Err(ClaimError::UnsupportedClaimType {
                claim_id: claim_id.0.clone(),
                claim_type: claim.claim_type,
            });
        }

        parse_recommendation_metadata(claim.metadata_json.as_deref())?;

        let rows_affected = update_recommendation_feedback_metadata(
            tx,
            &claim_id.0,
            &feedback_state_json,
            &conversion_state_json,
        )?;

        if rows_affected == 0 {
            return no_mutation_response(tx, &claim_id, recorded_at);
        }

        if let Some(action) = mapping.action {
            record_claim_feedback(
                ctx,
                tx,
                ClaimFeedbackInput {
                    claim_id: claim_id.0.clone(),
                    action,
                    actor: "user".to_string(),
                    actor_id: actor_id_for_feedback(actor),
                    payload_json: mapping.payload_json,
                },
            )?;
        }

        Ok(runtime::SubmitRecommendationFeedbackResponse {
            schema_version: runtime::SUBMIT_RECOMMENDATION_FEEDBACK_SCHEMA_VERSION,
            claim_id,
            feedback_state: decided_state,
            conversion_state: mapping.conversion_state,
            effect_kind: runtime::EffectKind::ClaimFeedbackRecorded,
            recorded_at,
        })
    })
}

struct FeedbackMapping {
    action: Option<FeedbackAction>,
    payload_json: Option<String>,
    conversion_state: runtime::ConversionState,
}

fn map_feedback_decision(
    decision: &runtime::RecommendationFeedbackDecision,
    context: &runtime::RecommendationFeedbackContext,
) -> Result<FeedbackMapping, ClaimError> {
    let conversion_state = conversion_state_for_decision(decision);
    let (action, payload_json) = match decision {
        runtime::RecommendationFeedbackDecision::Accept { .. } => {
            (Some(FeedbackAction::ConfirmCurrent), empty_payload())
        }
        runtime::RecommendationFeedbackDecision::Dismiss { reason, .. } => match reason {
            runtime::DismissReason::NotRelevant | runtime::DismissReason::AlreadyKnew => (
                Some(FeedbackAction::NotRelevantHere),
                Some(not_relevant_payload(context, None)),
            ),
            runtime::DismissReason::WrongSubject => {
                (Some(FeedbackAction::WrongSubject), empty_payload())
            }
            runtime::DismissReason::Other(note) => (
                Some(FeedbackAction::NotRelevantHere),
                Some(not_relevant_payload(context, Some(note.as_str()))),
            ),
        },
        runtime::RecommendationFeedbackDecision::NotUseful { .. }
        | runtime::RecommendationFeedbackDecision::TooNoisy { .. } => (
            Some(FeedbackAction::SurfaceInappropriate),
            Some(
                serde_json::json!({ "surface": render_surface_name(context.surface)? }).to_string(),
            ),
        ),
        runtime::RecommendationFeedbackDecision::Convert { into, .. } => match into {
            runtime::ConversionTarget::Action(_) => {
                (Some(FeedbackAction::ConfirmCurrent), empty_payload())
            }
            runtime::ConversionTarget::ClaimCorrection(claim_id) => (
                Some(FeedbackAction::NeedsNuance),
                Some(serde_json::json!({ "corrected_text": &claim_id.0 }).to_string()),
            ),
            runtime::ConversionTarget::ReviewQueue(_) => (None, None),
        },
    };

    Ok(FeedbackMapping {
        action,
        payload_json,
        conversion_state,
    })
}

fn empty_payload() -> Option<String> {
    Some(serde_json::json!({}).to_string())
}

fn not_relevant_payload(
    context: &runtime::RecommendationFeedbackContext,
    note: Option<&str>,
) -> String {
    let mut payload = serde_json::json!({
        "invocation_id": &context.invocation_id,
    });
    if let Some(note) = note {
        payload["note"] = serde_json::Value::String(note.to_string());
    }
    payload.to_string()
}

fn render_surface_name(
    surface: abilities_runtime::sensitivity::RenderSurface,
) -> Result<String, ClaimError> {
    let value = serde_json::to_value(surface)?;
    value.as_str().map(str::to_string).ok_or_else(|| {
        ClaimError::InvalidFeedback("surface did not serialize as a string".to_string())
    })
}

fn conversion_state_for_decision(
    decision: &runtime::RecommendationFeedbackDecision,
) -> runtime::ConversionState {
    match decision {
        runtime::RecommendationFeedbackDecision::Convert { into, .. } => match into {
            runtime::ConversionTarget::Action(action_id) => {
                runtime::ConversionState::ConvertedToAction {
                    action_id: action_id.clone(),
                }
            }
            runtime::ConversionTarget::ClaimCorrection(claim_id) => {
                runtime::ConversionState::ConvertedToClaimCorrection {
                    claim_id: claim_id.clone(),
                }
            }
            runtime::ConversionTarget::ReviewQueue(queue_item_id) => {
                runtime::ConversionState::ConvertedToReviewQueue {
                    queue_item_id: queue_item_id.clone(),
                }
            }
        },
        _ => runtime::ConversionState::NotConverted,
    }
}

fn actor_id_for_feedback(actor: &Actor) -> Option<String> {
    match actor {
        Actor::SurfaceClient { instance, .. } => Some(instance.to_string()),
        _ => None,
    }
}

fn no_mutation_response(
    tx: &ActionDb,
    claim_id: &runtime::ClaimId,
    recorded_at: DateTime<Utc>,
) -> Result<runtime::SubmitRecommendationFeedbackResponse, ClaimError> {
    let claim = load_claim_by_id(tx.conn_ref(), &claim_id.0)?
        .ok_or_else(|| ClaimError::UnknownClaimId(claim_id.0.clone()))?;
    let metadata = parse_recommendation_metadata(claim.metadata_json.as_deref())?;
    Ok(runtime::SubmitRecommendationFeedbackResponse {
        schema_version: runtime::SUBMIT_RECOMMENDATION_FEEDBACK_SCHEMA_VERSION,
        claim_id: claim_id.clone(),
        feedback_state: app_feedback_state_to_runtime(metadata.feedback_state)?,
        conversion_state: app_conversion_state_to_runtime(metadata.conversion_state),
        effect_kind: runtime::EffectKind::NoMutation,
        recorded_at,
    })
}

fn parse_recommendation_metadata(
    metadata_json: Option<&str>,
) -> Result<app::RecommendationMetadataPayload, ClaimError> {
    let raw = metadata_json
        .map(str::trim)
        .filter(|metadata| !metadata.is_empty())
        .ok_or_else(|| {
            ClaimError::InvalidFeedback("recommendation claim missing metadata_json".to_string())
        })?;
    let envelope: app::RecommendationMetadataEnvelope =
        serde_json::from_str(raw).map_err(|error| {
            ClaimError::InvalidFeedback(format!(
                "recommendation metadata_json did not match schema: {error}"
            ))
        })?;
    Ok(envelope.recommendation)
}

fn runtime_decision_to_app(
    decision: &runtime::RecommendationFeedbackDecision,
) -> Result<app::RecommendationFeedbackDecision, ClaimError> {
    Ok(match decision {
        runtime::RecommendationFeedbackDecision::Accept { at } => {
            app::RecommendationFeedbackDecision::Accept { at: *at }
        }
        runtime::RecommendationFeedbackDecision::Dismiss { at, reason } => {
            app::RecommendationFeedbackDecision::Dismiss {
                at: *at,
                reason: runtime_dismiss_reason_to_app(reason)?,
            }
        }
        runtime::RecommendationFeedbackDecision::NotUseful { at } => {
            app::RecommendationFeedbackDecision::NotUseful { at: *at }
        }
        runtime::RecommendationFeedbackDecision::TooNoisy { at } => {
            app::RecommendationFeedbackDecision::TooNoisy { at: *at }
        }
        runtime::RecommendationFeedbackDecision::Convert { at, into } => {
            app::RecommendationFeedbackDecision::Convert {
                at: *at,
                into: runtime_conversion_target_to_app(into),
            }
        }
    })
}

fn runtime_dismiss_reason_to_app(
    reason: &runtime::DismissReason,
) -> Result<app::DismissReason, ClaimError> {
    Ok(match reason {
        runtime::DismissReason::NotRelevant => app::DismissReason::NotRelevant,
        runtime::DismissReason::AlreadyKnew => app::DismissReason::AlreadyKnew,
        runtime::DismissReason::WrongSubject => app::DismissReason::WrongSubject,
        runtime::DismissReason::Other(note) => app::DismissReason::Other(
            app::BoundedNote::try_from(note.as_str().to_string())
                .map_err(|error| ClaimError::InvalidFeedback(error.to_string()))?,
        ),
    })
}

fn runtime_conversion_target_to_app(target: &runtime::ConversionTarget) -> app::ConversionTarget {
    match target {
        runtime::ConversionTarget::Action(action_id) => {
            app::ConversionTarget::Action(action_id.clone())
        }
        runtime::ConversionTarget::ClaimCorrection(claim_id) => {
            app::ConversionTarget::ClaimCorrection(app::ClaimId(claim_id.0.clone()))
        }
        runtime::ConversionTarget::ReviewQueue(queue_item_id) => {
            app::ConversionTarget::ReviewQueue(queue_item_id.clone())
        }
    }
}

fn runtime_conversion_state_to_app(state: &runtime::ConversionState) -> app::ConversionState {
    match state {
        runtime::ConversionState::NotConverted => app::ConversionState::NotConverted,
        runtime::ConversionState::ConvertedToAction { action_id } => {
            app::ConversionState::ConvertedToAction {
                action_id: action_id.clone(),
            }
        }
        runtime::ConversionState::ConvertedToClaimCorrection { claim_id } => {
            app::ConversionState::ConvertedToClaimCorrection {
                claim_id: app::ClaimId(claim_id.0.clone()),
            }
        }
        runtime::ConversionState::ConvertedToReviewQueue { queue_item_id } => {
            app::ConversionState::ConvertedToReviewQueue {
                queue_item_id: queue_item_id.clone(),
            }
        }
    }
}

fn app_feedback_state_to_runtime(
    state: app::FeedbackState,
) -> Result<runtime::FeedbackState, ClaimError> {
    Ok(match state {
        app::FeedbackState::Pending => runtime::FeedbackState::Pending,
        app::FeedbackState::Decided(decision) => {
            runtime::FeedbackState::Decided(app_decision_to_runtime(decision)?)
        }
    })
}

fn app_decision_to_runtime(
    decision: app::RecommendationFeedbackDecision,
) -> Result<runtime::RecommendationFeedbackDecision, ClaimError> {
    Ok(match decision {
        app::RecommendationFeedbackDecision::Accept { at } => {
            runtime::RecommendationFeedbackDecision::Accept { at }
        }
        app::RecommendationFeedbackDecision::Dismiss { at, reason } => {
            runtime::RecommendationFeedbackDecision::Dismiss {
                at,
                reason: app_dismiss_reason_to_runtime(reason)?,
            }
        }
        app::RecommendationFeedbackDecision::NotUseful { at } => {
            runtime::RecommendationFeedbackDecision::NotUseful { at }
        }
        app::RecommendationFeedbackDecision::TooNoisy { at } => {
            runtime::RecommendationFeedbackDecision::TooNoisy { at }
        }
        app::RecommendationFeedbackDecision::Convert { at, into } => {
            runtime::RecommendationFeedbackDecision::Convert {
                at,
                into: app_conversion_target_to_runtime(into),
            }
        }
    })
}

fn app_dismiss_reason_to_runtime(
    reason: app::DismissReason,
) -> Result<runtime::DismissReason, ClaimError> {
    Ok(match reason {
        app::DismissReason::NotRelevant => runtime::DismissReason::NotRelevant,
        app::DismissReason::AlreadyKnew => runtime::DismissReason::AlreadyKnew,
        app::DismissReason::WrongSubject => runtime::DismissReason::WrongSubject,
        app::DismissReason::Other(note) => runtime::DismissReason::Other(
            runtime::BoundedNote::try_from(note.as_str().to_string())
                .map_err(|error| ClaimError::InvalidFeedback(error.to_string()))?,
        ),
    })
}

fn app_conversion_target_to_runtime(target: app::ConversionTarget) -> runtime::ConversionTarget {
    match target {
        app::ConversionTarget::Action(action_id) => runtime::ConversionTarget::Action(action_id),
        app::ConversionTarget::ClaimCorrection(claim_id) => {
            runtime::ConversionTarget::ClaimCorrection(runtime::ClaimId(claim_id.0))
        }
        app::ConversionTarget::ReviewQueue(queue_item_id) => {
            runtime::ConversionTarget::ReviewQueue(queue_item_id)
        }
    }
}

fn app_conversion_state_to_runtime(state: app::ConversionState) -> runtime::ConversionState {
    match state {
        app::ConversionState::NotConverted => runtime::ConversionState::NotConverted,
        app::ConversionState::ConvertedToAction { action_id } => {
            runtime::ConversionState::ConvertedToAction { action_id }
        }
        app::ConversionState::ConvertedToClaimCorrection { claim_id } => {
            runtime::ConversionState::ConvertedToClaimCorrection {
                claim_id: runtime::ClaimId(claim_id.0),
            }
        }
        app::ConversionState::ConvertedToReviewQueue { queue_item_id } => {
            runtime::ConversionState::ConvertedToReviewQueue { queue_item_id }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use abilities_runtime::abilities::provenance::SubjectRef;
    use abilities_runtime::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
    use abilities_runtime::sensitivity::RenderSurface;
    use chrono::TimeZone;
    use rusqlite::params;
    use serde_json::json;

    use crate::db::ActionDb;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::services::recommendations::recommendation::metadata_envelope;

    #[test]
    fn recommendation_feedback_mapping_enumerates_all_locked_rows() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        for case in mapping_cases() {
            let claim_id = format!("claim-{}", case.name);
            seed_recommendation_claim(&db, &claim_id, app::FeedbackState::Pending);

            let response = record_recommendation_feedback(
                &ctx,
                &db,
                runtime::ClaimId(claim_id.clone()),
                case.decision.clone(),
                feedback_context(),
                &Actor::User,
            )
            .expect("feedback write succeeds");

            assert_eq!(
                response.effect_kind,
                runtime::EffectKind::ClaimFeedbackRecorded,
                "{}",
                case.name
            );
            assert_eq!(response.conversion_state, case.expected_conversion);

            let rows = claim_feedback_rows(&db, &claim_id);
            match case.expected_action {
                Some(action) => {
                    assert_eq!(rows.len(), 1, "{}", case.name);
                    assert_eq!(rows[0].feedback_type, action.as_str(), "{}", case.name);
                    let payload = rows[0]
                        .payload_json
                        .as_deref()
                        .map(|raw| serde_json::from_str::<serde_json::Value>(raw).unwrap())
                        .unwrap_or(serde_json::Value::Null);
                    assert_eq!(payload, case.expected_payload, "{}", case.name);
                }
                None => assert!(rows.is_empty(), "{}", case.name),
            }

            let metadata = recommendation_metadata_value(&db, &claim_id);
            assert_ne!(
                metadata["recommendation"]["feedbackState"],
                serde_json::Value::String("pending".to_string()),
                "{}",
                case.name
            );
            assert_eq!(
                metadata["recommendation"]["conversionState"],
                serde_json::to_value(&case.expected_conversion).unwrap(),
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn recommendation_feedback_is_idempotent_after_decision() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        seed_recommendation_claim(&db, "claim-idempotent", app::FeedbackState::Pending);

        let decision = runtime::RecommendationFeedbackDecision::Accept { at: at() };
        let first = record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-idempotent".to_string()),
            decision.clone(),
            feedback_context(),
            &Actor::User,
        )
        .expect("first feedback succeeds");
        assert_eq!(
            first.effect_kind,
            runtime::EffectKind::ClaimFeedbackRecorded
        );

        let second = record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-idempotent".to_string()),
            decision,
            feedback_context(),
            &Actor::User,
        )
        .expect("second feedback is idempotent");

        assert_eq!(second.effect_kind, runtime::EffectKind::NoMutation);
        assert_eq!(claim_feedback_rows(&db, "claim-idempotent").len(), 1);
    }

    #[test]
    fn recommendation_feedback_populates_actor_id_only_for_surface_client() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        seed_recommendation_claim(&db, "claim-user-actor", app::FeedbackState::Pending);
        seed_recommendation_claim(&db, "claim-surface-actor", app::FeedbackState::Pending);

        record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-user-actor".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &Actor::User,
        )
        .expect("user feedback succeeds");
        record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-surface-actor".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &surface_actor(),
        )
        .expect("surface feedback succeeds");

        let user_rows = claim_feedback_rows(&db, "claim-user-actor");
        let surface_rows = claim_feedback_rows(&db, "claim-surface-actor");
        assert_eq!(user_rows[0].actor, "user");
        assert_eq!(user_rows[0].actor_id, None);
        assert_eq!(surface_rows[0].actor, "user");
        assert_eq!(surface_rows[0].actor_id.as_deref(), Some("surface-1"));
    }

    #[test]
    fn w4_surface_client_recommendation_feedback_cannot_direct_write_claim_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external)
            .with_actor("surface_client")
            .with_ability_id(runtime::SUBMIT_RECOMMENDATION_FEEDBACK_ABILITY_NAME);
        seed_recommendation_claim(&db, "claim-surface-direct", app::FeedbackState::Pending);

        let error = record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-surface-direct".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &surface_actor(),
        )
        .expect_err("surface client recommendation feedback needs verified delegation");

        assert!(
            matches!(error, ClaimError::Mode(_)),
            "surface client direct feedback should fail at service actor boundary, got {error:?}",
        );
        assert!(
            claim_feedback_rows(&db, "claim-surface-direct").is_empty(),
            "rejected surface recommendation feedback must not create claim_feedback rows",
        );
    }

    #[test]
    fn recommendation_feedback_rejects_unknown_claim_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let error = record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("missing-claim".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &Actor::User,
        )
        .expect_err("unknown claim rejected");

        assert!(matches!(error, ClaimError::UnknownClaimId(id) if id == "missing-claim"));
    }

    #[test]
    fn recommendation_feedback_rejects_non_recommendation_claim_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        seed_claim_row(&db, "claim-risk", "risk", None);

        let error = record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-risk".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &Actor::User,
        )
        .expect_err("non-recommendation claim rejected");

        assert!(matches!(
            error,
            ClaimError::UnsupportedClaimType {
                claim_id,
                claim_type,
            } if claim_id == "claim-risk" && claim_type == "risk"
        ));
    }

    #[test]
    fn recommendation_feedback_reuses_claim_feedback_recorded_signal() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        seed_recommendation_claim(&db, "claim-signal", app::FeedbackState::Pending);

        record_recommendation_feedback(
            &ctx,
            &db,
            runtime::ClaimId("claim-signal".to_string()),
            runtime::RecommendationFeedbackDecision::Accept { at: at() },
            feedback_context(),
            &Actor::User,
        )
        .expect("feedback succeeds");

        assert_eq!(signal_count(&db, "claim_feedback_recorded"), 1);
        let payload = first_signal_value(&db, "claim_feedback_recorded");
        let payload_json: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload_json["action"], "confirm_current");
        assert_eq!(payload_json["claim_id"], "claim-signal");
        assert_eq!(payload_json["verification_state_before"], "active");
        assert_eq!(payload_json["verification_state_after"], "active");
    }

    struct MappingCase {
        name: &'static str,
        decision: runtime::RecommendationFeedbackDecision,
        expected_action: Option<FeedbackAction>,
        expected_payload: serde_json::Value,
        expected_conversion: runtime::ConversionState,
    }

    fn mapping_cases() -> Vec<MappingCase> {
        vec![
            MappingCase {
                name: "accept",
                decision: runtime::RecommendationFeedbackDecision::Accept { at: at() },
                expected_action: Some(FeedbackAction::ConfirmCurrent),
                expected_payload: json!({}),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "dismiss-not-relevant",
                decision: runtime::RecommendationFeedbackDecision::Dismiss {
                    at: at(),
                    reason: runtime::DismissReason::NotRelevant,
                },
                expected_action: Some(FeedbackAction::NotRelevantHere),
                expected_payload: json!({ "invocation_id": "invocation-1" }),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "dismiss-already-knew",
                decision: runtime::RecommendationFeedbackDecision::Dismiss {
                    at: at(),
                    reason: runtime::DismissReason::AlreadyKnew,
                },
                expected_action: Some(FeedbackAction::NotRelevantHere),
                expected_payload: json!({ "invocation_id": "invocation-1" }),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "dismiss-wrong-subject",
                decision: runtime::RecommendationFeedbackDecision::Dismiss {
                    at: at(),
                    reason: runtime::DismissReason::WrongSubject,
                },
                expected_action: Some(FeedbackAction::WrongSubject),
                expected_payload: json!({}),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "dismiss-other-note",
                decision: runtime::RecommendationFeedbackDecision::Dismiss {
                    at: at(),
                    reason: runtime::DismissReason::Other(
                        runtime::BoundedNote::try_from("handled elsewhere".to_string()).unwrap(),
                    ),
                },
                expected_action: Some(FeedbackAction::NotRelevantHere),
                expected_payload: json!({
                    "invocation_id": "invocation-1",
                    "note": "handled elsewhere"
                }),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "not-useful",
                decision: runtime::RecommendationFeedbackDecision::NotUseful { at: at() },
                expected_action: Some(FeedbackAction::SurfaceInappropriate),
                expected_payload: json!({ "surface": "tauri_entity_detail" }),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "too-noisy",
                decision: runtime::RecommendationFeedbackDecision::TooNoisy { at: at() },
                expected_action: Some(FeedbackAction::SurfaceInappropriate),
                expected_payload: json!({ "surface": "tauri_entity_detail" }),
                expected_conversion: runtime::ConversionState::NotConverted,
            },
            MappingCase {
                name: "convert-action",
                decision: runtime::RecommendationFeedbackDecision::Convert {
                    at: at(),
                    into: runtime::ConversionTarget::Action("action-1".to_string()),
                },
                expected_action: Some(FeedbackAction::ConfirmCurrent),
                expected_payload: json!({}),
                expected_conversion: runtime::ConversionState::ConvertedToAction {
                    action_id: "action-1".to_string(),
                },
            },
            MappingCase {
                name: "convert-claim-correction",
                decision: runtime::RecommendationFeedbackDecision::Convert {
                    at: at(),
                    into: runtime::ConversionTarget::ClaimCorrection(runtime::ClaimId(
                        "claim-correction-1".to_string(),
                    )),
                },
                expected_action: Some(FeedbackAction::NeedsNuance),
                expected_payload: json!({ "corrected_text": "claim-correction-1" }),
                expected_conversion: runtime::ConversionState::ConvertedToClaimCorrection {
                    claim_id: runtime::ClaimId("claim-correction-1".to_string()),
                },
            },
            MappingCase {
                name: "convert-review-queue",
                decision: runtime::RecommendationFeedbackDecision::Convert {
                    at: at(),
                    into: runtime::ConversionTarget::ReviewQueue("queue-item-1".to_string()),
                },
                expected_action: None,
                expected_payload: serde_json::Value::Null,
                expected_conversion: runtime::ConversionState::ConvertedToReviewQueue {
                    queue_item_id: "queue-item-1".to_string(),
                },
            },
        ]
    }

    #[derive(Debug)]
    struct FeedbackRow {
        feedback_type: String,
        actor: String,
        actor_id: Option<String>,
        payload_json: Option<String>,
    }

    fn claim_feedback_rows(db: &ActionDb, claim_id: &str) -> Vec<FeedbackRow> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT feedback_type, actor, actor_id, payload_json
                 FROM claim_feedback
                 WHERE claim_id = ?1
                 ORDER BY rowid",
            )
            .expect("prepare feedback rows query");
        stmt.query_map(params![claim_id], |row| {
            Ok(FeedbackRow {
                feedback_type: row.get(0)?,
                actor: row.get(1)?,
                actor_id: row.get(2)?,
                payload_json: row.get(3)?,
            })
        })
        .expect("query feedback rows")
        .map(|row| row.expect("read feedback row"))
        .collect()
    }

    fn recommendation_metadata_value(db: &ActionDb, claim_id: &str) -> serde_json::Value {
        let raw: String = db
            .conn_ref()
            .query_row(
                "SELECT metadata_json FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .expect("read recommendation metadata");
        serde_json::from_str(&raw).expect("metadata json")
    }

    fn signal_count(db: &ActionDb, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM signal_events WHERE signal_type = ?1",
                params![signal_type],
                |row| row.get(0),
            )
            .expect("count signals")
    }

    fn first_signal_value(db: &ActionDb, signal_type: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT value FROM signal_events WHERE signal_type = ?1 ORDER BY rowid LIMIT 1",
                params![signal_type],
                |row| row.get(0),
            )
            .expect("read signal value")
    }

    fn seed_recommendation_claim(
        db: &ActionDb,
        claim_id: &str,
        feedback_state: app::FeedbackState,
    ) {
        let metadata = recommendation_metadata(claim_id, feedback_state);
        seed_claim_row(
            db,
            claim_id,
            ClaimType::Recommendation.as_str(),
            Some(serde_json::to_string(&metadata).expect("metadata serializes")),
        );
    }

    fn seed_claim_row(
        db: &ActionDb,
        claim_id: &str,
        claim_type: &str,
        metadata_json: Option<String>,
    ) {
        let now = at().to_rfc3339();
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: recommendation feedback unit test seed */ (
                    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                    item_hash, actor, data_source, source_ref, source_asof, observed_at,
                    created_at, provenance_json, metadata_json, claim_state, surfacing_state,
                    demotion_reason, reactivated_at, retraction_reason, expires_at,
                    superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
                    temporal_scope, sensitivity, verification_state, verification_reason,
                    needs_user_decision_at, claim_version, canonical_status,
                    non_semantic_mergeable
                ) VALUES (
                    ?1, ?2, ?3, 'recommendation.reviewClaim', ?1,
                    'Review the account before the next customer conversation.',
                    ?4, ?5, 'agent:test', 'recommendation', 'run:test', ?6, ?6,
                    ?6, '{\"sources\":[]}', ?7, 'active', 'active',
                    NULL, NULL, NULL, NULL, NULL, 0.82, ?6, 1, NULL, 'state',
                    'internal', 'active', NULL, NULL, 1, 'live', 0
                )",
                params![
                    claim_id,
                    r#"{"kind":"account","id":"acct-1"}"#,
                    claim_type,
                    format!("dedup-{claim_id}"),
                    format!("hash-{claim_id}"),
                    now,
                    metadata_json,
                ],
            )
            .expect("seed claim row");
    }

    fn recommendation_metadata(
        claim_id: &str,
        feedback_state: app::FeedbackState,
    ) -> app::RecommendationMetadataEnvelope {
        let draft = app::RecommendationDraft {
            subject: SubjectRef::Account("acct-1".to_string()),
            recommended_action: app::RecommendedAction::ReviewClaim {
                claim_id: app::ClaimId("claim-source-1".to_string()),
                reason: "verify before next customer conversation".to_string(),
            },
            evidence: vec![app::EvidenceRef {
                source: format!("claim:{claim_id}"),
                chunk: None,
            }],
            provenance_json: r#"{"sources":[]}"#.to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(at()),
            observed_at: at(),
            text: "Review the account before the next customer conversation.".to_string(),
            salience: app::SalienceScore {
                total: 0.91,
                factors: vec![app::SalienceFactor {
                    kind: app::SalienceFactorKind::Urgency,
                    value: Some(0.91),
                    weight: 0.15,
                    rationale: app::FactorRationale::Urgency {
                        deadline: Some(at()),
                        decay_factor: 0.91,
                    },
                }],
            },
        };
        let mut envelope = metadata_envelope(&draft);
        envelope.recommendation.feedback_state = feedback_state;
        envelope.recommendation.conversion_state = app::ConversionState::NotConverted;
        envelope
    }

    fn feedback_context() -> runtime::RecommendationFeedbackContext {
        runtime::RecommendationFeedbackContext {
            surface: RenderSurface::TauriEntityDetail,
            invocation_id: "invocation-1".to_string(),
        }
    }

    fn surface_actor() -> Actor {
        // Include the production scopes used by other modules' tests so this
        // global-allowlist write doesn't pollute interleaved test runs.
        // See `recommendations/mod.rs::set_recommendation_scope_allowlist_for_tests`
        // rationale (cross-test-pollution rule).
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new(runtime::SUBMIT_RECOMMENDATION_FEEDBACK_SCOPE),
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("read.entity_names"),
            SurfaceScope::new("read.markdown_preview"),
            SurfaceScope::new("read.recommendations"),
            SurfaceScope::new("read.workspace_graph"),
            SurfaceScope::new("read.workspace_sources"),
            SurfaceScope::new("submit.feedback"),
            SurfaceScope::new("write.entity_intake"),
            SurfaceScope::new("write.feedback"),
        ]);
        let scopes = ScopeSet::new([SurfaceScope::new(
            runtime::SUBMIT_RECOMMENDATION_FEEDBACK_SCOPE,
        )])
        .expect("scope set");
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("surface-1"),
            scopes,
        }
    }

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(at()),
            SeedableRng::new(7),
            ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
            .with_actor("user")
            .with_ability_id(runtime::SUBMIT_RECOMMENDATION_FEEDBACK_ABILITY_NAME)
    }

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0)
            .single()
            .unwrap()
    }
}
