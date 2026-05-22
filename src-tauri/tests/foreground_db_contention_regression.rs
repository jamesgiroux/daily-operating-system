use std::fs;
use std::path::PathBuf;

fn source_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_source(relative: &str) -> String {
    fs::read_to_string(source_path(relative)).unwrap_or_else(|error| {
        panic!("read {relative}: {error}");
    })
}

fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing function signature: {signature}"));
    let after = &source[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("missing function body: {signature}"));
    let mut depth = 0_i32;
    for (offset, byte) in after[open..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &after[..open + offset + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated function body: {signature}");
}

fn assert_body_excludes(body: &str, forbidden: &[&str], context: &str) {
    for token in forbidden {
        assert!(
            !body.contains(token),
            "{context} must not contain `{token}`"
        );
    }
}

#[test]
fn foreground_account_and_meeting_get_routes_do_not_mutate() {
    let accounts = read_source("src/services/accounts.rs");
    let meetings = read_source("src/services/meetings.rs");

    let account_detail = function_body(&accounts, "pub async fn get_account_detail");
    assert_body_excludes(
        account_detail,
        &[
            ".db_write",
            "tokio::spawn",
            "spawn_blocking",
            ".enqueue(",
            "ensure_account_lifecycle_state",
        ],
        "get_account_detail",
    );

    let meeting_intelligence = function_body(&meetings, "pub async fn get_meeting_intelligence");
    assert_body_excludes(
        meeting_intelligence,
        &[
            ".db_write",
            "tokio::spawn",
            "spawn_blocking",
            ".enqueue(",
            "ensure_meeting_in_history",
            "mark_prep_reviewed",
            "clear_meeting_new_signals",
        ],
        "get_meeting_intelligence",
    );
}

#[test]
fn settings_status_commands_use_read_only_db_access() {
    let integrations = read_source("src/commands/integrations.rs");

    for signature in [
        "pub async fn get_context_mode",
        "pub async fn get_gravatar_status",
        "pub async fn get_linear_status",
    ] {
        let body = function_body(&integrations, signature);
        assert!(
            body.contains(".db_read("),
            "{signature} must use AppState::db_read"
        );
        assert_body_excludes(
            body,
            &["ActionDb::open", ".with_db(", "open_fresh_serialized"],
            signature,
        );
    }
}

#[test]
fn calendar_hot_path_routes_writes_through_service_batch() {
    let google = read_source("src/google.rs");
    let people_service = read_source("src/services/people.rs");

    let populate = function_body(&google, "async fn populate_people_from_events");
    assert!(
        populate.contains("record_calendar_attendance_batch"),
        "populate_people_from_events must route the sampled hot path through the service batch"
    );
    assert_body_excludes(
        populate,
        &[
            "ActionDb::open",
            "ensure_meeting_in_history",
            "record_meeting_attendance",
            "write_person_json",
            "write_person_markdown",
        ],
        "populate_people_from_events",
    );

    let service_batch = function_body(
        &people_service,
        "pub(crate) fn record_calendar_attendance_batch",
    );
    assert!(service_batch.contains("db.with_transaction"));
    assert!(service_batch.contains("ensure_meeting_in_history"));
    assert!(service_batch.contains("record_meeting_attendance"));
    assert_body_excludes(
        service_batch,
        &[
            "ActionDb::open",
            "write_person_json",
            "write_person_markdown",
        ],
        "record_calendar_attendance_batch",
    );
}

#[test]
fn touched_calendar_diagnostics_use_opaque_ids_or_counts() {
    let google = read_source("src/google.rs");
    let people_service = read_source("src/services/people.rs");
    let populate = function_body(&google, "async fn populate_people_from_events");
    let service_batch = function_body(
        &people_service,
        "pub(crate) fn record_calendar_attendance_batch",
    );

    for old_pii_log in [
        "Failed to ensure meeting '",
        "Failed to write person.json for '",
        "Failed to write person.md for '",
        "person.name",
    ] {
        assert!(
            !populate.contains(old_pii_log) && !service_batch.contains(old_pii_log),
            "touched calendar diagnostics must not log names/titles: {old_pii_log}"
        );
    }

    assert!(people_service.contains("meeting_id={}"));
    assert!(google.contains("person_id={}"));
    assert!(google.contains("discovered {} new people"));
}

#[test]
fn latency_labels_are_stable_and_non_pii() {
    let db_service = read_source("src/db_service.rs");
    let state = read_source("src/state.rs");
    let integrations = read_source("src/commands/integrations.rs");

    for label in [
        "db_read",
        "db_write",
        "app_state.db_read.total",
        "app_state.db_write.total",
        "open_fresh_serialized",
        "get_context_mode",
        "get_linear_status",
        "get_gravatar_status",
        "get_audit_log_records",
    ] {
        assert!(
            db_service.contains(label) || state.contains(label) || integrations.contains(label),
            "missing latency label `{label}`"
        );
    }

    let record_worker_latency = function_body(&db_service, "fn record_worker_latency");
    assert_body_excludes(
        record_worker_latency,
        &["account", "meeting", "email", "domain", "title", "sql"],
        "record_worker_latency",
    );
}
