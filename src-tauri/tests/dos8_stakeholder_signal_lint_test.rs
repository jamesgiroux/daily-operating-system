use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    std::env::current_dir()
        .expect("cwd")
        .ancestors()
        .find(|p| p.join("src-tauri/Cargo.toml").exists())
        .expect("repo root with src-tauri/Cargo.toml")
        .to_path_buf()
}

fn run_lint(root: &std::path::Path) -> std::process::Output {
    Command::new("bash")
        .arg(repo_root().join("scripts/check_stakeholder_writer_emits_signal.sh"))
        .env("STAKEHOLDER_LINT_ROOT", root)
        .output()
        .expect("run lint")
}

#[test]
fn lint_stakeholder_writer_emits_signal_catches_missing_emission() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("bad_writer.rs"),
        r#"
fn bad(tx: &ActionDb) {
    tx.conn_ref()
        .execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)",
            params,
        )
        .unwrap();
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        !output.status.success(),
        "lint must fail when stakeholder writer omits stakeholders_changed signal. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_stakeholder_writer_allows_canonical_wrapper() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("good_writer.rs"),
        r#"
fn good(ctx: &ServiceContext<'_>, tx: &ActionDb) {
    crate::services::stakeholder_writer::write_with_stakeholders_changed(
        ctx,
        tx,
        "account",
        "acc-1",
        "fixture",
        |tx| {
            tx.conn_ref()
                .execute(
                    "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)",
                    params,
                )
                .unwrap();
            Ok(())
        },
    )
    .unwrap();
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        output.status.success(),
        "lint must pass for canonical stakeholder writer wrapper. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_stakeholder_writer_rejects_comment_only_wrapper_mention() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("comment_bypass.rs"),
        r#"
fn bad(tx: &ActionDb) {
    // crate::services::stakeholder_writer::write_with_stakeholders_changed(...)
    tx.conn_ref()
        .execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)",
            params,
        )
        .unwrap();
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        !output.status.success(),
        "lint must reject comment-only stakeholder writer mentions. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_stakeholder_writer_rejects_sibling_wrapper_bypass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("sibling_bypass.rs"),
        r#"
fn unrelated(ctx: &ServiceContext<'_>, tx: &ActionDb) {
    crate::services::stakeholder_writer::write_with_stakeholders_changed(
        ctx,
        tx,
        "account",
        "acc-1",
        "fixture",
        |_tx| Ok(()),
    )
    .unwrap();
}

fn bad(tx: &ActionDb) {
    tx.conn_ref()
        .execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)",
            params,
        )
        .unwrap();
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        !output.status.success(),
        "lint must reject wrapper calls outside the mutating function. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_stakeholder_writer_checks_cfg_test_modules() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("test_module_bypass.rs"),
        r#"
#[cfg(test)]
mod tests {
    fn bad_test_fixture(tx: &ActionDb) {
        tx.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)",
                params,
            )
            .unwrap();
    }
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        !output.status.success(),
        "lint must inspect cfg(test) modules. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn lint_stakeholder_writer_rejects_direct_stakeholders_changed_emit() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src-tauri/src/services");
    std::fs::create_dir_all(&src_dir).expect("mkdir fixture");
    std::fs::write(
        src_dir.join("bad_emit.rs"),
        r#"
fn bad_emit(ctx: &ServiceContext<'_>, tx: &ActionDb) {
    crate::services::signals::emit_in_transaction(
        ctx,
        tx,
        "account",
        "acc-1",
        crate::services::signals::STAKEHOLDERS_CHANGED_SIGNAL,
        "fixture",
        serde_json::json!({"entity_id":"acc-1","entity_type":"account","mutation_source":"fixture"}),
    )
    .unwrap();
}
"#,
    )
    .expect("write fixture");

    let output = run_lint(tmp.path());

    assert!(
        !output.status.success(),
        "lint must reject direct stakeholders_changed emit. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
