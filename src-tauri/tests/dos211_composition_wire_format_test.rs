use std::collections::BTreeMap;

use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    provenance_for_test, ComposedProvenance, CompositionId, FieldAttribution, FieldPath,
    SubjectAttribution, SubjectRef,
};

#[test]
fn children_serialize_as_bare_provenance_array_per_adr_0105() {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
    let child = provenance_for_test(
        "child_lookup",
        produced_at,
        subject.clone(),
        Vec::new(),
        Vec::new(),
        BTreeMap::from([(
            FieldPath::new("/name").unwrap(),
            FieldAttribution::constant(subject.clone()),
        )]),
        None,
        Vec::new(),
    );
    let parent = provenance_for_test(
        "parent_summary",
        produced_at,
        subject.clone(),
        Vec::new(),
        vec![ComposedProvenance::new(CompositionId::new("lookup"), child)],
        BTreeMap::from([(
            FieldPath::new("/name").unwrap(),
            FieldAttribution::constant(subject),
        )]),
        None,
        Vec::new(),
    );

    let value = serde_json::to_value(parent).unwrap();
    let first_child = &value["children"][0];

    assert_eq!(first_child["ability_name"], "child_lookup");
    assert!(first_child.get("composition_id").is_none());
    assert!(first_child.get("provenance").is_none());
}
