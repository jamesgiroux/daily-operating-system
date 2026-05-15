use dailyos_lib::abilities::extractors::commitment::derive_commitment_id;

#[test]
fn duplicate_open_loop_commitments_collapse_to_same_identity() {
    let transcript = derive_commitment_id(
        "Follow up with procurement",
        "account-example",
        Some("2026-05-20"),
        Some("Alex Example"),
    );
    let email = derive_commitment_id(
        " follow-up with procurement ",
        "account-example",
        Some("2026-05-20T14:30:00Z"),
        Some("alex.example"),
    );
    let genuinely_new = derive_commitment_id(
        "Follow up with legal",
        "account-example",
        Some("2026-05-20"),
        Some("Alex Example"),
    );

    assert_eq!(transcript, email);
    assert_ne!(transcript, genuinely_new);
}
