use abilities_runtime::sensitivity::{
    renderable_claim_text_with_value, RenderActor, RenderSurface,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};

use crate::db::ActionDb;
use crate::services::claim_receipt::contracts::*;
use crate::services::claim_receipt::privacy::{build_receipt_for_audience, Audience, PrivacyError};
use crate::state::AppState;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("target not found")]
    TargetNotFound,
    #[error("storage error: {0}")]
    Storage(#[from] anyhow::Error),
    #[error("privacy gate dropped the claim for this audience")]
    PrivacyDrop,
}

/// Map a [`SurfaceContext`] to the [`Audience`] that constructs its receipt.
///
/// Tauri surfaces (ActionsWork / EntityDetail / DailyBriefing / MeetingDetail)
/// share the local-to-local trust boundary per ADR-0129 and therefore map to
/// [`Audience::UserTauri`]. The Mcp surface routes through
/// [`Audience::AgentMcp`] which strips claim IDs, subject IDs, source labels,
/// and source_asof timing oracles (CSO cycle-1 F12).
///
/// AC-341.4: this mapping is the production wiring that ensures the Mcp render
/// path goes through the AgentMcp allowlist + denylist rather than emitting
/// raw claim fields.
pub fn audience_for_surface(surface: SurfaceContext) -> Audience {
    match surface {
        SurfaceContext::ActionsWork
        | SurfaceContext::EntityDetail
        | SurfaceContext::DailyBriefing
        | SurfaceContext::MeetingDetail => Audience::UserTauri,
        SurfaceContext::Mcp => Audience::AgentMcp,
    }
}

/// Render the receipt projection for a target on a given surface.
///
/// **Cycle-2 wiring:** dispatches through
/// [`build_receipt_for_audience`] with an [`Audience`] derived from the
/// supplied `surface`. This is the production path that ensures the audience
/// allowlist (USER_TAURI_ALLOWED_FIELDS / AGENT_MCP_ALLOWED_FIELDS) governs
/// every field rather than being a test-only primitive
/// (cycle-1 code-reviewer F1).
///
/// **Proposal-receipt deferral** (per L0-W1 §5.6 + cycle-1 codex-consult F3):
/// only the `Claim` arm of [`ReceiptTarget`] resolves. `Proposal` and
/// `WorkItem` arms return [`RenderError::TargetNotFound`]. The DTO ships all
/// three variants so consumers compile, but receipt rendering for
/// proposal/work-item targets is W4 (Actions/Work) territory — NOT in v1.4.4
/// W1 scope. `submit_claim_feedback` on a Proposal target returns a typed
/// "no receipt yet, target-only" response per §5.7.
pub async fn render_receipt_for(
    state: &AppState,
    target: ReceiptTarget,
    surface: SurfaceContext,
) -> Result<ClaimReceipt, RenderError> {
    // Reject non-claim targets up front so DB roundtrips are cheap for the
    // deferral cases.
    match &target {
        ReceiptTarget::Claim { .. } => {}
        ReceiptTarget::Proposal { .. } | ReceiptTarget::WorkItem { .. } => {
            return Err(RenderError::TargetNotFound);
        }
    }

    let audience = audience_for_surface(surface);
    let target_for_db = target.clone();

    // Build the per-audience receipt under a read connection. The privacy
    // module owns audience-specific construction; we re-attach the
    // surface-specific rendered_text projection afterward for UserTauri
    // surfaces (AgentMcp deliberately keeps rendered_text = None).
    //
    // `db_read` requires `Result<T, String>`, so we encode the typed privacy
    // error variants as discriminated strings and re-type them at the
    // boundary.
    let build_result: Result<ClaimReceipt, String> = state
        .db_read(move |db| {
            Ok(
                build_receipt_for_audience(&target_for_db, audience, db.conn_ref())
                    .map_err(privacy_error_tag),
            )
        })
        .await
        .map_err(|message| RenderError::Storage(anyhow::anyhow!(message)))?;
    let mut receipt = build_result.map_err(render_error_from_tagged)?;

    // Preserve the caller-requested surface context on the receipt; the
    // privacy builders use canonical defaults (EntityDetail / Mcp /
    // ActionsWork) which the Tauri command boundary needs to override so the
    // round-trip back to the TS hook is structurally identical.
    receipt.surface_context = surface;

    // For UserTauri-class surfaces, attach the surface-policy-resolved
    // rendered text (the privacy module sets this to None to keep the
    // builder pure; rendering text is a sensitivity-policy call that we
    // make here against the actual SurfaceContext).
    if matches!(audience, Audience::UserTauri) {
        if let Some(rendered_text) =
            render_text_for_user_tauri_surface(state, &target, surface).await?
        {
            receipt.rendered_text = Some(rendered_text);
        }
    }

    Ok(receipt)
}

