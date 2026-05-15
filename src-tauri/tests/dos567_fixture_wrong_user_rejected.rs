//! W4-B ac §44 + §17 — wp_user_id session binding rejects body /
//! query / header channels.
//!
//! `validate_session_bound_wp_user_id` walks any request payload, collects
//! every `wp_user_id` occurrence at any depth, and rejects 403 `wrong_user`
//! when the asserted value diverges from the session-bound id (established at
//! pairing). This fixture exercises all three channels (body, query-style,
//! header-style) — the substrate-sweep agent is wiring query+header coverage;
//! this test pins the contract across all three.

use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::bridges::surface_client::{validate_session_bound_wp_user_id, WrongUserRejection};
use dailyos_lib::services::surface_pairing::ValidatedSurfaceSession;

fn session_bound_to(wp_user_id: u64) -> ValidatedSurfaceSession {
    let scope_set =
        ScopeSet::new([SurfaceScope::new("read.account_overview")]).expect("non-empty scopes");
    ValidatedSurfaceSession {
        surface_client_id: "sc-wrong-user-test".to_string(),
        session_id: "sess-wrong-user-test".to_string(),
        actor: Actor::SurfaceClient {
            instance: SurfaceClientId::new("sc-wrong-user-test"),
            scopes: scope_set,
        },
        wp_user_id: Some(wp_user_id),
        wp_user_hash: Some("hash-100".to_string()),
        wp_site_id: "site-1".to_string(),
        wp_site_id_hash: "site-hash-1".to_string(),
        site_binding_digest: "site-digest-1".to_string(),
        site_nonce: "nonce-1".to_string(),
        scope_digest: "scope-digest-1".to_string(),
        granted_scopes: vec!["read.account_overview".to_string()],
    }
}

#[test]
fn dos567_wrong_user_rejected_via_body_assertion() {
    let session = session_bound_to(100);
    // Channel 1: forged wp_user_id in the JSON body.
    let body = serde_json::json!({
        "wp_user_id": 200,
        "ability": "get_account_overview",
        "input": {},
    });
    let rejection =
        validate_session_bound_wp_user_id(&session, &body).expect_err("body mismatch must reject");
    let WrongUserRejection {
        asserted_wp_user_id,
        session_wp_user_id,
        surface_client_id,
    } = rejection;
    assert_eq!(asserted_wp_user_id, 200);
    assert_eq!(session_wp_user_id, Some(100));
    assert_eq!(surface_client_id, "sc-wrong-user-test");
}

#[test]
fn dos567_wrong_user_rejected_via_query_style_assertion() {
    let session = session_bound_to(100);
    // Channel 2: query-string-flavored payload (the harness lifts query
    // params into the same JSON shape before validation).
    let query = serde_json::json!({
        "query": {
            "wp_user_id": "200",
            "scope": "read.account_overview",
        },
    });
    let rejection = validate_session_bound_wp_user_id(&session, &query)
        .expect_err("query-style mismatch must reject");
    assert_eq!(rejection.asserted_wp_user_id, 200);
    assert_eq!(rejection.session_wp_user_id, Some(100));
}

#[test]
fn dos567_wrong_user_rejected_via_header_style_assertion() {
    let session = session_bound_to(100);
    // Channel 3: header-flavored payload (X-WP-User-Id lifted to JSON).
    let header_envelope = serde_json::json!({
        "headers": {
            "X-WP-User-Id": "200",
            "wp_user_id": 200,
        },
    });
    let rejection = validate_session_bound_wp_user_id(&session, &header_envelope)
        .expect_err("header-style mismatch must reject");
    assert_eq!(rejection.asserted_wp_user_id, 200);
    assert_eq!(rejection.session_wp_user_id, Some(100));
}

#[test]
fn dos567_matching_wp_user_id_in_body_passes() {
    let session = session_bound_to(100);
    let body = serde_json::json!({
        "wp_user_id": 100,
        "ability": "get_account_overview",
    });
    assert!(validate_session_bound_wp_user_id(&session, &body).is_ok());
}

#[test]
fn dos567_nested_wp_user_id_assertion_caught_at_depth() {
    // §17 says "at any depth" — the walker recurses through nested objects
    // and arrays. A forged wp_user_id buried inside a sub-object is still
    // rejected.
    let session = session_bound_to(100);
    let body = serde_json::json!({
        "ability": "submit_feedback",
        "input": {
            "metadata": {
                "wp_user_id": 200,
            },
        },
    });
    let rejection = validate_session_bound_wp_user_id(&session, &body)
        .expect_err("nested wp_user_id must reject");
    assert_eq!(rejection.asserted_wp_user_id, 200);
}
