use std::path::{Path, PathBuf};

fn manifest_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(manifest_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

fn has_dos7_allowlist(lines: &[&str], index: usize) -> bool {
    let current = lines[index];
    let previous = index
        .checked_sub(1)
        .and_then(|idx| lines.get(idx))
        .copied()
        .unwrap_or("");
    current.contains("dos7-allowed:") || previous.contains("dos7-allowed:")
}

#[test]
fn workspace_ingestion_w2b_no_new_direct_writes() {
    let touched_files = [
        "src/watcher.rs",
        "src/google_drive/poller.rs",
        "src/granola/poller.rs",
        "src/quill/poller.rs",
        "src/quill/sync.rs",
        "src/processor/transcript.rs",
    ];
    let direct_write_needles = [
        "std::fs::write",
        "std::fs::create_dir_all",
        "tokio::fs::write",
    ];

    for relative in touched_files {
        let source = read(relative);
        let lines = source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if direct_write_needles
                .iter()
                .any(|needle| line.contains(needle))
            {
                assert!(
                    has_dos7_allowlist(&lines, index),
                    "{relative}:{} direct workspace write lacks dos7-allowed rationale: {line}",
                    index + 1
                );
            }
        }
    }

    let watcher = read("src/watcher.rs");
    assert!(watcher.contains("dos7-allowed: inbox-bootstrap"));
    assert!(watcher.contains("dos7-allowed: entity-markdown-regen"));
    assert!(watcher.contains("dos7-allowed: content-index-cache"));
    // L2 cycle-1 BLOCK fold: §4 V1.2 preserves the processor trigger for
    // user attachments; do NOT remove this call. The W2-A pipeline shell
    // produces zero claims and does not populate db_content_files, so
    // routing user attachments through it instead of the processor would
    // silently break semantic retrieval for entity_type='user_context'.
    assert!(watcher.contains("crate::processor::process_user_attachment"));
    assert!(watcher.contains("dos7-allowed: user-attachment-processor-trigger"));

    let drive = read("src/google_drive/poller.rs");
    assert_eq!(drive.matches("dos7-allowed: drive-staging-v146").count(), 2);

    let transcript = read("src/processor/transcript.rs");
    assert_eq!(
        transcript
            .matches("dos7-allowed: transcript-direct-write-v146")
            .count(),
        4
    );

    let script = read("scripts/check_workspace_mutation_allowlist.sh");
    for token in [
        "drive-staging-v146",
        "inbox-bootstrap",
        "entity-markdown-regen",
        "content-index-cache",
        "transcript-direct-write-v146",
    ] {
        assert!(script.contains(token), "script recognizes {token}");
    }
}

#[test]
fn watcher_account_change_preserves_account_upsert_and_builds_ingest_request() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_account_changes");
    assert!(function.contains("db.upsert_account(&account)"));
    assert!(function.contains("accounts::write_account_markdown"));
    assert_order(
        function,
        "accounts::write_account_markdown",
        "ingest_after_upsert",
    );
    assert!(function.contains("WorkspaceFileKind::EntityDoc"));
    assert!(function.contains("EntityType::Account"));
}

#[test]
fn watcher_project_change_preserves_project_upsert_and_builds_ingest_request() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_project_changes");
    assert!(function.contains("db.upsert_project(&project)"));
    assert!(function.contains("projects::write_project_markdown"));
    assert_order(
        function,
        "projects::write_project_markdown",
        "ingest_after_upsert",
    );
    assert!(function.contains("WorkspaceFileKind::EntityDoc"));
    assert!(function.contains("EntityType::Project"));
}

#[test]
fn watcher_people_change_preserves_person_table_write_and_builds_ingest_request() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_people_changes");
    assert!(function.contains("people::upsert_person_and_restore_entity_links"));
    assert!(function.contains("people::write_person_markdown"));
    assert_order(
        function,
        "people::write_person_markdown",
        "ingest_after_upsert",
    );
    assert!(function.contains("WorkspaceFileKind::EntityDoc"));
    assert!(function.contains("EntityType::Person"));
}

#[test]
fn watcher_account_content_change_preserves_content_index_enrichment_trigger() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_account_content_changes");
    assert!(function.contains("accounts::sync_content_index_for_account"));
    assert!(function.contains("dos7-allowed: content-index-cache"));
    assert!(function.contains("ingest_after_upsert"));
}

#[test]
fn watcher_project_content_change_preserves_content_index_enrichment_trigger() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_project_content_changes");
    assert!(function.contains("projects::sync_content_index_for_project"));
    assert!(function.contains("dos7-allowed: content-index-cache"));
    assert!(function.contains("ingest_after_upsert"));
}

#[test]
fn watcher_user_attachment_change_preserves_processor_trigger_and_embedding_queue_wake() {
    let watcher = read("src/watcher.rs");
    let function = function_body(&watcher, "fn handle_user_attachment_changes");
    // §4 V1.2: W2-B preserves the trigger — the processor call does text
    // extraction + mechanical_summary + db_content_files upsert.
    assert!(function.contains("crate::processor::process_user_attachment"));
    assert!(function.contains("dos7-allowed: user-attachment-processor-trigger"));
    // Embedding wake remains so the embedding worker picks up the new
    // db_content_files row.
    assert!(function.contains("embedding_queue_wake.notify_one()"));
}

#[test]
fn ingest_request_compile_shape_uses_canonical_w2a_fields_and_test_pipeline() {
    let watcher = read("src/watcher.rs");
    let helper = function_body(&watcher, "fn ingest_after_upsert");
    assert!(helper.contains("WorkspaceSourceRegistry::open_validated"));
    assert_order(
        helper,
        "WorkspaceSourceRegistry::open_validated",
        "file_id_from_identity",
    );
    assert_order(helper, "file_id_from_identity", "IngestRequest {");
    for field in [
        "file,",
        "identity,",
        "file_id,",
        "source_asof,",
        "source_type,",
        "entity,",
        "mode: IngestionMode::Realtime",
        "category_hint: None",
        "invocation_actor:",
    ] {
        assert!(helper.contains(field), "missing canonical field {field}");
    }
    assert!(helper.contains("pipeline.run(&ctx, db, request)"));
    assert!(!helper.contains("source_path"));
    assert!(!helper.contains("data_source"));
}

#[test]
fn workspace_mutation_allowlist_lint_remains_green_after_w2_b_refactor() {
    let status = std::process::Command::new("bash")
        .arg("scripts/check_workspace_mutation_allowlist.sh")
        .current_dir(manifest_root())
        .status()
        .expect("run workspace mutation allowlist");
    assert!(status.success());
}

fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing {signature}"));
    let rest = &source[start..];
    let next_fn = rest[signature.len()..]
        .find("\nfn ")
        .or_else(|| rest[signature.len()..].find("\n/// Handle"))
        .unwrap_or(rest.len() - signature.len());
    &rest[..signature.len() + next_fn]
}

fn assert_order(source: &str, before: &str, after: &str) {
    let before_index = source
        .find(before)
        .unwrap_or_else(|| panic!("missing {before}"));
    let after_index = source
        .find(after)
        .unwrap_or_else(|| panic!("missing {after}"));
    assert!(
        before_index < after_index,
        "expected {before} before {after}"
    );
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
