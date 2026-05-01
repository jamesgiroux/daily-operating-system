use std::collections::BTreeMap;

use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    provenance_for_test, FieldAttribution, FieldPath, Provenance, PROVENANCE_SCHEMA_VERSION,
    SubjectAttribution, SubjectRef, ThreadId,
};

fn fixture_provenance() -> Provenance {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
    provenance_for_test(
        "thread_fixture",
        produced_at,
        subject.clone(),
        Vec::new(),
        Vec::new(),
        BTreeMap::from([(
            FieldPath::new("/name").unwrap(),
            FieldAttribution::constant(subject),
        )]),
        None,
        Vec::new(),
    )
}

#[test]
fn thread_ids_default_empty_roundtrip() {
    let provenance = fixture_provenance();
    let mut value = serde_json::to_value(&provenance).unwrap();
    value.as_object_mut().unwrap().remove("thread_ids");

    let decoded: Provenance = serde_json::from_value(value).unwrap();

    assert!(decoded.thread_ids.is_empty());
}

#[test]
fn thread_ids_two_ids_roundtrip() {
    let mut provenance = fixture_provenance();
    provenance.thread_ids = vec![ThreadId::new("thread-a"), ThreadId::new("thread-b")];

    let decoded: Provenance =
        serde_json::from_value(serde_json::to_value(&provenance).unwrap()).unwrap();

    assert_eq!(decoded.thread_ids, provenance.thread_ids);
}

#[test]
fn provenance_schema_version_is_one() {
    let provenance = fixture_provenance();

    assert_eq!(provenance.provenance_schema_version, PROVENANCE_SCHEMA_VERSION);
}
