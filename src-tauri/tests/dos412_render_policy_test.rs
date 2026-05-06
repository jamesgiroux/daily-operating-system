use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::services::sensitivity::{
    render_policy_for_sensitivity_name, render_policy_for_surface, render_policy_for_surface_name,
    renderable_claim_text, RedactionAffordance, RenderActor, RenderDecision, RenderPolicyKind,
    RenderSurface,
};
use rusqlite::Connection;

#[test]
fn adr_0108_surface_policy_matrix_is_fail_closed() {
    let user = RenderActor::user("user", Some("user"));
    let other_user = RenderActor::user("user", Some("other-user"));
    let agent = RenderActor::agent("agent:mcp");

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::TauriEntityDetail,
            &user,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::LogStructured,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::TauriBriefingPrep,
            &user,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::McpToolDetail,
            &agent,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::P2Publication,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_confidential_click_to_reveal(render_policy_for_surface(
        &claim(ClaimSensitivity::Confidential, "agent:test"),
        RenderSurface::TauriEntityDetail,
        &user,
    ));
    assert!(matches!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Confidential, "agent:test"),
            RenderSurface::TauriBriefingPrep,
            &user,
        ),
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialHidden { .. }
        }
    ));
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Confidential, "agent:test"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::TauriEntityDetail,
            &user,
        ),
        RenderDecision::Render
    );
    assert!(matches!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::TauriEntityDetail,
            &other_user,
        ),
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::UserOnlyHidden { .. }
        }
    ));
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface_name(
            &claim(ClaimSensitivity::Public, "user"),
            "unknown_surface",
            &user,
        ),
        RenderDecision::Drop
    );
    assert_eq!(
        render_policy_for_sensitivity_name("unknown", "tauri_entity_detail", "user", &user),
        RenderDecision::Drop
    );
}

#[test]
fn renderable_claim_text_never_embeds_confidential_source_text_in_redaction() {
    let user = RenderActor::user("user", Some("user"));
    let source = "Confidential renewal blocker";
    let rendered = renderable_claim_text(
        &claim_with_text(ClaimSensitivity::Confidential, "agent:test", source),
        RenderSurface::TauriEntityDetail,
        &user,
    )
    .expect("confidential first-party UI receives redaction affordance");

    assert_eq!(rendered.text, "Confidential claim hidden");
    assert!(!rendered.text.contains(source));
    assert_eq!(rendered.policy.kind, RenderPolicyKind::Redacted);
}

#[test]
fn sensitivity_reveal_audit_migration_declares_required_columns() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(include_str!(
        "../src/migrations/142_sensitivity_reveal_audit.sql"
    ))
    .expect("migration applies");

    let mut stmt = conn
        .prepare("PRAGMA table_info(sensitivity_reveal_audit)")
        .expect("query audit schema");
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("read columns")
        .map(|row| row.expect("column row"))
        .collect();

    assert!(columns.contains(&"claim_id".to_string()));
    assert!(columns.contains(&"user_id".to_string()));
    assert!(columns.contains(&"revealed_at".to_string()));
}

fn assert_confidential_click_to_reveal(decision: RenderDecision) {
    assert!(matches!(
        decision,
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialClickToReveal {
                audit_required: true,
                ..
            }
        }
    ));
}

fn claim(sensitivity: ClaimSensitivity, actor: &str) -> IntelligenceClaim {
    claim_with_text(sensitivity, actor, "Source text")
}

fn claim_with_text(sensitivity: ClaimSensitivity, actor: &str, text: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-dos412".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-dos412-example"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: None,
        topic_key: None,
        text: text.to_string(),
        dedup_key: "claim-dos412".to_string(),
        item_hash: None,
        actor: actor.to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: "2026-05-06T00:00:00Z".to_string(),
        created_at: "2026-05-06T00:00:00Z".to_string(),
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
