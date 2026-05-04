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

fn run_lint(root: &std::path::Path) -> (bool, String, String) {
    let script = repo_root().join("scripts/check_intelligence_disk_writes.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg(root)
        .output()
        .expect("run lint");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn intelligence_disk_write_lint_catches_production_direct_write() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src-tauri/src");
    std::fs::create_dir_all(&src).expect("mkdir src");
    std::fs::write(
        src.join("bad.rs"),
        r#"
fn persist_cache() {
    write_intelligence_json(dir, intel).unwrap();
}
"#,
    )
    .expect("write fixture");

    let (ok, stdout, stderr) = run_lint(tmp.path());
    assert!(
        !ok,
        "lint must fail on production direct write. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(
        stdout.contains(
            "intelligence.json disk writes must run only after the DB transaction commits"
        ),
        "lint should print rationale, got: {stdout}"
    );
}

#[test]
fn intelligence_disk_write_lint_allows_post_commit_helper() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src-tauri/src");
    std::fs::create_dir_all(&src).expect("mkdir src");
    std::fs::write(
        src.join("good.rs"),
        r#"
fn refresh_post_commit_cache() {
    fenced_write_intelligence_json(cycle, db, dir, intel).unwrap();
}
"#,
    )
    .expect("write fixture");

    let (ok, stdout, stderr) = run_lint(tmp.path());
    assert!(
        ok,
        "lint must allow post-commit helper. stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn intelligence_disk_write_lint_passes_current_tree() {
    let root = repo_root();
    let (ok, stdout, stderr) = run_lint(&root);
    assert!(
        ok,
        "scripts/check_intelligence_disk_writes.sh failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}
