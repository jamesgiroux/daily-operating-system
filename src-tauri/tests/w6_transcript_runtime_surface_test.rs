mod dos568_support;

use abilities_runtime::abilities::feedback::FeedbackAction;
use chrono::{TimeZone, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{record_claim_feedback, ClaimFeedbackInput};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::transcript_processor_test_api::{
    process_transcript_with_kind, TranscriptContentKind,
};
use dailyos_lib::types::{AiModelConfig, CalendarEvent, LinkedEntity, MeetingType};
use dos568_support::{
    composition_version, fresh_full_db, invoke_account_overview_json, seed_account, shared,
    surface_actor_with_account_scope, ACCOUNT_ID,
};
use rusqlite::params;
use std::path::Path;

#[tokio::test]
async fn w6_transcript_claim_reaches_account_overview_runtime_surface() {
    let conn = fresh_full_db();
    seed_account(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 7, 15, 0, 0).unwrap());
    let rng = SeedableRng::new(1496);
    let external = ExternalClients::default();

    let (claim_ids, dismissed_claim_id, db) = {
        seed_processor_meeting(&conn);
        let db = ActionDb::from_conn(&conn);
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let fake_bin = workspace.path().join("bin");
        install_fake_claude(&fake_bin);
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", fake_bin.display(), old_path);
        // SAFETY: This integration file has one test; PATH is restored before the scoped setup exits.
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        let transcript_path = workspace.path().join("quill-runtime-transcript.md");
        std::fs::write(
            &transcript_path,
            "Customer: We can expand once onboarding is easier.\nCustomer: Security review is still blocking rollout.",
        )
        .expect("write synthetic transcript");

        let result = process_transcript_with_kind(
            workspace.path(),
            transcript_path.to_str().expect("transcript path utf8"),
            &processor_meeting(),
            None,
            Some(&db),
            "default",
            Some(&AiModelConfig::default()),
            TranscriptContentKind::Transcript,
        );
        // SAFETY: Restores the PATH captured above after the processor call completes.
        unsafe {
            std::env::set_var("PATH", old_path);
        }

        assert_eq!(result.status, "success");
        let claim_ids = transcript_claim_ids(&conn);
        assert_eq!(claim_ids.len(), 2);
        let source_type_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                   FROM workspace_file_lifecycle
                  WHERE source_type = 'quill_transcript'
                    AND lifecycle_state = 'ingested'",
                [],
                |row| row.get(0),
            )
            .expect("read workspace lifecycle proof");
        assert_eq!(source_type_count, 1);
        let dismissed_claim_id =
            transcript_claim_id_for_quote(&conn, "We can expand once onboarding is easier.");
        (claim_ids, dismissed_claim_id, shared(conn))
    };
    let output =
        invoke_account_overview_json(db.clone(), surface_actor_with_account_scope(), 0).await;
    let rendered = output.to_string();

    assert_eq!(composition_version(&output), 1);
    assert!(
        claim_ids.iter().all(|claim_id| rendered.contains(claim_id)),
        "runtime composition must carry the transcript-backed claim id"
    );
    assert!(
        rendered.contains("Their voice"),
        "account overview must render the claim-backed transcript quote block"
    );
    assert!(
        rendered.contains("We can expand once onboarding is easier."),
        "verified transcript quote text must reach the runtime surface"
    );
    assert!(
        rendered.contains("Security review is still blocking rollout."),
        "second verified transcript quote from the same source must reach the runtime surface"
    );
    assert!(
        rendered.contains("Workspace file (Quill transcript)"),
        "runtime surface must preserve workspace-file source labeling"
    );
    assert!(
        rendered.contains("2026-06-07T14:30:00+00:00"),
        "runtime surface must carry transcript source-as-of"
    );
    assert!(
        rendered.contains("\"workspace_file_kind\":\"quill_transcript\""),
        "runtime quote payload must preserve transcript source kind"
    );
    assert!(
        rendered.contains("\"trust_band\":\"needs_verification\""),
        "unscored transcript claim must render cautiously"
    );
    assert!(
        rendered.contains("\"sensitivity\":\"internal\""),
        "runtime quote payload must carry claim sensitivity"
    );
    assert!(
        rendered.contains("\"redaction_state\":\"policy_allowed\""),
        "runtime quote payload must expose quote redaction state"
    );

    {
        let guard = db.lock().expect("lock test db for feedback");
        let feedback_ctx =
            ServiceContext::new_live(&clock, &rng, &external).with_actor("user:w6_transcript_test");
        record_claim_feedback(
            &feedback_ctx,
            ActionDb::from_conn(&guard),
            ClaimFeedbackInput {
                claim_id: dismissed_claim_id.clone(),
                action: FeedbackAction::MarkFalse,
                actor: "user:w6_transcript_test".to_string(),
                actor_id: Some("user-w6-transcript-test".to_string()),
                payload_json: None,
            },
        )
        .expect("standard claim feedback applies to transcript-backed claim");

        let (feedback_count, surface_state): (i64, String) = guard
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM claim_feedback WHERE claim_id = ?1),
                    surfacing_state
                 FROM intelligence_claims
                 WHERE id = ?1",
                [&dismissed_claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read transcript claim feedback state");
        assert_eq!(feedback_count, 1);
        assert_eq!(surface_state, "dormant");
    }

    let rerendered =
        invoke_account_overview_json(db.clone(), surface_actor_with_account_scope(), 1).await;
    let rerendered_text = rerendered.to_string();

    assert!(
        !rerendered_text.contains("We can expand once onboarding is easier."),
        "follow-up account overview must stop rendering the corrected transcript claim"
    );
    assert!(
        rerendered_text.contains("Security review is still blocking rollout."),
        "unaffected transcript claim should remain visible after targeted feedback"
    );
}

