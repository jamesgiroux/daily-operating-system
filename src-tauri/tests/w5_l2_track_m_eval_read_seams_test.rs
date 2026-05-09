mod w5_l2_track_m_eval_read_seams_test {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use dailyos_lib::abilities::get_entity_context::{
        get_entity_context, ContextDepth, GetEntityContextInput,
    };
    use dailyos_lib::abilities::{
        AbilityContext, AbilityError, AbilityErrorKind, AbilityRegistry, Actor, NOOP_ABILITY_TRACER,
    };
    use dailyos_lib::intelligence::provider::ReplayProvider;
    use dailyos_lib::services::context::{
        ClaimDismissalSurface, ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };
    use dailyos_lib::services::external_replay::JsonExternalReplayFixture;
    use serde_json::json;

    const AUTH_SCOPE_ID: &str = "auth-scope-w5-l2-track-m";

    fn empty_external_clients() -> ExternalClients {
        let fixture = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [],
            }),
            "w5-l2-track-m-empty",
        )
        .expect("empty external replay fixture should load");
        ExternalClients::from_replay(Arc::new(fixture), AUTH_SCOPE_ID.to_string())
    }

    fn fixture_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap())
    }

    fn assert_fixture_reader_required(err: AbilityError, code: &str, reader: &str) {
        assert_eq!(err.kind, AbilityErrorKind::HardError(code.to_string()));
        assert_reader_error_message(&err.message, reader);
    }

    fn assert_reader_error_message(message: &str, reader: &str) {
        assert!(
            message.contains("evaluate mode requires an injected fixture reader"),
            "unexpected error message: {}",
            message
        );
        assert!(
            message.contains(reader),
            "unexpected error message: {}",
            message
        );
        assert!(
            message.contains("refusing to read from live workspace DB"),
            "unexpected error message: {}",
            message
        );
    }

    #[tokio::test]
    async fn evaluate_get_entity_context_without_claim_reader_fails_closed() {
        let clock = fixture_clock();
        let rng = SeedableRng::new(218);
        let external = empty_external_clients();
        let services = ServiceContext::new_evaluate(&clock, &rng, &external);
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::Eval,
        );

        let err = get_entity_context(
            &ctx,
            GetEntityContextInput {
                schema_version: 2,
                entity_type: "account".to_string(),
                entity_id: "acct-track-m".to_string(),
                depth: ContextDepth::Standard,
            },
        )
        .await
        .expect_err("missing claim reader must fail before live DB fallback");

        assert_fixture_reader_required(
            err,
            "entity context claim read failed",
            "entity_context_claim_reader",
        );
    }

    #[tokio::test]
    async fn evaluate_prepare_meeting_without_context_reader_fails_closed() {
        let registry = AbilityRegistry::from_inventory_checked().unwrap();
        let clock = fixture_clock();
        let rng = SeedableRng::new(219);
        let external = empty_external_clients();
        let services = ServiceContext::new_evaluate(&clock, &rng, &external);
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::Eval,
        );

        let err = registry
            .invoke_by_name_json(
                &ctx,
                "prepare_meeting",
                json!({
                    "meeting_id": "meeting-track-m",
                    "depth": 2,
                    "include_open_loops": true,
                    "schema_version": 1,
                }),
            )
            .await
            .expect_err("missing prepare_meeting reader must fail before live DB fallback");

        assert_fixture_reader_required(
            err,
            "prepare_meeting_context_read",
            "prepare_meeting_context_reader",
        );
    }

    #[tokio::test]
    async fn evaluate_legacy_entity_context_without_reader_fails_closed() {
        let clock = fixture_clock();
        let rng = SeedableRng::new(220);
        let external = empty_external_clients();
        let services = ServiceContext::new_evaluate(&clock, &rng, &external);

        let err = services
            .read_entity_context_entries("account".to_string(), "acct-track-m".to_string())
            .await
            .expect_err("missing legacy entity context reader must fail before live DB fallback");

        assert_reader_error_message(&err, "entity_context_reader");
    }
}
