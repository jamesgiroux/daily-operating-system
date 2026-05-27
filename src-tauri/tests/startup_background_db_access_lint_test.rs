use std::path::PathBuf;

fn manifest_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(manifest_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

#[test]
fn startup_background_workers_use_app_state_db_service_paths() {
    let startup_worker_files = [
        "src/processor/embeddings.rs",
        "src/hygiene/loop_runner.rs",
        "src/proactive/scanner.rs",
        "src/prepare/email_enrich.rs",
        "src/linear/sync.rs",
    ];
    let forbidden_needles = [
        "ActionDb::open(",
        "ActionDb::open_readonly(",
        "crate::db::ActionDb::open(",
        "crate::db::ActionDb::open_readonly(",
    ];

    for relative in startup_worker_files {
        let source = read(relative);
        for needle in forbidden_needles {
            assert!(
                !source.contains(needle),
                "{relative} must route startup background DB work through AppState::db_read/db_write, not `{needle}`"
            );
        }
    }
}

#[test]
fn startup_sync_uses_async_db_service_path() {
    let source = read("src/state.rs");
    let start = source
        .find("pub async fn run_startup_sync")
        .expect("run_startup_sync function exists");
    let end = source[start..]
        .find("\n/// Recover from an unclean dev-mode exit")
        .map(|offset| start + offset)
        .expect("run_startup_sync end marker exists");
    let body = &source[start..end];

    assert!(
        body.contains(".db_write("),
        "run_startup_sync must use AppState::db_write"
    );
    assert!(
        !body.contains("ActionDb::open(") && !body.contains("crate::db::ActionDb::open("),
        "run_startup_sync must not open an independent ActionDb handle"
    );

    let lib = read("src/lib.rs");
    assert!(
        !lib.contains(
            "spawn_blocking(move || {\n                    crate::state::run_startup_sync"
        ),
        "run_startup_sync should be awaited on the async runtime so it can use DbService"
    );
}
