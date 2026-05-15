use dailyos_lib::abilities::extractors::commitment::derive_commitment_id;

#[test]
fn derive_commitment_id_is_stable_for_same_normalized_inputs() {
    let first = derive_commitment_id(
        " Send   Renewal Deck! ",
        "account-example",
        Some("2026-05-15T10:00:00Z"),
        Some(" Alex.Example "),
    );
    let second = derive_commitment_id(
        "send renewal deck",
        "account-example",
        Some("2026-05-15"),
        Some("alex example"),
    );

    assert_eq!(first, second);
    assert!(first.starts_with("commitment:"));
}

#[test]
fn derive_commitment_id_separates_near_collision_inputs() {
    let base = derive_commitment_id("Send renewal deck", "account-example", None, Some("Alex"));
    let changed_owner =
        derive_commitment_id("Send renewal deck", "account-example", None, Some("Jamie"));
    let changed_subject =
        derive_commitment_id("Send renewal deck", "other-account-example", None, Some("Alex"));

    assert_ne!(base, changed_owner);
    assert_ne!(base, changed_subject);
}