pub(crate) fn render_receipt_for_db(
    db: &ActionDb,
    target: ReceiptTarget,
    surface: SurfaceContext,
) -> Result<ClaimReceipt, RenderError> {
    match &target {
        ReceiptTarget::Claim { .. } => {}
        ReceiptTarget::Proposal { .. } | ReceiptTarget::WorkItem { .. } => {
            return Err(RenderError::TargetNotFound);
        }
    }

    let audience = audience_for_surface(surface);
    let mut receipt = build_receipt_for_audience(&target, audience, db.conn_ref())
        .map_err(render_error_from_privacy)?;
    receipt.surface_context = surface;

    if matches!(audience, Audience::UserTauri) {
        if let Some(rendered_text) = render_text_for_user_tauri_surface_db(db, &target, surface)? {
            receipt.rendered_text = Some(rendered_text);
        }
    }

    Ok(receipt)
}

/// Discriminator prefixes for [`PrivacyError`] → [`RenderError`] transport
/// across the `db_read` boundary (which insists on `Result<T, String>`).
const PRIVACY_TAG_NOT_FOUND: &str = "privacy/not_found:";
const PRIVACY_TAG_DROP: &str = "privacy/drop:";
const PRIVACY_TAG_STORAGE: &str = "privacy/storage:";

fn privacy_error_tag(err: PrivacyError) -> String {
    match err {
        PrivacyError::ClaimNotFound(id) => format!("{PRIVACY_TAG_NOT_FOUND}{id}"),
        PrivacyError::NonDisclosureAudience => {
            format!("{PRIVACY_TAG_DROP}operational_audit_storage")
        }
        PrivacyError::ComposedClaimDropped => format!("{PRIVACY_TAG_DROP}composed_claim_dropped"),
        PrivacyError::SurfaceDrop => format!("{PRIVACY_TAG_DROP}surface_drop"),
        PrivacyError::Storage(e) => format!("{PRIVACY_TAG_STORAGE}{e}"),
        PrivacyError::InvalidMetadata(m) => format!("{PRIVACY_TAG_STORAGE}invalid metadata: {m}"),
    }
}

fn render_error_from_tagged(tag: String) -> RenderError {
    if let Some(rest) = tag.strip_prefix(PRIVACY_TAG_NOT_FOUND) {
        let _ = rest;
        RenderError::TargetNotFound
    } else if tag.starts_with(PRIVACY_TAG_DROP) {
        RenderError::PrivacyDrop
    } else {
        RenderError::Storage(anyhow::anyhow!(tag))
    }
}

fn render_error_from_privacy(error: PrivacyError) -> RenderError {
    match error {
        PrivacyError::ClaimNotFound(_) => RenderError::TargetNotFound,
        PrivacyError::NonDisclosureAudience
        | PrivacyError::ComposedClaimDropped
        | PrivacyError::SurfaceDrop => RenderError::PrivacyDrop,
        PrivacyError::Storage(error) => RenderError::Storage(error.into()),
        PrivacyError::InvalidMetadata(message) => {
            RenderError::Storage(anyhow::anyhow!("invalid metadata: {message}"))
        }
    }
}

