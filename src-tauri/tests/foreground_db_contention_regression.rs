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
    let core = read_source("src/commands/core.rs");
    let meeting_detail_page = read_source("../src/pages/MeetingDetailPage.tsx");

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

    let mark_viewed_service =
        function_body(&meetings, "pub async fn mark_meeting_intelligence_viewed");
    assert!(
        mark_viewed_service.contains(".db_write(")
            && mark_viewed_service.contains("mark_prep_reviewed")
            && mark_viewed_service.contains("clear_meeting_new_signals"),
        "viewed-state mutation must be an explicit service write, not hidden in the read route"
    );
    let mark_viewed_command = function_body(&core, "pub async fn mark_meeting_intelligence_viewed");
    assert!(
        mark_viewed_command.contains("services::meetings::mark_meeting_intelligence_viewed"),
        "mark_meeting_intelligence_viewed command must route through the meetings service"
    );
    let load_meeting_intelligence = function_body(
        &meeting_detail_page,
        "const loadMeetingIntelligence = useCallback(async () =>",
    );
    assert!(
        load_meeting_intelligence.contains("mark_meeting_intelligence_viewed")
            && load_meeting_intelligence.contains("const hadNewSignals"),
        "meeting detail must mark viewed after every successful payload, with badge clearing gated separately"
    );
    assert!(
        !load_meeting_intelligence.contains("if (intel.intelligenceQuality?.hasNewSignals)"),
        "mark_meeting_intelligence_viewed must not be gated by hasNewSignals"
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
        if signature == "pub async fn get_gravatar_status" {
            assert!(
                body.contains("spawn_blocking"),
                "get_gravatar_status must not block the async worker on keychain access"
            );
        }
    }
}

#[test]
fn calendar_hot_path_routes_writes_through_service_batch() {
    let google = read_source("src/google.rs");
    let people_service = read_source("src/services/people.rs");

    for signature in [
        "async fn poll_calendar",
        "async fn save_attendee_display_names",
        "async fn generate_preps_for_new_meetings",
        "async fn detect_cancelled_meetings",
        "async fn load_last_sync_success",
    ] {
        let body = function_body(&google, signature);
        assert_body_excludes(body, &["ActionDb::open", ".with_db("], signature);
    }

    let populate = function_body(&google, "async fn populate_people_from_events");
    let artifacts = function_body(
        &google,
        "async fn write_person_artifacts_after_calendar_batch",
    );
    assert!(
        populate.contains("record_calendar_attendance_batch"),
        "populate_people_from_events must route the sampled hot path through the service batch"
    );
    assert!(
        populate.contains("chunks(CALENDAR_ATTENDANCE_BATCH_EVENT_CHUNK)"),
        "calendar attendance writes must be bounded into chunked writer transactions"
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
    assert!(
        artifacts.contains("build_person_artifact_snapshot")
            && artifacts.contains("write_person_artifacts_from_snapshot"),
        "calendar artifact writes must snapshot DB data before filesystem I/O"
    );
    assert_body_excludes(
        artifacts,
        &["write_person_json", "write_person_markdown"],
        "write_person_artifacts_after_calendar_batch",
    );

    let service_batch = function_body(
        &people_service,
        "pub(crate) fn record_calendar_attendance_batch",
    );
    assert!(service_batch.contains("db.with_transaction"));
    assert!(service_batch.contains("ensure_meeting_in_history"));
    assert!(service_batch.contains("record_meeting_attendance"));
    assert!(
        service_batch.contains("emit_and_propagate(")
            && !service_batch.contains("emit_and_propagate_or_log"),
        "calendar person-created signal failures must avoid the generic helper that logs email-derived person IDs"
    );
    assert_body_excludes(
        service_batch,
        &[
            "ActionDb::open",
            "write_person_json",
            "write_person_markdown",
            "{entity_type}/{entity_id}",
            "person_id={}",
        ],
        "record_calendar_attendance_batch",
    );

    let display_names = function_body(
        &people_service,
        "pub(crate) fn record_attendee_display_names",
    );
    assert!(display_names.contains("db.with_transaction"));
    assert_body_excludes(
        display_names,
        &["ActionDb::open"],
        "record_attendee_display_names",
    );
}

#[test]
fn touched_calendar_diagnostics_use_opaque_ids_or_counts() {
    let google = read_source("src/google.rs");
    let people_service = read_source("src/services/people.rs");
    let poller = function_body(&google, "pub async fn run_calendar_poller");
    let generate_preps = function_body(&google, "async fn generate_preps_for_new_meetings");
    let populate = function_body(&google, "async fn populate_people_from_events");
    let detect_cancelled = function_body(&google, "async fn detect_cancelled_meetings");
    let service_batch = function_body(
        &people_service,
        "pub(crate) fn record_calendar_attendance_batch",
    );

    for old_pii_log in [
        "Failed to ensure meeting '",
        "Failed to write person.json for '",
        "Failed to write person.md for '",
        "person.name",
        "person_id={}",
        "meeting '{}' cancelled",
        "Failed to archive cancelled meeting",
        "artifact write failed: {}",
        "evaluate_meeting '{}'",
        "Generated reactive prep for '",
        "Failed to write reactive prep for '",
    ] {
        assert!(
            !poller.contains(old_pii_log)
                && !generate_preps.contains(old_pii_log)
                && !populate.contains(old_pii_log)
                && !detect_cancelled.contains(old_pii_log)
                && !service_batch.contains(old_pii_log),
            "touched calendar diagnostics must not log names/titles: {old_pii_log}"
        );
    }

    assert!(google.contains("discovered {} new people"));
    assert!(google.contains("artifact writes failed; count={}"));
}

#[test]
fn live_calendar_only_meeting_fallback_is_not_editable() {
    let meetings = read_source("src/services/meetings.rs");
    let live_fallback = function_body(&meetings, "fn build_live_calendar_meeting_intelligence");
    assert!(
        live_fallback.contains("can_edit_user_layer: false"),
        "live-calendar-only meeting detail cannot expose save controls without a persisted row"
    );
}

#[test]
fn latency_labels_are_stable_and_non_pii() {
    let db_service = read_source("src/db_service.rs");
    let state = read_source("src/state.rs");
    let integrations = read_source("src/commands/integrations.rs");
    let core = read_source("src/commands/core.rs");

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
        "mark_meeting_intelligence_viewed",
    ] {
        assert!(
            db_service.contains(label)
                || state.contains(label)
                || integrations.contains(label)
                || core.contains(label),
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