fn processor_meeting() -> CalendarEvent {
    CalendarEvent {
        id: "meeting-w6-runtime".to_string(),
        title: "Example Account working session".to_string(),
        start: Utc.with_ymd_and_hms(2026, 6, 7, 14, 30, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 6, 7, 15, 0, 0).unwrap(),
        meeting_type: MeetingType::Customer,
        attendees: Vec::new(),
        is_all_day: false,
        series_id: None,
        account: Some("Example Account".to_string()),
        linked_entities: Some(vec![LinkedEntity {
            id: ACCOUNT_ID.to_string(),
            name: "Example Account".to_string(),
            entity_type: "account".to_string(),
            confidence: 0.95,
            is_primary: true,
            suggested: false,
            ..Default::default()
        }]),
        classified_entities: None,
        scored_classified_entities: None,
    }
}

fn seed_processor_meeting(conn: &rusqlite::Connection) {
    conn.execute(
        "INSERT INTO meetings (id, title, meeting_type, start_time, end_time, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "meeting-w6-runtime",
            "Example Account working session",
            "customer",
            "2026-06-07T14:30:00Z",
            "2026-06-07T15:00:00Z",
            "2026-06-07T14:30:00Z"
        ],
    )
    .expect("seed meeting");
    conn.execute(
        "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
         VALUES (?1, ?2, 'account', 0.95, 1)",
        params!["meeting-w6-runtime", ACCOUNT_ID],
    )
    .expect("seed meeting entity");
}

fn transcript_claim_ids(conn: &rusqlite::Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare(
            "SELECT id
               FROM intelligence_claims
              WHERE data_source = 'workspace_file:quill_transcript'
                AND claim_state = 'active'
              ORDER BY claim_type",
        )
        .expect("prepare transcript claim query");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("query transcript claims")
        .collect::<Result<Vec<_>, _>>()
        .expect("read transcript claim ids")
}

fn transcript_claim_id_for_quote(conn: &rusqlite::Connection, quote: &str) -> String {
    conn.query_row(
        "SELECT id
           FROM intelligence_claims
          WHERE data_source = 'workspace_file:quill_transcript'
            AND claim_state = 'active'
            AND json_extract(metadata_json, '$.quote.text') = ?1
          LIMIT 1",
        params![quote],
        |row| row.get(0),
    )
    .expect("read transcript claim id for quote")
}

fn install_fake_claude(bin_dir: &Path) {
    std::fs::create_dir_all(bin_dir).expect("create fake bin dir");
    let fake = bin_dir.join("claude");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
prompt="$*"
if printf '%s' "$prompt" | grep -q 'INTERACTION_DYNAMICS'; then
cat <<'EOF'
COMMITMENTS:
END_COMMITMENTS
INTERACTION_DYNAMICS:
END_INTERACTION_DYNAMICS
ROLE_CHANGES:
END_ROLE_CHANGES
EOF
elif printf '%s' "$prompt" | grep -q 'CHAMPION_HEALTH'; then
cat <<'EOF'
WINS:
- [EXPANSION] Onboarding expansion path was confirmed #"We can expand once onboarding is easier."
END_WINS
RISKS:
- [YELLOW] Security review remains the blocker #"Security review is still blocking rollout."
END_RISKS
DECISIONS:
END_DECISIONS
CHAMPION_HEALTH:
- champion_name: unidentified
- champion_status: none
- champion_evidence: none
- champion_risk: none
END_CHAMPION_HEALTH
SENTIMENT:
- overall: neutral
- customer: neutral
- engagement: moderate
- forward_looking: yes
- competitor_mentions: none
- champion_present: unknown
- champion_engaged: n/a
- ownership_language: vendor
- past_tense_references: no
- data_export_interest: no
- internal_advocacy_visible: no
- roadmap_interest: no
END_SENTIMENT
EOF
else
cat <<'EOF'
SUMMARY: Example Account confirmed expansion depends on onboarding and security review.
DISCUSSION:
- Onboarding: The customer said onboarding gates expansion.
END_DISCUSSION
ANALYSIS: Expansion is plausible if onboarding and security blockers clear.
ACTIONS:
END_ACTIONS
EOF
fi
"#,
    )
    .expect("write fake claude");
    let mut perms = std::fs::metadata(&fake)
        .expect("fake claude metadata")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).expect("chmod fake claude");
    }
}
