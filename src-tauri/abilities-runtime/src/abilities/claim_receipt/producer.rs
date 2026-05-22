//! `claim_receipt` ability producer.
//!
//! Read-side wrapper over the app crate's
//! `services::claim_receipt::render::render_receipt_for` substrate. The
//! adapter is the narrow `ClaimReceiptReadHandle` attached on
//! `ServiceContext`; this producer is intentionally thin — schema validation,
//! provenance attribution, and translating the read-handle error surface to
//! the typed `AbilityError`.
//!
//! Only the `Claim` arm of `ClaimReceiptTarget` resolves today; `Proposal`
//! and `WorkItem` arms return a typed `TargetNotFound` rejection at the
//! reader boundary so the deferral surfaces as a runtime negative rather
//! than a contract gap.

use super::contracts::{ClaimReceiptInput, ClaimReceiptSnapshot};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::ClaimReceiptReadError;

pub(super) const ABILITY_NAME: &str = "claim_receipt";
pub(super) const ABILITY_SCHEMA_VERSION: u32 = 1;

pub async fn build_claim_receipt(
    ctx: &AbilityContext<'_>,
    input: ClaimReceiptInput,
) -> AbilityResult<ClaimReceiptSnapshot> {
    validate_schema_version(input.schema_version)?;

    let subject_ref = subject_ref_for_target(&input.target);
    let snapshot = ctx
        .services()
        .read_claim_receipt(input.target.clone(), input.surface)
        .await
        .map_err(read_error)?;

    // Provenance: the receipt is itself a render-time projection over the
    // underlying claim. We attribute the envelope to the resolved subject so
    // the receipt carries a typed `SubjectAttribution` for the surface
    // renderer; field-level attribution rides on the claim's own
    // `provenance.sources` block which the privacy module populates inside
    // `build_receipt_for_audience`.
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    let subject_attribution = SubjectAttribution::direct_confident(subject_ref);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/target").map_err(field_error)?,
            FieldAttribution::constant(subject_attribution.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/surfaceContext").map_err(field_error)?,
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;

    builder.finalize(snapshot).map_err(provenance_error)
}

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == ABILITY_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn subject_ref_for_target(target: &crate::services::context::ClaimReceiptTarget) -> SubjectRef {
    use crate::services::context::ClaimReceiptTarget;
    match target {
        ClaimReceiptTarget::Claim { subject, .. } => subject.clone(),
        ClaimReceiptTarget::Proposal { subject, .. } => subject.clone(),
        ClaimReceiptTarget::WorkItem { subject, .. } => {
            subject.clone().unwrap_or(SubjectRef::Global)
        }
    }
}

fn read_error(error: ClaimReceiptReadError) -> AbilityError {
    match error {
        ClaimReceiptReadError::TargetNotFound => AbilityError {
            kind: AbilityErrorKind::HardError("claim_receipt_target_not_found".to_string()),
            message: "claim receipt target not found".to_string(),
        },
        ClaimReceiptReadError::PrivacyDrop => AbilityError {
            kind: AbilityErrorKind::HardError("claim_receipt_privacy_drop".to_string()),
            message: "claim receipt dropped by privacy gate".to_string(),
        },
        ClaimReceiptReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError("claim_receipt_read_failed".to_string()),
            message,
        },
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field attribution path failed: {error}"))
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp_client".to_string(),
            version: "unknown".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use chrono::TimeZone;

    use crate::abilities::registry::AbilityContext;
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::ReplayProvider;
    use crate::services::context::ClaimReceiptTarget;
    use crate::services::context::{
        ClaimReceiptFreshness, ClaimReceiptLifecycle, ClaimReceiptProvenance,
        ClaimReceiptReadFuture, ClaimReceiptReadHandle, ClaimReceiptRedactionLevel,
        ClaimReceiptSnapshot, ClaimReceiptSurfaceContext, ClaimReceiptTrust, FixedClock,
        ServiceContext, SystemRng,
    };
    use crate::types::{ClaimState, SurfacingState};
    use crate::{abilities::trust::types::TrustBand, sensitivity::ClaimVerificationState};

    struct StaticReceiptReader {
        snapshot: ClaimReceiptSnapshot,
    }

    impl ClaimReceiptReadHandle for StaticReceiptReader {
        fn read_claim_receipt<'a>(
            &'a self,
            _target: ClaimReceiptTarget,
            _surface: ClaimReceiptSurfaceContext,
        ) -> ClaimReceiptReadFuture<'a> {
            let snapshot = self.snapshot.clone();
            Box::pin(async move { Ok(snapshot) })
        }
    }

    #[test]
    fn schema_version_validation_rejects_wrong_version() {
        let result = validate_schema_version(99);
        assert!(matches!(
            result,
            Err(AbilityError {
                kind: AbilityErrorKind::Validation,
                ..
            })
        ));
    }

    #[test]
    fn schema_version_validation_accepts_v1() {
        assert!(validate_schema_version(1).is_ok());
    }

    #[tokio::test]
    async fn producer_finalizes_receipt_with_outer_provenance() {
        let target = ClaimReceiptTarget::Claim {
            claim_id: "claim-test-001".into(),
            subject: SubjectRef::Account("acct-test-001".into()),
            field_path: Some("status".into()),
        };
        let surface = ClaimReceiptSurfaceContext::EntityDetail;
        let snapshot = ClaimReceiptSnapshot {
            target: target.clone(),
            surface_context: surface,
            rendered_text: None,
            trust: ClaimReceiptTrust {
                band: TrustBand::LikelyCurrent,
                source_asof: None,
                freshness: ClaimReceiptFreshness::Current,
                caveat: None,
                rationale: None,
            },
            lifecycle: ClaimReceiptLifecycle {
                claim_state: ClaimState::Active,
                surfacing_state: SurfacingState::Active,
                verification_state: ClaimVerificationState::Active,
                updated_at: None,
            },
            provenance: ClaimReceiptProvenance {
                sources: Vec::new(),
                field_path: Some("status".into()),
                evidence_summary: None,
                redaction: ClaimReceiptRedactionLevel::None,
            },
            actions: Vec::new(),
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_claim_receipt_reader(Arc::new(StaticReceiptReader { snapshot }));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            crate::services::context::ClaimDismissalSurface::TauriEntityDetail,
        );

        let output = build_claim_receipt(
            &ctx,
            ClaimReceiptInput {
                schema_version: 1,
                target,
                surface,
            },
        )
        .await
        .expect("producer should return a finalized receipt");

        assert_eq!(
            output.data().target,
            ClaimReceiptTarget::Claim {
                claim_id: "claim-test-001".into(),
                subject: SubjectRef::Account("acct-test-001".into()),
                field_path: Some("status".into()),
            }
        );
    }

    #[test]
    fn subject_ref_for_claim_target_extracts_account_subject() {
        let target = ClaimReceiptTarget::Claim {
            claim_id: "c1".into(),
            subject: SubjectRef::Account("acct-1".into()),
            field_path: Some("health.risk".into()),
        };
        assert_eq!(
            subject_ref_for_target(&target),
            SubjectRef::Account("acct-1".into())
        );
    }

    #[test]
    fn subject_ref_for_work_item_without_subject_falls_back_to_global() {
        let target = ClaimReceiptTarget::WorkItem {
            action_id: "a1".into(),
            backing_claim_id: None,
            subject: None,
        };
        assert_eq!(subject_ref_for_target(&target), SubjectRef::Global);
    }

    #[test]
    fn target_not_found_error_maps_to_hard_error() {
        let err = read_error(ClaimReceiptReadError::TargetNotFound);
        assert!(matches!(
            err.kind,
            AbilityErrorKind::HardError(ref code) if code == "claim_receipt_target_not_found"
        ));
    }

    #[test]
    fn privacy_drop_error_maps_to_hard_error_with_distinct_code() {
        let err = read_error(ClaimReceiptReadError::PrivacyDrop);
        assert!(matches!(
            err.kind,
            AbilityErrorKind::HardError(ref code) if code == "claim_receipt_privacy_drop"
        ));
    }

    #[test]
    fn read_failed_error_preserves_message() {
        let err = read_error(ClaimReceiptReadError::ReadFailed("db unavailable".into()));
        assert_eq!(err.message, "db unavailable");
    }
}
