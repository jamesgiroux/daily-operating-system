use dailyos_lib::db::ActionDb;
use rusqlite::Connection;

#[test]
fn assign_inbox_entity_rejects_invalid_source_type_slug() {
    let conn = Connection::open_in_memory().expect("sqlite");
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let err = dailyos_lib::command_test_api::assign_inbox_entity_for_tests(
        db,
        workspace.path(),
        "missing".to_string(),
        "account".to_string(),
        "acme".to_string(),
        "acme".to_string(),
        "bogus".to_string(),
    )
    .expect_err("invalid source type slug should reject");

    assert_eq!(err, "invalid source_type_slug");
}
