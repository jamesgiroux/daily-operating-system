mod dos383_external_replay_integration_test {
    use std::sync::Arc;

    use base64::Engine;
    use chrono::{TimeZone, Utc};
    use dailyos_lib::services::context::{
        ExecutionMode, ExternalClientError, ExternalClients, FixedClock, GleanAccountFacts,
        GleanClientHandle, SeedableRng, ServiceContext,
    };
    use dailyos_lib::services::external_replay::JsonExternalReplayFixture;
    use serde_json::json;

    const AUTH_SCOPE_ID: &str = "auth-scope-acme-example";
    const ACCOUNT_ID: &str = "acme.example.com";

    fn empty_fixture() -> JsonExternalReplayFixture {
        JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [],
            }),
            "inline-empty",
        )
        .expect("empty external replay fixture should load")
    }

    fn fixture_with_glean_account_facts() -> JsonExternalReplayFixture {
        let key =
            GleanClientHandle::request_key_for_fetch_account_facts(ACCOUNT_ID, AUTH_SCOPE_ID);
        let body = br#"{"account_id":"acme.example.com","facts":["example account fact","example renewal fact"]}"#;
        let body_base64 = base64::engine::general_purpose::STANDARD.encode(body);

        JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [
                    {
                        "request_key_hex": key.to_hex(),
                        "auth_scope_id": AUTH_SCOPE_ID,
                        "response": {
                            "status": 200,
                            "headers": [["Content-Type", "application/json"]],
                            "body_base64": body_base64,
                        },
                    },
                ],
            }),
            "inline-glean-account-facts",
        )
        .expect("single-response external replay fixture should load")
    }

    fn clients_from_fixture(fixture: JsonExternalReplayFixture) -> ExternalClients {
        ExternalClients::from_replay(Arc::new(fixture), AUTH_SCOPE_ID.to_string())
    }

    fn fixture_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap())
    }

    fn fixture_rng() -> SeedableRng {
        SeedableRng::new(383)
    }

    #[test]
    fn harness_replay_miss_propagates_typed_error_via_external_clients() {
        let clients = clients_from_fixture(empty_fixture());
        let expected_key =
            GleanClientHandle::request_key_for_fetch_account_facts(ACCOUNT_ID, AUTH_SCOPE_ID);

        let err = clients
            .glean
            .fetch_account_facts(ACCOUNT_ID)
            .expect_err("empty replay fixture should miss");

        match err {
            ExternalClientError::ReplayFixtureMissing(missing) => {
                assert_eq!(missing.request_key_hex, expected_key.to_hex());
                assert!(!missing.request_key_hex.is_empty());
                assert_eq!(missing.method, "GET");
                assert_eq!(missing.url_redacted, "https://glean.example.com/<redacted>");
            }
            other => panic!("expected ExternalReplayFixtureMissing, got {other:?}"),
        }
    }

    #[test]
    fn harness_replay_hit_returns_expected_response_via_external_clients() {
        let clients = clients_from_fixture(fixture_with_glean_account_facts());

        let response = clients
            .glean
            .fetch_account_facts(ACCOUNT_ID)
            .expect("matching replay fixture should return canned response");

        assert_eq!(
            response,
            GleanAccountFacts {
                account_id: ACCOUNT_ID.to_string(),
                facts: vec![
                    "example account fact".to_string(),
                    "example renewal fact".to_string(),
                ],
            }
        );
    }

    #[test]
    fn harness_external_clients_default_constructor_uses_live_in_production() {
        let clients = ExternalClients::default();

        assert!(!clients.is_replay_mode());
    }

    #[test]
    fn harness_external_clients_from_replay_constructor_uses_replay_mode() {
        let clients = clients_from_fixture(empty_fixture());

        assert!(clients.is_replay_mode());
    }

    #[test]
    fn harness_service_context_new_evaluate_default_constructs_replay_context_with_no_fixture_required() {
        let clock = fixture_clock();
        let rng = fixture_rng();

        let ctx = ServiceContext::new_evaluate_default(&clock, &rng);

        assert_eq!(ctx.mode, ExecutionMode::Evaluate);
        assert!(ctx.external.is_replay_mode());
    }

    #[test]
    #[should_panic(expected = "Evaluate ServiceContext requires replay-mode ExternalClients")]
    fn harness_service_context_new_evaluate_panics_on_live_external_clients() {
        let clock = fixture_clock();
        let rng = fixture_rng();
        let clients = ExternalClients::default();

        let _ = ServiceContext::new_evaluate(&clock, &rng, &clients);
    }
}
