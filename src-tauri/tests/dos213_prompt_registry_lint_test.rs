use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn run_script(script: &str, root: &Path) -> (bool, String, String) {
    run_script_with_env(script, root, &[])
}

fn run_script_with_env(script: &str, root: &Path, envs: &[(&str, &str)]) -> (bool, String, String) {
    let mut command = Command::new("bash");
    command.arg(repo_root().join(script)).arg(root);
    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command.output().expect("run lint script");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn write_registry(root: &Path, template_body: &str, sha: &str) {
    let prompt_dir = root.join("src-tauri/src/abilities/prompts");
    std::fs::create_dir_all(&prompt_dir).expect("mkdir prompt dir");
    std::fs::write(
        prompt_dir.join("manifest.toml"),
        format!(
            r#"schema_version = 1

[[template]]
id = "fixture"
version = "1.0.0"
path = "fixture.v1.0.0.txt"
sha256 = "{sha}"
"#
        ),
    )
    .expect("write manifest");
    std::fs::write(prompt_dir.join("fixture.v1.0.0.txt"), template_body).expect("write template");
}

fn sha256_hex(bytes: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_bytes());
    hex::encode(hasher.finalize())
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_all(root: &Path, message: &str) {
    run_git(root, &["add", "-A"]);
    run_git(
        root,
        &[
            "-c",
            "user.name=DailyOS Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            message,
        ],
    );
}

#[test]
fn prompt_template_registry_lint_passes_current_tree() {
    let root = repo_root();
    let (ok, stdout, stderr) = run_script("scripts/check_prompt_template_registry.sh", &root);

    assert!(
        ok,
        "prompt template registry lint failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn prompt_template_registry_lint_blocks_hash_mismatch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_registry(
        tmp.path(),
        "Fixture prompt\n",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );

    let (ok, stdout, stderr) = run_script("scripts/check_prompt_template_registry.sh", tmp.path());

    assert!(
        !ok,
        "lint must fail for hash mismatch. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(stdout.contains("sha256 mismatch"), "stdout: {stdout}");
}

#[test]
fn prompt_template_registry_lint_blocks_existing_version_edits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let original_sha = "5a719b9095e28e9d12eb1e4f1ecd63393825a25666c5608c36f49af6cb5e0440";
    write_registry(tmp.path(), "Fixture prompt\n", original_sha);

    let output = Command::new("git")
        .arg("init")
        .current_dir(tmp.path())
        .output()
        .expect("git init");
    assert!(output.status.success(), "git init failed");
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    assert!(output.status.success(), "git add failed");
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=DailyOS Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "initial registry",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let changed_sha = "f1f7602673bf73ba3a674823e47770b54504a640619b503ec2954b752a29686b";
    write_registry(tmp.path(), "Fixture prompt changed\n", changed_sha);

    let (ok, stdout, stderr) = run_script("scripts/check_prompt_template_registry.sh", tmp.path());

    assert!(
        !ok,
        "lint must fail for existing version edit. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(
        stdout.contains("existing template version changed"),
        "stdout: {stdout}"
    );
}

#[test]
fn prompt_template_registry_lint_blocks_committed_existing_version_edits_against_base_ref() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let original_sha = "5a719b9095e28e9d12eb1e4f1ecd63393825a25666c5608c36f49af6cb5e0440";
    write_registry(tmp.path(), "Fixture prompt\n", original_sha);

    let output = Command::new("git")
        .arg("init")
        .current_dir(tmp.path())
        .output()
        .expect("git init");
    assert!(output.status.success(), "git init failed");
    let output = Command::new("git")
        .args(["checkout", "-b", "main"])
        .current_dir(tmp.path())
        .output()
        .expect("git checkout main");
    assert!(
        output.status.success(),
        "git checkout main failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .expect("git add");
    assert!(output.status.success(), "git add failed");
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=DailyOS Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "initial registry",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("git commit");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(tmp.path())
        .output()
        .expect("git checkout feature");
    assert!(
        output.status.success(),
        "git checkout feature failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let changed_sha = "f1f7602673bf73ba3a674823e47770b54504a640619b503ec2954b752a29686b";
    write_registry(tmp.path(), "Fixture prompt changed\n", changed_sha);
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .expect("git add changed registry");
    assert!(output.status.success(), "git add failed");
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=DailyOS Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "mutate existing registry version",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("git commit changed registry");
    assert!(
        output.status.success(),
        "git commit changed registry failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let (ok, stdout, stderr) = run_script_with_env(
        "scripts/check_prompt_template_registry.sh",
        tmp.path(),
        &[("BASE_REF", "main")],
    );

    assert!(
        !ok,
        "lint must fail for a committed existing-version edit. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(
        stdout.contains("existing template version changed"),
        "stdout: {stdout}"
    );
}

#[test]
fn prompt_registry_lint_uses_origin_dev_merge_base_for_committed_edits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let original_body = "Fixture prompt\n";
    write_registry(tmp.path(), original_body, &sha256_hex(original_body));

    run_git(tmp.path(), &["init"]);
    run_git(tmp.path(), &["checkout", "-b", "dev"]);
    commit_all(tmp.path(), "initial registry");

    run_git(tmp.path(), &["checkout", "-b", "feature"]);
    let changed_body = "Fixture prompt changed in feature\n";
    write_registry(tmp.path(), changed_body, &sha256_hex(changed_body));
    commit_all(tmp.path(), "mutate existing registry version");

    // Advance origin/dev away from the feature branch and remove the original
    // file there. A tip-vs-HEAD comparison would miss the mutation because the
    // file no longer exists at origin/dev; merge-base(origin/dev, HEAD) still
    // has the immutable template and must catch the committed edit.
    run_git(tmp.path(), &["checkout", "dev"]);
    run_git(
        tmp.path(),
        &["rm", "src-tauri/src/abilities/prompts/fixture.v1.0.0.txt"],
    );
    commit_all(tmp.path(), "remove registry template on dev");
    run_git(
        tmp.path(),
        &["update-ref", "refs/remotes/origin/dev", "HEAD"],
    );
    run_git(tmp.path(), &["checkout", "feature"]);

    let (ok, stdout, stderr) = run_script_with_env(
        "scripts/check_prompt_template_registry.sh",
        tmp.path(),
        &[("BASE_REF", "origin/dev")],
    );

    assert!(
        !ok,
        "lint must fail for committed existing-version edit against origin/dev merge-base. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(
        stdout.contains("existing template version changed"),
        "stdout: {stdout}"
    );
}

#[test]
fn prompt_fingerprint_boundary_lint_passes_current_tree() {
    let root = repo_root();
    let (ok, stdout, stderr) = run_script("scripts/check_prompt_fingerprint_boundary.sh", &root);

    assert!(
        ok,
        "prompt fingerprint boundary lint failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn prompt_fingerprint_boundary_lint_blocks_direct_hash_calls() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("src-tauri/src/abilities");
    std::fs::create_dir_all(&src).expect("mkdir abilities dir");
    std::fs::write(
        src.join("bad.rs"),
        r#"
fn bad(request: crate::intelligence::provider::CanonicalPromptRequest<'_>) {
    let _hash = canonical_prompt_hash(request);
}
"#,
    )
    .expect("write bad file");

    let (ok, stdout, stderr) =
        run_script("scripts/check_prompt_fingerprint_boundary.sh", tmp.path());

    assert!(
        !ok,
        "lint must fail for direct hash call. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(stdout.contains("canonical_prompt_hash"), "stdout: {stdout}");
}