/// Resolve the policy-aware rendered text for a Tauri-class surface. We
/// re-load the claim under a fresh read connection to keep the privacy module
/// pure (it does not surface rendered_text).
async fn render_text_for_user_tauri_surface(
    state: &AppState,
    target: &ReceiptTarget,
    surface: SurfaceContext,
) -> Result<Option<abilities_runtime::sensitivity::RenderableClaimText>, RenderError> {
    let claim_id = match target {
        ReceiptTarget::Claim { claim_id, .. } => claim_id.clone(),
        _ => return Ok(None),
    };
    let claim = state
        .db_read(move |db| {
            crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|m| RenderError::Storage(anyhow::anyhow!(m)))?
        .ok_or(RenderError::TargetNotFound)?;

    let render_surface = render_surface_for(surface);
    let actor = RenderActor {
        actor: "user".to_string(),
        user_id: None,
    };
    Ok(renderable_claim_text_with_value(
        &claim,
        &claim.text,
        render_surface,
        &actor,
    ))
}

fn render_text_for_user_tauri_surface_db(
    db: &ActionDb,
    target: &ReceiptTarget,
    surface: SurfaceContext,
) -> Result<Option<abilities_runtime::sensitivity::RenderableClaimText>, RenderError> {
    let claim_id = match target {
        ReceiptTarget::Claim { claim_id, .. } => claim_id,
        _ => return Ok(None),
    };
    let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
        .map_err(|error| RenderError::Storage(anyhow::anyhow!(error.to_string())))?
        .ok_or(RenderError::TargetNotFound)?;

    let render_surface = render_surface_for(surface);
    let actor = RenderActor {
        actor: "user".to_string(),
        user_id: None,
    };
    Ok(renderable_claim_text_with_value(
        &claim,
        &claim.text,
        render_surface,
        &actor,
    ))
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
                        "INSERT INTO intelligence_claims /* dos7-allowed: claim receipt render unit test seed */ (
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
    async fn ac_341_4_mcp_surface_routes_through_agent_mcp_audience() {
        // Cycle-2 fix for code-reviewer F1: render_receipt_for must dispatch
        // through Audience::AgentMcp when surface == Mcp. This is the canonical
        // production wiring assertion — the receipt MUST NOT carry the
        // source_type / source_asof / claim_id timing-oracle fields when
        // emitted to the MCP surface.
        let (state, _tempdir) = test_state().await;
        let source_asof = Utc::now() - Duration::days(3);
        seed_claim(&state, "claim-mcp-1", Some(source_asof)).await;

        let target = ReceiptTarget::Claim {
            claim_id: "claim-mcp-1".to_string(),
            subject: SubjectRef::Account("acct-1".to_string()),
            field_path: Some("health.risk".to_string()),
        };
        let receipt = render_receipt_for(&state, target, SurfaceContext::Mcp)
            .await
            .expect("mcp render should succeed for internal-sensitivity claim");

        // AgentMcp denylist: no source labels, no source_asof, no field_path.
        assert!(
            receipt.provenance.sources.is_empty(),
            "AgentMcp must not emit source labels (timing oracle)"
        );
        assert!(
            receipt.trust.source_asof.is_none(),
            "AgentMcp must not emit source_asof (timing oracle)"
        );
        assert!(
            receipt.lifecycle.updated_at.is_none(),
            "AgentMcp must not emit updated_at (graph timing leak)"
        );
        assert!(
            receipt.provenance.field_path.is_none(),
            "AgentMcp must not emit field_path (graph leak)"
        );
        assert!(
            receipt.rendered_text.is_none(),
            "AgentMcp must not pre-render text — consumer renders its own"
        );

        // Surface context override: the receipt MUST carry the caller-
        // requested surface, not the privacy builder's canonical default.
        assert_eq!(receipt.surface_context, SurfaceContext::Mcp);

        // Claim id + subject id must be scrubbed.
        match &receipt.target {
            ReceiptTarget::Claim {
                claim_id, subject, ..
            } => {
                assert!(claim_id.is_empty(), "AgentMcp must strip claim_id");
                match subject {
                    SubjectRef::Account(id) => {
                        assert!(id.is_empty(), "AgentMcp must strip subject_id");
                    }
                    other => panic!("expected Account subject type, got {other:?}"),
                }
            }
            other => panic!("expected Claim target, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ac_341_4_tauri_surface_routes_through_user_tauri_audience() {
        // Companion to the Mcp test: confirm Tauri-class surfaces (ActionsWork
        // / EntityDetail / DailyBriefing / MeetingDetail) all dispatch through
        // Audience::UserTauri and therefore see source labels + source_asof.
        let (state, _tempdir) = test_state().await;
        let source_asof = Utc::now() - Duration::days(3);
        seed_claim(&state, "claim-tauri-mix-1", Some(source_asof)).await;

        for surface in [
            SurfaceContext::ActionsWork,
            SurfaceContext::EntityDetail,
            SurfaceContext::DailyBriefing,
            SurfaceContext::MeetingDetail,
        ] {
            let target = ReceiptTarget::Claim {
                claim_id: "claim-tauri-mix-1".to_string(),
                subject: SubjectRef::Account("acct-1".to_string()),
                field_path: Some("health.risk".to_string()),
            };
            let receipt = render_receipt_for(&state, target, surface)
                .await
                .expect("Tauri surface render");
            assert_eq!(
                receipt.surface_context, surface,
                "render_receipt_for must echo caller surface back"
            );
            assert_eq!(
                receipt.provenance.sources.len(),
                1,
                "UserTauri must surface exactly one provenance source"
            );
            assert!(
                receipt.trust.source_asof.is_some(),
                "UserTauri must carry source_asof"
            );
        }
    }

    #[test]
    fn audience_for_surface_mapping() {
        // Lock the cycle-2 wiring contract: Mcp → AgentMcp, all Tauri-class
        // surfaces → UserTauri. A future regression that broadens the Mcp
        // surface back to UserTauri (the cycle-1 F1 leak) fails this test.
        assert_eq!(
            audience_for_surface(SurfaceContext::Mcp),
            Audience::AgentMcp
        );
        for tauri_surface in [
            SurfaceContext::ActionsWork,
            SurfaceContext::EntityDetail,
            SurfaceContext::DailyBriefing,
            SurfaceContext::MeetingDetail,
        ] {
            assert_eq!(
                audience_for_surface(tauri_surface),
                Audience::UserTauri,
                "{tauri_surface:?} must route through UserTauri"
            );
        }
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
