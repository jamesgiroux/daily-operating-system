use abilities_runtime::sensitivity::{
    render_policy_for_surface, RenderActor, RenderDecision, RenderSurface,
};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("claim not found: {0}")]
    ClaimNotFound(String),
    #[error("surface drop: actor {actor:?} cannot surface claim {claim_id} on {surface:?}")]
    CannotSurface {
        claim_id: String,
        surface: String,
        actor: String,
    },
    #[error("storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

pub async fn can_surface_for(
    state: &crate::state::AppState,
    actor: &RenderActor,
    surface: RenderSurface,
    claim_id: &str,
) -> Result<(), AuthError> {
    let claim_id_owned = claim_id.to_string();
    let claim = state
        .db_read(move |db| {
            crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id_owned)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| AuthError::Storage(anyhow::Error::msg(error)))?
        .ok_or_else(|| AuthError::ClaimNotFound(claim_id.to_string()))?;

    match render_policy_for_surface(&claim, surface, actor) {
        RenderDecision::Render | RenderDecision::RenderRedacted { .. } => Ok(()),
        RenderDecision::Drop => Err(AuthError::CannotSurface {
            claim_id: claim.id,
            surface: format!("{surface:?}"),
            actor: actor.actor.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::sensitivity::ClaimVerificationState;
    use abilities_runtime::types::{
        ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
    };
    use rusqlite::params;

    async fn test_state() -> (crate::state::AppState, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("claim-receipt-auth-test.db");
        let db_service = crate::db_service::DbService::open_at_unencrypted(db_path)
            .await
            .expect("open test db service");
        (
            crate::state::AppState::test_with_db_service(db_service),
            tempdir,
        )
    }

    fn fixture_claim(
        id: &str,
        sensitivity: ClaimSensitivity,
        actor: impl Into<String>,
    ) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            claim_version: 1,
            subject_ref: r#"{"kind":"account","id":"acct-1"}"#.to_string(),
            claim_type: "account_status".to_string(),
            field_path: Some("status".to_string()),
            topic_key: None,
            text: format!("claim {id}"),
            dedup_key: format!("dedup:{id}"),
            item_hash: Some(format!("hash:{id}")),
            actor: actor.into(),
            data_source: "test_fixture".to_string(),
            source_ref: Some(format!("fixture://{id}")),
            source_asof: Some("2026-05-19T00:00:00Z".to_string()),
            observed_at: "2026-05-19T00:00:00Z".to_string(),
            created_at: "2026-05-19T00:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    async fn seed_claim(state: &crate::state::AppState, claim: IntelligenceClaim) {
        state
            .db_write(move |db| {
                db.conn_ref()
                    .execute(
                        "INSERT INTO intelligence_claims /* dos7-allowed: claim receipt auth unit test seed */ (
                            id, claim_version, subject_ref, claim_type, field_path, topic_key,
                            text, dedup_key, item_hash, actor, data_source, source_ref,
                            source_asof, observed_at, created_at, provenance_json, metadata_json,
                            claim_state, surfacing_state, demotion_reason, reactivated_at,
                            retraction_reason, expires_at, superseded_by, trust_score,
                            trust_computed_at, trust_version, thread_id, temporal_scope,
                            sensitivity, verification_state, verification_reason,
                            needs_user_decision_at
                        ) VALUES (
                            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                            ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22,
                            ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33
                        )",
                        params![
                            claim.id,
                            i64::try_from(claim.claim_version).map_err(|error| error.to_string())?,
                            claim.subject_ref,
                            claim.claim_type,
                            claim.field_path,
                            claim.topic_key,
                            claim.text,
                            claim.dedup_key,
                            claim.item_hash,
                            claim.actor,
                            claim.data_source,
                            claim.source_ref,
                            claim.source_asof,
                            claim.observed_at,
                            claim.created_at,
                            claim.provenance_json,
                            claim.metadata_json,
                            claim_state_name(&claim.claim_state),
                            surfacing_state_name(&claim.surfacing_state),
                            claim.demotion_reason,
                            claim.reactivated_at,
                            claim.retraction_reason,
                            claim.expires_at,
                            claim.superseded_by,
                            claim.trust_score,
                            claim.trust_computed_at,
                            claim.trust_version,
                            claim.thread_id,
                            temporal_scope_name(&claim.temporal_scope),
                            sensitivity_name(&claim.sensitivity),
                            verification_state_name(&claim.verification_state),
                            claim.verification_reason,
                            claim.needs_user_decision_at,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed claim");
    }

    fn claim_state_name(value: &ClaimState) -> &'static str {
        match value {
            ClaimState::Active => "active",
            ClaimState::Dormant => "dormant",
            ClaimState::Tombstoned => "tombstoned",
            ClaimState::Withdrawn => "withdrawn",
        }
    }

    fn surfacing_state_name(value: &SurfacingState) -> &'static str {
        match value {
            SurfacingState::Active => "active",
            SurfacingState::Dormant => "dormant",
        }
    }

    fn temporal_scope_name(value: &TemporalScope) -> &'static str {
        match value {
            TemporalScope::State => "state",
            TemporalScope::PointInTime => "point_in_time",
            TemporalScope::Trend => "trend",
            TemporalScope::Closed => "closed",
        }
    }

    fn sensitivity_name(value: &ClaimSensitivity) -> &'static str {
        match value {
            ClaimSensitivity::Public => "public",
            ClaimSensitivity::Internal => "internal",
            ClaimSensitivity::Confidential => "confidential",
            ClaimSensitivity::UserOnly => "user_only",
        }
    }

    fn verification_state_name(value: &ClaimVerificationState) -> &'static str {
        match value {
            ClaimVerificationState::Active => "active",
            ClaimVerificationState::Contested => "contested",
            ClaimVerificationState::NeedsUserDecision => "needs_user_decision",
        }
    }

    async fn assert_allowed(claim: IntelligenceClaim, surface: RenderSurface, actor: RenderActor) {
        let (state, _tempdir) = test_state().await;
        let claim_id = claim.id.clone();
        seed_claim(&state, claim).await;

        can_surface_for(&state, &actor, surface, &claim_id)
            .await
            .expect("claim should surface");
    }

    async fn assert_denied(claim: IntelligenceClaim, surface: RenderSurface, actor: RenderActor) {
        let (state, _tempdir) = test_state().await;
        let claim_id = claim.id.clone();
        seed_claim(&state, claim).await;

        let error = can_surface_for(&state, &actor, surface, &claim_id)
            .await
            .expect_err("claim should not surface");
        assert!(matches!(error, AuthError::CannotSurface { .. }));
    }

    #[tokio::test]
    async fn public_claim_allows_tauri_and_drops_structured_logs() {
        assert_allowed(
            fixture_claim("public-allowed", ClaimSensitivity::Public, "agent:test"),
            RenderSurface::TauriEntityDetail,
            RenderActor::agent("agent:test"),
        )
        .await;
        assert_denied(
            fixture_claim("public-denied", ClaimSensitivity::Public, "agent:test"),
            RenderSurface::LogStructured,
            RenderActor::agent("agent:test"),
        )
        .await;
    }

    #[tokio::test]
    async fn internal_claim_allows_tauri_and_drops_external_publication() {
        assert_allowed(
            fixture_claim("internal-allowed", ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::TauriBriefingPrep,
            RenderActor::agent("agent:test"),
        )
        .await;
        assert_denied(
            fixture_claim("internal-denied", ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::P2Publication,
            RenderActor::agent("agent:test"),
        )
        .await;
    }

    #[tokio::test]
    async fn confidential_claim_redacts_on_tauri_and_drops_chat() {
        assert_allowed(
            fixture_claim(
                "confidential-allowed",
                ClaimSensitivity::Confidential,
                "agent:test",
            ),
            RenderSurface::TauriEntityDetail,
            RenderActor::agent("agent:test"),
        )
        .await;
        assert_denied(
            fixture_claim(
                "confidential-denied",
                ClaimSensitivity::Confidential,
                "agent:test",
            ),
            RenderSurface::TauriChat,
            RenderActor::agent("agent:test"),
        )
        .await;
    }

    #[tokio::test]
    async fn user_only_claim_allows_owner_and_drops_chat() {
        assert_allowed(
            fixture_claim("user-only-allowed", ClaimSensitivity::UserOnly, "user-1"),
            RenderSurface::TauriMeetingDetail,
            RenderActor::user("user", Some("user-1")),
        )
        .await;
        assert_denied(
            fixture_claim("user-only-denied", ClaimSensitivity::UserOnly, "user-1"),
            RenderSurface::TauriChat,
            RenderActor::user("user", Some("user-1")),
        )
        .await;
    }

    #[tokio::test]
    async fn missing_claim_returns_claim_not_found() {
        let (state, _tempdir) = test_state().await;
        let error = can_surface_for(
            &state,
            &RenderActor::agent("agent:test"),
            RenderSurface::TauriEntityDetail,
            "missing-claim",
        )
        .await
        .expect_err("missing claim should fail");

        assert!(matches!(error, AuthError::ClaimNotFound(id) if id == "missing-claim"));
    }
}
