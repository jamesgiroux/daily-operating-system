use abilities_runtime::abilities::provenance::claim_trust_band_from_score;
use abilities_runtime::sensitivity::{
    renderable_claim_text_with_value, RenderActor, RenderSurface,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};

use crate::services::claim_receipt::contracts::*;
use crate::state::AppState;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("target not found")]
    TargetNotFound,
    #[error("storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

pub async fn render_receipt_for(
    state: &AppState,
    target: ReceiptTarget,
    surface: SurfaceContext,
) -> Result<ClaimReceipt, RenderError> {
    let (claim_id, target_field_path) = match &target {
        ReceiptTarget::Claim {
            claim_id,
            field_path,
            ..
        } => (claim_id.clone(), field_path.clone()),
        ReceiptTarget::Proposal { .. } | ReceiptTarget::WorkItem { .. } => {
            return Err(RenderError::TargetNotFound);
        }
    };

    let claim = state
        .db_read(move |db| {
            crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|message| RenderError::Storage(anyhow::anyhow!(message)))?
        .ok_or(RenderError::TargetNotFound)?;

    let render_surface = render_surface_for(surface);
    let actor = RenderActor {
        actor: "user".to_string(),
        user_id: None,
    };
    let rendered_text =
        renderable_claim_text_with_value(&claim, &claim.text, render_surface, &actor);

    let source_asof = claim.source_asof.as_deref().and_then(parse_claim_timestamp);
    let freshness = freshness_for(source_asof, Utc::now());

    Ok(ClaimReceipt {
        target,
        surface_context: surface,
        rendered_text,
        trust: ReceiptTrust {
            band: claim_trust_band_from_score(claim.trust_score),
            source_asof,
            freshness,
            caveat: None,
            rationale: None,
        },
        lifecycle: ReceiptLifecycle {
            claim_state: claim.claim_state,
            surfacing_state: claim.surfacing_state,
            verification_state: claim.verification_state,
            updated_at: claim
                .reactivated_at
                .as_deref()
                .and_then(parse_claim_timestamp)
                .or_else(|| parse_claim_timestamp(&claim.created_at)),
        },
        provenance: ReceiptProvenance {
            sources: vec![ProvenanceSource {
                label: "primary source".to_string(),
                source_type: Some(claim.data_source),
                as_of: source_asof,
                href: None,
                redacted: false,
            }],
            field_path: target_field_path.or(claim.field_path),
            evidence_summary: None,
            redaction: RedactionLevel::None,
        },
        actions: Vec::new(),
    })
}

fn render_surface_for(surface: SurfaceContext) -> RenderSurface {
    match surface {
        SurfaceContext::ActionsWork => RenderSurface::Action,
        SurfaceContext::EntityDetail => RenderSurface::TauriEntityDetail,
        SurfaceContext::DailyBriefing => RenderSurface::TauriBriefingPrep,
        SurfaceContext::MeetingDetail => RenderSurface::TauriMeetingDetail,
        SurfaceContext::Mcp => RenderSurface::McpTool,
    }
}

fn freshness_for(source_asof: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Freshness {
    let Some(source_asof) = source_asof else {
        return Freshness::Unknown;
    };
    let age = now.signed_duration_since(source_asof);
    if age <= Duration::days(7) {
        Freshness::Current
    } else if age <= Duration::days(30) {
        Freshness::Aging
    } else {
        Freshness::Stale
    }
}

fn parse_claim_timestamp(value: &str) -> Option<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(parsed.with_timezone(&Utc));
    }

    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]
        .iter()
        .find_map(|format| {
            NaiveDateTime::parse_from_str(trimmed, format)
                .ok()
                .map(|parsed| DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use abilities_runtime::abilities::provenance::subject::SubjectRef;
    use abilities_runtime::abilities::trust::types::TrustBand;
    use abilities_runtime::sensitivity::{ClaimVerificationState, RenderPolicyKind};
    use abilities_runtime::types::{ClaimState, SurfacingState};
    use chrono::TimeZone;
    use rusqlite::params;

    async fn test_state() -> (AppState, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("claim-receipt-render.db");
        let db_service = crate::db_service::DbService::open_at_unencrypted(db_path)
            .await
            .expect("open test db service");
        (AppState::test_with_db_service(db_service), tempdir)
    }

    async fn seed_claim(state: &AppState, claim_id: &str, source_asof: Option<DateTime<Utc>>) {
        let claim_id = claim_id.to_string();
        let source_asof = source_asof.map(|value| value.to_rfc3339());
        state
            .db_write(move |db| {
                let observed_at = Utc::now().to_rfc3339();
                db.conn_ref()
                    .execute(
                        "INSERT INTO intelligence_claims (
                            id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                            item_hash, actor, data_source, source_ref, source_asof, observed_at,
                            created_at, provenance_json, metadata_json, claim_state, surfacing_state,
                            demotion_reason, reactivated_at, retraction_reason, expires_at,
                            superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
                            temporal_scope, sensitivity, verification_state, verification_reason,
                            needs_user_decision_at, claim_version, canonical_status,
                            non_semantic_mergeable
                        ) VALUES (
                            ?1, ?2, 'risk', 'health.risk', 'renewal',
                            'Renewal risk is elevated', 'dedup-render-1', 'hash-render-1',
                            'agent:test', 'unit_test', ?3, ?4, ?5, ?5, '{}', ?6, 'active',
                            'active', ?7, ?8, ?9, ?10, ?11, 0.82, ?5, 1, ?12, 'state',
                            'internal', 'contested', ?13, ?14, 2, 'live', 0
                        )",
                        params![
                            claim_id,
                            r#"{"kind":"account","id":"acct-1"}"#,
                            r#"{"kind":"fixture","id":"source-1"}"#,
                            source_asof.as_deref(),
                            observed_at,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Some("thread-1"),
                            Some("needs review"),
                            Option::<&str>::None,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed claim");
    }

    #[tokio::test]
    async fn renders_claim_receipt_happy_path() {
        let (state, _tempdir) = test_state().await;
        let source_asof = Utc::now() - Duration::days(3);
        seed_claim(&state, "claim-render-1", Some(source_asof)).await;

        let target = ReceiptTarget::Claim {
            claim_id: "claim-render-1".to_string(),
            subject: SubjectRef::Account("acct-1".to_string()),
            field_path: Some("health.risk".to_string()),
        };
        let receipt = render_receipt_for(&state, target.clone(), SurfaceContext::EntityDetail)
            .await
            .expect("render receipt");

        assert_eq!(receipt.target, target);
        assert_eq!(receipt.surface_context, SurfaceContext::EntityDetail);
        let rendered = receipt.rendered_text.expect("rendered text");
        assert_eq!(rendered.text, "Renewal risk is elevated");
        assert_eq!(rendered.policy.kind, RenderPolicyKind::Render);
        assert_eq!(rendered.policy.surface, RenderSurface::TauriEntityDetail);
        assert_eq!(rendered.policy.claim_id.as_deref(), Some("claim-render-1"));

        assert_eq!(receipt.trust.band, TrustBand::LikelyCurrent);
        assert_eq!(receipt.trust.freshness, Freshness::Current);
        assert!(receipt.trust.source_asof.is_some());

        assert_eq!(receipt.lifecycle.claim_state, ClaimState::Active);
        assert_eq!(receipt.lifecycle.surfacing_state, SurfacingState::Active);
        assert_eq!(
            receipt.lifecycle.verification_state,
            ClaimVerificationState::Contested
        );
        assert!(receipt.lifecycle.updated_at.is_some());

        assert_eq!(receipt.provenance.redaction, RedactionLevel::None);
        assert_eq!(
            receipt.provenance.field_path.as_deref(),
            Some("health.risk")
        );
        assert_eq!(receipt.provenance.sources.len(), 1);
        let source = &receipt.provenance.sources[0];
        assert_eq!(source.label, "primary source");
        assert_eq!(source.source_type.as_deref(), Some("unit_test"));
        assert_eq!(source.as_of, receipt.trust.source_asof);
        assert_eq!(source.href, None);
        assert!(!source.redacted);
        assert!(receipt.actions.is_empty());
    }

    #[tokio::test]
    async fn unknown_claim_id_returns_target_not_found() {
        let (state, _tempdir) = test_state().await;
        let target = ReceiptTarget::Claim {
            claim_id: "missing-claim".to_string(),
            subject: SubjectRef::Account("acct-1".to_string()),
            field_path: None,
        };

        let error = render_receipt_for(&state, target, SurfaceContext::ActionsWork)
            .await
            .expect_err("unknown claim should not render");

        assert!(matches!(error, RenderError::TargetNotFound));
    }

    #[test]
    fn freshness_boundaries_are_inclusive() {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();

        assert_eq!(freshness_for(None, now), Freshness::Unknown);
        assert_eq!(
            freshness_for(Some(now - Duration::days(7)), now),
            Freshness::Current
        );
        assert_eq!(
            freshness_for(Some(now - Duration::days(7) - Duration::seconds(1)), now),
            Freshness::Aging
        );
        assert_eq!(
            freshness_for(Some(now - Duration::days(30)), now),
            Freshness::Aging
        );
        assert_eq!(
            freshness_for(Some(now - Duration::days(30) - Duration::seconds(1)), now),
            Freshness::Stale
        );
    }
}
