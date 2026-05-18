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

#[test]
fn dos673_load_session_master_key_returns_classified_lookup() {
    let source = read_source("src/services/surface_session_keychain.rs");
    let body = function_body(&source, "pub fn load_session_master_key");
    assert!(
        body.contains("-> SessionKeyLookup"),
        "load_session_master_key must return SessionKeyLookup"
    );
    assert!(
        !body.contains("-> Option<"),
        "load_session_master_key must not collapse lookup failures into Option"
    );
    assert!(source.contains("pub enum SessionKeyLookup"));
    assert!(source.contains("Found([u8; KEY_BYTES])"));
    assert!(source.contains("NotFound"));
    assert!(source.contains("Unavailable { reason: String }"));
}

#[test]
fn dos674_no_direct_keychain_delete_inside_pairing_writer_closures() {
    let pairing = read_source("src/services/surface_pairing.rs");
    let runtime = read_source("src/surface_runtime/mod.rs");
    let commands = read_source("src/commands/surface_runtime.rs");

    assert_eq!(
        pairing.matches("delete_session_master_key(").count(),
        1,
        "surface_pairing should call delete_session_master_key only from cleanup_session_keychain_entries"
    );
    assert!(
        function_body(&pairing, "pub fn cleanup_session_keychain_entries")
            .contains("delete_session_master_key("),
        "the single keychain delete should live in the post-commit cleanup helper"
    );

    for source in [&pairing, &runtime, &commands] {
        for keychain_call in [
            "delete_session_master_key(",
            "load_session_master_key(",
            "persist_session_master_key(",
        ] {
            for line in source.lines().filter(|line| line.contains(keychain_call)) {
                assert!(
                    !line.contains("db_write") && !line.contains("with_transaction"),
                    "keychain IO must not be inlined into writer closure lines: {line}"
                );
            }
        }
    }
}

#[test]
fn dos673_rehydration_keychain_missing_revocation_is_notfound_only() {
    let source = read_source("src/surface_runtime/mod.rs");
    let body = function_body(&source, "async fn rehydrate_sessions_from_keychain");
    let not_found = body
        .find("SessionKeyLookup::NotFound")
        .expect("rehydration must match NotFound");
    let unavailable = body
        .find("SessionKeyLookup::Unavailable")
        .expect("rehydration must match Unavailable separately");
    let not_found_arm = &body[not_found..unavailable];

    assert!(
        not_found_arm.contains("missing.push(row);"),
        "NotFound must be the only arm that queues keychain_entry_missing revocation"
    );
    assert!(
        !body[unavailable..].contains("missing.push(row);"),
        "Unavailable must not queue keychain_entry_missing revocation"
    );
    assert!(body.contains("keychain_entry_missing"));
}

#[test]
fn dos675_stop_and_drop_cleanup_before_abort() {
    let source = read_source("src/surface_runtime/mod.rs");
    let stop = function_body(&source, "pub fn stop(&self)");
    let drop_body = function_body(&source, "fn drop(&mut self)");

    for (name, body) in [("stop", stop), ("drop", drop_body)] {
        let cleanup = body
            .find("explicit_sentinel_cleanup();")
            .unwrap_or_else(|| panic!("{name} must call explicit_sentinel_cleanup"));
        let abort = body
            .find("endpoint.abort.abort();")
            .unwrap_or_else(|| panic!("{name} must abort the endpoint"));
        assert!(
            cleanup < abort,
            "{name} must remove the sentinel before aborting the listener"
        );
    }
}

#[test]
fn dos675_drop_does_not_call_async_flush_or_db_writer() {
    let source = read_source("src/surface_runtime/mod.rs");
    let drop_body = function_body(&source, "fn drop(&mut self)");
    assert!(!drop_body.contains(".await"));
    assert!(!drop_body.contains("db_write"));
    assert!(!drop_body.contains("flush_session_activity_on_shutdown"));
    assert!(!drop_body.contains("stop_async"));
}
