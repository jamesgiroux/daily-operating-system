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
        let script =
            repo_root().join("src-tauri/scripts/check_no_live_external_clients_in_eval.sh");
        assert!(
            script.is_file(),
            "missing lint script: {}",
            script.display()
        );

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

    fn write_synthetic_rust_file(root: &Path, relative_path: &str, contents: &str) {
        let path = root.join(relative_path);
        std::fs::create_dir_all(path.parent().expect("synthetic file parent"))
            .expect("mkdir synthetic file parent");
        std::fs::write(path, contents).expect("write synthetic rust file");
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
        assert!(
            stdout.contains("bad_live_external_client.rs"),
            "stdout: {stdout}"
        );
        assert!(stdout.contains("reqwest"), "stdout: {stdout}");
    }

    #[test]
    fn lint_blocks_reqwest_alias_constructor_in_ability_code() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let contents = [
            "use reqwest::",
            "Client as HttpClient;\nfn bad() { let _client = HttpClient::",
            "builder(); }\n",
        ]
        .concat();
        write_synthetic_rust_file(
            tmp.path(),
            "src-tauri/src/abilities/bad_alias.rs",
            &contents,
        );

        let (ok, stdout, stderr) = run_lint(tmp.path());

        assert!(
            !ok,
            "lint must fail for aliased reqwest constructors. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("bad_alias.rs"), "stdout: {stdout}");
        assert!(
            stdout.contains("external HTTP client constructor"),
            "stdout: {stdout}"
        );
    }

    #[test]
    fn lint_blocks_replay_wrapper_constructor_outside_context_seam() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let contents = [
            "fn bad(fixture: std::sync::Arc<dyn Send + Sync>) { let _client = Replay",
            "GmailClient::",
            "new(fixture, \"tenant-test\"); }\n",
        ]
        .concat();
        write_synthetic_rust_file(
            tmp.path(),
            "src-tauri/tests/bad_replay_wrapper.rs",
            &contents,
        );

        let (ok, stdout, stderr) = run_lint(tmp.path());

        assert!(
            !ok,
            "lint must fail for replay wrapper constructors outside services::context. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("bad_replay_wrapper.rs"), "stdout: {stdout}");
        assert!(
            stdout.contains("ReplayGmailClient::new"),
            "stdout: {stdout}"
        );
    }

    #[test]
    fn lint_blocks_legacy_file_level_allow_comment() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let contents = [
            "// LINT-ALLOW: live-external-client (justification: legacy bypass)\n",
            "fn bad() { let _client = reqwest::",
            "Client::",
            "new(); }\n",
        ]
        .concat();
        write_synthetic_rust_file(tmp.path(), "src-tauri/tests/bad_legacy_allow.rs", &contents);

        let (ok, stdout, stderr) = run_lint(tmp.path());

        assert!(
            !ok,
            "lint must ignore legacy file-level bypass comments. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("bad_legacy_allow.rs"), "stdout: {stdout}");
        assert!(stdout.contains("reqwest::Client::new"), "stdout: {stdout}");
    }

    #[test]
    fn lint_blocks_tokio_net_import_in_ability_code() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let contents = [
            "use tokio::",
            "net::TcpStream;\nasync fn bad() { let _stream = TcpStream::",
            "connect(\"127.0.0.1:1\").await; }\n",
        ]
        .concat();
        write_synthetic_rust_file(
            tmp.path(),
            "src-tauri/src/abilities/bad_socket.rs",
            &contents,
        );

        let (ok, stdout, stderr) = run_lint(tmp.path());

        assert!(
            !ok,
            "lint must fail for raw async socket imports in ability code. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("bad_socket.rs"), "stdout: {stdout}");
        assert!(
            stdout.contains("tokio::net imported raw socket"),
            "stdout: {stdout}"
        );
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
