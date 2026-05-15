use dailyos_lib::intelligence::canonicalization::{item_hash, ItemKind};

#[test]
fn canonicalization_collapses_exact_and_whitespace_duplicates() {
    assert_eq!(
        item_hash(ItemKind::Risk, "  Renewal\tis\nblocked  "),
        item_hash(ItemKind::Risk, "Renewal is blocked")
    );
}

#[test]
fn canonicalization_keeps_near_duplicate_and_negative_cases_apart() {
    let original = item_hash(ItemKind::Risk, "Renewal is blocked by legal");
    let near = item_hash(ItemKind::Risk, "Renewal may be blocked by legal");
    let different_kind = item_hash(ItemKind::Win, "Renewal is blocked by legal");

    assert_ne!(original, near);
    assert_eq!(
        original, different_kind,
        "current substrate hashes canonical item text; claim kind is reserved for forward compatibility"
    );
}
