mod dos383_external_replay_lint_test {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .to_path_buf()
    }

    fn run_lint(current_dir: &Path) -> (bool, String, String) {
        let script = repo_root().join("src-tauri/scripts/check_no_live_external_clients_in_eval.sh");
        assert!(script.is_file(), "missing lint script: {}", script.display());

        let output = Command::new("bash")
            .arg(script)
            .current_dir(current_dir)
            .output()
            .expect("run DOS-383 external replay lint");

        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
    }

    #[test]
    fn lint_blocks_live_reqwest_client_in_test_crate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tests_dir = tmp.path().join("src-tauri/tests");
        std::fs::create_dir_all(&tests_dir).expect("mkdir tests");

        let ctor = ["reqwest::Client::", "new()"].concat();
        std::fs::write(
            tests_dir.join("bad_live_external_client.rs"),
            format!("fn bad() {{ let _client = {ctor}; }}\n"),
        )
        .expect("write synthetic violation");

        let (ok, stdout, stderr) = run_lint(tmp.path());

        assert!(
            !ok,
            "lint must fail for a live HTTP client constructor. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("bad_live_external_client.rs"), "stdout: {stdout}");
        assert!(stdout.contains("reqwest"), "stdout: {stdout}");
    }

    #[test]
    fn lint_passes_when_test_crate_uses_replay_clients_only() {
        let root = repo_root();
        let (ok, stdout, stderr) = run_lint(&root);

        assert!(
            ok,
            "DOS-383 external replay lint failed on current tree:\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
}
