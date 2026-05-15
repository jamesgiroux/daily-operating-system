mod dos568_support;

use dailyos_lib::abilities::registry::AbilityRegistry;
use dos568_support::{
    claim_event_count, composition_event_count, composition_version, fresh_full_db,
    invoke_account_overview_json, seed_base_account_state, seed_claim, shared,
    surface_actor_with_account_scope, ACCOUNT_OVERVIEW_ABILITY,
};

#[tokio::test]
async fn dos568_claim_trigger_reinvokes_and_commits_new_composition_version() {
    let conn = fresh_full_db();
    seed_base_account_state(&conn);
    let db = shared(conn);

    let first =
        invoke_account_overview_json(db.clone(), surface_actor_with_account_scope(), 0).await;
    assert_eq!(composition_version(&first), 1);

    {
        let conn = db.lock().expect("db lock");
        seed_claim(
            &conn,
            "claim-dos568-invalidation",
            "commitment",
            "/commitments/next",
            "Follow up on implementation checklist",
        );
        assert!(
            claim_event_count(&conn) >= 3,
            "claim commits emit DOS-589 trigger rows"
        );
    }

    let descriptor = AbilityRegistry::global_checked()
        .expect("global registry builds")
        .iter_all()
        .find(|descriptor| descriptor.name == ACCOUNT_OVERVIEW_ABILITY)
        .expect("account overview descriptor registered");
    for trigger in [
        "claim.version",
        "account_subject.claim_changed",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation",
    ] {
        assert!(
            descriptor
                .signal_policy
                .emits_on_output_change
                .contains(&trigger),
            "missing DOS-589 invalidation trigger {trigger}"
        );
    }

    let second =
        invoke_account_overview_json(db.clone(), surface_actor_with_account_scope(), 1).await;
    assert_eq!(composition_version(&second), 2);
    assert!(second.to_string().contains("claim-dos568-invalidation"));

    let conn = db.lock().expect("db lock");
    assert_eq!(composition_event_count(&conn), 2);
}
