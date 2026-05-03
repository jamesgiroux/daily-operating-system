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

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/check_stakeholder_writer_emits_signal.sh"))
        .env("STAKEHOLDER_LINT_ROOT", tmp.path())
        .output()
        .expect("run lint");

    assert!(
        !output.status.success(),
        "lint must fail when stakeholder writer omits stakeholders_changed signal. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
